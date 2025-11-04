use escpos::{driver::SerialPortDriver, printer::Printer, utils::*};
use image::{ImageBuffer, Rgb, RgbImage, GrayImage, Luma};
use imageproc::drawing::{draw_text_mut, text_size};
use ab_glyph::{Font, FontRef, PxScale};
use ar_reshaper::reshape_line;
use serde::Deserialize;

// ================================================================
// Arabic receipt (bitmap via ESC * 24-dot) — RTL, crisp (NCR 7197)
// Frontend provides data via `print_receipt(...)` (Tauri command).
// ================================================================

const DEFAULT_COM_PORT: &str = "COM7";
const DEFAULT_BAUD_RATE: u32 = 9600;

fn get_com_port() -> String {
    std::env::var("PRINTER_COM_PORT").unwrap_or_else(|_| DEFAULT_COM_PORT.to_string())
}
fn get_baud_rate() -> u32 {
    std::env::var("PRINTER_BAUD_RATE").ok().and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_BAUD_RATE)
}
fn normalize_com_port(port: &str) -> String {
    #[cfg(windows)]
    {
        let up = port.to_uppercase();
        if up.starts_with("COM") {
            if let Ok(n) = up[3..].parse::<u32>() {
                if n > 9 { return format!("\\\\.\\{}", up); }
            }
        }
        port.to_string()
    }
    #[cfg(not(windows))] { port.to_string() }
}

// ---------------- Data & Layout ----------------

#[derive(Clone, Deserialize)]
struct Item { name: String, qty: f32, price: f32 }
impl Item { fn value(&self) -> f32 { self.qty * self.price } }

#[derive(Clone, Deserialize)]
struct ReceiptData {
    store_name: String,
    date_time_line: String,
    invoice_no: String,
    items: Vec<Item>,
    discount: f32,
    footer_address: String,
    footer_delivery: String,
    footer_phones: String,
}

#[derive(Clone, Deserialize)]
struct Layout {
    paper_width_px: u32,
    threshold: u8,
    margin_h: i32,
    margin_top: i32,
    margin_bottom: i32,
    row_gap: i32,
    fonts: Fonts,
    // RTL columns as percentages of inner width: [name, qty, price, total]
    cols: [f32; 4],
}
#[derive(Clone, Deserialize)]
struct Fonts {
    title: f32,
    header_dt: f32,
    header_no: f32,
    header_cols: f32,
    item: f32,
    total_label: f32,
    total_value: f32,
    footer: f32,
    footer_phones: f32,
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            paper_width_px: 576,
            threshold: 150,
            margin_h: 1,
            margin_top: -18,
            margin_bottom: 0,
            row_gap: 36,
            fonts: Fonts {
                title: 84.0,
                header_dt: 40.0,
                header_no: 46.0,
                header_cols: 42.0,
                item: 44.0,
                total_label: 48.0,
                total_value: 66.0,
                footer: 44.0,
                footer_phones: 56.0,
            },
            cols: [0.60, 0.16, 0.14, 0.10],
        }
    }
}

// ---------- Frontend payload types (from invoke) ----------

#[derive(Deserialize, Clone)]
struct FrontendItem { name: String, qty: f32, price: f32 }

#[derive(Deserialize, Clone)]
struct FrontendFooter {
    address: String,
    #[serde(alias = "last line", alias = "lastLine")]
    last_line: String,
    #[serde(default)]
    phones: Option<String>,
}

// ---------------- Arabic shaping + drawing ----------------

