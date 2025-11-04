use escpos::{driver::SerialPortDriver, printer::Printer, utils::*};
use image::{ImageBuffer, Rgb, RgbImage, GrayImage, Luma};
use imageproc::drawing::{draw_text_mut, text_size};
use ab_glyph::{FontRef, PxScale};
use ar_reshaper::reshape_line;
use serde::Deserialize;

// ================================================================
// Arabic receipt (bitmap via ESC * 24-dot) — Crisp, RTL, 2-column
// NCR 7197 compatible
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
    store_name: String,       // supports "\n" for manual 2-line title
    date_time_line: String,   // e.g. "٤ نوفمبر - ٩:٠٤ صباحا"
    invoice_no: String,       // "123456"
    items: Vec<Item>,
    discount: f32,
    footer_address: String,
    footer_delivery: String,  // "خدمة توصيل للمنازل 24 ساعة"
    footer_phones: String,    // "01533333161 - 01533333262"
}

#[derive(Clone, Deserialize)]
struct Layout {
    paper_width_px: u32,
    threshold: u8,
    margin_h: i32,      // horizontal margin
    margin_top: i32,    // smaller top space
    margin_bottom: i32, // smaller bottom space
    row_gap: i32,       // item vertical gap
    fonts: Fonts,
    // Two-column (RTL): [name %, price %] must sum <= 1.0
    cols2: [f32; 2],
}
#[derive(Clone, Deserialize)]
struct Fonts {
    title: f32,         // store title (each line)
    header_dt: f32,     // date/time line
    header_no: f32,     // number
    header_cols: f32,   // "الصنف" | "السعر"
    item: f32,          // item row
    total_label: f32,   // "إجمالي الفاتورة"
    total_value: f32,   // amount
    footer: f32,        // address + delivery
    footer_phones: f32, // phones (larger)
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            paper_width_px: 576,
            threshold: 150, // crisper edges
            margin_h: 18,
            margin_top: 2,       // tighter top
            margin_bottom: 6,    // tighter bottom
            row_gap: 44,         // closer rows
            fonts: Fonts {
                title: 84.0,
                header_dt: 40.0,
                header_no: 46.0,
                header_cols: 42.0,
                item: 44.0,
                total_label: 42.0,
                total_value: 66.0,
                footer: 44.0,
                footer_phones: 52.0, // bigger phones
            },
            // more space for the product name (right), small for price (left)
            cols2: [0.82, 0.18],
        }
    }
}

// ---------------- Arabic shaping + drawing ----------------

fn shape(s: &str) -> String { reshape_line(s) }

// single-pass draw (crisp)
fn draw_crisp(img: &mut RgbImage, s: &str, x: i32, y: i32, scale: PxScale, font: &FontRef) {
    draw_text_mut(img, Rgb([0,0,0]), x, y, scale, font, s);
}

// RTL right-aligned (Arabic)
fn draw_rtl_right(img: &mut RgbImage, font: &FontRef, scale: PxScale, logical: &str, x_right: i32, y: i32) {
    let shaped = shape(logical);
    let chars: Vec<char> = shaped.chars().collect();
    let mut widths = Vec::with_capacity(chars.len());
    for &ch in &chars {
        let (w, _) = text_size(scale, font, &ch.to_string());
        widths.push(w as i32);
    }
    let total_w: i32 = widths.iter().sum();
    let mut x = x_right - total_w;
    for i in (0..chars.len()).rev() {
        draw_crisp(img, &chars[i].to_string(), x, y, scale, font);
        x += widths[i];
    }
}

// RTL centered
fn draw_rtl_center(img: &mut RgbImage, font: &FontRef, scale: PxScale, logical: &str, paper_w: i32, y: i32) {
    let shaped = shape(logical);
    let chars: Vec<char> = shaped.chars().collect();
    let mut widths = Vec::with_capacity(chars.len());
    for &ch in &chars {
        let (w, _) = text_size(scale, font, &ch.to_string());
        widths.push(w as i32);
    }
    let total_w: i32 = widths.iter().sum();
    let mut x = (paper_w - total_w) / 2;
    for i in (0..chars.len()).rev() {
        draw_crisp(img, &chars[i].to_string(), x, y, scale, font);
        x += widths[i];
    }
}

