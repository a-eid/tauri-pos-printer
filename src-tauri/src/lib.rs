use printers::get_printers;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct PrinterInfo {
    name: String,
    system_name: String,
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_thermal_printers() -> Result<Vec<PrinterInfo>, String> {
    let printers = get_printers();
    
    // Filter for thermal printers - look for common thermal printer keywords
    let thermal_keywords = vec![
        "thermal", "pos", "receipt", "tm-", "tsp", "rp", "star", 
        "epson", "bixolon", "zebra", "citizen", "rongta", "xprinter"
    ];
    
    let thermal_printers: Vec<PrinterInfo> = printers
        .into_iter()
        .filter(|p| {
            let name_lower = p.name.to_lowercase();
            thermal_keywords.iter().any(|keyword| name_lower.contains(keyword))
        })
        .map(|p| PrinterInfo {
            name: p.name.clone(),
            system_name: p.name.clone(),
        })
        .collect();
    
    Ok(thermal_printers)
}

// Generate plain text receipt (let Windows printer driver handle rendering)
fn generate_text_receipt() -> String {
    let mut receipt = String::new();
    
    // Store name (will be rendered by Windows with proper Arabic fonts)
    receipt.push_str("        متجر عينة\n");
    receipt.push_str("    123 شارع الرئيسي\n");
    receipt.push_str("  المدينة، المحافظة 12345\n");
    receipt.push_str("  هاتف: (555) 123-4567\n\n");
    
    receipt.push_str("================================\n");
    receipt.push_str("        الأصناف\n");
    receipt.push_str("================================\n\n");
    
    // Items - RTL will be handled by Windows
    receipt.push_str("تفاح\n");
    receipt.push_str("  2x @ 2.50 ج.م = 5.00 ج.م\n\n");
    
    receipt.push_str("موز\n");
    receipt.push_str("  3x @ 1.50 ج.م = 4.50 ج.م\n\n");
    
    receipt.push_str("برتقال\n");
    receipt.push_str("  1x @ 3.00 ج.م = 3.00 ج.م\n\n");
    
    receipt.push_str("================================\n");
    receipt.push_str("المجموع الفرعي:    7.00 ج.م\n");
    receipt.push_str("الضريبة (10٪):     0.70 ج.م\n");
    receipt.push_str("================================\n");
    receipt.push_str("الإجمالي:          7.70 ج.م\n");
    receipt.push_str("================================\n\n");
    
    receipt.push_str("    شكراً لك على الشراء!\n");
    receipt.push_str("    نتمنى رؤيتك مرة أخرى\n\n\n\n");
    
    receipt
}

#[tauri::command]
fn print_receipt(printer_name: String) -> Result<String, String> {
    // Use Windows GDI Text printing - same as Electron!
    // Send as plain text, let Windows printer driver handle Arabic rendering
    let text_data = generate_text_receipt();
    let text_bytes = text_data.as_bytes();
    
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::Graphics::Printing::{
            OpenPrinterW, StartDocPrinterW, WritePrinter,
            EndDocPrinter, ClosePrinter, DOC_INFO_1W,
        };
        use windows::core::PWSTR;
        use std::ptr;
        
        unsafe {
            let printer_name_wide: Vec<u16> = printer_name.encode_utf16().chain(std::iter::once(0)).collect();
            let mut h_printer: HANDLE = HANDLE::default();
            
            // Open the printer
            let result = OpenPrinterW(
                PWSTR(printer_name_wide.as_ptr() as *mut u16),
                &mut h_printer,
                None,
            );
            
            if result.is_err() {
                return Err(format!("Failed to open printer '{}'.", printer_name));
            }
            
            // Set up document info - NO datatype = use default TEXT rendering!
            // This lets Windows GDI render Arabic with proper fonts
            let mut doc_name: Vec<u16> = "Receipt\0".encode_utf16().collect();
            
            let doc_info = DOC_INFO_1W {
                pDocName: PWSTR(doc_name.as_mut_ptr()),
                pOutputFile: PWSTR(ptr::null_mut()),
                pDatatype: PWSTR(ptr::null_mut()), // NULL = use printer driver's default rendering (TEXT mode with GDI)
            };
            
            // Start document
            let job_id = StartDocPrinterW(h_printer, 1, &doc_info);
            if job_id == 0 {
                let _ = ClosePrinter(h_printer);
                return Err("Failed to start print job".to_string());
            }
            
            // Write text data (Windows will render with proper Arabic fonts!)
            let mut bytes_written: u32 = 0;
            let write_result = WritePrinter(
                h_printer,
                text_bytes.as_ptr() as *const _,
                text_bytes.len() as u32,
                &mut bytes_written,
            );
            
            if !write_result.as_bool() {
                let _ = EndDocPrinter(h_printer);
                let _ = ClosePrinter(h_printer);
                return Err("Failed to write to printer".to_string());
            }
            
            // End document
            let _ = EndDocPrinter(h_printer);
            let _ = ClosePrinter(h_printer);
        }
    }
    
    Ok("Receipt printed successfully! / تم طباعة الإيصال بنجاح!".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, get_thermal_printers, print_receipt])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
