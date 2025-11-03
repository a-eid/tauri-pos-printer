use escpos::{driver::SerialPortDriver, errors::Result as EscposResult, printer::Printer, printer_options::PrinterOptions, utils::*};
use image::{ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::{draw_line_segment_mut, draw_text_mut};
use ab_glyph::{FontRef, PxScale};
use ar_reshaper::reshape_line;

// ============================================================================
// Configuration
// ============================================================================

const DEFAULT_COM_PORT: &str = "COM7";
const DEFAULT_BAUD_RATE: u32 = 9600;

// ============================================================================
// Utilities
// ============================================================================

fn get_com_port() -> String {
    std::env::var("PRINTER_COM_PORT")
        .unwrap_or_else(|_| DEFAULT_COM_PORT.to_string())
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

// --- Minimal RTL visualizer for bitmap text: shape Arabic, reverse Arabic runs, keep digits/LTR runs ---
fn is_arabic_char(c: char) -> bool {
    (('\u{0600}'..='\u{06FF}').contains(&c))
        || (('\u{0750}'..='\u{077F}').contains(&c))
        || (('\u{08A0}'..='\u{08FF}').contains(&c))
        || (('\u{FB50}'..='\u{FDFF}').contains(&c))
        || (('\u{FE70}'..='\u{FEFF}').contains(&c))
}

fn arabic_visual(src: &str) -> String {
    let shaped = reshape_line(src);
    #[derive(Clone, Copy, PartialEq)]
    enum K {
        Ar,
        Other,
    }
    let mut runs: Vec<(K, String)> = Vec::new();
    let mut cur_k: Option<K> = None;
    let mut buf = String::new();

    for ch in shaped.chars() {
        let k = if is_arabic_char(ch) { K::Ar } else { K::Other };
        if cur_k == Some(k) || cur_k.is_none() {
            buf.push(ch);
            cur_k.get_or_insert(k);
        } else {
            runs.push((cur_k.unwrap(), std::mem::take(&mut buf)));
            cur_k = Some(k);
            buf.push(ch);
        }
    }
    if !buf.is_empty() {
        runs.push((cur_k.unwrap_or(K::Other), buf));
    }

    let mut out = String::new();
    for (k, run) in runs.into_iter().rev() {
        if k == K::Ar {
            out.extend(run.chars().rev());
        } else {
            out.push_str(&run);
        }
    }
    out
}

// ============================================================================
// Receipt Data
// ============================================================================

struct ReceiptItem {
    name_ar: &'static str,
    quantity: u32,
    price: f64,
}

impl ReceiptItem {
    fn total(&self) -> f64 {
        self.quantity as f64 * self.price
    }
}

fn get_receipt_items() -> Vec<ReceiptItem> {
    vec![
        ReceiptItem { name_ar: "تفاح", quantity: 2, price: 2.50 },
        ReceiptItem { name_ar: "موز", quantity: 3, price: 1.50 },
        ReceiptItem { name_ar: "برتقال", quantity: 1, price: 3.00 },
    ]
}

fn calculate_totals() -> (f64, f64, f64) {
    let items = get_receipt_items();
    let subtotal: f64 = items.iter().map(|i| i.total()).sum();
    let tax = subtotal * 0.10;
    let total = subtotal + tax;
    (subtotal, tax, total)
}

// ============================================================================
// Tauri Commands
// ============================================================================

#[tauri::command]
async fn print_receipt() -> Result<String, String> {
    let port = normalize_com_port(&get_com_port());
    let baud = get_baud_rate();

    let driver = SerialPortDriver::open(&port, baud, None)
        .map_err(|e| format!("Failed to open printer on {} @{}: {}", port, baud, e))?;

    // NCR 7197 (ESC/POS compatible): use PC864 and pre-shape lines.
    let opts = PrinterOptions::new(Some(PageCode::PC864), None, 42);

    // Avoid temporary-drop borrow: bind the printer first, then call methods.
    let mut printer_obj = Printer::new(driver, Protocol::default(), Some(opts));
    printer_obj.debug_mode(None);
    let printer = printer_obj.init().map_err(|e| e.to_string())?;

    let line1 = reshape_line("متجر عينة");
    let line2 = reshape_line("اختبار الطباعة");

    // Chain with and_then to keep EscposResult throughout, then convert once at the end
    let res: EscposResult<()> = printer
        // Force code page in case firmware ignores options: ESC t 37 => PC864
        .custom(&[0x1B, 0x74, 37])
        .and_then(|p| p.justify(JustifyMode::CENTER))
        .and_then(|p| p.writeln(&line1))
        .and_then(|p| p.writeln(&line2))
        .and_then(|p| p.feed())
        .and_then(|p| p.print_cut())
        .and_then(|p| p.print())
        .map(|_| ());
    
    match res {
        Ok(()) => Ok(format!("✅ Arabic test sent (PC864) on {}", port)),
        Err(e) => Err(format!("Failed to print: {}", e)),
    }
}

#[tauri::command]
fn get_receipt_data() -> Result<serde_json::Value, String> {
    let items = get_receipt_items();
    let (subtotal, tax, total) = calculate_totals();

    Ok(serde_json::json!({
        "header": {
            "storeName": "متجر عينة",
            "address": "123 شارع الرئيسي"
        },
        "items": items.iter().map(|item| {
            serde_json::json!({
                "name": item.name_ar,
                "quantity": item.quantity,
                "price": item.price,
                "total": item.total()
            })
        }).collect::<Vec<_>>(),
        "totals": {
            "subtotal": subtotal,
            "tax": tax,
            "total": total
        },
        "footer": {
            "thanks": "شكراً لك على الشراء!",
            "comeback": "نتمنى رؤيتك مرة أخرى"
        }
    }))
}

#[tauri::command]
fn generate_receipt_image() -> Result<String, String> {
    let items = get_receipt_items();
    let (subtotal, tax, total) = calculate_totals();

    // Image dimensions (80mm = 576px at ~72–203 DPI printers)
    let width = 576u32;
    let mut height = 900u32;

    // Create white background
    let mut img: RgbImage = ImageBuffer::from_pixel(width, height, Rgb([255u8, 255u8, 255u8]));

    // Load Noto Sans Arabic font
    let font_data = include_bytes!("../fonts/NotoSansArabic-Regular.ttf");
    let font = FontRef::try_from_slice(font_data)
        .map_err(|e| format!("Failed to load font: {}", e))?;

    let black = Rgb([0u8, 0u8, 0u8]);
    let gray = Rgb([128u8, 128u8, 128u8]);

    let mut y = 30.0f32;
    let right_x = (width - 40) as i32;

    // Helper to draw text centered
    let draw_centered_text = |img: &mut RgbImage, text: &str, y_pos: f32, scale: f32| {
        let text_vis = arabic_visual(text);
        let scale = PxScale::from(scale);
        let approx_w = text_vis.chars().count() as f32 * scale.x * 0.5;
        let x = (width as f32 / 2.0 - approx_w / 2.0) as i32;
        draw_text_mut(img, black, x.max(20), y_pos as i32, scale, &font, &text_vis);
    };

    // Helper to draw text right-aligned
    let draw_right_text = |img: &mut RgbImage, text: &str, y_pos: f32, scale: f32| {
        let text_vis = arabic_visual(text);
        let scale_obj = PxScale::from(scale);
        let approx_w = text_vis.chars().count() as f32 * scale * 0.5;
        let x = (right_x as f32 - approx_w) as i32;
        draw_text_mut(img, black, x.max(20), y_pos as i32, scale_obj, &font, &text_vis);
    };

    // Helper to draw divider
    let draw_divider = |img: &mut RgbImage, y_pos: &mut f32| {
        let y_int = *y_pos as i32;
        draw_line_segment_mut(img, (20.0, y_int as f32), ((width - 20) as f32, y_int as f32), gray);
        *y_pos += 25.0;
    };

    // Header
    draw_centered_text(&mut img, "متجر عينة", y, 32.0);
    y += 45.0;

    draw_centered_text(&mut img, "123 شارع الرئيسي", y, 18.0);
    y += 35.0;

    draw_divider(&mut img, &mut y);

    // Items header
    draw_centered_text(&mut img, "الأصناف", y, 24.0);
    y += 35.0;

    draw_divider(&mut img, &mut y);

    // Items
    for item in &items {
        draw_right_text(&mut img, item.name_ar, y, 22.0);
        y += 30.0;

        let item_line = format!("{}x @ {:.2} ج.م = {:.2} ج.م", item.quantity, item.price, item.total());
        draw_centered_text(&mut img, &item_line, y, 18.0);
        y += 35.0;
    }

    y += 10.0;
    draw_divider(&mut img, &mut y);

    // Totals
    let subtotal_text = format!("المجموع الفرعي: {:.2} ج.م", subtotal);
    draw_right_text(&mut img, &subtotal_text, y, 18.0);
    y += 30.0;

    let tax_text = format!("الضريبة (10٪): {:.2} ج.م", tax);
    draw_right_text(&mut img, &tax_text, y, 18.0);
    y += 35.0;

    draw_divider(&mut img, &mut y);

    let total_text = format!("الإجمالي: {:.2} ج.م", total);
    draw_right_text(&mut img, &total_text, y, 26.0);
    y += 40.0;

    draw_divider(&mut img, &mut y);

    // Footer
    draw_centered_text(&mut img, "شكراً لك على الشراء!", y, 22.0);
    y += 30.0;

    draw_centered_text(&mut img, "نتمنى رؤيتك مرة أخرى", y, 18.0);
    y += 40.0;

    // Trim image to actual height
    height = (y as u32).min(height);
    let img = image::imageops::crop_imm(&img, 0, 0, width, height).to_image();

    // Save to desktop
    let desktop = dirs::desktop_dir()
        .ok_or_else(|| "Could not find desktop directory".to_string())?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let filename = format!("receipt_{}.png", timestamp);
    let filepath = desktop.join(&filename);

    img.save(&filepath)
        .map_err(|e| format!("Failed to save image: {}", e))?;

    Ok(format!("✅ Receipt saved to Desktop: {}", filename))
}

// ============================================================================
// Application Entry Point
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            print_receipt,
            get_receipt_data,
            generate_receipt_image,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
