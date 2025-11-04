use escpos::{driver::SerialPortDriver, printer::Printer, utils::*};
use image::{ImageBuffer, Rgb, RgbImage, GrayImage, Luma};
use imageproc::drawing::{draw_text_mut, text_size};
use ab_glyph::{FontRef, PxScale};
use ar_reshaper::reshape_line;
use serde::Deserialize;

// ================================================================
// Arabic receipt (bitmap via ESC * 24-dot) — Crisp, larger, RTL
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
    store_name: String,
    // kept for JSON API compatibility (not rendered now)
    customer_type: String,
    date: String,
    time: String,
    invoice_no: String,
    cashier_name: String,
    payment_label: String,
    payment_amount: String,
    items: Vec<Item>,
    discount: f32,
    footer_address: String,
    footer_phones: String,
}

#[derive(Clone, Deserialize)]
struct Layout {
    paper_width_px: u32,
    threshold: u8,
    margin: i32,
    row_gap: i32,        // vertical gap between item rows
    fonts: Fonts,
    // Column widths as percentages (sum <= 1.0): [name, qty, price, value] (RTL)
    cols: [f32; 4],
}
#[derive(Clone, Deserialize)]
struct Fonts {
    title: f32,
    header_dt: f32,   // date/time + number
    header_no: f32,   // receipt number line
    header_cols: f32, // labels: الصنف | الكمية | السعر | قيمة
    item: f32,
    total_label: f32,
    total_value: f32,
    footer: f32,
    footer_small: f32,
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            paper_width_px: 576,
            threshold: 150,  // lower = crisper edges
            margin: 20,
            row_gap: 56,
            fonts: Fonts {
                title: 84.0,
                header_dt: 40.0,
                header_no: 46.0,
                header_cols: 42.0,
                item: 44.0,
                total_label: 42.0,
                total_value: 66.0,
                footer: 40.0,
                footer_small: 34.0,
            },
            // RTL: name (right) 52%, qty 12%, price 16%, value 20% (left)
            cols: [0.52, 0.12, 0.16, 0.20],
        }
    }
}

// ---------------- Arabic shaping + drawing ----------------

fn shape(s: &str) -> String { reshape_line(s) }

fn draw_crisp(img: &mut RgbImage, s: &str, x: i32, y: i32, scale: PxScale, font: &FontRef) {
    draw_text_mut(img, Rgb([0,0,0]), x, y, scale, font, s);
}

// RTL right-aligned: draw characters in reverse from a right edge
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

// LTR centered
fn draw_ltr_center(img: &mut RgbImage, font: &FontRef, scale: PxScale, s: &str, paper_w: i32, y: i32) {
    let (w, _) = text_size(scale, font, s);
    let x = (paper_w - w as i32) / 2;
    draw_crisp(img, s, x, y, scale, font);
}

// ---------------- Rendering ----------------

fn render_receipt(data: &ReceiptData, layout: &Layout) -> GrayImage {
    let paper_w = layout.paper_width_px as i32;
    let mut img: RgbImage = ImageBuffer::from_pixel(layout.paper_width_px, 1600, Rgb([255,255,255]));
    let margin = layout.margin;
    let inner_w = paper_w - margin*2;
    let right_edge = margin + inner_w;
    let mut y = 6i32; // tight top margin

    // If you have a crisper Arabic font, replace here (e.g., NotoKufiArabic-Regular.ttf)
    let font_bytes = include_bytes!("../fonts/NotoSansArabic-Regular.ttf");
    let font = FontRef::try_from_slice(font_bytes).expect("font");

    // === Title ===
    let title = "اسواق ابو عمر";
    draw_rtl_center(&mut img, &font, PxScale::from(layout.fonts.title), title, paper_w, y);
    y += (layout.fonts.title as i32) + 8;

    // === Date/Time line (no labels) ===
    // Example required: "٤ نوفمبر - ٤:٠٩ صباحا"
    let dt_line = "٤ نوفمبر - ٤:٠٩ صباحا";
    draw_rtl_center(&mut img, &font, PxScale::from(layout.fonts.header_dt), dt_line, paper_w, y);
    y += (layout.fonts.header_dt as i32) + 8;

    // === Receipt number (no label) ===
    draw_ltr_center(&mut img, &font, PxScale::from(layout.fonts.header_no), &data.invoice_no, paper_w, y);
    y += (layout.fonts.header_no as i32) + 18;

    // === Column headers (RTL) ===
    let s_head = PxScale::from(layout.fonts.header_cols);
    let col_name_r  = right_edge;
    let col_qty_r   = right_edge - (inner_w as f32 * layout.cols[0]) as i32;
    let col_price_r = col_qty_r   - (inner_w as f32 * layout.cols[1]) as i32;
    let col_value_r = col_price_r - (inner_w as f32 * layout.cols[2]) as i32;

    draw_rtl_right(&mut img, &font, s_head, "الصنف",  col_name_r,  y);
    draw_rtl_right(&mut img, &font, s_head, "الكمية", col_qty_r,   y);
    draw_rtl_right(&mut img, &font, s_head, "السعر",  col_price_r, y);
    draw_rtl_right(&mut img, &font, s_head, "قيمة",   col_value_r, y);
    y += (layout.row_gap - 8);

    // === Items ===
    let s_item = PxScale::from(layout.fonts.item);
    for it in &data.items {
        draw_rtl_right(&mut img, &font, s_item, &it.name,      col_name_r,  y);
        draw_ltr_right(&mut img, &font, s_item, &format!("{:.2}", it.qty),   col_qty_r,   y);
        draw_ltr_right(&mut img, &font, s_item, &format!("{:.2}", it.price), col_price_r, y);
        draw_ltr_right(&mut img, &font, s_item, &format!("{:.2}", it.value()), col_value_r, y);
        y += layout.row_gap;
    }

    // === Discount (still shown as value only) ===
    draw_rtl_right(&mut img, &font, s_head, "الخصم", col_name_r, y);
    draw_ltr_right(&mut img, &font, s_head, &format!("{:.2}", data.discount), col_value_r, y);
    y += layout.row_gap;

    // === Total ===
    let total: f32 = data.items.iter().map(|i| i.value()).sum::<f32>() - data.discount;
    draw_rtl_right(&mut img, &font, PxScale::from(layout.fonts.total_label), "إجمالي الفاتورة", col_name_r, y);
    draw_ltr_right(&mut img, &font, PxScale::from(layout.fonts.total_value), &format!("{:.2}", total), col_value_r, y - 10);
    y += (layout.row_gap + 24);

    // === Footer (centered & large) ===
    draw_rtl_center(&mut img, &font, PxScale::from(layout.fonts.footer), &data.footer_address, paper_w, y);
    y += (layout.fonts.footer as i32) + 8;

    draw_rtl_center(&mut img, &font, PxScale::from(layout.fonts.footer), "خدمه توصيل للمنازل ٢٤ ساعه", paper_w, y);
    y += (layout.fonts.footer as i32) + 8;

    draw_ltr_center(&mut img, &font, PxScale::from(layout.fonts.footer_small), &data.footer_phones, paper_w, y);
    y += (layout.fonts.footer_small as i32) + 20;

    // Crop and grayscale
    let used_h = (y + 8) as u32;
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
        customer_type: "".into(),
        // these two are not rendered; we show the hardcoded Arabic dt line above
        date: "".into(),
        time: "".into(),
        invoice_no: "123456".into(),
        cashier_name: "".into(),
        payment_label: "".into(),
        payment_amount: "".into(),
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

    p = p.feed().map_err(|e| e.to_string())?;
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