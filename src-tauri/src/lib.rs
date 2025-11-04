use escpos::{driver::SerialPortDriver, printer::Printer, utils::*};
use image::{ImageBuffer, Rgb, RgbImage, GrayImage, Luma};
use imageproc::drawing::{draw_text_mut, text_size};
use ab_glyph::{FontRef, PxScale};
use ar_reshaper::reshape_line; // proper Arabic shaping

// ============================================================================
// NCR 7197 — Arabic bitmap via ESC * 24-dot with correct RTL layout
// ============================================================================

const DEFAULT_COM_PORT: &str = "COM7";
const DEFAULT_BAUD_RATE: u32 = 9600;
const PAPER_WIDTH_PX: u32 = 576;   // 80mm head
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

// ---- shaping + RTL layout ---------------------------------------------------
// Draw one shaped Arabic line, *properly laid out RTL*, centered.
fn draw_rtl_shaped_centered(
    img: &mut RgbImage,
    font: &FontRef,
    scale: PxScale,
    logical_text: &str,
    y: i32,
) {
    // 1) Shape letters into contextual forms
    let shaped = reshape_line(logical_text);

    // 2) Measure each character width
    let chars: Vec<char> = shaped.chars().collect();
    let mut widths: Vec<i32> = Vec::with_capacity(chars.len());
    for &ch in &chars {
        let s = ch.to_string();
        let (w, _h) = text_size(scale, font, &s);
        widths.push(w as i32);
    }
    let total_w: i32 = widths.iter().sum();

    // 3) Start from left edge of the centered block
    let mut x = (PAPER_WIDTH_PX as i32 - total_w) / 2;

    // 4) Draw characters in *reverse order* so the visual result is RTL
    for i in (0..chars.len()).rev() {
        let s = chars[i].to_string();
        draw_bold_text(img, &s, x, y, scale, font);
        x += widths[i];
    }
}

// Bold by overpainting a few offsets
fn draw_bold_text(img: &mut RgbImage, text: &str, x: i32, y: i32, scale: PxScale, font: &FontRef) {
    let black = Rgb([0u8, 0u8, 0u8]);
    for (dx, dy) in [(0,0), (1,0), (0,1), (1,1), (1,2), (2,1)] {
        draw_text_mut(img, black, x + dx, y + dy, scale, font, text);
    }
}

// ---- render two big bold lines to grayscale image ---------------------------
fn render_lines(line1: &str, line2: &str) -> GrayImage {
    // Taller canvas for big type
    let mut img: RgbImage = ImageBuffer::from_pixel(PAPER_WIDTH_PX, 240, Rgb([255, 255, 255]));
    let font_data = include_bytes!("../fonts/NotoSansArabic-Regular.ttf");
    let font = FontRef::try_from_slice(font_data).expect("load font");

    let s1 = PxScale::from(70.0);
    let s2 = PxScale::from(52.0);

    draw_rtl_shaped_centered(&mut img, &font, s1, line1, 60);
    draw_rtl_shaped_centered(&mut img, &font, s2, line2, 150);

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
                    if pix <= THRESHOLD { b |= 1 << (7 - bit); }
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

    // keep object alive across method calls
    let mut obj = Printer::new(driver, Protocol::default(), None);
    obj.debug_mode(None);
    let mut p = obj.init().map_err(|e| e.to_string())?;

    // Two lines (logical order)
    let gray = render_lines("متجر عينة", "اختبار الطباعة");

    // Stream as ESC * 24-dot double density (m=33)
    let w = gray.width();
    let n = w as u16;
    let nL = (n & 0xFF) as u8;
    let nH = ((n >> 8) & 0xFF) as u8;

    let mut y0 = 0u32;
    while y0 < gray.height() {
        let band = pack_esc_star_24(&gray, y0);
        p = p.custom(&[0x1B, 0x2A, 33, nL, nH]).map_err(|e| e.to_string())?;
        p = p.custom(&band).map_err(|e| e.to_string())?;
        p = p.custom(&[0x0A]).map_err(|e| e.to_string())?; // LF to strobe
        y0 += 24;
    }

    p = p.feed().map_err(|e| e.to_string())?;
    p = p.print_cut().map_err(|e| e.to_string())?;
    p.print().map_err(|e| e.to_string())?;

    Ok(format!("✅ Arabic bitmap (RTL fixed) printed on {}", port))
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