// LTR right-aligned (numbers/latin)
fn draw_ltr_right(img: &mut RgbImage, font: &FontRef, scale: PxScale, s: &str, x_right: i32, y: i32) {
    let (w, _) = text_size(scale, font, s);
    draw_crisp(img, s, x_right - w as i32, y, scale, font);
}

// LTR centered (e.g., number line)
fn draw_ltr_center(img: &mut RgbImage, font: &FontRef, scale: PxScale, s: &str, paper_w: i32, y: i32) {
    let (w, _) = text_size(scale, font, s);
    let x = (paper_w - w as i32) / 2;
    draw_crisp(img, s, x, y, scale, font);
}

// Centered title with optional auto 2-line wrap by width
fn draw_title_two_lines(img: &mut RgbImage, font: &FontRef, scale: PxScale, title: &str, paper_w: i32, y: &mut i32) {
    let max_w = (paper_w as f32 * 0.92) as i32;
    let lines: Vec<String> = if title.contains('\n') {
        title.split('\n').map(|s| s.trim().to_string()).collect()
    } else {
        // simple greedy wrap by words to at most 2 lines
        let words: Vec<&str> = title.split_whitespace().collect();
        let mut l1 = String::new();
        let mut l2 = String::new();
        for w in words {
            let test = if l1.is_empty() { w.to_string() } else { format!("{l1} {w}") };
            let (_, h) = text_size(scale, font, &shape(&test));
            let (tw, _) = text_size(scale, font, &shape(&test));
            if tw as i32 <= max_w || l1.is_empty() {
                l1 = test;
            } else {
                if l2.is_empty() { l2.push_str(w) } else { l2.push_str(&format!(" {w}")); }
            }
        }
        if l2.is_empty() { vec![l1] } else { vec![l1, l2] }
    };

    for line in lines {
        draw_rtl_center(img, font, scale, &line, paper_w, *y);
        *y += scale.y as i32 + 10; // line gap
    }
}

// ---------------- Rendering ----------------

