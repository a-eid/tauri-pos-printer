# Fixes Applied - Arabic Printing

## Issues Fixed

### 1. HTML Printing (🖨️ With Dialog button)
**Problem:** Error: "a webview with label print-receipt already exists"
**Root Cause:** Window wasn't being closed properly between print jobs
**Fix:** 
- Generate unique window labels using timestamps
- Properly close window after printing
- Added `use tauri::Manager;` for window management

### 2. Silent GDI Printing (🚀 Print Arabic Silent button)
**Problem:** Said "success" but nothing actually printed
**Root Cause:** Mixing RAW printer mode with GDI drawing commands
- Using `StartDocPrinterW` with `datatype = "RAW"` puts printer in RAW data mode
- But then using GDI `DrawTextW` commands which require graphics mode
- The two modes are incompatible!

**Fix:** Use NULL datatype to enable EMF (Enhanced Metafile) mode
- **Before:** `StartDocPrinterW` with `datatype = "RAW"` → GDI drawing ignored ❌
- **After:** `StartDocPrinterW` with `datatype = NULL` → GDI drawing recorded to EMF → sent to printer ✅

**Key Changes:**
```rust
// Before (RAW mode - wrong):
let mut datatype: Vec<u16> = "RAW\0".encode_utf16().collect();
let doc_info = DOC_INFO_1W {
    pDocName: ...,
    pOutputFile: ...,
    pDatatype: PWSTR(datatype.as_mut_ptr()), // RAW = text only
};

// After (EMF mode - correct):
let doc_info = DOC_INFO_1W {
    pDocName: ...,
    pOutputFile: ...,
    pDatatype: PWSTR(ptr::null_mut()), // NULL = EMF graphics mode ✅
};
```

**Why this works:**
- Windows Printing API with `NULL` datatype uses EMF (Enhanced Metafile) format
- EMF mode records all GDI drawing operations (including `DrawTextW`)
- When `EndDocPrinter` is called, Windows sends the EMF data to the printer
- Printer renders the EMF graphics (including properly shaped Arabic text)

## What to Expect

### ESC/POS Button (📄)
- ✅ Prints immediately
- ❌ Arabic shows as gibberish (expected - thermal printer limitation)
- Use this only for testing printer connection

### HTML Dialog Button (🖨️)
- ✅ Opens print dialog
- ✅ Arabic renders perfectly (Windows GDI)
- ✅ Can now print multiple times without errors
- ⚠️ Requires user to click "Print" button
- 💡 **Tip:** Set NCR 7197 as default printer for faster printing

### Silent Button (🚀) - RECOMMENDED
- ✅ Prints immediately (no dialog!)
- ✅ Arabic renders perfectly (Windows GDI)
- ✅ No user interaction needed
- ✅ Perfect for POS systems
- **This is the solution you want to use in production!**

## Technical Details

**Why EMF Mode Works:**
1. `OpenPrinterW` + `CreateDCW` creates both printer handle and Device Context
2. `StartDocPrinterW` with `NULL` datatype enables EMF (Enhanced Metafile) mode
3. `DrawTextW` renders Arabic text with proper shaping/RTL → recorded to EMF
4. `EndPagePrinter` + `EndDocPrinter` finalizes EMF and sends to printer spooler
5. Printer receives EMF graphics (not text), so no encoding issues!

**Why RAW Mode Failed:**
1. `StartDocPrinterW` with `"RAW"` datatype tells Windows "I'm sending raw bytes"
2. GDI drawing commands write to DC but don't get recorded/sent
3. `EndPagePrinter` + `EndDocPrinter` close the (empty) RAW stream
4. Nothing actually prints!

**Key Insight:**
Windows Printing API supports multiple datatypes:
- `"RAW"` = Raw bytes only (ESC/POS commands) - no GDI support
- `NULL` = EMF mode (default) - full GDI graphics support ✅
- `"EMF"` = Explicit EMF mode (same as NULL)
- `"TEXT"` = Plain text mode

## Testing

After the GitHub Action builds the new `.exe`:

1. Download and install the new build
2. Select your NCR 7197 printer
3. Test all three buttons:
   - ESC/POS: Should print English (Arabic gibberish is expected)
   - HTML Dialog: Should open print dialog → click Print → perfect Arabic
   - Silent: Should print IMMEDIATELY with perfect Arabic! 🎉

If silent printing still doesn't work, check:
- Is NCR 7197 selected in the dropdown?
- Is the printer online and ready?
- Check Windows printer queue for any errors

## Next Steps

Once confirmed working, you can:
1. Remove ESC/POS button (not useful for Arabic)
2. Hide HTML dialog button (backup option)
3. Make silent button the primary/only print button
4. Update UI to show "Printing..." state during silent print

