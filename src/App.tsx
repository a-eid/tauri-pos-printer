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
				onClick={printReceiptHTML}
				disabled={loading || !selectedPrinter}
				className="print-btn primary"
			>
				{loading ? "Opening..." : "🖨️ Print Arabic Receipt"}
			</button>

			<p style={{ fontSize: "12px", color: "#666", margin: "10px 0" }}>
				💡 Tip: Set NCR 7197 as your default printer for faster printing
			</p>

			<details style={{ marginTop: "20px" }}>
				<summary style={{ cursor: "pointer", fontSize: "14px", color: "#666" }}>
					⚙️ Advanced Options (for testing)
				</summary>
				<div className="secondary-buttons" style={{ marginTop: "10px" }}>
					<button
						type="button"
						onClick={printReceipt}
						disabled={loading || !selectedPrinter}
						className="print-btn-small secondary"
						title="Works but Arabic shows as gibberish"
					>
						{loading ? "..." : "📄 ESC/POS"}
					</button>
					
					<button
						type="button"
						onClick={printReceiptAsImage}
						disabled={loading || !selectedPrinter}
						className="print-btn-small secondary"
						title="May print excessive paper on some printers"
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
