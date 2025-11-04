use escpos::{driver::SerialPortDriver, printer::Printer, utils::*};
use image::{ImageBuffer, Rgb, RgbImage, GrayImage, Luma};
use imageproc::drawing::{draw_line_segment_mut, draw_text_mut, text_size};
use ab_glyph::{FontRef, PxScale};
use ar_reshaper::reshape_line;

// ============================================================================
// NCR 7197 — Arabic receipt as bitmap via ESC * 24-dot (layout like the photo)
// ============================================================================

const DEFAULT_COM_PORT: &str = "COM7";
const DEFAULT_BAUD_RATE: u32 = 9600;
const PAPER_WIDTH_PX: u32 = 576;   // 80mm print head
const THRESHOLD: u8 = 200;         // higher => darker

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

// ── Arabic shaping helpers ───────────────────────────────────────────────────
fn shape(s: &str) -> String { reshape_line(s) }

// Draw single glyph string several times (bold)
fn draw_bold(img: &mut RgbImage, s: &str, x: i32, y: i32, scale: PxScale, font: &FontRef) {
    let black = Rgb([0u8, 0u8, 0u8]);
    for (dx, dy) in [(0,0), (1,0), (0,1), (1,1)] {
        draw_text_mut(img, black, x + dx, y + dy, scale, font, s);
    }
}

// Proper RTL: measure each shaped char, draw from right edge going left
fn draw_rtl_right_aligned(
    img: &mut RgbImage,
    font: &FontRef,
    scale: PxScale,
    logical_text: &str,
    x_right: i32,
    y: i32,
) {
    let shaped = shape(logical_text);
    let chars: Vec<char> = shaped.chars().collect();
    // measure widths
    let mut widths: Vec<i32> = Vec::with_capacity(chars.len());
    for &ch in &chars {
        let (w, _) = text_size(scale, font, &ch.to_string());
        widths.push(w as i32);
    }
    // total width
    let total_w: i32 = widths.iter().sum();
    // start from right edge minus total width (right-aligned)
    let mut x = x_right - total_w;
    for i in (0..chars.len()).rev() {
        let s = chars[i].to_string();
        draw_bold(img, &s, x, y, scale, font);
        x += widths[i];
    }
}

// LTR text right aligned (numbers / latin)
fn draw_ltr_right_aligned(
    img: &mut RgbImage,
    font: &FontRef,
    scale: PxScale,
    s: &str,
    x_right: i32,
    y: i32,
) {
    let (w, _) = text_size(scale, font, s);
    draw_bold(img, s, x_right - w as i32, y, scale, font);
}

// ── Drawing primitives ───────────────────────────────────────────────────────
fn hline(img: &mut RgbImage, y: i32, x0: i32, x1: i32) {
    let c = Rgb([0u8, 0u8, 0u8]);
    draw_line_segment_mut(img, (x0 as f32, y as f32), (x1 as f32, y as f32), c);
}
fn vline(img: &mut RgbImage, x: i32, y0: i32, y1: i32) {
    let c = Rgb([0u8, 0u8, 0u8]);
    draw_line_segment_mut(img, (x as f32, y0 as f32), (x as f32, y1 as f32), c);
}
fn rect(img: &mut RgbImage, x0: i32, y0: i32, x1: i32, y1: i32) {
    hline(img, y0, x0, x1); hline(img, y1, x0, x1);
    vline(img, x0, y0, y1); vline(img, x1, y0, y1);
}

