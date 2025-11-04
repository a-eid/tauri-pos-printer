use escpos::{driver::SerialPortDriver, printer::Printer, utils::*};
use image::{ImageBuffer, Rgb, RgbImage, GrayImage, Luma};
use imageproc::drawing::{draw_line_segment_mut, draw_text_mut, text_size};
use ab_glyph::{FontRef, PxScale};
use ar_reshaper::reshape_line;
use serde::Deserialize;

// ================================================================
// Configurable Arabic receipt as bitmap (ESC * 24-dot) – NCR 7197
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

// -------- Data & layout (JSON) ----------------------------------

#[derive(Clone, Deserialize)]
struct Item { name: String, qty: f32, price: f32 }
impl Item { fn value(&self) -> f32 { self.qty * self.price } }

#[derive(Clone, Deserialize)]
struct ReceiptData {
    store_name: String,
    customer_type: String,  // "عميل نقدي"...
    date: String,           // yyyy-mm-dd
    time: String,           // hh:mm:ss
    invoice_no: String,     // "52"
    cashier_name: String,   // "احمد"
    payment_label: String,  // "كاش"
    payment_amount: String, // "35348.00"
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
    row_h: i32,
    fonts: Fonts,
}
#[derive(Clone, Deserialize)]
struct Fonts {
    title: f32,
    table: f32,
    small: f32,
    total: f32,
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            paper_width_px: 576,
            threshold: 200,
            margin: 16,
            row_h: 42,
            fonts: Fonts { title: 48.0, table: 26.0, small: 22.0, total: 40.0 },
        }
    }
}

// -------- Arabic shaping + RTL drawing --------------------------

fn shape(s: &str) -> String { reshape_line(s) }

fn draw_bold(img: &mut RgbImage, s: &str, x: i32, y: i32, scale: PxScale, font: &FontRef) {
    let black = Rgb([0u8, 0u8, 0u8]);
    for (dx, dy) in [(0,0), (1,0), (0,1), (1,1)] {
        draw_text_mut(img, black, x + dx, y + dy, scale, font, s);
    }
}

// RTL right-aligned: draw chars in reverse from a right edge
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
        draw_bold(img, &chars[i].to_string(), x, y, scale, font);
        x += widths[i];
    }
}

// RTL centered: same model but center the whole run
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
        draw_bold(img, &chars[i].to_string(), x, y, scale, font);
        x += widths[i];
    }
}

// LTR right-aligned (numbers/latin)
fn draw_ltr_right(img: &mut RgbImage, font: &FontRef, scale: PxScale, s: &str, x_right: i32, y: i32) {
    let (w, _) = text_size(scale, font, s);
    draw_bold(img, s, x_right - w as i32, y, scale, font);
}

// lines
fn hline(img: &mut RgbImage, y: i32, x0: i32, x1: i32) {
    let c = Rgb([0,0,0]);
    draw_line_segment_mut(img, (x0 as f32, y as f32), (x1 as f32, y as f32), c);
}
fn vline(img: &mut RgbImage, x: i32, y0: i32, y1: i32) {
    let c = Rgb([0,0,0]);
    draw_line_segment_mut(img, (x as f32, y0 as f32), (x as f32, y1 as f32), c);
}
fn rect(img: &mut RgbImage, x0: i32, y0: i32, x1: i32, y1: i32) {
    hline(img, y0, x0, x1); hline(img, y1, x0, x1);
    vline(img, x0, y0, y1); vline(img, x1, y0, y1);
}

// -------- Rendering ----------------------------------------------------------

