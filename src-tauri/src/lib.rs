use escpos::{
    driver::SerialPortDriver,
    printer::Printer,
    printer_options::PrinterOptions,
    utils::*,
};

// ============================================================================
// NCR 7197 – Probe Windows-1256 with FS C (contextual/RTL) values
// ============================================================================

const DEFAULT_COM_PORT: &str = "COM7";
const DEFAULT_BAUD_RATE: u32 = 9600;
// ESC t n candidates already tested; we’ll fix on Win-1256 (n=50) and sweep FS C.
const FS_C_CANDIDATES: &[u8] = &[0, 1, 2, 3, 4, 5, 6];

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

#[tauri::command]
async fn print_receipt() -> Result<String, String> {
    let port = normalize_com_port(&get_com_port());
    let baud = get_baud_rate();

    let driver = SerialPortDriver::open(&port, baud, None)
        .map_err(|e| format!("open {} @{}: {}", port, baud, e))?;

    // Software encoder: Windows-1256; device page will be ESC t 50
    let opts = PrinterOptions::new(Some(PageCode::WPC1256), None, 42);

    // Build printer without temporary drops
    let mut p_obj = Printer::new(driver, Protocol::default(), Some(opts));
    p_obj.debug_mode(None);
    let mut p = p_obj.init().map_err(|e| e.to_string())?;

    // ESC t 50 = Win-1256 on NCR/Epson class devices
    p = p.custom(&[0x1B, 0x74, 50]).map_err(|e| e.to_string())?;

    let line1 = "متجر عينة";
    let line2 = "اختبار الطباعة";

    // Probe FS C values (contextual Arabic/RTL). Look for the block that prints connected RTL.
    for &fs in FS_C_CANDIDATES {
        // FS C <fs>
        p = p.custom(&[0x1C, 0x43, fs]).map_err(|e| e.to_string())?;
        let label = format!("FS C {} →", fs);

        p = p.justify(JustifyMode::CENTER).map_err(|e| e.to_string())?
            .writeln(&label).map_err(|e| e.to_string())?
            .writeln(line1).map_err(|e| e.to_string())?
            .writeln(line2).map_err(|e| e.to_string())?
            .feed().map_err(|e| e.to_string())?;
    }

    p = p.print_cut().map_err(|e| e.to_string())?;
    p.print().map_err(|e| e.to_string())?;
    Ok(format!("✅ Probed FS C values {:?} using ESC t 50 (Win-1256) on {}", FS_C_CANDIDATES, port))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![print_receipt])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
