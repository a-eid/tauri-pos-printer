# Bitmap Arabic Printing - Current Status

## ✅ Completed

1. **Dependencies Added** (`Cargo.toml`):
   - `cosmic-text` 0.12 - Arabic shaping & RTL
   - `fontdb` 0.21 - Font loading
   - `image` 0.25 - Bitmap manipulation  
   - `ab_glyph` 0.2 - Font rasterization
   - `once_cell` 1.19 - Static initialization

2. **Core Functions Implemented** (`lib.rs`):
   - ✅ `FONT_SYSTEM` - Static lazy-initialized font system with Arabic support
   - ✅ `render_text_to_bitmap()` - Renders text to grayscale bitmap with RTL/shaping
   - ✅ `to_1bpp_packed()` - Converts grayscale to 1-bit packed bitmap
   - ✅ `escpos_raster_command()` - Generates ESC/POS GS v 0 bitmap command

## ⚠️ Issues to Fix

### 1. Compilation Errors Expected
The `print_receipt()` function still references `encode_utf8()` which was removed. This will cause compilation errors.

### 2. `print_receipt()` Needs Complete Rewrite
The function still uses the old text-based approach. It needs to be replaced with:

```rust
#[tauri::command]
fn print_receipt(printer_name: String) -> Result<String, String> {
    let mut commands = Vec::new();
    let width_px = 576; // 80mm printer at 72dpi
    
    // ESC @ - Initialize
    commands.extend_from_slice(&[0x1B, 0x40]);
    
    // Render each line as bitmap
    // Store name (large)
    let (gray, w, h) = render_text_to_bitmap("متجر عينة", width_px, 36.0);
    let bitmap = to_1bpp_packed(w, h, &gray);
    commands.extend(escpos_raster_command(w, h, &bitmap));
    
    // Address lines (normal size)
    for line in ["123 شارع الرئيسي", "المدينة، المحافظة 12345", "هاتف: (555) 123-4567"] {
        let (gray, w, h) = render_text_to_bitmap(line, width_px, 24.0);
        let bitmap = to_1bpp_packed(w, h, &gray);
        commands.extend(escpos_raster_command(w, h, &bitmap));
    }
    
    // Divider (plain text - no Arabic)
    commands.extend_from_slice(b"\n================================\n");
    
    // Items header
    let (gray, w, h) = render_text_to_bitmap("الصنف          الكمية    السعر", width_px, 24.0);
    let bitmap = to_1bpp_packed(w, h, &gray);
    commands.extend(escpos_raster_command(w, h, &bitmap));
    
    commands.extend_from_slice(b"================================\n");
    
    // Items
    for item in [("تفاح", "2x", "2.50 ج.م"), ("موز", "3x", "1.50 ج.م"), ("برتقال", "1x", "3.00 ج.م")] {
        let line = format!("{}    {}    {}", item.0, item.1, item.2);
        let (gray, w, h) = render_text_to_bitmap(&line, width_px, 24.0);
        let bitmap = to_1bpp_packed(w, h, &gray);
        commands.extend(escpos_raster_command(w, h, &bitmap));
    }
    
    commands.extend_from_slice(b"================================\n");
    
    // Totals
    for line in ["المجموع الفرعي:        7.00 ج.م", "الضريبة (10٪):         0.70 ج.م"] {
        let (gray, w, h) = render_text_to_bitmap(line, width_px, 24.0);
        let bitmap = to_1bpp_packed(w, h, &bitmap);
        commands.extend(escpos_raster_command(w, h, &bitmap));
    }
    
    // Total (large + bold - simulate with larger font)
    let (gray, w, h) = render_text_to_bitmap("الإجمالي:             7.70 ج.م", width_px, 32.0);
    let bitmap = to_1bpp_packed(w, h, &gray);
    commands.extend(escpos_raster_command(w, h, &bitmap));
    
    // Footer
    for line in ["شكراً لك على الشراء!", "نتمنى رؤيتك مرة أخرى"] {
        let (gray, w, h) = render_text_to_bitmap(line, width_px, 24.0);
        let bitmap = to_1bpp_packed(w, h, &gray);
        commands.extend(escpos_raster_command(w, h, &bitmap));
    }
    
    // Padding & cut
    commands.extend_from_slice(&[0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A]);
    commands.extend_from_slice(&[0x1D, 0x56, 0x00]); // Cut
    
    // Send to printer (existing platform-specific code remains)
    #[cfg(target_os = "windows")]
    {
        // ... Windows printing code ...
    }
    
    Ok("Receipt printed successfully!".to_string())
}
```

### 3. cosmic-text API Issues
The current `render_text_to_bitmap()` implementation may have API mismatches with cosmic-text 0.12. Need to verify:
- `Buffer::new()` signature
- `Buffer::set_size()` parameters
- `SwashCache` image format

## 📋 Next Steps to Complete

1. **Fix Compilation**:
   - Remove all `encode_utf8()` references
   - Rewrite `print_receipt()` function body completely

2. **Test Bitmap Rendering**:
   - Verify cosmic-text renders Arabic correctly
   - Check RTL order and shaping

3. **Test on Printer**:
   - Build on Windows
   - Test with NCR 7197
   - Adjust font sizes if needed

4. **Performance Optimization**:
   - Cache rendered bitmaps for common text
   - Reuse buffers to avoid allocations

## 🔧 Build Command

This will likely NOT compile yet due to the `encode_utf8()` references:

```bash
cd src-tauri
cargo build
```

Expected errors:
- `cannot find function encode_utf8 in this scope`

## 🎯 Estimated Time to Complete

- Fix compilation errors: ~15 minutes
- Test & debug bitmap rendering: ~30 minutes
- Test on real printer: ~15 minutes
- Polish & optimize: ~30 minutes

**Total: ~1.5 hours of focused work**

## ⚡ Why This Will Work

Unlike text encoding (UTF-8, Windows-1256, CP864) which all failed:
- Bitmap is just pixels - no encoding issues
- cosmic-text handles Arabic shaping & RTL in software
- All thermal printers support ESC/POS bitmap printing
- Same approach Electron uses (render → bitmap → print)

This is the **correct solution** - it just needs completion!

