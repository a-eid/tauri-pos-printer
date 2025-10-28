use printers::get_printers;
use serde::{Deserialize, Serialize};
use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, SwashCache, fontdb};
use image::{GrayImage, Luma};
use once_cell::sync::Lazy;
use std::sync::Mutex;

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

// Global font system for text rendering with Arabic support
static FONT_SYSTEM: Lazy<Mutex<FontSystem>> = Lazy::new(|| {
    let mut db = fontdb::Database::new();
    db.load_system_fonts(); // Load all Windows fonts including Arabic ones
    Mutex::new(FontSystem::new_with_locale_and_db("ar".to_string(), db))
});

// Generate ESC/POS GS v 0 command for printing bitmap
fn escpos_raster_command(width_px: u32, height_px: u32, bitmap: &[u8]) -> Vec<u8> {
    let mut cmd = Vec::new();
    
    // GS v 0 m xL xH yL yH [data]
    cmd.push(0x1D); // GS
    cmd.push(b'v');
    cmd.push(0x30); // '0'
    cmd.push(0x00); // m = 0 (normal mode)
    
    // Width in bytes
    let width_bytes = (width_px + 7) / 8;
    cmd.push((width_bytes & 0xFF) as u8); // xL
    cmd.push(((width_bytes >> 8) & 0xFF) as u8); // xH
    
    // Height in pixels
    cmd.push((height_px & 0xFF) as u8); // yL
    cmd.push(((height_px >> 8) & 0xFF) as u8); // yH
    
    // Bitmap data
    cmd.extend_from_slice(bitmap);
    
    cmd
}

// Render Arabic text to grayscale bitmap
fn render_text_to_bitmap(text: &str, width_px: u32, font_size: f32) -> (Vec<u8>, u32, u32) {
    let mut font_system = FONT_SYSTEM.lock().unwrap();
    let mut swash_cache = SwashCache::new();
    
    // Create buffer with metrics
    let metrics = Metrics::new(font_size, font_size * 1.2);
    let mut buffer = Buffer::new(&mut font_system, metrics);
    buffer.set_size(&mut font_system, Some(width_px as f32), Some(f32::INFINITY));
    
    // Set text with RTL support
    let attrs = Attrs::new();
    buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);
    
    // Layout the text
    buffer.shape_until_scroll(&mut font_system, false);
    
    // Calculate required height
    let mut max_height = 0;
    for run in buffer.layout_runs() {
        max_height = max_height.max((run.line_y + metrics.line_height) as u32);
    }
    
    let height = max_height + 10; // Add padding
    let mut img = GrayImage::new(width_px, height);
    
    // Draw text
    for run in buffer.layout_runs() {
        for glyph in run.glyphs.iter() {
            let physical_glyph = glyph.physical((0.0, 0.0), 1.0);
            
            if let Some(image) = swash_cache.get_image(&mut font_system, physical_glyph.cache_key) {
                let x = physical_glyph.x as i32;
                let y = (run.line_y as i32) + physical_glyph.y;
                
                // Draw glyph
                for (gx, gy, pixel) in image.data.iter().enumerate().filter_map(|(i, &p)| {
                    if p > 0 {
                        let gx = (i % image.placement.width as usize) as i32;
                        let gy = (i / image.placement.width as usize) as i32;
                        Some((gx, gy, p))
                    } else {
                        None
                    }
                }) {
                    let px = x + gx;
                    let py = y + gy;
                    if px >= 0 && px < width_px as i32 && py >= 0 && py < height as i32 {
                        img.put_pixel(px as u32, py as u32, Luma([255 - pixel]));
                    }
                }
            }
        }
    }
    
    // Convert to grayscale Vec
    (img.to_vec(), width_px, height)
}

// Convert grayscale to 1-bit packed bitmap (MSB first)
fn to_1bpp_packed(width: u32, height: u32, gray: &[u8]) -> Vec<u8> {
    let bytes_per_row = (width + 7) / 8;
    let mut packed = vec![0u8; (bytes_per_row * height) as usize];
    
    for y in 0..height {
        for x in 0..width {
            let gray_val = gray[(y * width + x) as usize];
            // Threshold: > 128 = white (0), <= 128 = black (1)
            if gray_val <= 128 {
                let byte_idx = (y * bytes_per_row + x / 8) as usize;
                let bit_idx = 7 - (x % 8);
                packed[byte_idx] |= 1 << bit_idx;
            }
        }
    }
    
    packed
}

