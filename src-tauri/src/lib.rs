use escpos::{driver::SerialPortDriver, printer::Printer, utils::*};
use image::{ImageBuffer, Rgb, RgbImage, Luma};
use imageproc::drawing::draw_text_mut;
use ab_glyph::{FontRef, PxScale};

// ============================================================================
// Print two Arabic lines as a raster image (bitmap) — NCR 7197 safe path
// ============================================================================

const DEFAULT_COM_PORT: &str = "COM7";
const DEFAULT_BAUD_RATE: u32 = 9600;
const PAPER_WIDTH_PX: u32 = 576; // 80mm head common width

fn get_com_port() -> String {
    std::env::var("PRINTER_COM_PORT").unwrap_or_else(|_| DEFAULT_COM_PORT.to_string())
}
fn get_baud_rate() -> u32 {
    std::env::var("PRINTER_BAUD_RATE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_BAUD_RATE)
}
fn normalize_com_port(port: &str) -> String {
    #[cfg(windows)]
    {
        let upper = port.to_uppercase();
        if upper.starts_with("COM") {
            if let Ok(n) = upper[3..].parse::<u32>() {
                if n > 9 {
                    return format!("\\\\.\\{}", upper);
                }
            }
        }
        port.to_string()
    }
    #[cfg(not(windows))]
    {
        port.to_string()
    }
}

// --- basic RTL visual (reverse Arabic runs only) for image drawing -----------
fn is_arabic(c: char) -> bool {
    (('\u{0600}'..='\u{06FF}').contains(&c))
        || (('\u{0750}'..='\u{077F}').contains(&c))
        || (('\u{08A0}'..='\u{08FF}').contains(&c))
        || (('\u{FB50}'..='\u{FDFF}').contains(&c))
        || (('\u{FE70}'..='\u{FEFF}').contains(&c))
}
fn rtl_visual(s: &str) -> String {
    #[derive(Clone, Copy, PartialEq)] enum K { Ar, Other }
    let mut runs: Vec<(K, String)> = Vec::new();
    let mut cur: Option<K> = None;
    let mut buf = String::new();
    for ch in s.chars() {
        let k = if is_arabic(ch) { K::Ar } else { K::Other };
        if cur == Some(k) || cur.is_none() {
            buf.push(ch);
            cur.get_or_insert(k);
        } else {
            runs.push((cur.unwrap(), std::mem::take(&mut buf)));
            cur = Some(k);
            buf.push(ch);
        }
    }
    if !buf.is_empty() {
        runs.push((cur.unwrap_or(K::Other), buf));
    }
    let mut out = String::new();
    for (k, run) in runs.into_iter().rev() {
        if k == K::Ar { out.extend(run.chars().rev()); } else { out.push_str(&run); }
    }
    out
}

// Render two lines into packed 1bpp bitmap for GS v 0
fn render_two_lines_bitmap(line1: &str, line2: &str) -> (Vec<u8>, u16, u16) {
    let mut img: RgbImage = ImageBuffer::from_pixel(PAPER_WIDTH_PX, 120, Rgb([255, 255, 255]));
    let font_data = include_bytes!("../fonts/NotoSansArabic-Regular.ttf");
    let font = FontRef::try_from_slice(font_data).expect("load font");
    let black = Rgb([0u8, 0u8, 0u8]);

    let l1 = rtl_visual(line1);
    let l2 = rtl_visual(line2);

    let scale1 = PxScale::from(32.0);
    let scale2 = PxScale::from(26.0);
    let approx_w1 = l1.chars().count() as i32 * (scale1.x as i32) / 2;
    let approx_w2 = l2.chars().count() as i32 * (scale2.x as i32) / 2;
    let x1 = ((PAPER_WIDTH_PX as i32) / 2 - approx_w1 / 2).max(0);
    let x2 = ((PAPER_WIDTH_PX as i32) / 2 - approx_w2 / 2).max(0);

    draw_text_mut(&mut img, black, x1, 25, scale1, &font, &l1);
    draw_text_mut(&mut img, black, x2, 70, scale2, &font, &l2);

    // 1-bit pack MSB-first
    let gray = image::DynamicImage::ImageRgb8(img).to_luma8();
    let w = gray.width();
    let h = gray.height();
    let bytes_per_row = ((w + 7) / 8) as usize;
    let mut packed = vec![0u8; bytes_per_row * h as usize];

    for y in 0..h {
        for x in 0..w {
            let Luma([pix]) = *gray.get_pixel(x, y);
            let bit = if pix < 128 { 1u8 } else { 0u8 }; // black pixel
            let byte_index = y as usize * bytes_per_row + (x as usize / 8);
            let bit_pos = 7 - (x as u8 & 7);
            if bit == 1 {
                packed[byte_index] |= 1u8 << bit_pos;
            }
        }
    }

    (packed, bytes_per_row as u16, h as u16)
}

#[tauri::command]
async fn print_receipt() -> Result<String, String> {
    let port = normalize_com_port(&get_com_port());
    let baud = get_baud_rate();

    let driver = SerialPortDriver::open(&port, baud, None)
        .map_err(|e| format!("open {} @{}: {}", port, baud, e))?;

    // FIX for E0716: hold the Printer object in a binding first
    let mut printer_obj = Printer::new(driver, Protocol::default(), None);
    printer_obj.debug_mode(None);
    let mut p = printer_obj.init().map_err(|e| e.to_string())?;

    // Render bitmap
    let (data, x_bytes, y) = render_two_lines_bitmap("متجر عينة", "اختبار الطباعة");

    // GS v 0 m=0 (normal) xL xH yL yH + data
    let xL = (x_bytes & 0xFF) as u8;
    let xH = ((x_bytes >> 8) & 0xFF) as u8;
    let yL = (y & 0xFF) as u8;
    let yH = ((y >> 8) & 0xFF) as u8;

    p = p.custom(&[0x1D, 0x76, 0x30, 0x00, xL, xH, yL, yH]).map_err(|e| e.to_string())?;
    p = p.custom(&data).map_err(|e| e.to_string())?;
    p = p.feed().map_err(|e| e.to_string())?;
    p = p.print_cut().map_err(|e| e.to_string())?;
    p.print().map_err(|e| e.to_string())?;

    Ok(format!("✅ Arabic bitmap printed on {}", port))
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
