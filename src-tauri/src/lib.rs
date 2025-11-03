use escpos::{driver::SerialPortDriver, printer::Printer, printer_options::PrinterOptions, utils::*};

#[tauri::command]
async fn print_receipt() -> Result<String, String> {
    let port = std::env::var("PRINTER_COM_PORT").unwrap_or_else(|_| "COM7".to_string());
    let baud: u32 = std::env::var("PRINTER_BAUD_RATE").ok().and_then(|s| s.parse().ok()).unwrap_or(9600);

    let driver = SerialPortDriver::open(&port, baud, None)
        .map_err(|e| format!("open {} @{}: {}", port, baud, e))?;

    // 👉 Use Windows-1256 (not PC864) and let the printer shape/RTL
    let opts = PrinterOptions::new(Some(PageCode::WPC1256), None, 42);

    let mut p = Printer::new(driver, Protocol::default(), Some(opts))
        .debug_mode(None)
        .init().map_err(|e| e.to_string())?;

    // ESC t 50 = Windows-1256 on most NCR/Epson
    p = p.custom(&[0x1B, 0x74, 50]).map_err(|e| e.to_string())?;
    // FS C 5 = RTL + contextual Arabic (try 1 or 4 if needed)
    p = p.custom(&[0x1C, 0x43, 0x05]).map_err(|e| e.to_string())?;

    p = p.justify(JustifyMode::CENTER).map_err(|e| e.to_string())?
        .writeln("متجر عينة").map_err(|e| e.to_string())?
        .writeln("اختبار الطباعة").map_err(|e| e.to_string())?
        .feed().map_err(|e| e.to_string())?
        .print_cut().map_err(|e| e.to_string())?;

    p.print().map_err(|e| e.to_string())?;
    Ok(format!("✅ Arabic test (Win-1256 + FS C 5) sent on {}", port))
}
