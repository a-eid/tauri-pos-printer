# 🔍 Debugging HTML Print

## 🎯 **What Changed**

Made the print window **VISIBLE** so you can see what's happening:

```rust
.visible(true)  // You'll see the receipt window now!
```

## 🧪 **What Should Happen**

When you click **"🖨️ Print Arabic Receipt"**:

### **✅ Success:**
1. A new window opens showing the receipt
2. You see **Arabic text rendered perfectly**
3. A blue "Print This Receipt" button appears at top
4. Print dialog automatically opens after 0.5 seconds
5. You can also click the button or press Ctrl+P to print

### **❌ If Window Shows Error:**
The error message will tell you what's wrong:
- "Failed to create print window" → HTML file not found
- "Failed to execute print script" → JavaScript error

### **❌ If Window Shows Blank:**
- HTML file loaded but has errors
- Check browser console (F12)

## 🔧 **Troubleshooting**

### **Issue 1: "Failed to create print window"**

**Cause:** `print-receipt.html` not found

**Fix:**
```bash
# Make sure the file exists
ls public/print-receipt.html

# If not, it's still in the project root
ls print-receipt.html

# If found in root, move it:
mv print-receipt.html public/
```

### **Issue 2: Window Opens But Blank**

**Cause:** HTML file not being served properly

**Fix:** Try using the index.html approach instead (see below)

### **Issue 3: Arabic Shows as Boxes**

**Cause:** Fonts not available

**Fix:** The HTML uses Arial and Tahoma which are built into Windows, so this shouldn't happen. But if it does:
```html
<!-- Edit public/print-receipt.html -->
<style>
  body {
    font-family: 'Segoe UI', 'Tahoma', 'Arial', sans-serif;
  }
</style>
```

## 🎯 **Alternative: Embed HTML in Rust**

If the file loading doesn't work, we can embed the HTML directly:

```rust
#[tauri::command]
async fn print_receipt_html(app: tauri::AppHandle, printer_name: String) -> Result<String, String> {
    use tauri::Manager;
    
    let html_content = r#"
<!DOCTYPE html>
<html dir="rtl">
<head>
    <meta charset="UTF-8">
    <style>
        @page { size: 80mm auto; margin: 0; }
        body {
            font-family: 'Arial', 'Tahoma', sans-serif;
            font-size: 12px;
            margin: 5mm;
            direction: rtl;
            text-align: center;
        }
        .large { font-size: 16px; font-weight: bold; }
        .divider { border-top: 1px dashed #000; margin: 5px 0; }
    </style>
</head>
<body>
    <div class="large">متجر عينة</div>
    <div>123 شارع الرئيسي</div>
    <div class="divider"></div>
    <div>تفاح - 2x @ 2.50 ج.م</div>
    <div>موز - 3x @ 1.50 ج.م</div>
    <div>برتقال - 1x @ 3.00 ج.م</div>
    <div class="divider"></div>
    <div class="large">الإجمالي: 7.70 ج.م</div>
    <div class="divider"></div>
    <div>شكراً لك!</div>
</body>
</html>
    "#;
    
    // Use data URL instead of file
    let data_url = format!("data:text/html;charset=utf-8,{}", 
        urlencoding::encode(html_content));
    
    let webview = tauri::WebviewWindowBuilder::new(
        &app,
        "print-receipt",
        tauri::WebviewUrl::External(data_url.parse().unwrap())
    )
    .title("Receipt Preview")
    .inner_size(400.0, 700.0)
    .visible(true)
    .center()
    .build()
    .map_err(|e| format!("Failed to create window: {}", e))?;
    
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    
    webview.eval("setTimeout(() => window.print(), 500);")
        .map_err(|e| format!("Failed to print: {}", e))?;
    
    Ok("Receipt window opened!".to_string())
}
```

**Note:** This requires adding `urlencoding = "2"` to Cargo.toml

## 📊 **What to Report**

After clicking **"🖨️ Print Arabic Receipt"**, tell me:

### **Scenario 1: Window Opens with Arabic** ✅
```
"Window opened! I can see Arabic text:
- متجر عينة shows correctly
- Print button appeared
- Print dialog opened"
```

→ **SUCCESS!** Just need to hide the window and make it automatic

### **Scenario 2: Window Opens, Blank/Error**
```
"Window opened but shows: [describe what you see]"
```

→ We'll fix the HTML loading

### **Scenario 3: Error Message**
```
"Got error: [paste error message]"
```

→ We'll fix the path/configuration

### **Scenario 4: Nothing Happens**
```
"Clicked button, nothing happened, no error message"
```

→ JavaScript/async issue, we'll add logging

## 🎯 **Expected Flow**

```
Click Button
    ↓
[You see: "Opening print dialog..."]
    ↓
New Window Opens (400x700px)
    ↓
You See:
  ┌────────────────────────────┐
  │ [Print This Receipt] button│ ← Blue button at top
  │                             │
  │        متجر عينة           │ ← Arabic!
  │    123 شارع الرئيسي        │
  │ ────────────────────────── │
  │         الأصناف            │
  │ ────────────────────────── │
  │ تفاح                       │
  │   2x @ 2.50 ج.م           │
  │ ...                        │
  └────────────────────────────┘
    ↓
Print Dialog Opens Automatically
    ↓
Select Printer → Print
    ↓
✅ Perfect Arabic Receipt!
```

## 🚀 **Build & Test**

```bash
git add .
git commit -m "Make print window visible for debugging"
git push
```

Then test and report what you see!

---

**Key Changes:**
- ✅ Window now VISIBLE (not hidden)
- ✅ Added blue Print button
- ✅ Auto-triggers print dialog
- ✅ Better error messages
- ✅ Window stays open (you can reprint)

**This will help us see exactly what's happening!** 🔍

