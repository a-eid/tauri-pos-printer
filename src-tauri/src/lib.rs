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
    // For now: Simple test with ASCII to verify cutting works
    // Then we'll tackle Arabic separately
    let mut commands = Vec::new();
    
    // ESC @ - Initialize
    commands.extend_from_slice(&[0x1B, 0x40]);
    
    // Center align
    commands.extend_from_slice(&[0x1B, 0x61, 0x01]);
    
    // Receipt with both English and Arabic (to test if Arabic works)
    commands.extend_from_slice(b"\n");
    
    // Store name - English + Arabic
    commands.extend_from_slice("     SAMPLE STORE\n".as_bytes());
    commands.extend_from_slice("        متجر عينة\n\n".as_bytes());
    
    commands.extend_from_slice("   123 Main Street\n".as_bytes());
    commands.extend_from_slice("    123 شارع الرئيسي\n".as_bytes());
    
    commands.extend_from_slice(b"================================\n");
    
    // Items header
    commands.extend_from_slice("       ITEMS / الأصناف\n".as_bytes());
    commands.extend_from_slice(b"================================\n\n");
    
    // Items with Arabic names
    commands.extend_from_slice("Apple / تفاح\n".as_bytes());
    commands.extend_from_slice(b"  2x @ 2.50 EGP = 5.00 EGP\n\n");
    
    commands.extend_from_slice("Banana / موز\n".as_bytes());
    commands.extend_from_slice(b"  3x @ 1.50 EGP = 4.50 EGP\n\n");
    
    commands.extend_from_slice("Orange / برتقال\n".as_bytes());
    commands.extend_from_slice(b"  1x @ 3.00 EGP = 3.00 EGP\n\n");
    
    commands.extend_from_slice(b"================================\n");
    
    // Totals
    commands.extend_from_slice("Subtotal / المجموع الفرعي\n".as_bytes());
    commands.extend_from_slice(b"                     7.00 EGP\n");
    
    commands.extend_from_slice("Tax (10%) / الضريبة\n".as_bytes());
    commands.extend_from_slice(b"                     0.70 EGP\n");
    
    commands.extend_from_slice(b"================================\n");
    
    commands.extend_from_slice("TOTAL / الإجمالي\n".as_bytes());
    commands.extend_from_slice(b"                     7.70 EGP\n");
    
    commands.extend_from_slice(b"================================\n\n");
    
    // Footer
    commands.extend_from_slice("   Thank you!\n".as_bytes());
    commands.extend_from_slice("    شكراً لك!\n".as_bytes());
    
    // INCREASED bottom padding (from 4 to 8 line feeds)
    commands.extend_from_slice(b"\n\n\n\n\n\n\n\n");
    
    // Paper cut
    commands.extend_from_slice(&[0x1D, 0x56, 0x00]);
    
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
            
            let result = OpenPrinterW(
                PWSTR(printer_name_wide.as_ptr() as *mut u16),
                &mut h_printer,
                None,
            );
            
            if result.is_err() {
                return Err(format!("Failed to open printer"));
            }
            
            let mut doc_name: Vec<u16> = "Receipt\0".encode_utf16().collect();
            let mut datatype: Vec<u16> = "RAW\0".encode_utf16().collect();
            
            let doc_info = DOC_INFO_1W {
                pDocName: PWSTR(doc_name.as_mut_ptr()),
                pOutputFile: PWSTR(ptr::null_mut()),
                pDatatype: PWSTR(datatype.as_mut_ptr()),
            };
            
            let job_id = StartDocPrinterW(h_printer, 1, &doc_info);
            if job_id == 0 {
                let _ = ClosePrinter(h_printer);
                return Err("Failed to start print job".to_string());
            }
            
            let mut bytes_written: u32 = 0;
            let write_result = WritePrinter(
                h_printer,
                commands.as_ptr() as *const _,
                commands.len() as u32,
                &mut bytes_written,
            );
            
            if !write_result.as_bool() {
                let _ = EndDocPrinter(h_printer);
                let _ = ClosePrinter(h_printer);
                return Err("Failed to write to printer".to_string());
            }
            
            let _ = EndDocPrinter(h_printer);
            let _ = ClosePrinter(h_printer);
        }
    }
    
    Ok("Receipt printed! ✓ English ✓ Cut ✓ Padding | If Arabic is gibberish, we'll use HTML printing next.".to_string())
}

