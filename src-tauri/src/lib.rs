use escpos::{
    driver::SerialPortDriver,
    printer::Printer,
    printer_options::PrinterOptions,
    utils::*,
};
use ar_reshaper::reshape_line;

// ============================================================================
// Minimal config (NCR 7197 probe)
// ============================================================================

const DEFAULT_COM_PORT: &str = "COM7";
const DEFAULT_BAUD_RATE: u32 = 9600;

// Try common device code-page indexes (ESC t n)
const TRY_PAGES: &[u8] = &[37, 50, 23, 22, 17, 18, 33];

// ============================================================================
// Helpers
// ============================================================================

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

// ============================================================================
// Tauri command: probe a few code pages and print two Arabic lines each
// ============================================================================

#[tauri::command]
async fn print_receipt() -> Result<String, String> {
    let port = normalize_com_port(&get_com_port());
    let baud = get_baud_rate();

    let driver = SerialPortDriver::open(&port, baud, None)
        .map_err(|e| format!("Failed to open printer on {} @{}: {}", port, baud, e))?;

    // Software encoder: PC864 (works with shaped Arabic we send)
    let opts = PrinterOptions::new(Some(PageCode::PC864), None, 42);

    // Build printer without temporary drops
    let mut printer_obj = Printer::new(driver, Protocol::default(), Some(opts));
    printer_obj.debug_mode(None);
    let mut p = printer_obj.init().map_err(|e| e.to_string())?;

    let line1 = reshape_line("متجر عينة");
    let line2 = reshape_line("اختبار الطباعة");

    // Print a small block with different ESC t values
    for &n in TRY_PAGES {
        p = p.custom(&[0x1B, 0x74, n]).map_err(|e| e.to_string())?; // set device page
        let label = format!("ESC t {} →", n);

        p = p.justify(JustifyMode::CENTER).map_err(|e| e.to_string())?
            .writeln(&label).map_err(|e| e.to_string())?
            .writeln(&line1).map_err(|e| e.to_string())?
            .writeln(&line2).map_err(|e| e.to_string())?
            .feed().map_err(|e| e.to_string())?;
    }

    p = p.print_cut().map_err(|e| e.to_string())?;
    p.print().map_err(|e| e.to_string())?;

    Ok(format!("✅ Probe sent to {} (pages: {:?})", port, TRY_PAGES))
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
