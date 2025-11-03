use encoding_rs::WINDOWS_1256;
use escpos::{driver::SerialPortDriver, errors::Result as EscposResult, printer::Printer, printer_options::PrinterOptions, utils::*};

// ============================================================================
// Configuration
// ============================================================================

const DEFAULT_COM_PORT: &str = "COM7";
const DEFAULT_BAUD_RATE: u32 = 9600;
const CODEPAGE_WIN1256: u8 = 28;
const CONTEXTUAL_MODE: u8 = 5;

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

fn encode_arabic(text: &str) -> Vec<u8> {
    let (encoded, _, _) = WINDOWS_1256.encode(text);
    encoded.to_vec()
}

fn reverse_for_display(text: &str) -> String {
    text.chars().rev().collect()
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

    let printer = Printer::new(driver, Protocol::default(), Some(PrinterOptions::default()));
    
    let mut cmd: Vec<u8> = Vec::new();
    
    // Initialize and set codepage
    cmd.extend_from_slice(&[0x1B, 0x40]); // ESC @ - Initialize
    cmd.extend_from_slice(&[0x1B, 0x74, CODEPAGE_WIN1256]); // ESC t - Select code page
    cmd.extend_from_slice(&[0x1C, 0x43, CONTEXTUAL_MODE]); // FS C - Contextual mode
    
    // Center align
    cmd.extend_from_slice(&[0x1B, 0x61, 0x01]); // ESC a 1 - Center
    cmd.extend_from_slice(b"\n");
    
    // Header
    cmd.extend_from_slice(&encode_arabic(&reverse_for_display("متجر عينة")));
    cmd.extend_from_slice(b"\n");
    cmd.extend_from_slice(&encode_arabic(&reverse_for_display("123 شارع الرئيسي")));
    cmd.extend_from_slice(b"\n\n");
    
    // Divider
    cmd.extend_from_slice(b"================================\n");
    cmd.extend_from_slice(&encode_arabic(&reverse_for_display("الأصناف")));
    cmd.extend_from_slice(b"\n");
    cmd.extend_from_slice(b"================================\n\n");
    
    // Items
    let items = get_receipt_items();
    for item in items {
        cmd.extend_from_slice(&encode_arabic(&reverse_for_display(item.name_ar)));
        cmd.extend_from_slice(b"\n");
        let line = format!("{}x @ {:.2} EGP = {:.2} EGP\n\n", item.quantity, item.price, item.total());
        cmd.extend_from_slice(line.as_bytes());
    }
    
    // Totals
    let (subtotal, tax, total) = calculate_totals();
    cmd.extend_from_slice(b"================================\n");
    cmd.extend_from_slice(&encode_arabic(&reverse_for_display(&format!("المجموع الفرعي: {:.2} ج.م", subtotal))));
    cmd.extend_from_slice(b"\n");
    cmd.extend_from_slice(&encode_arabic(&reverse_for_display(&format!("الضريبة (10٪): {:.2} ج.م", tax))));
    cmd.extend_from_slice(b"\n");
    cmd.extend_from_slice(b"================================\n");
    cmd.extend_from_slice(&encode_arabic(&reverse_for_display(&format!("الإجمالي: {:.2} ج.م", total))));
    cmd.extend_from_slice(b"\n");
    cmd.extend_from_slice(b"================================\n\n");
    
    // Footer
    cmd.extend_from_slice(&encode_arabic(&reverse_for_display("شكراً لك على الشراء!")));
    cmd.extend_from_slice(b"\n");
    cmd.extend_from_slice(&encode_arabic(&reverse_for_display("نتمنى رؤيتك مرة أخرى")));
    cmd.extend_from_slice(b"\n\n\n\n");
    
    // Cut
    cmd.extend_from_slice(&[0x1D, 0x56, 0x00]); // GS V 0 - Full cut
    
    // Send to printer
    let result: EscposResult<()> = printer
        .init()
        .and_then(|p| p.custom(&cmd))
        .and_then(|p| p.print())
        .map(|_| ());
    
    match result {
        Ok(_) => Ok(format!("✅ Receipt printed successfully on {}", port)),
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

// ============================================================================
// Application Entry Point
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            print_receipt,
            get_receipt_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