// Print receipt by rendering HTML to image and sending as bitmap
#[tauri::command]
async fn print_receipt_image(printer_name: String, image_data_url: String) -> Result<String, String> {
    use base64::{Engine as _, engine::general_purpose};
    
    // Extract base64 data from data URL (format: "data:image/png;base64,...")
    let base64_data = image_data_url
        .strip_prefix("data:image/png;base64,")
        .ok_or("Invalid image data URL format")?;
    
    // Decode base64 to bytes
    let image_bytes = general_purpose::STANDARD.decode(base64_data)
        .map_err(|e| format!("Failed to decode base64: {}", e))?;
    
    // Load image
    let img = image::load_from_memory(&image_bytes)
        .map_err(|e| format!("Failed to load image: {}", e))?;
    
    // Convert to grayscale and resize to fit thermal printer width (576px for 80mm)
    let img = img.resize(576, u32::MAX, image::imageops::FilterType::Lanczos3);
    let gray_img = img.to_luma8();
    
    // Convert to 1-bit monochrome using dithering
    let (width, height) = gray_img.dimensions();
    
    // SAFETY CHECK: Prevent excessive paper printing
    if height > 2000 {
        return Err(format!("❌ Image too large ({} pixels high). Maximum 2000px to prevent paper waste. Use HTML Dialog method instead.", height));
    }
    
    let monochrome = apply_dithering(&gray_img);
    
    // Pack pixels into bytes (8 pixels per byte, MSB first)
    let bytes_per_line = (width + 7) / 8;
    let mut packed_data: Vec<u8> = Vec::new();
    
    for y in 0..height {
        let mut line_bytes = vec![0u8; bytes_per_line as usize];
        for x in 0..width {
            let pixel = monochrome.get_pixel(x, y)[0];
            if pixel > 128 {
                // White pixel = 0 (no print)
                // Already 0
            } else {
                // Black pixel = 1 (print)
                let byte_index = (x / 8) as usize;
                let bit_index = 7 - (x % 8);
                line_bytes[byte_index] |= 1 << bit_index;
            }
        }
        packed_data.extend_from_slice(&line_bytes);
    }
    
    // Build ESC/POS commands
    let mut commands: Vec<u8> = Vec::new();
    
    // Initialize printer
    commands.extend_from_slice(&[0x1B, 0x40]); // ESC @
    
    // Center alignment
    commands.extend_from_slice(&[0x1B, 0x61, 0x01]); // ESC a 1
    
    // GS v 0: Print raster bitmap
    // Format: GS v 0 m xL xH yL yH [data]
    commands.extend_from_slice(&[0x1D, 0x76, 0x30, 0x00]); // GS v 0 0 (normal mode)
    
    // Width in bytes (little endian)
    let width_bytes = bytes_per_line as u16;
    commands.push((width_bytes & 0xFF) as u8);
    commands.push(((width_bytes >> 8) & 0xFF) as u8);
    
    // Height in pixels (little endian)
    let height_u16 = height as u16;
    commands.push((height_u16 & 0xFF) as u8);
    commands.push(((height_u16 >> 8) & 0xFF) as u8);
    
    // Image data
    commands.extend_from_slice(&packed_data);
    
    // Feed paper and cut
    commands.extend_from_slice(&[0x0A, 0x0A, 0x0A, 0x0A]); // 4 line feeds
    commands.extend_from_slice(&[0x1D, 0x56, 0x00]); // GS V 0 (cut)
    
    // Print using platform-specific method
    print_raw_bytes(&printer_name, &commands)?;
    
    Ok(format!("✅ Receipt image printed! ({} x {} px)", width, height))
}

