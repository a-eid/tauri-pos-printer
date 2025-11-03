import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import html2canvas from "html2canvas";
import "./App.css";

interface PrinterInfo {
	name: string;
	system_name: string;
}

function App() {
	const [printers, setPrinters] = useState<PrinterInfo[]>([]);
	const [selectedPrinter, setSelectedPrinter] = useState<string>("");
	const [message, setMessage] = useState<string>("");
	const [loading, setLoading] = useState<boolean>(false);
  const [escposHost, setEscposHost] = useState<string>("");
  const [escposPort, setEscposPort] = useState<number>(9100);

	// biome-ignore lint/correctness/useExhaustiveDependencies: loadPrinters is not a dependency of this effect
	useEffect(() => {
		loadPrinters();
	}, []);

	async function loadPrinters() {
		try {
			setLoading(true);
			const thermalPrinters = await invoke<PrinterInfo[]>(
				"get_thermal_printers",
			);
			setPrinters(thermalPrinters);
			if (thermalPrinters.length > 0) {
				setSelectedPrinter(thermalPrinters[0].system_name);
				setMessage(`Found ${thermalPrinters.length} thermal printer(s)`);
			} else {
				setMessage(
					"No thermal printers found. Please connect a thermal printer.",
				);
			}
		} catch (error) {
			setMessage(`Error loading printers: ${error}`);
		} finally {
			setLoading(false);
		}
	}

	async function printReceipt() {
		if (!selectedPrinter) {
			setMessage("Please select a printer first");
			return;
		}

		try {
			setLoading(true);
			setMessage("Printing...");
			const result = await invoke<string>("print_receipt", {
				printerName: selectedPrinter,
			});
			setMessage(result);
		} catch (error) {
			setMessage(`Error printing: ${error}`);
		} finally {
			setLoading(false);
		}
	}

	const printReceiptHTML = async () => {
		if (!selectedPrinter) {
			setMessage("Please select a printer first");
			return;
		}

		try {
			setLoading(true);
			setMessage("Opening print dialog...");
			const result = await invoke<string>("print_receipt_html", {
				printerName: selectedPrinter,
			});
			setMessage(result);
		} catch (error) {
			setMessage(`Error printing: ${error}`);
		} finally {
			setLoading(false);
		}
	}

	const printReceiptSilent = async () => {
		if (!selectedPrinter) {
			setMessage("Please select a printer first");
			return;
		}

		try {
			setLoading(true);
			setMessage("Printing silently...");
			const result = await invoke<string>("print_receipt_silent", {
				printerName: selectedPrinter,
			});
			setMessage(result);
		} catch (error) {
			setMessage(`Error printing: ${error}`);
		} finally {
			setLoading(false);
		}
	}

	const printReceiptTextMode = async () => {
		if (!selectedPrinter) {
			setMessage("Please select a printer first");
			return;
		}

		try {
			setLoading(true);
			setMessage("Printing silently (TEXT mode)...");
			const result = await invoke<string>("print_receipt_text_mode", {
				printerName: selectedPrinter,
			});
			setMessage(result);
		} catch (error) {
			setMessage(`Error printing: ${error}`);
		} finally {
			setLoading(false);
		}
	}

	const printReceiptAsImage = async () => {
		if (!selectedPrinter) {
			setMessage("Please select a printer first");
			return;
		}

		try {
			setLoading(true);
			setMessage("Rendering receipt...");

			// Create a temporary div with the receipt HTML
			const container = document.createElement("div");
			container.style.cssText = `
				position: fixed;
				left: -9999px;
				top: 0;
				width: 576px;
				background: white;
				padding: 20px;
				font-family: 'Arial', 'Tahoma', sans-serif;
				direction: rtl;
				text-align: center;
			`;
			
			container.innerHTML = `
				<div style="font-size: 24px; font-weight: bold; margin: 10px 0;">متجر عينة</div>
				<div style="font-size: 14px; margin: 5px 0;">123 شارع الرئيسي</div>
				<div style="font-size: 14px; margin: 5px 0;">المدينة، المحافظة 12345</div>
				<div style="font-size: 14px; margin: 5px 0;">هاتف: (555) 123-4567</div>
				<div style="margin: 15px 0; border-top: 2px dashed #000;"></div>
				<div style="font-size: 16px; font-weight: bold; margin: 10px 0;">الأصناف</div>
				<div style="margin: 10px 0; border-top: 2px dashed #000;"></div>
				
				<div style="text-align: right; font-size: 16px; font-weight: bold; margin: 8px 0;">تفاح</div>
				<div style="text-align: center; font-size: 14px; margin: 5px 0;">2x @ 2.50 ج.م = 5.00 ج.م</div>
				
				<div style="text-align: right; font-size: 16px; font-weight: bold; margin: 8px 0;">موز</div>
				<div style="text-align: center; font-size: 14px; margin: 5px 0;">3x @ 1.50 ج.م = 4.50 ج.م</div>
				
				<div style="text-align: right; font-size: 16px; font-weight: bold; margin: 8px 0;">برتقال</div>
				<div style="text-align: center; font-size: 14px; margin: 5px 0;">1x @ 3.00 ج.م = 3.00 ج.م</div>
				
				<div style="margin: 15px 0; border-top: 2px dashed #000;"></div>
				<div style="text-align: right; font-size: 14px; margin: 5px 0;">المجموع الفرعي: 7.00 ج.م</div>
				<div style="text-align: right; font-size: 14px; margin: 5px 0;">الضريبة (10٪): 0.70 ج.م</div>
				<div style="margin: 10px 0; border-top: 2px dashed #000;"></div>
				<div style="text-align: right; font-size: 20px; font-weight: bold; margin: 10px 0;">الإجمالي: 7.70 ج.م</div>
				<div style="margin: 15px 0; border-top: 2px dashed #000;"></div>
				<div style="font-size: 16px; font-weight: bold; margin: 10px 0;">شكراً لك على الشراء!</div>
				<div style="font-size: 14px; margin: 5px 0;">نتمنى رؤيتك مرة أخرى</div>
			`;
			
			document.body.appendChild(container);

			setMessage("Capturing as image...");

			// Render to canvas
			const canvas = await html2canvas(container, {
				backgroundColor: "#ffffff",
				scale: 2, // Higher quality
				logging: false,
			});

			// Convert to base64 PNG
			const imageDataUrl = canvas.toDataURL("image/png");

			// Remove temporary container
			document.body.removeChild(container);

			setMessage("Sending to printer...");

			// Send to printer
			const result = await invoke<string>("print_receipt_image", {
				printerName: selectedPrinter,
				imageDataUrl,
			});

			setMessage(result);
		} catch (error) {
			setMessage(`Error printing: ${error}`);
		} finally {
			setLoading(false);
		}
	}

	// ============ escpos-rs (Network) experiments ============
	const escposPrintText = async () => {
		if (!escposHost || !escposPort) {
			setMessage("Please enter printer IP/Host and port (default 9100).");
			return;
		}
		try {
			setLoading(true);
			setMessage("Sending Arabic text via escpos-rs (network)...");
			const result = await invoke<string>("escpos_print_text_ar", {
				host: escposHost,
				port: escposPort,
			});
			setMessage(result);
		} catch (error) {
			setMessage(`Error (escpos text): ${error}`);
		} finally {
			setLoading(false);
		}
	};

	const escposPrintImage = async () => {
		if (!escposHost || !escposPort) {
			setMessage("Please enter printer IP/Host and port (default 9100).");
			return;
		}
		try {
			setLoading(true);
			setMessage("Rendering receipt as image (RTL Arabic) and sending via escpos-rs...");

			// Reuse the same HTML-render-to-canvas routine
			const container = document.createElement("div");
			container.style.cssText = `
				position: fixed;
				left: -9999px;
				top: 0;
				width: 576px;
				background: white;
				padding: 20px;
				font-family: 'Arial', 'Tahoma', sans-serif;
				direction: rtl;
				text-align: center;
			`;
			container.innerHTML = `
				<div style="font-size: 24px; font-weight: bold; margin: 10px 0;">متجر عينة</div>
				<div style="font-size: 14px; margin: 5px 0;">123 شارع الرئيسي</div>
				<div style="font-size: 14px; margin: 5px 0;">المدينة، المحافظة 12345</div>
				<div style="font-size: 14px; margin: 5px 0;">هاتف: (555) 123-4567</div>
				<div style="margin: 15px 0; border-top: 2px dashed #000;"></div>
				<div style="font-size: 16px; font-weight: bold; margin: 10px 0;">الأصناف</div>
				<div style="margin: 10px 0; border-top: 2px dashed #000;"></div>
				<div style="text-align: right; font-size: 16px; font-weight: bold; margin: 8px 0;">تفاح</div>
				<div style="text-align: center; font-size: 14px; margin: 5px 0;">2x @ 2.50 ج.م = 5.00 ج.م</div>
				<div style="text-align: right; font-size: 16px; font-weight: bold; margin: 8px 0;">موز</div>
				<div style="text-align: center; font-size: 14px; margin: 5px 0;">3x @ 1.50 ج.م = 4.50 ج.م</div>
				<div style="text-align: right; font-size: 16px; font-weight: bold; margin: 8px 0;">برتقال</div>
				<div style="text-align: center; font-size: 14px; margin: 5px 0;">1x @ 3.00 ج.م = 3.00 ج.م</div>
				<div style="margin: 15px 0; border-top: 2px dashed #000;"></div>
				<div style="text-align: right; font-size: 14px; margin: 5px 0;">المجموع الفرعي: 7.00 ج.م</div>
				<div style="text-align: right; font-size: 14px; margin: 5px 0;">الضريبة (10٪): 0.70 ج.م</div>
				<div style="margin: 10px 0; border-top: 2px dashed #000;"></div>
				<div style="text-align: right; font-size: 20px; font-weight: bold; margin: 10px 0;">الإجمالي: 7.70 ج.م</div>
				<div style="margin: 15px 0; border-top: 2px dashed #000;"></div>
				<div style="font-size: 16px; font-weight: bold; margin: 10px 0;">شكراً لك على الشراء!</div>
				<div style="font-size: 14px; margin: 5px 0;">نتمنى رؤيتك مرة أخرى</div>
			`;
			document.body.appendChild(container);
			const canvas = await html2canvas(container, { backgroundColor: "#ffffff", scale: 2, logging: false });
			const imageDataUrl = canvas.toDataURL("image/png");
			document.body.removeChild(container);

			const result = await invoke<string>("escpos_print_image", {
				host: escposHost,
				port: escposPort,
				imageDataUrl,
			});
			setMessage(result);
		} catch (error) {
			setMessage(`Error (escpos image): ${error}`);
		} finally {
			setLoading(false);
		}
	};

	return (
		<main className="container">
			<h1>Thermal POS Printer</h1>

			<div className="printer-section">
				<div className="printer-controls">
					<label htmlFor="printer-select">Select Thermal Printer:</label>
					<div className="select-wrapper">
						<select
							id="printer-select"
							value={selectedPrinter}
							onChange={(e) => setSelectedPrinter(e.target.value)}
							disabled={loading || printers.length === 0}
						>
							{printers.length === 0 ? (
								<option value="">No thermal printers found</option>
							) : (
								printers.map((printer) => (
									<option key={printer.system_name} value={printer.system_name}>
										{printer.name}
									</option>
								))
							)}
						</select>
						<button
							type="button"
							onClick={loadPrinters}
							disabled={loading}
							className="refresh-btn"
						>
							{loading ? "Loading..." : "Refresh"}
						</button>
					</div>
				</div>

			<button
				type="button"
				onClick={printReceipt}
				disabled={loading || !selectedPrinter}
				className="print-btn primary"
			>
				{loading ? "Printing..." : "🚀 Print Arabic Receipt (Silent)"}
			</button>

			<p style={{ fontSize: "12px", color: "#666", margin: "10px 0" }}>
				✅ NCR 7197 Mode: Code Page 1256 + Contextual Font Mode 5. Zero dialogs!
			</p>

			<details style={{ marginTop: "20px" }}>
				<summary style={{ cursor: "pointer", fontSize: "14px", color: "#666" }}>
					⚙️ Alternative Methods (backup options)
				</summary>
				<div className="secondary-buttons" style={{ marginTop: "10px" }}>
					<button
						type="button"
						onClick={printReceiptHTML}
						disabled={loading || !selectedPrinter}
						className="print-btn-small secondary"
						title="Works perfectly but shows print dialog"
					>
						{loading ? "..." : "🖨️ Dialog"}
					</button>

					<button
						type="button"
						onClick={printReceiptTextMode}
						disabled={loading || !selectedPrinter}
						className="print-btn-small secondary"
						title="TEXT mode - relies on Windows driver"
					>
						{loading ? "..." : "📄 TEXT"}
					</button>
					
					<button
						type="button"
						onClick={printReceiptAsImage}
						disabled={loading || !selectedPrinter}
						className="print-btn-small secondary"
						title="⚠️ Prints 2 meters of paper on NCR 7197"
					>
						{loading ? "..." : "🖼️ Image"}
					</button>

					<button
						type="button"
						onClick={printReceiptSilent}
						disabled={loading || !selectedPrinter}
						className="print-btn-small secondary"
						title="GDI mode - doesn't work on thermal printers"
					>
						{loading ? "..." : "🔧 GDI"}
					</button>
				</div>
			</details>

				{message && (
					<div
						className={`message ${message.includes("Error") ? "error" : "success"}`}
					>
						{message}
					</div>
				)}
			</div>

			<div style={{ marginTop: 24, paddingTop: 16, borderTop: '1px solid #eee' }}>
				<h2 style={{ fontSize: 18, margin: '0 0 8px' }}>ESC/POS (Network) experiment</h2>
				<p style={{ color: '#666', marginTop: 0, fontSize: 12 }}>
					Uses escpos-rs to talk to the printer over TCP (e.g., port 9100). Text may not render Arabic due to code page limits; the image path should render Arabic correctly.
				</p>
				<div style={{ display: 'flex', gap: 8, alignItems: 'center', marginBottom: 8 }}>
					<input
						type="text"
						placeholder="Printer IP/Host (e.g., 192.168.1.50)"
						value={escposHost}
						onChange={(e) => setEscposHost(e.target.value)}
						disabled={loading}
						style={{ flex: 1 }}
					/>
					<input
						type="number"
						placeholder="Port"
						value={escposPort}
						onChange={(e) => setEscposPort(Number(e.target.value))}
						disabled={loading}
						style={{ width: 100 }}
					/>
				</div>
				<div className="secondary-buttons" style={{ display: 'flex', gap: 8 }}>
					<button type="button" className="print-btn-small secondary" disabled={loading || !escposHost}
						onClick={escposPrintText}
						title="Sends Arabic text via ESC/POS – likely gibberish, but we want to observe behavior">
						{loading ? "..." : "📝 escpos Text"}
					</button>
					<button type="button" className="print-btn-small secondary" disabled={loading || !escposHost}
						onClick={escposPrintImage}
						title="Renders Arabic as PNG and prints via ESC/POS raster image">
						{loading ? "..." : "🖼️ escpos Image"}
					</button>
				</div>
			</div>
		</main>
	);
}

export default App;

export const printer = {
	printReceipt: () => {},
	printShiftSummary: () => {},
	loadPrinters: () => {},
	overideDefaultPrinter: () => {},
};
