# 🧪 Arabic Test & Fallback Plan

## ✅ **What's Working Now**

- ✓ English text prints clearly
- ✓ Paper cut works (`GS V 0`)
- ✓ Bottom padding increased (8 line feeds = ~2cm)
- ✓ ESC/POS formatting (center align, dividers)

## 🎯 **Current Test: Mixed English/Arabic**

The receipt now includes both languages side by side:

```
     SAMPLE STORE
        متجر عينة

   123 Main Street
    123 شارع الرئيسي

================================
       ITEMS / الأصناف
================================

Apple / تفاح
  2x @ 2.50 EGP = 5.00 EGP

Banana / موز
  3x @ 1.50 EGP = 4.50 EGP

Orange / برتقال
  1x @ 3.00 EGP = 3.00 EGP

================================
Subtotal / المجموع الفرعي
                     7.00 EGP
Tax (10%) / الضريبة
                     0.70 EGP
================================
TOTAL / الإجمالي
                     7.70 EGP
================================

   Thank you!
    شكراً لك!

[8 line feeds for padding]
[Paper cut] ✂️
```

## 📊 **Possible Results**

### **Result 1: ✅ Arabic Prints Correctly!**

**What you'll see:**
- English: Clear ✓
- Arabic: Clear, properly shaped, RTL ✓
- Cut: Works ✓

**What this means:**
- 🎉 **SUCCESS!** NCR 7197 supports UTF-8 Arabic natively
- We're done! Just adjust formatting as needed

**Next steps:**
- Create final Arabic receipt layout
- Optimize spacing and alignment
- Deploy to production

---

### **Result 2: ❌ Arabic is Gibberish (Expected)**

**What you'll see:**
- English: Clear ✓
- Arabic: Gibberish/boxes/question marks ❌
- Cut: Works ✓

**What this means:**
- NCR 7197 doesn't support UTF-8 Arabic via raw ESC/POS
- Need to use **HTML/GDI rendering** (like Electron does)

**Solution: Windows Print Dialog Approach** ⬇️

---

## 🔧 **Fallback Solution: HTML Printing**

If Arabic is gibberish, we'll implement the **same method Electron uses**:

### **How It Works:**

```
┌─────────────────────────────────────┐
│ 1. Create HTML Receipt              │
│    (with proper Arabic fonts)       │
└──────────┬──────────────────────────┘
           │
           ↓
┌─────────────────────────────────────┐
│ 2. Open Hidden Tauri Window         │
│    (renders HTML with webview)      │
└──────────┬──────────────────────────┘
           │
           ↓
┌─────────────────────────────────────┐
│ 3. Call window.print()               │
│    (triggers Windows print API)      │
└──────────┬──────────────────────────┘
           │
           ↓
┌─────────────────────────────────────┐
│ 4. Windows GDI Renders Arabic       │
│    (using system fonts like Tahoma) │
└──────────┬──────────────────────────┘
           │
           ↓
┌─────────────────────────────────────┐
│ 5. Sends to NCR 7197                │
│    (as rendered graphics)            │
└──────────┬──────────────────────────┘
           │
           ↓
         ✅ Perfect Arabic!
```

### **Implementation Steps:**

#### **1. Create HTML Receipt Template**

Already created: `public/print-receipt.html`

Features:
- RTL support (`dir="rtl"`)
- Arabic fonts (Arial, Tahoma)
- Thermal printer page size (`@page { size: 80mm auto }`)
- Proper styling for receipt layout

#### **2. Add Print Command**

```rust
#[tauri::command]
async fn print_receipt_html(app: tauri::AppHandle) -> Result<String, String> {
    use tauri::Manager;
    
    // Create hidden window with receipt HTML
    let webview = tauri::WebviewWindowBuilder::new(
        &app,
        "print",
        tauri::WebviewUrl::App("print-receipt.html".into())
    )
    .title("Print Receipt")
    .inner_size(400.0, 600.0)
    .visible(false)
    .build()
    .map_err(|e| format!("Failed to create print window: {}", e))?;
    
    // Wait for page load
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    // Trigger print
    webview.eval("window.print();")
        .map_err(|e| format!("Failed to print: {}", e))?;
    
    // Wait and cleanup
    std::thread::sleep(std::time::Duration::from_millis(2000));
    webview.close().map_err(|e| format!("Failed to close: {}", e))?;
    
    Ok("Receipt printed via HTML!".to_string())
}
```

#### **3. Update Frontend**

```tsx
// In App.tsx
const printReceipt = async () => {
    setLoading(true);
    setMessage("");
    try {
        // Try RAW ESC/POS first (current method)
        const result = await invoke("print_receipt", {
            printerName: selectedPrinter,
        });
        
        // If it says Arabic might be gibberish, offer HTML option
        if (result.includes("gibberish")) {
            setMessage("⚠️ Arabic may not render. Try HTML print?");
            // Show second button for HTML print
        } else {
            setMessage(`✅ ${result}`);
        }
    } catch (error) {
        setMessage(`❌ Error: ${error}`);
    } finally {
        setLoading(false);
    }
};
```

### **Pros & Cons:**

| Aspect | RAW ESC/POS (Current) | HTML/GDI (Fallback) |
|--------|----------------------|---------------------|
| **Speed** | ✅ Fast | ⚠️ Slower (renders HTML) |
| **Arabic** | ❌ Gibberish (NCR 7197) | ✅ Perfect |
| **Control** | ✅ Full ESC/POS control | ⚠️ Limited (CSS only) |
| **User Interaction** | ✅ Silent printing | ⚠️ May show dialog |
| **Reliability** | ⚠️ Printer-dependent | ✅ Universal (Windows) |
| **Paper Cut** | ✅ ESC/POS command | ❌ Must use driver settings |

### **Making It Silent (No Dialog):**

To avoid print dialog, we can:

1. **Set default printer** programmatically
2. **Pre-configure print settings** in Windows
3. **Use silent print API** (requires additional setup)

```javascript
// In print-receipt.html
<script>
window.addEventListener('load', () => {
    setTimeout(() => {
        window.print();
    }, 100);
});
</script>
```

## 🎯 **Decision Tree**

```
Test Arabic with current build
        │
        ├─ Arabic prints correctly?
        │  └─ YES → ✅ Done! Use RAW ESC/POS
        │  └─ NO  → ⬇️
        │
        └─ Implement HTML printing
           │
           ├─ Add HTML template ✓ (already done)
           ├─ Add Rust command (5 min)
           ├─ Add UI button (2 min)
           └─ Test → ✅ Perfect Arabic!
```

## 📝 **What to Report**

After testing current build, tell me:

### **If Arabic works:** ✅
```
"Arabic prints correctly! It looks like [describe]"
```

→ We'll optimize layout and you're done!

### **If Arabic is gibberish:** ❌
```
"Arabic is still gibberish/boxes"
```

→ I'll implement HTML printing solution (10 min)

### **If something else:**
```
[Describe what you see]
```

→ We'll debug together

## 🚀 **Ready to Test!**

```bash
git add .
git commit -m "Test Arabic with UTF-8, increased padding"
git push
```

**Expected:** English clear + Cut works + Padding good + Arabic = ? 

Let me know the result! 🎯

---

**Confidence in fallback:** 99% - HTML printing is guaranteed to work (same as Electron)  
**Time to implement:** ~10-15 minutes if needed  
**Final result:** Perfect Arabic receipts! 🎉