// ── Receipt rendering ────────────────────────────────────────────────────────
#[derive(Clone)]
struct Item { name: &'static str, qty: f32, price: f32 }
impl Item { fn value(&self) -> f32 { self.qty * self.price } }

fn render_receipt() -> GrayImage {
    let margin = 16i32;
    let inner_w = (PAPER_WIDTH_PX as i32) - margin * 2;
    let mut y = 20i32;

    // canvas tall enough; we’ll print only the used part
    let mut img: RgbImage = ImageBuffer::from_pixel(PAPER_WIDTH_PX, 1000, Rgb([255,255,255]));

    let font_data = include_bytes!("../fonts/NotoSansArabic-Regular.ttf");
    let font = FontRef::try_from_slice(font_data).expect("font");

    // Header boxed store name
    let header_h = 70;
    rect(&mut img, margin, y, margin + inner_w, y + header_h);
    let title = "أسواق الجدر";
    let s_title = PxScale::from(48.0);
    // center title by measuring block width (approx by text_size over shaped)
    let shaped_title = shape(title);
    let (tw, th) = text_size(s_title, &font, &shaped_title);
    let tx = margin + (inner_w - tw as i32) / 2;
    let ty = y + (header_h - th as i32) / 2 - 4;
    draw_bold(&mut img, &shaped_title, tx, ty, s_title, &font);
    y += header_h + 12;

    // Row: العميل / عميل نقدي   |   التاريخ / الوقت   |  رقم الفاتورة
    let s_small = PxScale::from(24.0);
    let s_mid = PxScale::from(28.0);

    let right_edge = margin + inner_w;

    // Left block (invoice total big value like sample – we’ll show dummy)
    // We’ll render right side fields first (Arabic RTL)
    draw_rtl_right_aligned(&mut img, &font, s_small, "العميل", right_edge, y);
    draw_rtl_right_aligned(&mut img, &font, s_mid, "عميل نقدي", right_edge - 140, y - 4);

    // Date and time (middle)
    let date = "2025-11-04";
    let time = "03:00:44";
    draw_rtl_right_aligned(&mut img, &font, s_small, "تاريخ", right_edge - 260, y);
    draw_ltr_right_aligned(&mut img, &font, s_small, date, right_edge - 340, y);
    draw_ltr_right_aligned(&mut img, &font, s_small, time, right_edge - 430, y);

    // Invoice number (boxed small black)
    let inv_box_w = 90;
    let inv_box_h = 36;
    let inv_x1 = margin + 160;
    let inv_y1 = y - 10;
    rect(&mut img, inv_x1, inv_y1, inv_x1 + inv_box_w, inv_y1 + inv_box_h);
    draw_ltr_right_aligned(&mut img, &font, PxScale::from(26.0), "52", inv_x1 + inv_box_w - 8, inv_y1 + 4);
    y += 44;

    // Row: المستخدم أحمد   |   كاش 35348.00
    draw_rtl_right_aligned(&mut img, &font, s_small, "المستخدم", right_edge - 120, y);
    draw_rtl_right_aligned(&mut img, &font, s_small, "احمد", right_edge - 210, y);

    draw_rtl_right_aligned(&mut img, &font, s_small, "كاش", right_edge - 370, y);
    draw_ltr_right_aligned(&mut img, &font, s_small, "35348.00", right_edge - 450, y);
    y += 18;

    // Table header box
    y += 16;
    rect(&mut img, margin, y, margin + inner_w, y + 44);

    // Columns (right-to-left): قيمة | السعر | الكمية | الصنف
    let value_w = 120;
    let price_w = 110;
    let qty_w   = 90;
    let name_w  = inner_w - (value_w + price_w + qty_w);

    let x_name_l = margin;
    let x_name_r = x_name_l + name_w;
    let x_qty_r  = x_name_r + qty_w;
    let x_price_r= x_qty_r  + price_w;
    let x_value_r= x_price_r+ value_w;

    // vertical lines
    vline(&mut img, x_name_r, y, y + 44);
    vline(&mut img, x_qty_r,  y, y + 44);
    vline(&mut img, x_price_r,y, y + 44);

    // Header labels
    draw_rtl_right_aligned(&mut img, &font, s_small, "الصنف",  x_name_r - 8, y + 10);
    draw_rtl_right_aligned(&mut img, &font, s_small, "الكمية", x_qty_r - 8,  y + 10);
    draw_rtl_right_aligned(&mut img, &font, s_small, "السعر",  x_price_r - 8, y + 10);
    draw_rtl_right_aligned(&mut img, &font, s_small, "قيمة",   x_value_r - 8, y + 10);
    y += 44;

    // Items
    let items = vec![
        Item { name: "تفاح عرض", qty: 0.96, price: 70.00 },
        Item { name: "تفاح",     qty: 1.95, price: 30.00 },
        Item { name: "خيار",     qty: 1.02, price: 25.00 },
        Item { name: "ليمون بلدي", qty: 0.44, price: 30.00 },
        Item { name: "بطاطا",    qty: 2.16, price: 20.00 },
        Item { name: "ربطة جرجير", qty: 4.00, price: 3.00 },
        Item { name: "نعناع فريش", qty: 1.00, price: 5.00 },
    ];

    let row_h = 42;
    for (i, it) in items.iter().enumerate() {
        // row box
        rect(&mut img, margin, y, margin + inner_w, y + row_h);

        // verticals
        vline(&mut img, x_name_r,  y, y + row_h);
        vline(&mut img, x_qty_r,   y, y + row_h);
        vline(&mut img, x_price_r, y, y + row_h);

        // text in cells (right aligned with padding)
        draw_rtl_right_aligned(&mut img, &font, s_small, it.name, x_name_r - 8, y + 10);
        draw_ltr_right_aligned(&mut img, &font, s_small, &format!("{:.2}", it.qty),   x_qty_r - 8,  y + 10);
        draw_ltr_right_aligned(&mut img, &font, s_small, &format!("{:.2}", it.price), x_price_r - 8, y + 10);
        draw_ltr_right_aligned(&mut img, &font, s_small, &format!("{:.2}", it.value()), x_value_r - 8, y + 10);

        y += row_h;
        if i == items.len() - 1 { /* last row */ }
    }

    // Discount box
    y += 10;
    rect(&mut img, margin, y, margin + inner_w, y + 44);
    draw_rtl_right_aligned(&mut img, &font, s_small, "الخصم", x_value_r - 8, y + 10);
    draw_ltr_right_aligned(&mut img, &font, s_small, "0.00",  x_name_r - 8, y + 10); // left area just to mirror photo
    y += 60;

    // Total big box
    let total_val: f32 = items.iter().map(|i| i.value()).sum(); // no discount
    let total_box_w = 230;
    let x_total_l = margin + (inner_w - total_box_w) / 2;
    let x_total_r = x_total_l + total_box_w;
    rect(&mut img, x_total_l, y, x_total_r, y + 56);
    let s_total = PxScale::from(40.0);
    draw_ltr_right_aligned(&mut img, &font, s_total, &format!("{:.2}", total_val), x_total_r - 10, y + 10);

    // Total label on the right
    draw_rtl_right_aligned(&mut img, &font, s_mid, "إجمالي الفاتورة", margin + inner_w, y + 12);
    y += 80;

    // Footer
    let s_footer = PxScale::from(22.0);
    draw_rtl_right_aligned(&mut img, &font, s_footer, "دمياط الجديدة - المركزية - مقابل البنك الأهلي القديم", margin + inner_w - 4, y);
    y += 28;
    draw_ltr_right_aligned(&mut img, &font, s_footer, "01116060144 - 01116781700", margin + inner_w - 4, y);
    y += 30;

    // Crop to used height
    let used_h = (y + 30) as u32;
    image::DynamicImage::ImageRgb8(img)
        .crop_imm(0, 0, PAPER_WIDTH_PX, used_h)
        .to_luma8()
}

// Pack 24-dot bands for ESC * (m=33, double density)
fn pack_esc_star_24(gray: &GrayImage, y0: u32) -> Vec<u8> {
    let w = gray.width();
    let h = gray.height();
    let mut band: Vec<u8> = Vec::with_capacity((w * 3) as usize);
    for x in 0..w {
        for byte in 0..3 {
            let mut b = 0u8;
            for bit in 0..8 {
                let yy = y0 + (byte * 8 + bit) as u32;
                if yy < h {
                    let Luma([pix]) = *gray.get_pixel(x, yy);
                    if pix <= THRESHOLD { b |= 1 << (7 - bit); }
                }
            }
            band.push(b);
        }
    }
    band
}

// ============================================================================
// Tauri command
// ============================================================================

#[tauri::command]
async fn print_receipt() -> Result<String, String> {
    let port = normalize_com_port(&get_com_port());
    let baud = get_baud_rate();
    let driver = SerialPortDriver::open(&port, baud, None)
        .map_err(|e| format!("open {} @{}: {}", port, baud, e))?;

    // keep printer object alive
    let mut obj = Printer::new(driver, Protocol::default(), None);
    obj.debug_mode(None);
    let mut p = obj.init().map_err(|e| e.to_string())?;

    // Render receipt
    let gray = render_receipt();

    // Stream as ESC * 24-dot bands
    let w = gray.width();
    let n = w as u16;
    let nL = (n & 0xFF) as u8;
    let nH = ((n >> 8) & 0xFF) as u8;

    let mut y0 = 0u32;
    while y0 < gray.height() {
        let band = pack_esc_star_24(&gray, y0);
        p = p.custom(&[0x1B, 0x2A, 33, nL, nH]).map_err(|e| e.to_string())?;
        p = p.custom(&band).map_err(|e| e.to_string())?;
        p = p.custom(&[0x0A]).map_err(|e| e.to_string())?; // LF
        y0 += 24;
    }

    p = p.feed().map_err(|e| e.to_string())?;
    p = p.print_cut().map_err(|e| e.to_string())?;
    p.print().map_err(|e| e.to_string())?;
    Ok(format!("✅ Arabic receipt printed on {}", port))
}

// ============================================================================
// App entry
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![print_receipt])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}