fn shape(s: &str) -> String { reshape_line(s) }
fn draw_crisp(img: &mut RgbImage, s: &str, x: i32, y: i32, scale: PxScale, font: &FontRef) {
    draw_text_mut(img, Rgb([0,0,0]), x, y, scale, font, s);
}
fn is_ltr_char(c: char) -> bool {
    c.is_ascii()
        || ('\u{0660}'..='\u{0669}').contains(&c)
        || ('\u{06F0}'..='\u{06F9}').contains(&c)
        || ":./-–—,".contains(c)
}
fn draw_mixed_rtl_right(img: &mut RgbImage, font: &FontRef, scale: PxScale, logical: &str, x_right: i32, y: i32) {
    let shaped = shape(logical);
    let mut runs: Vec<(bool, String, i32)> = Vec::new();
    let mut cur = String::new();
    let mut cur_is_ltr = None::<bool>;

    for ch in shaped.chars() {
        let ltr = is_ltr_char(ch);
        match cur_is_ltr {
            None => { cur_is_ltr = Some(ltr); cur.push(ch); }
            Some(kind) if kind == ltr => cur.push(ch),
            Some(_) => {
                let w = if cur_is_ltr.unwrap() { text_size(scale, font, &cur).0 as i32 }
                        else { cur.chars().map(|c| text_size(scale, font, &c.to_string()).0 as i32).sum() };
                runs.push((cur_is_ltr.unwrap(), cur.clone(), w));
                cur.clear(); cur_is_ltr = Some(ltr); cur.push(ch);
            }
        }
    }
    if !cur.is_empty() {
        let w = if cur_is_ltr.unwrap() { text_size(scale, font, &cur).0 as i32 }
                else { cur.chars().map(|c| text_size(scale, font, &c.to_string()).0 as i32).sum() };
        runs.push((cur_is_ltr.unwrap(), cur.clone(), w));
    }

    let total_w: i32 = runs.iter().map(|r| r.2).sum();
    let mut right = x_right;

    for (is_ltr, seg, w) in runs.into_iter() {
        let start_x = right - w;
        if is_ltr {
            draw_ltr_right(img, font, scale, &seg, right, y);
        } else {
            let chars: Vec<char> = seg.chars().collect();
            let cw: Vec<i32> = chars.iter().map(|c| text_size(scale, font, &c.to_string()).0 as i32).collect();
            let mut x = start_x;
            for i in (0..chars.len()).rev() {
                draw_crisp(img, &chars[i].to_string(), x, y, scale, font);
                x += cw[i];
            }
        }
        right -= w;
        if right < x_right - total_w { break; }
    }
}
fn draw_mixed_rtl_center(img: &mut RgbImage, font: &FontRef, scale: PxScale, logical: &str, paper_w: i32, y: i32) {
    let shaped = shape(logical);
    let mut runs: Vec<(bool, String, i32)> = Vec::new();
    let mut cur = String::new();
    let mut cur_is_ltr = None::<bool>;
    for ch in shaped.chars() {
        let ltr = is_ltr_char(ch);
        match cur_is_ltr {
            None => { cur_is_ltr = Some(ltr); cur.push(ch); }
            Some(kind) if kind == ltr => cur.push(ch),
            Some(_) => {
                let w = if cur_is_ltr.unwrap() { text_size(scale, font, &cur).0 as i32 }
                        else { cur.chars().map(|c| text_size(scale, font, &c.to_string()).0 as i32).sum() };
                runs.push((cur_is_ltr.unwrap(), cur.clone(), w));
                cur.clear(); cur_is_ltr = Some(ltr); cur.push(ch);
            }
        }
    }
    if !cur.is_empty() {
        let w = if cur_is_ltr.unwrap() { text_size(scale, font, &cur).0 as i32 }
                else { cur.chars().map(|c| text_size(scale, font, &c.to_string()).0 as i32).sum() };
        runs.push((cur_is_ltr.unwrap(), cur.clone(), w));
    }
    let total_w: i32 = runs.iter().map(|r| r.2).sum();
    let mut right = (paper_w + total_w) / 2;
    for (is_ltr, seg, w) in runs.into_iter() {
        let start_x = right - w;
        if is_ltr {
            draw_ltr_right(img, font, scale, &seg, right, y);
        } else {
            let chars: Vec<char> = seg.chars().collect();
            let cw: Vec<i32> = chars.iter().map(|c| text_size(scale, font, &c.to_string()).0 as i32).collect();
            let mut x = start_x;
            for i in (0..chars.len()).rev() {
                draw_crisp(img, &chars[i].to_string(), x, y, scale, font);
                x += cw[i];
            }
        }
        right -= w;
    }
}
fn draw_ltr_right(img: &mut RgbImage, font: &FontRef, scale: PxScale, s: &str, x_right: i32, y: i32) {
    let (w, _) = text_size(scale, font, s);
    draw_crisp(img, s, x_right - w as i32, y, scale, font);
}
fn draw_ltr_center(img: &mut RgbImage, font: &FontRef, scale: PxScale, s: &str, paper_w: i32, y: i32) {
    let (w, _) = text_size(scale, font, s);
    let x = (paper_w - w as i32) / 2;
    draw_crisp(img, s, x, y, scale, font);
}
fn draw_dotted(img: &mut RgbImage, y: i32, left: i32, right: i32) {
    let y = y.max(0) as u32;
    let mut x = left.max(0);
    while x < right {
        for dx in 0..3 {
            if x + dx < right { img.put_pixel((x + dx) as u32, y, Rgb([0,0,0])); }
        }
        x += 10;
    }
}

