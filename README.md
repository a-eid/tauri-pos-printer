# 🖨️ Thermal POS Printer App

A modern desktop application for printing receipts to thermal POS printers, built with Tauri, React, and TypeScript.

![Tauri](https://img.shields.io/badge/Tauri-2.0-blue)
![React](https://img.shields.io/badge/React-18-61dafb)
![TypeScript](https://img.shields.io/badge/TypeScript-5-3178c6)
![Windows](https://img.shields.io/badge/Platform-Windows-0078d6)

## ✨ Features

- 🖨️ **Thermal Printer Detection** - Automatically detects and filters thermal/POS printers
- 📋 **Printer Selection** - Easy dropdown to choose from available thermal printers
- 🧾 **Sample Receipt Printing** - Print professional market receipts with ESC/POS commands
- 🎨 **Modern UI** - Clean interface with dark/light mode support
- 🔄 **Refresh Printers** - Dynamically reload printer list without restarting
- ⚡ **Fast & Lightweight** - Built with Tauri for minimal resource usage

## 🚀 Quick Start - Building on GitHub

**No Rust installation needed!** This project uses GitHub Actions to build the Windows app automatically.

### Step 1: Push to GitHub

```bash
git init
git add .
git commit -m "Initial commit"
git branch -M main
git remote add origin https://github.com/YOUR_USERNAME/tauri-pos-printer.git
git push -u origin main
```

### Step 2: Get Your Windows App

1. Go to your repository on GitHub
2. Click the **Actions** tab
3. Wait for the build to complete (~5-10 minutes)
4. Download the installer from **Artifacts**

**Or manually trigger a build:**
- Go to Actions → "Build Windows App" → "Run workflow"

📖 **[See detailed build instructions →](BUILD_INSTRUCTIONS.md)**

## 📦 Supported Thermal Printers

The app automatically detects thermal printers from major brands:

- ✅ Epson (TM series)
- ✅ Star Micronics
- ✅ Bixolon
- ✅ Zebra
- ✅ Citizen
- ✅ Rongta
- ✅ XPrinter
- ✅ Any printer with "thermal", "pos", or "receipt" in the name

## 🖥️ Installation (Windows)

1. Download the installer from GitHub Actions artifacts or Releases
2. Run the `.exe` installer
3. Follow the installation wizard
4. Launch the app and connect your thermal printer
5. Select your printer and print test receipts!

## 🧾 Sample Receipt Format

The app prints a professional receipt in **Arabic** including:

```
         متجر عينة
    123 شارع الرئيسي
  المدينة، المحافظة 12345
  هاتف: (555) 123-4567

================================
الصنف          الكمية    السعر
================================
                          تفاح
         2.50 ج.م    2x
                           موز
         1.50 ج.م    3x
                        برتقال
         3.00 ج.م    1x
================================
المجموع الفرعي:        7.00 ج.م
الضريبة (10٪):         0.70 ج.م
الإجمالي:             7.70 ج.م

   شكراً لك على الشراء!
   نتمنى رؤيتك مرة أخرى
```

**Features:**
- ✅ Arabic language support with Windows-1256 encoding
- ✅ Right-to-left text alignment
- ✅ Egyptian Pound (ج.م) currency
- ✅ Bold formatting for important information

## 🛠️ Local Development (Optional)

If you want to run the app locally for development:

### Prerequisites

- [Node.js](https://nodejs.org/) (v18 or higher)
- [pnpm](https://pnpm.io/) package manager
- [Rust](https://www.rust-lang.org/tools/install) (for local development only)

### Setup

```bash
# Install dependencies
pnpm install

# Run in development mode
pnpm run tauri dev

# Build locally (if you have Rust installed)
pnpm run tauri build
```

## 📁 Project Structure

```
tauri-pos-printer/
├── src/                    # React frontend
│   ├── App.tsx            # Main UI component
│   ├── App.css            # Styling
│   └── main.tsx           # React entry point
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── lib.rs         # Tauri commands & printer logic
│   │   └── main.rs        # App entry point
│   └── Cargo.toml         # Rust dependencies
├── .github/
│   └── workflows/         # GitHub Actions
│       ├── build-windows.yml  # Auto-build on push
│       └── release.yml        # Create releases
└── BUILD_INSTRUCTIONS.md  # Detailed build guide
```

## 🔧 Technical Details

### ESC/POS Commands

The app uses standard ESC/POS commands for thermal printing:

- `ESC @` - Initialize printer
- `ESC a` - Text alignment (left/center/right)
- `GS !` - Text size (width/height)
- `GS V` - Paper cut

### Platform-Specific Printing

- **Windows**: Direct write to printer device (`\\localhost\PrinterName`)
- **macOS/Linux**: Uses `lpr` command with raw option

### Dependencies

**Frontend:**
- React 18
- TypeScript
- Vite

**Backend (Rust):**
- `tauri` - Desktop framework
- `printers` - System printer enumeration
- `serde` - Serialization

## 📚 Documentation

- [Build Instructions](BUILD_INSTRUCTIONS.md) - How to build using GitHub Actions
- [Thermal Printer Guide](THERMAL_PRINTER_GUIDE.md) - Setup and troubleshooting

## 🤝 Contributing

Contributions are welcome! Feel free to:

- Report bugs
- Suggest features
- Submit pull requests

## 📄 License

MIT License - feel free to use this project for personal or commercial purposes.

## ⚙️ GitHub Actions Workflows

This project includes automated workflows:

### `build-windows.yml`
Automatically builds the Windows app on every push to main/master.

### `release.yml`
Creates GitHub releases with installers when you push a version tag:

```bash
git tag v1.0.0
git push origin v1.0.0
```

## 🎯 Use Cases

- Point of Sale (POS) systems
- Retail stores
- Restaurants and cafes
- Ticket printing
- Order confirmations
- Any business needing thermal receipt printing

## 💡 Tips

- Use NSIS installer (`.exe`) for easiest installation
- Ensure your thermal printer is properly installed in Windows
- Check printer is online and has paper loaded
- Some printers may need "raw" or "generic/text" drivers

## 🆘 Troubleshooting

**No printers found?**
- Ensure printer is connected and turned on
- Check printer appears in Windows Settings → Printers & Scanners
- Printer name should contain keywords like "thermal", "pos", or brand name

**Print fails?**
- Verify printer is online (not paused)
- Check paper is loaded
- Ensure printer supports ESC/POS commands (most thermal printers do)

See [THERMAL_PRINTER_GUIDE.md](THERMAL_PRINTER_GUIDE.md) for more troubleshooting tips.

---

**Built with ❤️ using Tauri**

## 🌐 ESC/POS (Network) experiment

This app includes an optional experiment to print directly to a networked thermal printer using the `escpos` Rust crate.

- Where to find it: in the UI under "ESC/POS (Network) experiment)"
- How it works:
  - Text path: sends Arabic text via ESC/POS. The crate currently lacks Arabic page codes (e.g., PC864/WIN-1256), so text may appear as gibberish on some devices. This button is meant to observe your device’s behavior.
  - Image path: renders the Arabic receipt as a PNG on the frontend and prints it as a raster image via ESC/POS. This should print correctly on any ESC/POS-compatible printer.
  - Expected port: 9100 (raw), configurable in the UI.

Steps:
1) Enter your printer IP/host and port (default: 9100).
2) Click "📝 escpos Text" (observe behavior) or "🖼️ escpos Image" (expected correct Arabic display).

Note: We build release artifacts on GitHub Actions. If you don’t have local Rust/Cargo, push your changes and download the artifact from the workflow run to test on Windows.
