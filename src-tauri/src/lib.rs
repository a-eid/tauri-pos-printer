use escpos::{driver::SerialPortDriver, printer::Printer, utils::*};
use image::{ImageBuffer, Rgb, RgbImage, GrayImage, Luma};
use imageproc::drawing::draw_text_mut;
use ab_glyph::{FontRef, PxScale};

// ============================================================================
// NCR 7197 — final: print Arabic as bitmap via ESC * 24-dot (works on your unit)
// ============================================================================

const DEFAULT_COM_PORT: &str = "COM7";
const DEFAULT_BAUD_RATE: u32 = 9600;
const PAPER_WIDTH_PX: u32 = 576;

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

// --- minimal RTL visual (reverse Arabic runs only) for image drawing ----------
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
        if cur == Some(k) || cur.is_none() { buf.push(ch); cur.get_or_insert(k); }
        else { runs.push((cur.unwrap(), std::mem::take(&mut buf))); cur = Some(k); buf.push(ch); }
    }
    if !buf.is_empty() { runs.push((cur.unwrap_or(K::Other), buf)); }
    let mut out = String::new();
    for (k, run) in runs.into_iter().rev() {
        if k == K::Ar { out.extend(run.chars().rev()); } else { out.push_str(&run); }
    }
    out
}

// ---- render two lines to grayscale image ------------------------------------
fn render_lines(line1: &str, line2: &str) -> GrayImage {
    let mut img: RgbImage = ImageBuffer::from_pixel(PAPER_WIDTH_PX, 140, Rgb([255, 255, 255]));
    let font_data = include_bytes!("../fonts/NotoSansArabic-Regular.ttf");
    let font = FontRef::try_from_slice(font_data).expect("load font");
    let black = Rgb([0u8, 0u8, 0u8]);

    let l1 = rtl_visual(line1);
    let l2 = rtl_visual(line2);

    let s1 = PxScale::from(34.0);
    let s2 = PxScale::from(26.0);
    let w1 = l1.chars().count() as i32 * (s1.x as i32) / 2;
    let w2 = l2.chars().count() as i32 * (s2.x as i32) / 2;
    let x1 = (PAPER_WIDTH_PX as i32 / 2 - w1 / 2).max(0);
    let x2 = (PAPER_WIDTH_PX as i32 / 2 - w2 / 2).max(0);

    draw_text_mut(&mut img, black, x1, 32, s1, &font, &l1);
    draw_text_mut(&mut img, black, x2, 86, s2, &font, &l2);

    image::DynamicImage::ImageRgb8(img).to_luma8()
}

// ---- pack vertical 24-dot bands for ESC * (m=33) ----------------------------
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
                    if pix < 128 { b |= 1 << (7 - bit); }
                }
            }
            band.push(b);
        }
    }
    band
}

#[tauri::command]
async fn print_receipt() -> Result<String, String> {
    let port = normalize_com_port(&get_com_port());
    let baud = get_baud_rate();
    let driver = SerialPortDriver::open(&port, baud, None)
        .map_err(|e| format!("open {} @{}: {}", port, baud, e))?;

    // hold object to avoid temporary-drop errors
    let mut obj = Printer::new(driver, Protocol::default(), None);
    obj.debug_mode(None);
    let mut p = obj.init().map_err(|e| e.to_string())?;

    let gray = render_lines("متجر عينة", "اختبار الطباعة");

    // ESC * 24-dot double density (m=33), stream image in 24-dot bands
    let w = gray.width();
    let n = w as u16; // number of columns
    let nL = (n & 0xFF) as u8;
    let nH = ((n >> 8) & 0xFF) as u8;

    let mut y0 = 0u32;
    while y0 < gray.height() {
        let band = pack_esc_star_24(&gray, y0);
        p = p.custom(&[0x1B, 0x2A, 33, nL, nH]).map_err(|e| e.to_string())?;
        p = p.custom(&band).map_err(|e| e.to_string())?;
        p = p.custom(&[0x0A]).map_err(|e| e.to_string())?; // LF to strobe the line
        y0 += 24;
    }

    p = p.feed().map_err(|e| e.to_string())?;
    p = p.print_cut().map_err(|e| e.to_string())?;
    p.print().map_err(|e| e.to_string())?;

    Ok(format!("✅ Arabic bitmap (ESC * 24-dot) printed on {}", port))
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