// ---------------- Rendering ----------------

fn render_receipt(data: &ReceiptData, layout: &Layout) -> GrayImage {
    let paper_w = layout.paper_width_px as i32;
    let mut img: RgbImage = ImageBuffer::from_pixel(layout.paper_width_px, 1800, Rgb([255,255,255]));
    let margin_h = layout.margin_h;
    let inner_w = paper_w - margin_h*2;
    let right_edge = margin_h + inner_w;
    let mut y = layout.margin_top;

    let font_bytes = include_bytes!("../fonts/NotoSansArabic-Regular.ttf");
    let font = FontRef::try_from_slice(font_bytes).expect("font");

    // Title (remove top whitespace)
    let title_scale = PxScale::from(layout.fonts.title);
    let ascent = font.v_metrics(title_scale).ascent;
    let y_title = y - ascent.ceil() as i32;
    draw_mixed_rtl_center(&mut img, &font, title_scale, &data.store_name, paper_w, y_title);
    y = y_title + layout.fonts.title as i32 - 8;

    // Date/Time
    draw_mixed_rtl_center(&mut img, &font, PxScale::from(layout.fonts.header_dt), &data.date_time_line, paper_w, y);
    y += layout.fonts.header_dt as i32 + 2;

    // Receipt number
    draw_ltr_center(&mut img, &font, PxScale::from(layout.fonts.header_no), &data.invoice_no, paper_w, y);
    y += layout.fonts.header_no as i32 + 2;

    // Columns (RTL)
    let w_name  = (inner_w as f32 * layout.cols[0]) as i32;
    let w_qty   = (inner_w as f32 * layout.cols[1]) as i32;
    let w_price = (inner_w as f32 * layout.cols[2]) as i32;
    let w_total = (inner_w as f32 * layout.cols[3]) as i32;

    let r_name  = right_edge;
    let r_qty   = r_name  - w_name;
    let r_price = r_qty   - w_qty;
    let r_total = r_price - w_price;

    let s_head = PxScale::from(layout.fonts.header_cols);
    draw_mixed_rtl_right(&mut img, &font, s_head, "الصنف",  r_name,  y);
    draw_mixed_rtl_right(&mut img, &font, s_head, "الكمية", r_qty,   y);
    draw_mixed_rtl_right(&mut img, &font, s_head, "السعر",  r_price, y);
    draw_mixed_rtl_right(&mut img, &font, s_head, "القيمة", r_total, y);
    y += layout.row_gap - 6;

    // Items
    let s_item = PxScale::from(layout.fonts.item);
    for it in &data.items {
        draw_mixed_rtl_right(&mut img, &font, s_item, &it.name,                 r_name,  y);
        draw_ltr_right(&mut img,      &font, s_item, &format!("{:.2}", it.qty), r_qty,   y);
        draw_ltr_right(&mut img,      &font, s_item, &format!("{:.2}", it.price), r_price, y);
        draw_ltr_right(&mut img,      &font, s_item, &format!("{:.2}", it.value()), r_total, y);
        y += layout.row_gap;
    }

    // Dotted line
    y += 18;
    draw_dotted(&mut img, y, margin_h, paper_w - margin_h);
    y += 12;

    // Discount (optional)
    if data.discount > 0.0001 {
        let gap = 12;
        let label = "الخصم";
        let (lw, _) = text_size(PxScale::from(layout.fonts.total_label), &font, &shape(label));
        let right = right_edge;
        draw_ltr_right(&mut img, &font, PxScale::from(layout.fonts.total_label),
                       &format!("{:.2}", data.discount), right - lw as i32 - gap, y);
        draw_mixed_rtl_right(&mut img, &font, PxScale::from(layout.fonts.total_label), label, right, y);
        y += layout.row_gap - 6;
    }

    // Total
    let gap = 12;
    let label = "إجمالي الفاتورة";
    let (lw, _) = text_size(PxScale::from(layout.fonts.total_label), &font, &shape(label));
    let right = right_edge;
    let total: f32 = data.items.iter().map(|i| i.value()).sum::<f32>() - data.discount;

    draw_ltr_right(&mut img, &font, PxScale::from(layout.fonts.total_value),
                   &format!("{:.2}", total), right - lw as i32 - gap, y - 10);
    draw_mixed_rtl_right(&mut img, &font, PxScale::from(layout.fonts.total_label), label, right, y);
    y += layout.row_gap;

    // Footer
    draw_mixed_rtl_center(&mut img, &font, PxScale::from(layout.fonts.footer), &data.footer_address,  paper_w, y);
    y += layout.fonts.footer as i32 + 2;

    draw_mixed_rtl_center(&mut img, &font, PxScale::from(layout.fonts.footer), &data.footer_delivery, paper_w, y);
    y += layout.fonts.footer as i32 + 2;

    if !data.footer_phones.is_empty() {
        draw_ltr_center(&mut img, &font, PxScale::from(layout.fonts.footer_phones), &data.footer_phones, paper_w, y);
    }
    y += layout.margin_bottom;

    // Crop & grayscale
    let used_h = (y as u32).min(1798);
    image::DynamicImage::ImageRgb8(img)
        .crop_imm(0, 0, layout.paper_width_px, used_h)
        .to_luma8()
}

