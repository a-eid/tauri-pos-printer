use escpos::{
    driver::SerialPortDriver,
    errors::Result as EscposResult,
    printer::Printer,
    printer_options::PrinterOptions,
    utils::*,
};

// ===================== Config =====================
const DEFAULT_COM_PORT: &str = "COM7";
const DEFAULT_BAUD_RATE: u32 = 9600;

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

// ===================== Tauri Command =====================
#[tauri::command]
async fn print_receipt() -> Result<String, String> {
    let port = normalize_com_port(&get_com_port());
    let baud = get_baud_rate();

    let driver = SerialPortDriver::open(&port, baud, None)
        .map_err(|e| format!("Failed to open printer on {} @{}: {}", port, baud, e))?;

    // Use Windows-1256 and let the NCR 7197 do contextual shaping/RTL.
    let opts = PrinterOptions::new(Some(PageCode::WPC1256), None, 42);

    let mut printer_obj = Printer::new(driver, Protocol::default(), Some(opts));
    printer_obj.debug_mode(None);
    let p = printer_obj.init().map_err(|e| e.to_string())?;

    let res: EscposResult<()> = p
        // ESC t 50 => Windows-1256
        .custom(&[0x1B, 0x74, 50])
        // FS C 5 => enable Arabic contextual/RTL (try 1 or 4 if needed)
        .and_then(|p| p.custom(&[0x1C, 0x43, 0x05]))
        .and_then(|p| p.justify(JustifyMode::CENTER))
        .and_then(|p| p.writeln("متجر عينة"))
        .and_then(|p| p.writeln("اختبار الطباعة"))
        .and_then(|p| p.feed())
        .and_then(|p| p.print_cut())
        .and_then(|p| p.print())
        .map(|_| ());
    
    match res {
        Ok(()) => Ok(format!("✅ Arabic test (Win-1256 + FS C 5) sent on {}", port)),
        Err(e) => Err(format!("Failed to print: {}", e)),
    }
}

// ===================== App entry =====================
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![print_receipt])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