// Helper: Apply Floyd-Steinberg dithering for better image quality
fn apply_dithering(img: &image::ImageBuffer<image::Luma<u8>, Vec<u8>>) -> image::ImageBuffer<image::Luma<u8>, Vec<u8>> {
    let (width, height) = img.dimensions();
    let mut result = img.clone();
    
    for y in 0..height {
        for x in 0..width {
            let old_pixel = result.get_pixel(x, y)[0] as i16;
            let new_pixel = if old_pixel > 128 { 255 } else { 0 };
            result.put_pixel(x, y, image::Luma([new_pixel as u8]));
            
            let error = old_pixel - new_pixel;
            
            // Distribute error to neighboring pixels
            if x + 1 < width {
                let p = result.get_pixel(x + 1, y)[0] as i16;
                result.put_pixel(x + 1, y, image::Luma([(p + error * 7 / 16).clamp(0, 255) as u8]));
            }
            if y + 1 < height {
                if x > 0 {
                    let p = result.get_pixel(x - 1, y + 1)[0] as i16;
                    result.put_pixel(x - 1, y + 1, image::Luma([(p + error * 3 / 16).clamp(0, 255) as u8]));
                }
                let p = result.get_pixel(x, y + 1)[0] as i16;
                result.put_pixel(x, y + 1, image::Luma([(p + error * 5 / 16).clamp(0, 255) as u8]));
                if x + 1 < width {
                    let p = result.get_pixel(x + 1, y + 1)[0] as i16;
                    result.put_pixel(x + 1, y + 1, image::Luma([(p + error * 1 / 16).clamp(0, 255) as u8]));
                }
            }
        }
    }
    
    result
}

// Helper: Print raw bytes to printer
fn print_raw_bytes(printer_name: &str, data: &[u8]) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::Graphics::Printing::{
            OpenPrinterW, ClosePrinter, StartDocPrinterW, EndDocPrinter,
            StartPagePrinter, EndPagePrinter, WritePrinter, DOC_INFO_1W,
        };
        use windows::core::PWSTR;
        use std::ptr;
        
        unsafe {
            let printer_name_wide: Vec<u16> = printer_name.encode_utf16().chain(std::iter::once(0)).collect();
            let mut h_printer: HANDLE = HANDLE::default();
            
            let result = OpenPrinterW(
                PWSTR(printer_name_wide.as_ptr() as *mut u16),
                &mut h_printer,
                None,
            );
            
            if result.is_err() {
                return Err(format!("Failed to open printer: {}", printer_name));
            }
            
            // Start document in RAW mode
            let mut doc_name: Vec<u16> = "Receipt Image\0".encode_utf16().collect();
            let mut datatype: Vec<u16> = "RAW\0".encode_utf16().collect();
            
            let doc_info = DOC_INFO_1W {
                pDocName: PWSTR(doc_name.as_mut_ptr()),
                pOutputFile: PWSTR(ptr::null_mut()),
                pDatatype: PWSTR(datatype.as_mut_ptr()),
            };
            
            let job_id = StartDocPrinterW(h_printer, 1, &doc_info);
            if job_id == 0 {
                let _ = ClosePrinter(h_printer);
                return Err("Failed to start document".to_string());
            }
            
            let page_result = StartPagePrinter(h_printer);
            if !page_result.as_bool() {
                let _ = EndDocPrinter(h_printer);
                let _ = ClosePrinter(h_printer);
                return Err("Failed to start page".to_string());
            }
            
            // Write raw data
            let mut bytes_written: u32 = 0;
            let write_result = WritePrinter(
                h_printer,
                data.as_ptr() as *const _,
                data.len() as u32,
                &mut bytes_written,
            );
            
            if !write_result.as_bool() {
                let _ = EndPagePrinter(h_printer);
                let _ = EndDocPrinter(h_printer);
                let _ = ClosePrinter(h_printer);
                return Err("Failed to write to printer".to_string());
            }
            
            let _ = EndPagePrinter(h_printer);
            let _ = EndDocPrinter(h_printer);
            let _ = ClosePrinter(h_printer);
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        use std::io::Write;
        
        let mut child = Command::new("lpr")
            .arg("-P")
            .arg(printer_name)
            .arg("-o")
            .arg("raw")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn lpr: {}", e))?;
        
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(data)
                .map_err(|e| format!("Failed to write to lpr: {}", e))?;
        }
        
        let status = child.wait()
            .map_err(|e| format!("Failed to wait for lpr: {}", e))?;
        
        if !status.success() {
            return Err("lpr command failed".to_string());
        }
    }
    
    Ok(())
}

