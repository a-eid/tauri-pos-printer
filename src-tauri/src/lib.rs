use escpos::{
    driver::SerialPortDriver,
    errors::Result as EscposResult,
    printer::Printer,
    printer_options::PrinterOptions,
    utils::*,
};
use ar_reshaper::reshape_line;

// ===================== Config =====================
const DEFAULT_COM_PORT: &str = "COM7";
const DEFAULT_BAUD_RATE: u32 = 9600;
// NCR 7197: from your probe, Arabic glyphs appeared around these pages.
// Default to 22; override with env PRINTER_ESC_T if needed.
const DEFAULT_ESC_T_PAGE: u8 = 22;

// ===================== Helpers =====================
fn get_com_port() -> String {
    std::env::var("PRINTER_COM_PORT").unwrap_or_else(|_| DEFAULT_COM_PORT.to_string())
}
fn get_baud_rate() -> u32 {
    std::env::var("PRINTER_BAUD_RATE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_BAUD_RATE)
}
fn get_esc_t_page() -> u8 {
    std::env::var("PRINTER_ESC_T")
        .ok()
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(DEFAULT_ESC_T_PAGE)
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

// ===================== Tauri Commands =====================
#[tauri::command]
async fn print_receipt() -> Result<String, String> {
    let port = normalize_com_port(&get_com_port());
    let baud = get_baud_rate();
    let esc_t = get_esc_t_page();

    let driver = SerialPortDriver::open(&port, baud, None)
        .map_err(|e| format!("Failed to open printer on {} @{}: {}", port, baud, e))?;

    // Use PC864 (this table exists in your escpos-rs commit).
    let opts = PrinterOptions::new(Some(PageCode::PC864), None, 42);
    let mut printer_obj = Printer::new(driver, Protocol::default(), Some(opts));
    printer_obj.debug_mode(None);
    let p = printer_obj.init().map_err(|e| e.to_string())?;

    // Pre-shape Arabic so glyphs join correctly with PC864.
    let line1 = reshape_line("متجر عينة");
    let line2 = reshape_line("اختبار الطباعة");

    let res: EscposResult<()> = p
        // Select the device code page (from your probe, start with 22).
        .custom(&[0x1B, 0x74, esc_t])
        .and_then(|p| p.justify(JustifyMode::CENTER))
        .and_then(|p| p.writeln(&line1))
        .and_then(|p| p.writeln(&line2))
        .and_then(|p| p.feed())
        .and_then(|p| p.print_cut())
        .and_then(|p| p.print())
        .map(|_| ());

    match res {
        Ok(()) => Ok(format!("✅ Arabic test sent on {} (ESC t {})", port, esc_t)),
        Err(e) => Err(format!("Failed to print: {}", e)),
    }
}

#[tauri::command]
fn get_receipt_data() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "header": { "storeName": "متجر عينة", "address": "123 شارع الرئيسي" },
        "items": [
            { "name": "تفاح", "quantity": 2, "price": 2.50, "total": 5.00 },
            { "name": "موز", "quantity": 3, "price": 1.50, "total": 4.50 }
        ],
        "totals": { "subtotal": 9.50, "tax": 0.95, "total": 10.45 },
        "footer": { "thanks": "شكراً لك على الشراء!", "comeback": "نتمنى رؤيتك مرة أخرى" }
    }))
}

// ===================== App entry =====================
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            print_receipt,
            get_receipt_data
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