#[tauri::command]
fn print_receipt(printer_name: String) -> Result<String, String> {
    // Bitmap-based Arabic printing - renders text as images with proper RTL & shaping
    let mut commands = Vec::new();
    let width_px = 576; // 80mm printer ~72dpi
    
    // ESC @ - Initialize printer
    commands.extend_from_slice(&[0x1B, 0x40]);
    
    // Store name (large font, centered)
    let (gray, w, h) = render_text_to_bitmap("متجر عينة", width_px, 36.0);
    let bitmap = to_1bpp_packed(w, h, &gray);
    commands.extend(escpos_raster_command(w, h, &bitmap));
    commands.push(0x0A); // Line feed
    
    // Address lines (normal font)
    for line in ["123 شارع الرئيسي", "المدينة، المحافظة 12345", "هاتف: (555) 123-4567"] {
        let (gray, w, h) = render_text_to_bitmap(line, width_px, 24.0);
        let bitmap = to_1bpp_packed(w, h, &gray);
        commands.extend(escpos_raster_command(w, h, &bitmap));
        commands.push(0x0A);
    }
    
    commands.push(0x0A); // Extra space
    
    // Divider (plain ASCII - no need for bitmap)
    commands.extend_from_slice(b"================================\n");
    
    // Items header
    let (gray, w, h) = render_text_to_bitmap("الصنف          الكمية    السعر", width_px, 24.0);
    let bitmap = to_1bpp_packed(w, h, &gray);
    commands.extend(escpos_raster_command(w, h, &bitmap));
    commands.push(0x0A);
    
    commands.extend_from_slice(b"================================\n");
    
    // Items with prices
    let items = vec![
        "تفاح                2x    2.50 ج.م",
        "موز                 3x    1.50 ج.م",
        "برتقال              1x    3.00 ج.م",
    ];
    
    for item in items {
        let (gray, w, h) = render_text_to_bitmap(item, width_px, 24.0);
        let bitmap = to_1bpp_packed(w, h, &gray);
        commands.extend(escpos_raster_command(w, h, &bitmap));
        commands.push(0x0A);
    }
    
    commands.extend_from_slice(b"================================\n");
    
    // Totals
    let totals = vec![
        "المجموع الفرعي:        7.00 ج.م",
        "الضريبة (10٪):         0.70 ج.م",
    ];
    
    for total in totals {
        let (gray, w, h) = render_text_to_bitmap(total, width_px, 24.0);
        let bitmap = to_1bpp_packed(w, h, &gray);
        commands.extend(escpos_raster_command(w, h, &bitmap));
        commands.push(0x0A);
    }
    
    // Grand total (larger font to emphasize)
    let (gray, w, h) = render_text_to_bitmap("الإجمالي:             7.70 ج.م", width_px, 32.0);
    let bitmap = to_1bpp_packed(w, h, &gray);
    commands.extend(escpos_raster_command(w, h, &bitmap));
    commands.push(0x0A);
    commands.push(0x0A);
    
    // Footer
    let footer = vec![
        "شكراً لك على الشراء!",
        "نتمنى رؤيتك مرة أخرى",
    ];
    
    for line in footer {
        let (gray, w, h) = render_text_to_bitmap(line, width_px, 24.0);
        let bitmap = to_1bpp_packed(w, h, &gray);
        commands.extend(escpos_raster_command(w, h, &bitmap));
        commands.push(0x0A);
    }
    
    // Padding before cut
    commands.extend_from_slice(&[0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A]);
    
    // Paper cut
    commands.extend_from_slice(&[0x1D, 0x56, 0x00]);
    
    // Send to printer
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        use std::fs;
        use std::path::PathBuf;
        
        // Create a temporary file with the ESC/POS commands
        let temp_path = PathBuf::from("/tmp/receipt.bin");
        fs::write(&temp_path, &commands)
            .map_err(|e| format!("Failed to write temp file: {}", e))?;
        
        // Use lpr to send the file to the printer
        let output = Command::new("lpr")
            .arg("-P")
            .arg(&printer_name)
            .arg("-o")
            .arg("raw")
            .arg(&temp_path)
            .output()
            .map_err(|e| format!("Failed to execute lpr: {}", e))?;
        
        // Clean up temp file
        let _ = fs::remove_file(&temp_path);
        
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Print command failed: {}", error));
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::Graphics::Printing::{
            OpenPrinterW, StartDocPrinterW, StartPagePrinter, WritePrinter,
            EndPagePrinter, EndDocPrinter, ClosePrinter, DOC_INFO_1W,
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
                return Err(format!("Failed to open printer '{}'. Make sure the printer is installed and accessible.", printer_name));
            }
            
            // Set up document info - Use RAW mode for ESC/POS commands
            let mut doc_name: Vec<u16> = "Receipt\0".encode_utf16().collect();
            let mut datatype: Vec<u16> = "RAW\0".encode_utf16().collect();
            
            let doc_info = DOC_INFO_1W {
                pDocName: PWSTR(doc_name.as_mut_ptr()),
                pOutputFile: PWSTR(ptr::null_mut()),
                pDatatype: PWSTR(datatype.as_mut_ptr()),
            };
            
            // Start document
            let job_id = StartDocPrinterW(h_printer, 1, &doc_info);
            if job_id == 0 {
                let _ = ClosePrinter(h_printer);
                return Err("Failed to start print job".to_string());
            }
            
            // Start page
            let page_result = StartPagePrinter(h_printer);
            if !page_result.as_bool() {
                let _ = EndDocPrinter(h_printer);
                let _ = ClosePrinter(h_printer);
                return Err("Failed to start page".to_string());
            }
            
            // Write data
            let mut bytes_written: u32 = 0;
            let write_result = WritePrinter(
                h_printer,
                commands.as_ptr() as *const _,
                commands.len() as u32,
                &mut bytes_written,
            );
            
            if !write_result.as_bool() {
                let _ = EndPagePrinter(h_printer);
                let _ = EndDocPrinter(h_printer);
                let _ = ClosePrinter(h_printer);
                return Err("Failed to write to printer".to_string());
            }
            
            // End page and document
            let _ = EndPagePrinter(h_printer);
            let _ = EndDocPrinter(h_printer);
            let _ = ClosePrinter(h_printer);
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        use std::fs;
        use std::path::PathBuf;
        
        // Create a temporary file with the ESC/POS commands
        let temp_path = PathBuf::from("/tmp/receipt.bin");
        fs::write(&temp_path, &commands)
            .map_err(|e| format!("Failed to write temp file: {}", e))?;
        
        // Use lpr to send the file to the printer
        let output = Command::new("lpr")
            .arg("-P")
            .arg(&printer_name)
            .arg("-o")
            .arg("raw")
            .arg(&temp_path)
            .output()
            .map_err(|e| format!("Failed to execute lpr: {}", e))?;
        
        // Clean up temp file
        let _ = fs::remove_file(&temp_path);
        
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Print command failed: {}", error));
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
