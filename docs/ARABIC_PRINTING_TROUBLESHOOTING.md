# Arabic Printing Troubleshooting Guide

If you're still getting gibberish when printing Arabic text, your NCR 7197 thermal printer may need a different code page. Follow this guide to find the right one.

## Current Configuration

The app is currently set to use **UTF-8 (code page 65)**. This works on some modern thermal printers but not all.

## How to Test Different Code Pages

### Step 1: Locate the Code Page Setting

Open `src-tauri/src/lib.rs` and find this line (around line 63):

```rust
commands.extend_from_slice(&[0x1B, 0x74, 0x41]); // 65 = 0x41 for UTF-8
```

### Step 2: Try Different Code Pages

Replace the line above with ONE of these options and rebuild:

#### Option 1: UTF-8 (Current - Code Page 65)
```rust
commands.extend_from_slice(&[0x1B, 0x74, 0x41]); // 65 = 0x41
```
**When it works:** Modern printers, some Epson models

---

#### Option 2: CP864 (IBM Arabic - Code Page 17)
```rust
commands.extend_from_slice(&[0x1B, 0x74, 0x11]); // 17 = 0x11
```
**When it works:** Older thermal printers, IBM-compatible models
**Also change:** Use `encode_arabic()` instead of `encode_utf8()` (requires Windows-1256 encoding)

---

#### Option 3: Windows-1256 (Code Page 28)
```rust
commands.extend_from_slice(&[0x1B, 0x74, 0x1C]); // 28 = 0x1C
```
**When it works:** Windows-based POS systems, modern printers
**Also change:** Use `encode_arabic()` instead of `encode_utf8()` (requires Windows-1256 encoding)

---

#### Option 4: CP1256 (Code Page 16)
```rust
commands.extend_from_slice(&[0x1B, 0x74, 0x10]); // 16 = 0x10
```
**When it works:** Some Epson/Star models
**Also change:** Use `encode_arabic()` instead of `encode_utf8()`

---

#### Option 5: No Code Page Command (Use Printer Default)
```rust
// Comment out or remove the code page line entirely
// commands.extend_from_slice(&[0x1B, 0x74, 0x41]);
```
**When it works:** When printer is already configured for Arabic in its settings

---

#### Option 6: International Character Set (ESC R)
```rust
commands.extend_from_slice(&[0x1B, 0x52, 0x0D]); // 13 = Arabic
```
**When it works:** Some Epson models that use character sets instead of code pages

## Step 3: Change Encoding Function (If Needed)

If you're using code pages 17, 28, or 16 (NOT UTF-8), you also need to switch from `encode_utf8()` to `encode_arabic()`:

### Find and Replace:
Look for lines like:
```rust
commands.extend(encode_utf8("متجر عينة\n"));
```

Replace with:
```rust
commands.extend(encode_arabic("متجر عينة\n"));
```

**Do this for ALL Arabic text lines** (about 13 occurrences in the file).

## Step 4: Test

1. Make the change
2. Push to GitHub
3. Let GitHub Actions build
4. Download and test

## Recommended Testing Order

1. **Try UTF-8 (current)** - Works on modern printers
2. **Try CP864 (code page 17) with encode_arabic** - Most common for older thermal printers
3. **Try Windows-1256 (code page 28) with encode_arabic** - Common in POS systems
4. **Try no code page at all** - Let printer use its default
5. **Try ESC R command** - Alternative method for Epson

## NCR 7197 Specific Notes

The NCR 7197 is a thermal receipt printer. Based on similar models, it likely supports:
- ✅ **CP864** (IBM Arabic)
- ✅ **Windows-1256**
- ⚠️ **UTF-8** (may not be supported)

**Recommended:** Try **Code Page 17 (CP864)** with `encode_arabic()` function.

## Still Not Working?

If none of the above work, your printer may not support Arabic character sets at all. In this case, you would need to:

1. **Use English/Numbers only** - Simplest solution
2. **Print Arabic as bitmap images** - Complex, requires image generation
3. **Configure the printer firmware** - May need special tools or firmware update

## Example: Switching to CP864

### In `src-tauri/src/lib.rs`:

1. **Change line 63:**
```rust
// FROM:
commands.extend_from_slice(&[0x1B, 0x74, 0x41]); // 65 = 0x41 for UTF-8

// TO:
commands.extend_from_slice(&[0x1B, 0x74, 0x11]); // 17 = 0x11 for CP864
```

2. **Change all text encoding (13 places total):**
```rust
// FROM:
commands.extend(encode_utf8("متجر عينة\n"));

// TO:
commands.extend(encode_arabic("متجر عينة\n"));
```

Use Find & Replace in your editor:
- Find: `encode_utf8(`
- Replace: `encode_arabic(`

3. **Rebuild and test**

## Need Help?

If you're still having issues:
1. Check your printer's manual for supported code pages
2. Look for a printer configuration utility
3. Contact NCR support for code page documentation
4. Consider using English text as a fallback

---

**Current Status:** Using UTF-8 (code page 65) with direct UTF-8 byte encoding.