// Pack ESC * 24-dot bands (m=33)
fn pack_esc_star_24(gray: &GrayImage, y0: u32, threshold: u8) -> Vec<u8> {
    let w = gray.width();
    let h = gray.height();
    let mut band = Vec::with_capacity((w * 3) as usize);
    for x in 0..w {
        for byte in 0..3 {
            let mut b = 0u8;
            for bit in 0..8 {
                let yy = y0 + (byte * 8 + bit) as u32;
                if yy < h {
                    let Luma([pix]) = *gray.get_pixel(x, yy);
                    if pix <= threshold { b |= 1 << (7 - bit); }
                }
            }
            band.push(b);
        }
    }
    band
}

// ---------------- Tauri Command (only this is exposed) ----------------

#[tauri::command]
async fn print_receipt(
    title: String,
    time: String,
    number: String,
    items: Vec<FrontendItem>,
    total: Option<f32>,     // accepted but ignored (we recompute)
    discount: Option<f32>,
    footer: FrontendFooter,
) -> Result<String, String> {
    let mapped_items: Vec<Item> = items.into_iter()
        .map(|i| Item { name: i.name, qty: i.qty, price: i.price })
        .collect();

    let data = ReceiptData {
        store_name: title,
        date_time_line: time,
        invoice_no: number,
        items: mapped_items,
        discount: discount.unwrap_or(0.0),
        footer_address: footer.address,
        footer_delivery: footer.last_line,
        footer_phones: footer.phones.unwrap_or_default(),
    };

    let layout = Layout::default();

    let port = normalize_com_port(&get_com_port());
    let baud = get_baud_rate();
    let driver = SerialPortDriver::open(&port, baud, None)
        .map_err(|e| format!("open {} @{}: {}", port, baud, e))?;

    let mut obj = Printer::new(driver, Protocol::default(), None);
    obj.debug_mode(None);
    let mut p = obj.init().map_err(|e| e.to_string())?;

    let gray = render_receipt(&data, &layout);

    // ESC * 24-dot double density
    let w = gray.width();
    let n = w as u16;
    let nL = (n & 0xFF) as u8;
    let nH = ((n >> 8) & 0xFF) as u8;

    let mut y0 = 0u32;
    while y0 < gray.height() {
        let band = pack_esc_star_24(&gray, y0, layout.threshold);
        p = p.custom(&[0x1B, 0x2A, 33, nL, nH]).map_err(|e| e.to_string())?;
        p = p.custom(&band).map_err(|e| e.to_string())?;
        p = p.custom(&[0x0A]).map_err(|e| e.to_string())?;
        y0 += 24;
    }

    // feed & cut
    p = p.custom(&[0x0A]).map_err(|e| e.to_string())?;
    p = p.print_cut().map_err(|e| e.to_string())?;
    p.print().map_err(|e| e.to_string())?;
    Ok(format!("✅ Receipt printed on {}", port))
}

// ---------------- App entry ----------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ print_receipt ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
