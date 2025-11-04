use escpos::{
    driver::SerialPortDriver,
    errors::Result as EscposResult,
    printer::Printer,
    printer_options::PrinterOptions,
    utils::*,
};
use ar_reshaper::reshape_line;

// ============================================================================
// NCR 7197 – Arabic print (software shaping) using PC864
// ============================================================================

const DEFAULT_COM_PORT: &str = "COM7";
const DEFAULT_BAUD_RATE: u32 = 9600;

// From your probe, Arabic glyphs appeared on some pages. Pick the one that
// showed Arabic letters (e.g. 22, 17, 18, 33). You can change this constant.
const ESC_T_PC864_INDEX: u8 = 22;

// ----------------------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------------------
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

// Reverse only Arabic runs after shaping so numbers/latin stay LTR.
fn is_arabic_char(c: char) -> bool {
    (('\u{0600}'..='\u{06FF}').contains(&c))
        || (('\u{0750}'..='\u{077F}').contains(&c))
        || (('\u{08A0}'..='\u{08FF}').contains(&c))
        || (('\u{FB50}'..='\u{FDFF}').contains(&c))
        || (('\u{FE70}'..='\u{FEFF}').contains(&c))
}
fn rtl_visual(src: &str) -> String {
    let shaped = reshape_line(src);
    #[derive(Clone, Copy, PartialEq)]
    enum K { Ar, Other }
    let mut runs: Vec<(K, String)> = Vec::new();
    let mut cur: Option<K> = None;
    let mut buf = String::new();
    for ch in shaped.chars() {
        let k = if is_arabic_char(ch) { K::Ar } else { K::Other };
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

// ============================================================================
// Tauri command
// ============================================================================
#[tauri::command]
async fn print_receipt() -> Result<String, String> {
    let port = normalize_com_port(&get_com_port());
    let baud = get_baud_rate();

    let driver = SerialPortDriver::open(&port, baud, None)
        .map_err(|e| format!("open {} @{}: {}", port, baud, e))?;

    // Use PC864 table in escpos-rs (the git commit you pinned adds it).
    let opts = PrinterOptions::new(Some(PageCode::PC864), None, 42);

    // Build printer without temporary drops
    let mut p_obj = Printer::new(driver, Protocol::default(), Some(opts));
    p_obj.debug_mode(None);
    let p = p_obj.init().map_err(|e| e.to_string())?;

    // Prepare lines (shape + RTL visual order)
    let line1 = rtl_visual("متجر عينة");
    let line2 = rtl_visual("اختبار الطباعة");

    // Keep EscposResult in the chain; convert to String only once.
    let res: EscposResult<()> = p
        // Select device page index that produced Arabic on your printer
        .custom(&[0x1B, 0x74, ESC_T_PC864_INDEX])
        .and_then(|p| p.justify(JustifyMode::CENTER))
        .and_then(|p| p.writeln(&line1))
        .and_then(|p| p.writeln(&line2))
        .and_then(|p| p.feed())
        .and_then(|p| p.print_cut())
        .and_then(|p| p.print())
        .map(|_| ());
    
    match res {
        Ok(()) => Ok(format!("✅ Arabic test (PC864, ESC t {}) sent on {}", ESC_T_PC864_INDEX, port)),
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