// Print receipt using HTML rendering (for Arabic support)
// This uses Windows GDI to render Arabic text properly - same as Electron!
#[tauri::command]
async fn print_receipt_html(app: tauri::AppHandle, _printer_name: String) -> Result<String, String> {
    // Generate unique label to avoid conflicts
    let label = format!("print-receipt-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    
    // Create a small minimized window (user will only see print dialog)
    let webview = tauri::WebviewWindowBuilder::new(
        &app,
        label.clone(),
        tauri::WebviewUrl::App("print-receipt.html".into())
    )
    .title("Printing Receipt...")
    .inner_size(400.0, 600.0)
    .visible(false) // Hidden - only print dialog will show
    .skip_taskbar(true)
    .build()
    .map_err(|e| format!("Failed to create print window: {}", e))?;
    
    // Wait for page to load
    tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
    
    // Trigger print dialog automatically
    webview.eval("window.print();")
        .map_err(|e| format!("Failed to trigger print: {}", e))?;
    
    // Close window after 3 seconds (gives time for print job to be submitted)
    let app_handle = app.clone();
    let window_label = label.clone();
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        if let Some(window) = app_handle.get_webview_window(&window_label) {
            let _ = window.close();
        }
    });
    
    Ok("✅ Print dialog opened! Select NCR 7197 and click Print. (Window will auto-close)".to_string())
}