fn render_receipt(data: &ReceiptData, layout: &Layout) -> GrayImage {
    let paper_w = layout.paper_width_px as i32;
    let mut img: RgbImage = ImageBuffer::from_pixel(layout.paper_width_px, 1400, Rgb([255,255,255]));
    let margin = layout.margin;
    let inner_w = paper_w - margin*2;
    let right_edge = margin + inner_w;
    let mut y = 20i32;

    let font_bytes = include_bytes!("../fonts/NotoSansArabic-Regular.ttf");
    let font = FontRef::try_from_slice(font_bytes).expect("font");

    // Header box + centered store name (RTL centered fix)
    let header_h = 70;
    rect(&mut img, margin, y, margin + inner_w, y + header_h);
    draw_rtl_center(&mut img, &font, PxScale::from(layout.fonts.title), &data.store_name, paper_w, y + 18);
    y += header_h + 12;

    // Row: عميل نقدي | تاريخ/وقت | رقم
    let s_small = PxScale::from(layout.fonts.small);
    let s_table = PxScale::from(layout.fonts.table);

    draw_rtl_right(&mut img, &font, s_small, "العميل", right_edge, y);
    draw_rtl_right(&mut img, &font, s_table, &data.customer_type, right_edge - 140, y - 4);

    draw_rtl_right(&mut img, &font, s_small, "تاريخ", right_edge - 270, y);
    draw_ltr_right(&mut img, &font, s_small, &data.date, right_edge - 350, y);
    draw_ltr_right(&mut img, &font, s_small, &data.time, right_edge - 440, y);

    let inv_box_w = 90;
    let inv_box_h = 36;
    let inv_x1 = margin + 150;
    let inv_y1 = y - 10;
    rect(&mut img, inv_x1, inv_y1, inv_x1 + inv_box_w, inv_y1 + inv_box_h);
    draw_ltr_right(&mut img, &font, s_table, &data.invoice_no, inv_x1 + inv_box_w - 8, inv_y1 + 6);
    y += 44;

    // Row: المستخدم أحمد | كاش 35348.00
    draw_rtl_right(&mut img, &font, s_small, "المستخدم", right_edge - 120, y);
    draw_rtl_right(&mut img, &font, s_small, &data.cashier_name, right_edge - 210, y);

    draw_rtl_right(&mut img, &font, s_small, &data.payment_label, right_edge - 370, y);
    draw_ltr_right(&mut img, &font, s_small, &data.payment_amount, right_edge - 450, y);
    y += 28;

    // Table
    y += 12;
    rect(&mut img, margin, y, margin + inner_w, y + 44);
    let value_w = 120; let price_w = 110; let qty_w = 90;
    let name_w  = inner_w - (value_w + price_w + qty_w);

    let x_name_l = margin;
    let x_name_r = x_name_l + name_w;
    let x_qty_r  = x_name_r  + qty_w;
    let x_price_r= x_qty_r   + price_w;
    let x_value_r= x_price_r + value_w;

    vline(&mut img, x_name_r, y, y + 44);
    vline(&mut img, x_qty_r,  y, y + 44);
    vline(&mut img, x_price_r,y, y + 44);

    draw_rtl_right(&mut img, &font, s_table, "الصنف",  x_name_r - 8, y + 10);
    draw_rtl_right(&mut img, &font, s_table, "الكمية", x_qty_r  - 8, y + 10);
    draw_rtl_right(&mut img, &font, s_table, "السعر",  x_price_r- 8, y + 10);
    draw_rtl_right(&mut img, &font, s_table, "قيمة",   x_value_r- 8, y + 10);
    y += 44;

    let row_h = layout.row_h;
    for it in &data.items {
        rect(&mut img, margin, y, margin + inner_w, y + row_h);
        vline(&mut img, x_name_r,  y, y + row_h);
        vline(&mut img, x_qty_r,   y, y + row_h);
        vline(&mut img, x_price_r, y, y + row_h);

        draw_rtl_right(&mut img, &font, s_table, &it.name, x_name_r - 8, y + 10);
        draw_ltr_right(&mut img, &font, s_table, &format!("{:.2}", it.qty),   x_qty_r  - 8, y + 10);
        draw_ltr_right(&mut img, &font, s_table, &format!("{:.2}", it.price), x_price_r- 8, y + 10);
        draw_ltr_right(&mut img, &font, s_table, &format!("{:.2}", it.value()), x_value_r- 8, y + 10);

        y += row_h;
    }

    // Discount row
    y += 10;
    rect(&mut img, margin, y, margin + inner_w, y + 44);
    draw_rtl_right(&mut img, &font, s_table, "الخصم", x_value_r - 8, y + 10);
    draw_ltr_right(&mut img, &font, s_table, &format!("{:.2}", data.discount), x_name_r - 8, y + 10);
    y += 60;

    // Total
    let total: f32 = data.items.iter().map(|i| i.value()).sum::<f32>() - data.discount;
    let total_box_w = 240;
    let x_total_l = margin + (inner_w - total_box_w) / 2;
    let x_total_r = x_total_l + total_box_w;
    rect(&mut img, x_total_l, y, x_total_r, y + 56);
    draw_ltr_right(&mut img, &font, PxScale::from(layout.fonts.total), &format!("{:.2}", total), x_total_r - 12, y + 10);
    draw_rtl_right(&mut img, &font, s_table, "إجمالي الفاتورة", right_edge, y + 12);
    y += 80;

    // Footer
    draw_rtl_right(&mut img, &font, PxScale::from(layout.fonts.small), &data.footer_address, right_edge - 4, y);
    y += 26;
    draw_ltr_right(&mut img, &font, PxScale::from(layout.fonts.small), &data.footer_phones, right_edge - 4, y);
    y += 30;

    // Crop and convert to grayscale
    let used_h = (y + 20) as u32;
    image::DynamicImage::ImageRgb8(img).crop_imm(0, 0, layout.paper_width_px, used_h).to_luma8()
}

// Pack ESC * 24-dot bands
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

// =============== Tauri commands =================

#[tauri::command]
async fn print_receipt_sample() -> Result<String, String> {
    // sample data close to your photo
    let data = ReceiptData {
        store_name: "أسواق الجدر".into(),
        customer_type: "عميل نقدي".into(),
        date: "2025-11-04".into(),
        time: "03:00:44".into(),
        invoice_no: "52".into(),
        cashier_name: "احمد".into(),
        payment_label: "كاش".into(),
        payment_amount: "35348.00".into(),
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
        footer_phones: "01116060144 - 01116781700".into(),
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

    // ESC * 24-dot double density (m=33)
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
    Ok(format!("✅ Receipt printed on {}.", port))
}

// add this near other commands
#[tauri::command]
async fn print_receipt() -> Result<String, String> {
    print_receipt_sample().await
}

// =============== App entry =====================

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
