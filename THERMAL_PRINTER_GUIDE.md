# Thermal POS Printer App

A Tauri-based application for printing receipts to thermal POS printers.

## Features

- **Thermal Printer Detection**: Automatically detects thermal printers connected to your system
- **Printer Selection**: Dropdown menu to select from available thermal printers
- **Sample Receipt Printing**: Print a sample market receipt with items, prices, and totals
- **ESC/POS Commands**: Uses standard ESC/POS commands compatible with most thermal printers

## Supported Thermal Printer Brands

The app filters for common thermal printer brands including:
- Epson (TM series)
- Star Micronics
- Bixolon
- Zebra
- Citizen
- Rongta
- XPrinter
- Any printer with "thermal", "pos", or "receipt" in the name

## How to Use

1. **Connect Your Thermal Printer**: Ensure your thermal printer is properly installed and recognized by your operating system

2. **Launch the App**: Run the application
   ```bash
   npm install
   npm run tauri dev
   ```

3. **Select Printer**: Choose your thermal printer from the dropdown menu

4. **Print Receipt**: Click the "Print Sample Receipt" button to print a test receipt

5. **Refresh Printers**: If you connect a new printer, click the "Refresh" button to update the list

## Sample Receipt Format

The app prints a sample receipt in **Arabic** with:
- Store header (large text, centered) - "متجر عينة"
- Store address and contact information in Arabic
- Itemized list of products with quantities and prices (تفاح, موز, برتقال)
- Subtotal (المجموع الفرعي), tax (الضريبة), and total (الإجمالي)
- Thank you message in Arabic - "شكراً لك على الشراء!"
- Egyptian Pound currency symbol (ج.م)
- Automatic paper cut (if supported by printer)

## Technical Details

### ESC/POS Commands Used

- **ESC @** (0x1B 0x40): Initialize printer
- **ESC t** (0x1B 0x74): Set code page (28 = Windows-1256 for Arabic)
- **ESC a** (0x1B 0x61): Text alignment (0=left, 1=center, 2=right)
- **ESC E** (0x1B 0x45): Bold text (1=on, 0=off)
- **GS !** (0x1D 0x21): Text size (width and height)
- **GS V** (0x1D 0x56): Paper cut

### Arabic Support

The app uses **Windows-1256** code page for proper Arabic text rendering. The receipt includes:
- **Encoding conversion**: UTF-8 text is converted to Windows-1256 using the `encoding_rs` library
- **Code page selection**: Sends `ESC t 28` to set the printer to Windows-1256
- Right-to-left text alignment for Arabic content
- Egyptian Pound (ج.م) currency symbol
- Mixed alignment (center for headers, right for Arabic text)

**Technical Note**: Thermal printers don't support UTF-8 natively. All Arabic text is converted from UTF-8 to Windows-1256 encoding before being sent to the printer.

### Platform-Specific Printing

- **macOS/Linux**: Uses `lpr` command with raw option
- **Windows**: Writes directly to printer device

## Troubleshooting

### No Printers Found
- Ensure your thermal printer is properly installed in your system
- Check that the printer name contains one of the supported keywords
- Try clicking the "Refresh" button after connecting the printer

### Print Command Fails
- Verify the printer is online and has paper
- Check printer permissions in your OS settings
- Ensure the printer supports ESC/POS commands (most thermal printers do)

### macOS Specific
- You may need to add the printer through System Preferences > Printers & Scanners
- Use the "Generic" or "Raw" printer driver for thermal printers

### Windows Specific
- Ensure the printer is shared or accessible
- Check that raw printing is enabled for the printer

## Development

### Dependencies

**Rust:**
- `printers`: For enumerating system printers
- `tauri`: Desktop application framework
- `serde`: Serialization/deserialization
- `encoding_rs`: UTF-8 to Windows-1256 encoding conversion for Arabic text
- `windows`: Windows API for direct printer access (Windows only)

**Frontend:**
- React
- TypeScript
- Vite

### Building for Production

```bash
npm run tauri build
```

This will create a platform-specific executable in `src-tauri/target/release`.

## License

MIT