fn render_receipt(data: &ReceiptData, layout: &Layout) -> GrayImage {
    let paper_w = layout.paper_width_px as i32;
    let mut img: RgbImage = ImageBuffer::from_pixel(layout.paper_width_px, 1600, Rgb([255,255,255]));
    let margin_h = layout.margin_h;
    let inner_w = paper_w - margin_h*2;
    let right_edge = margin_h + inner_w;
    let left_edge  = margin_h;
    let mut y = layout.margin_top; // tight top

    // Use your preferred Arabic font here
    let font_bytes = include_bytes!("../fonts/NotoSansArabic-Regular.ttf");
    let font = FontRef::try_from_slice(font_bytes).expect("font");

    // === Title (supports 2 lines) ===
    draw_title_two_lines(&mut img, &font, PxScale::from(layout.fonts.title), &data.store_name, paper_w, &mut y);

    // === Date/Time (no labels) ===
    draw_rtl_center(&mut img, &font, PxScale::from(layout.fonts.header_dt), &data.date_time_line, paper_w, y);
    y += layout.fonts.header_dt as i32 + 6;

    // === Receipt number (no label) ===
    draw_ltr_center(&mut img, &font, PxScale::from(layout.fonts.header_no), &data.invoice_no, paper_w, y);
    y += layout.fonts.header_no as i32 + 14;

    // === Column headers (RTL) : الصنف | السعر ===
    let s_head = PxScale::from(layout.fonts.header_cols);
    let col_name_r  = right_edge; // name at far right
    let col_price_r = left_edge + (inner_w as f32 * layout.cols2[0]) as i32; // price edge
    draw_rtl_right(&mut img, &font, s_head, "الصنف",  col_name_r,  y);
    draw_rtl_right(&mut img, &font, s_head, "السعر",  col_price_r, y);
    y += layout.row_gap - 6;

    // === Items (name + price only; price shows total per line) ===
    let s_item = PxScale::from(layout.fonts.item);
    for it in &data.items {
        let line_total = it.value();
        draw_rtl_right(&mut img, &font, s_item, &it.name,              col_name_r,  y);
        draw_ltr_right(&mut img, &font, s_item, &format!("{:.2}", line_total), col_price_r, y);
        y += layout.row_gap;
    }

    // === Discount (only if > 0) ===
    if data.discount > 0.0001 {
        draw_rtl_right(&mut img, &font, s_head, "الخصم", col_name_r, y);
        draw_ltr_right(&mut img, &font, s_head, &format!("{:.2}", data.discount), col_price_r, y);
        y += layout.row_gap;
    }

    // === Total ===
    let total: f32 = data.items.iter().map(|i| i.value()).sum::<f32>() - data.discount;
    draw_rtl_right(&mut img, &font, PxScale::from(layout.fonts.total_label), "إجمالي الفاتورة", col_name_r, y);
    draw_ltr_right(&mut img, &font, PxScale::from(layout.fonts.total_value), &format!("{:.2}", total), col_price_r, y - 10);
    y += layout.row_gap + 12;

    // === Footer (centered, larger; includes "24" explicitly) ===
    draw_rtl_center(&mut img, &font, PxScale::from(layout.fonts.footer), &data.footer_address, paper_w, y);
    y += layout.fonts.footer as i32 + 6;

    draw_rtl_center(&mut img, &font, PxScale::from(layout.fonts.footer), &data.footer_delivery, paper_w, y);
    y += layout.fonts.footer as i32 + 6;

    draw_ltr_center(&mut img, &font, PxScale::from(layout.fonts.footer_phones), &data.footer_phones, paper_w, y);
    y += layout.fonts.footer_phones as i32 + layout.margin_bottom;

    // Crop and grayscale
    let used_h = (y as u32).min(1598);
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

// ---------------- Tauri Commands ----------------

#[tauri::command]
async fn print_receipt() -> Result<String, String> {
    print_receipt_sample().await
}

#[tauri::command]
async fn print_receipt_sample() -> Result<String, String> {
    let data = ReceiptData {
        store_name: "اسواق ابو عمر".into(),
        date_time_line: "٤ نوفمبر - ٩:٠٤ صباحا".into(),
        invoice_no: "123456".into(),
        items: vec![
            Item { name: "تفاح عرض".into(), qty: 0.96, price: 70.00 },
            Item { name: "تفاح".into(),     qty: 1.95, price: 30.00 },
            Item { name: "خيار".into(),     qty: 1.02, price: 25.00 },
            Item { name: "ليمون بلدي".into(), qty: 0.44, price: 30.00 },
            Item { name: "بطاطا".into(),    qty: 2.16, price: 20.00 },
            Item { name: "ربطة جرجير".into(), qty: 4.00, price: 3.00 },
            Item { name: "نعناع فريش".into(), qty: 1.00, price: 5.00 },
        ],
        discount: 0.0,
        footer_address: "دمياط الجديدة - المركزية - مقابل البنك الأهلي القديم".into(),
        footer_delivery: "خدمة توصيل للمنازل 24 ساعة".into(),
        footer_phones: "01533333161 - 01533333262".into(),
    };
    let layout = Layout::default();
    do_print(&data, &layout).await
}

#[tauri::command]
async fn print_receipt_json(data_json: String, layout_json: Option<String>) -> Result<String, String> {
    let data: ReceiptData = serde_json::from_str(&data_json).map_err(|e| format!("data JSON: {e}"))?;
    let layout: Layout = match layout_json {
        Some(s) => serde_json::from_str(&s).map_err(|e| format!("layout JSON: {e}"))?,
        None => Layout::default(),
    };
    do_print(&data, &layout).await
}

async fn do_print(data: &ReceiptData, layout: &Layout) -> Result<String, String> {
    let port = normalize_com_port(&get_com_port());
    let baud = get_baud_rate();
    let driver = SerialPortDriver::open(&port, baud, None)
        .map_err(|e| format!("open {} @{}: {}", port, baud, e))?;

    let mut obj = Printer::new(driver, Protocol::default(), None);
    obj.debug_mode(None);
    let mut p = obj.init().map_err(|e| e.to_string())?;

    let gray = render_receipt(data, layout);

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

    // minimal feed then cut (less bottom whitespace)
    p = p.custom(&[0x0A]).map_err(|e| e.to_string())?;
    p = p.print_cut().map_err(|e| e.to_string())?;
    p.print().map_err(|e| e.to_string())?;
    Ok(format!("✅ Receipt printed on {}", port))
}

// ---------------- App entry ----------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            print_receipt,
            print_receipt_sample,
            print_receipt_json
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}