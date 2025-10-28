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

#[tauri::command]
fn print_receipt(printer_name: String) -> Result<String, String> {
    // Generate ESC/POS commands for a sample receipt
    let mut commands = Vec::new();
    
    // ESC @ - Initialize printer
    commands.extend_from_slice(&[0x1B, 0x40]);
    
    // ESC a 1 - Center alignment
    commands.extend_from_slice(&[0x1B, 0x61, 0x01]);
    
    // GS ! 0x11 - Double height and width
    commands.extend_from_slice(&[0x1D, 0x21, 0x11]);
    
    // Print store name
    commands.extend_from_slice(b"SAMPLE STORE\n");
    
    // GS ! 0x00 - Normal size
    commands.extend_from_slice(&[0x1D, 0x21, 0x00]);
    
    commands.extend_from_slice(b"123 Main Street\n");
    commands.extend_from_slice(b"City, State 12345\n");
    commands.extend_from_slice(b"Tel: (555) 123-4567\n");
    
    // Line feed
    commands.push(0x0A);
    
    // ESC a 0 - Left alignment
    commands.extend_from_slice(&[0x1B, 0x61, 0x00]);
    
    commands.extend_from_slice(b"================================\n");
    commands.extend_from_slice(b"Item          Qty    Price\n");
    commands.extend_from_slice(b"================================\n");
    commands.extend_from_slice(b"Apple          2x    $2.50\n");
    commands.extend_from_slice(b"Banana         3x    $1.50\n");
    commands.extend_from_slice(b"Orange         1x    $3.00\n");
    commands.extend_from_slice(b"================================\n");
    commands.extend_from_slice(b"SUBTOTAL:              $7.00\n");
    commands.extend_from_slice(b"TAX (10%):             $0.70\n");
    commands.extend_from_slice(b"TOTAL:                 $7.70\n");
    
    // Line feed
    commands.push(0x0A);
    
    // ESC a 1 - Center alignment
    commands.extend_from_slice(&[0x1B, 0x61, 0x01]);
    
    commands.extend_from_slice(b"Thank you for your purchase!\n");
    
    // Line feeds
    commands.extend_from_slice(&[0x0A, 0x0A]);
    
    // GS V 0 - Full cut
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
        use std::fs::OpenOptions;
        use std::io::Write as IoWrite;
        
        // On Windows, write directly to the printer share
        let printer_path = format!("\\\\localhost\\{}", printer_name);
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&printer_path)
            .map_err(|e| format!("Failed to open printer: {}", e))?;
        
        file.write_all(&commands)
            .map_err(|e| format!("Failed to write to printer: {}", e))?;
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
    
    Ok("Receipt printed successfully!".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, get_thermal_printers, print_receipt])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