// TRULY SILENT printing using Windows GDI directly (no dialogs!)
#[tauri::command]
fn print_receipt_silent(printer_name: String) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::{HANDLE, RECT};
        use windows::Win32::Graphics::Gdi::{
            CreateDCW, DeleteDC, CreateFontW, SelectObject, DeleteObject,
            SetTextAlign, SetBkMode, GetDeviceCaps, DrawTextW,
            HDC, HORZRES, VERTRES, LOGPIXELSY,
            FW_NORMAL, FW_BOLD, ARABIC_CHARSET, OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS, DEFAULT_QUALITY, DEFAULT_PITCH, FF_DONTCARE,
            TA_RIGHT, TA_RTLREADING, TRANSPARENT, DT_CENTER, DT_RTLREADING,
        };
        use windows::Win32::Graphics::Printing::{
            OpenPrinterW, ClosePrinter, StartDocPrinterW, EndDocPrinter,
            StartPagePrinter, EndPagePrinter, DOC_INFO_1W,
        };
        use windows::core::PWSTR;
        use std::ptr;
        
        unsafe {
            let printer_name_wide: Vec<u16> = printer_name.encode_utf16().chain(std::iter::once(0)).collect();
            let mut h_printer: HANDLE = HANDLE::default();
            
            // Open printer
            let result = OpenPrinterW(
                PWSTR(printer_name_wide.as_ptr() as *mut u16),
                &mut h_printer,
                None,
            );
            
            if result.is_err() {
                return Err(format!("Failed to open printer: {}", printer_name));
            }
            
            // Create GDI device context for the printer
            let h_dc = CreateDCW(
                None,
                PWSTR(printer_name_wide.as_ptr() as *mut u16),
                None,
                None,
            );
            
            if h_dc.is_invalid() {
                let _ = ClosePrinter(h_printer);
                return Err("Failed to create printer device context".to_string());
            }
            
            // Start document with NULL datatype (allows GDI rendering!)
            let mut doc_name: Vec<u16> = "Receipt\0".encode_utf16().collect();
            
            let doc_info = DOC_INFO_1W {
                pDocName: PWSTR(doc_name.as_mut_ptr()),
                pOutputFile: PWSTR(ptr::null_mut()),
                pDatatype: PWSTR(ptr::null_mut()), // NULL = EMF (GDI graphics mode)
            };
            
            let job_id = StartDocPrinterW(h_printer, 1, &doc_info);
            if job_id == 0 {
                let error = std::io::Error::last_os_error();
                let _ = DeleteDC(h_dc);
                let _ = ClosePrinter(h_printer);
                return Err(format!("Failed to start document: {} (error code: {})", error, error.raw_os_error().unwrap_or(0)));
            }
            
            // Start page
            let page_result = StartPagePrinter(h_printer);
            if !page_result.as_bool() {
                let error = std::io::Error::last_os_error();
                let _ = EndDocPrinter(h_printer);
                let _ = DeleteDC(h_dc);
                let _ = ClosePrinter(h_printer);
                return Err(format!("Failed to start page: {} (error code: {})", error, error.raw_os_error().unwrap_or(0)));
            }
            
            // Set up font for Arabic text
            // Using Tahoma which has excellent Arabic support
            let font_name: Vec<u16> = "Tahoma\0".encode_utf16().collect();
            
            // Calculate font size in pixels (manual MulDiv: 14pt * DPI / 72)
            let dpi = GetDeviceCaps(h_dc, LOGPIXELSY);
            let font_height = -(14 * dpi / 72); // Negative for char height
            
            let h_font = CreateFontW(
                font_height,
                0,
                0,
                0,
                FW_NORMAL.0 as i32,
                0,
                0,
                0,
                ARABIC_CHARSET.0 as u32,
                OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32,
                DEFAULT_QUALITY.0 as u32,
                (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
                PWSTR(font_name.as_ptr() as *mut u16),
            );
            
            let old_font = SelectObject(h_dc, h_font);
            
            // Set text alignment for RTL
            SetTextAlign(h_dc, TA_RIGHT | TA_RTLREADING);
            SetBkMode(h_dc, TRANSPARENT);
            
            // Get page dimensions
            let page_width = GetDeviceCaps(h_dc, HORZRES);
            let _page_height = GetDeviceCaps(h_dc, VERTRES);
            let margin_x = page_width / 10; // 10% margins
            let mut y_pos = 100; // Start position
            let line_height = 60; // Space between lines
            
            // Helper function to print centered text
            let print_centered_text = |dc: HDC, text: &str, y: i32| {
                let mut text_wide: Vec<u16> = text.encode_utf16().collect();
                let mut rect = RECT {
                    left: margin_x,
                    top: y,
                    right: page_width - margin_x,
                    bottom: y + line_height,
                };
                DrawTextW(
                    dc,
                    &mut text_wide,
                    &mut rect,
                    DT_CENTER | DT_RTLREADING,
                );
            };
            
            // Print receipt content
            // Store name
            print_centered_text(h_dc, "متجر عينة", y_pos);
            y_pos += line_height;
            
            // Address
            print_centered_text(h_dc, "123 شارع الرئيسي", y_pos);
            y_pos += line_height;
            
            print_centered_text(h_dc, "المدينة، المحافظة 12345", y_pos);
            y_pos += line_height;
            
            print_centered_text(h_dc, "هاتف: (555) 123-4567", y_pos);
            y_pos += line_height + 20;
            
            // Divider
            print_centered_text(h_dc, "================================", y_pos);
            y_pos += line_height;
            
            // Items header
            print_centered_text(h_dc, "الأصناف", y_pos);
            y_pos += line_height;
            
            print_centered_text(h_dc, "================================", y_pos);
            y_pos += line_height + 10;
            
            // Items
            print_centered_text(h_dc, "تفاح", y_pos);
            y_pos += line_height - 20;
            print_centered_text(h_dc, "2x @ 2.50 ج.م = 5.00 ج.م", y_pos);
            y_pos += line_height + 10;
            
            print_centered_text(h_dc, "موز", y_pos);
            y_pos += line_height - 20;
            print_centered_text(h_dc, "3x @ 1.50 ج.م = 4.50 ج.م", y_pos);
            y_pos += line_height + 10;
            
            print_centered_text(h_dc, "برتقال", y_pos);
            y_pos += line_height - 20;
            print_centered_text(h_dc, "1x @ 3.00 ج.م = 3.00 ج.م", y_pos);
            y_pos += line_height + 20;
            
            // Divider
            print_centered_text(h_dc, "================================", y_pos);
            y_pos += line_height;
            
            // Totals
            print_centered_text(h_dc, "المجموع الفرعي: 7.00 ج.م", y_pos);
            y_pos += line_height;
            
            print_centered_text(h_dc, "الضريبة (10٪): 0.70 ج.م", y_pos);
            y_pos += line_height;
            
            print_centered_text(h_dc, "================================", y_pos);
            y_pos += line_height;
            
            // Total (bold - increase font size)
            let bold_font_height = -(18 * dpi / 72); // 18pt font
            let h_font_bold = CreateFontW(
                bold_font_height,
                0, 0, 0,
                FW_BOLD.0 as i32,
                0, 0, 0,
                ARABIC_CHARSET.0 as u32,
                OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32,
                DEFAULT_QUALITY.0 as u32,
                (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
                PWSTR(font_name.as_ptr() as *mut u16),
            );
            SelectObject(h_dc, h_font_bold);
            
            print_centered_text(h_dc, "الإجمالي: 7.70 ج.م", y_pos);
            y_pos += line_height + 20;
            
            // Back to normal font
            SelectObject(h_dc, h_font);
            
            print_centered_text(h_dc, "================================", y_pos);
            y_pos += line_height + 10;
            
            // Footer
            print_centered_text(h_dc, "شكراً لك على الشراء!", y_pos);
            y_pos += line_height;
            
            print_centered_text(h_dc, "نتمنى رؤيتك مرة أخرى", y_pos);
            
            // Cleanup fonts
            SelectObject(h_dc, old_font);
            let _ = DeleteObject(h_font);
            let _ = DeleteObject(h_font_bold);
            
            // End page and document (EMF mode sends GDI graphics to printer!)
            let end_page_result = EndPagePrinter(h_printer);
            if !end_page_result.as_bool() {
                let error = std::io::Error::last_os_error();
                eprintln!("⚠️ EndPagePrinter warning: {} (continuing...)", error);
            }
            
            let end_doc_result = EndDocPrinter(h_printer);
            if !end_doc_result.as_bool() {
                let error = std::io::Error::last_os_error();
                let _ = DeleteDC(h_dc);
                let _ = ClosePrinter(h_printer);
                return Err(format!("Failed to end document: {} (error code: {})", error, error.raw_os_error().unwrap_or(0)));
            }
            
            let _ = DeleteDC(h_dc);
            let _ = ClosePrinter(h_printer);
            
            println!("✅ Print job #{} completed successfully!", job_id);
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        return Err("Silent printing is only supported on Windows".to_string());
    }
    
    Ok(format!("✅ Receipt sent to printer! Check printer queue if nothing prints. (Job may be processing EMF data)"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, get_thermal_printers, print_receipt, print_receipt_image, print_receipt_html, print_receipt_silent])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
