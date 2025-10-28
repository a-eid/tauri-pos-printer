# Bitmap Arabic Printing - Implementation Plan

## ✅ What I've Started

1. **Added Dependencies** (`Cargo.toml`):
   - `cosmic-text` - Arabic shaping & RTL support
   - `fontdb` - Font loading
   - `image` - Bitmap manipulation
   - `ab_glyph` - Font rasterization

2. **Created Helper Functions** (`lib.rs`):
   - `get_font_system()` - Loads Windows Arabic fonts (Tahoma, Arial, Segoe UI)
   - `render_text_to_bitmap()` - Renders text to grayscale image with proper RTL/shaping
   - `to_1bpp_packed()` - Converts grayscale to 1-bit packed bitmap for thermal printers

## 🚧 What Needs to be Done

### Step 1: Update `print_receipt()` Function
Replace the current text-based approach with:

```rust
#[tauri::command]
fn print_receipt(printer_name: String) -> Result<String, String> {
    let mut commands = Vec::new();
    
    // ESC @ - Initialize
    commands.extend_from_slice(&[0x1B, 0x40]);
    
    // Render and print each line as bitmap
    let width_px = 576; // 80mm printer ~72dpi
    
    // Store name (larger font)
    let (gray, w, h) = render_text_to_bitmap("متجر عينة", width_px, 32.0);
    let bitmap = to_1bpp_packed(w, h, &gray);
    commands.extend(escpos_raster_command(w, h, &bitmap));
    
    // Address lines
    let lines = vec![
        "123 شارع الرئيسي",
        "المدينة، المحافظة 12345",
        "هاتف: (555) 123-4567",
    ];
    
    for line in lines {
        let (gray, w, h) = render_text_to_bitmap(line, width_px, 24.0);
        let bitmap = to_1bpp_packed(w, h, &gray);
        commands.extend(escpos_raster_command(w, h, &bitmap));
    }
    
    // ... continue for all receipt sections
    
    // Send to printer (existing Windows code)
    // ...
}
```

### Step 2: Implement ESC/POS Raster Command

```rust
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
```

### Step 3: Fix Font System (Currently has issue)

The current `get_font_system()` has a problem - it creates a new FontSystem each time instead of reusing. Fix:

```rust
use once_cell::sync::Lazy;
use std::sync::Mutex;

static FONT_SYSTEM: Lazy<Mutex<FontSystem>> = Lazy::new(|| {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    Mutex::new(FontSystem::new_with_locale_and_db("ar".to_string(), db))
});
```

Add to `Cargo.toml`:
```toml
once_cell = "1.19"
```

### Step 4: Test & Debug

1. Build and test basic bitmap generation
2. Verify Arabic text renders correctly (RTL, shaping)
3. Test on actual NCR 7197 printer
4. Adjust font sizes for optimal readability

## 📋 Complete Code Structure

```
src-tauri/
├── Cargo.toml (updated with dependencies)
└── src/
    ├── lib.rs
    │   ├── get_thermal_printers() ✅
    │   ├── get_font_system() ✅ (needs fix)
    │   ├── render_text_to_bitmap() ✅
    │   ├── to_1bpp_packed() ✅
    │   ├── escpos_raster_command() ❌ TODO
    │   └── print_receipt() ❌ TODO (needs complete rewrite)
    └── main.rs ✅
```

## 🎯 Benefits of This Approach

1. ✅ **Proper Arabic rendering** - cosmic-text handles RTL & shaping
2. ✅ **Works on ALL printers** - bitmap printing is universal
3. ✅ **No encoding issues** - it's just pixels
4. ✅ **Professional quality** - uses TrueType fonts
5. ✅ **Fast** - renders in Rust, not slower than Electron GDI

## ⚠️ Current Status

- Dependencies: ✅ Added
- Helper functions: ✅ Created (with one fix needed)
- ESC/POS raster command: ❌ Not implemented
- print_receipt rewrite: ❌ Not started
- Testing: ❌ Not done

## 🔧 Next Action

I can complete the implementation by:
1. Adding `once_cell` dependency
2. Fixing `get_font_system()`
3. Implementing `escpos_raster_command()`
4. Rewriting `print_receipt()` to use bitmap rendering

**Estimated time:** ~30-45 minutes to complete + test

Should I continue with the full implementation?

