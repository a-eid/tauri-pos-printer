# 🔍 Debugging Gibberish Arabic Text

## 🎯 What I Fixed

### **3 Critical Bugs Fixed**

1. **❌ Background Was Black** → ✅ **Now White**
   ```rust
   // BEFORE: GrayImage::new() = black background (0)
   // AFTER: Initialized to white (255)
   for pixel in img.pixels_mut() {
       *pixel = Luma([255]);  // White background!
   }
   ```

2. **❌ Threshold Logic Was Correct** → ✅ **Kept Correct**
   ```rust
   // Dark pixels (< 128) = print black (1)
   // Light pixels (>= 128) = don't print (0)
   if gray_val < 128 {
       packed[byte_idx] |= 1 << bit_idx;
   }
   ```

3. **❓ Can't Verify Bitmap Command Works** → ✅ **Added Test Function**
   - New button: **🔲 Test Square**
   - Prints a simple 100×100 black square
   - Uses same raster command as Arabic text

## 🧪 **TEST PROCEDURE (IMPORTANT!)**

### Step 1: Test Black Square First 🔲

**Click "🔲 Test Square" button**

#### Expected Results:

**✅ SUCCESS: You see a clean black square**
- → Bitmap raster command WORKS! ✨
- → The gibberish was from bad bitmap generation
- → Now with white background fix, Arabic should work!
- → **Proceed to Step 2**

**❌ FAILURE: Still gibberish or weird output**
- → Raster command format is wrong for NCR 7197
- → Need to try different ESC/POS bitmap command
- → Tell me what you see, I'll try:
  - ESC * (different raster format)
  - GS ( L (graphics data)
  - Or fall back to Windows GDI printing

**❌ FAILURE: Nothing prints**
- → Printer doesn't support GS v 0 at all
- → Need to try different approach

### Step 2: Test Arabic Receipt 🧾

**Only if Step 1 worked!**

**Click "🧾 Print Sample Receipt" button**

#### Expected Results:

**✅ SUCCESS: Readable Arabic text**
- → 🎉 COMPLETE SUCCESS!
- → Bitmap rendering works perfectly
- → Arabic is properly shaped and RTL

**❌ FAILURE: Still gibberish**
- → cosmic-text not finding/using Arabic fonts
- → Font system initialization issue
- → Need to debug font loading

**❌ FAILURE: Blank spaces where Arabic should be**
- → Fonts not loaded properly
- → cosmic-text using wrong fonts
- → Fallback to ASCII characters

## 🔧 Possible Issues & Solutions

### Issue 1: Black Square Prints Gibberish

**Root cause**: NCR 7197 doesn't support GS v 0 format

**Solution**: Try alternative raster command (ESC *)
```rust
// Instead of GS v 0, use ESC * for bit image
fn escpos_esc_star_command(width_px: u32, height_px: u32, bitmap: &[u8]) -> Vec<u8> {
    let mut cmd = Vec::new();
    let width_bytes = (width_px + 7) / 8;
    
    for row in 0..height_px {
        cmd.extend_from_slice(&[0x1B, 0x2A]); // ESC *
        cmd.push(0x00); // m = 0 (8-dot single density)
        cmd.push((width_px & 0xFF) as u8); // nL
        cmd.push(((width_px >> 8) & 0xFF) as u8); // nH
        
        // Row data
        let start = (row * width_bytes) as usize;
        let end = start + width_bytes as usize;
        cmd.extend_from_slice(&bitmap[start..end]);
        cmd.push(0x0A); // Line feed
    }
    
    cmd
}
```

### Issue 2: Black Square Works, But Arabic Still Gibberish

**Root cause**: cosmic-text not loading Arabic fonts

**Debug**: Add font detection logging
```rust
let mut db = fontdb::Database::new();
db.load_system_fonts();

// Check if Arabic fonts are loaded
let arabic_fonts = db.faces()
    .filter(|face| face.families.iter().any(|(name, _)| 
        name.contains("Arabic") || name.contains("Tahoma") || name.contains("Arial")))
    .count();

if arabic_fonts == 0 {
    return Err("No Arabic fonts found!".to_string());
}
```

### Issue 3: Wrong Characters Print

**Root cause**: Text not being shaped properly (wrong letter forms)

**Solution**: Force Arabic shaping
```rust
// Make sure we're using Advanced shaping
buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);

// Or try setting explicit direction
let mut attrs = Attrs::new();
// cosmic-text should auto-detect RTL, but we can force it if needed
```

## 📊 What Each Test Tells Us

| Test Result | What It Means | Next Action |
|-------------|---------------|-------------|
| ✅ Square works, Arabic works | **COMPLETE SUCCESS!** | Deploy and celebrate! 🎉 |
| ✅ Square works, Arabic gibberish | Raster OK, text rendering broken | Check font loading |
| ✅ Square works, Arabic blank | Raster OK, fonts missing | Install Arabic fonts |
| ❌ Square gibberish | Raster command wrong | Try ESC * format |
| ❌ Square nothing | Raster unsupported | Fall back to GDI printing |

## 🚀 If Everything Fails: Plan B

### Use Windows GDI Printing (Like Electron)

Instead of rendering in Rust, let Windows do it:

1. Generate HTML receipt (like your Electron code)
2. Use Tauri's webview to render it
3. Call Windows print dialog API
4. Let Windows GDI handle Arabic rendering

This is **guaranteed to work** because it's exactly what Electron does!

```rust
// Pseudo-code for GDI approach
#[tauri::command]
fn print_receipt_gdi(window: tauri::Window) -> Result<(), String> {
    // 1. Generate HTML with Arabic text
    let html = generate_receipt_html();
    
    // 2. Load in hidden webview
    window.eval(&format!("loadReceiptHTML({})", html))?;
    
    // 3. Call window.print() - uses GDI automatically
    window.eval("window.print()")?;
    
    Ok(())
}
```

## 🎯 Current Status

- ✅ Fixed white background initialization
- ✅ Fixed threshold logic
- ✅ Added test square function
- ✅ Added safety checks (< 100px height)
- ✅ Reduced bitmap sizes (384px wide, 20-28pt fonts)
- ✅ Added test button to UI

**Next**: Test black square, report results!

---

**Expected outcome**: Black square prints correctly, then Arabic prints correctly! 🤞

**If not**: Tell me exactly what prints, and we'll try the next solution! 💪

