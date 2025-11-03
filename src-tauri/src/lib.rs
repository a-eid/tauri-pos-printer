use escpos::{
    driver::SerialPortDriver,
    errors::Result as EscposResult,
    printer::Printer,
    printer_options::PrinterOptions,
    utils::*,
};
use ar_reshaper::reshape_line;

// ============================================================================
// Minimal config
// ============================================================================

const DEFAULT_COM_PORT: &str = "COM7";
const DEFAULT_BAUD_RATE: u32 = 9600;

// For NCR 7197 the ESC t index for Arabic can differ from Epson.
// We'll probe a few likely pages: PC864 (Epson=37), Win-1256 (often=50),
// and some nearby vendor slots that NCR firmwares sometimes use.
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

    // Tell escpos-rs to encode using PC864 bytes (works with our shaped text).
    // We will then vary ESC t 'n' on the device to find the matching slot.
    let opts = PrinterOptions::new(Some(PageCode::PC864), None, 42);

    let mut printer_obj = Printer::new(driver, Protocol::default(), Some(opts));
    printer_obj.debug_mode(None);
    let mut p = printer_obj.init().map_err(|e| e.to_string())?;

    let line1 = reshape_line("متجر عينة");
    let line2 = reshape_line("اختبار الطباعة");

    // Print a short block for each candidate ESC t value
    for &n in TRY_PAGES {
        // Select device code page
        p = p.custom(&[0x1B, 0x74, n])?;
        // Label in ASCII so you can see which 'n' worked
        let label = format!("ESC t {} →", n);

        p = p.justify(JustifyMode::CENTER)?
            .writeln(&label)?
            .writeln(&line1)?
            .writeln(&line2)?
            .feed()?;
    }

    // Cut & send
    let res: EscposResult<()> = p.print_cut().and_then(|p| p.print()).map(|_| ());
    match res {
        Ok(()) => Ok(format!("✅ Probe sent to {} (pages: {:?})", port, TRY_PAGES)),
        Err(e) => Err(format!("Failed to print: {}", e)),
    }
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
