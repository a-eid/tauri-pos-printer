import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

interface ReceiptData {
	header: {
		storeName: string;
		address: string;
	};
	items: Array<{
		name: string;
		quantity: number;
		price: number;
		total: number;
	}>;
	totals: {
		subtotal: number;
		tax: number;
		total: number;
	};
	footer: {
		thanks: string;
		comeback: string;
	};
}

function App() {
	const [message, setMessage] = useState<string>("");
	const [loading, setLoading] = useState<boolean>(false);
	const [receiptData, setReceiptData] = useState<ReceiptData | null>(null);
	const canvasRef = useRef<HTMLCanvasElement>(null);

	// biome-ignore lint/correctness/useExhaustiveDependencies: ...
	useEffect(() => {
		loadReceiptData();
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, []);

	const loadReceiptData = async () => {
		try {
			const data = await invoke<ReceiptData>("get_receipt_data");
			setReceiptData(data);
		} catch (error) {
			console.error("Failed to load receipt data:", error);
		}
	};

	const handlePrint = async () => {
		try {
			setLoading(true);
			setMessage("Printing receipt...");
			const result = await invoke<string>("print_receipt");
			setMessage(result);
		} catch (error) {
			setMessage(`❌ Error: ${error}`);
		} finally {
			setLoading(false);
		}
	};

	const handlePreview = () => {
		if (!receiptData) return;

		const canvas = canvasRef.current;
		if (!canvas) return;

		const ctx = canvas.getContext("2d");
		if (!ctx) return;

		// 80mm width = 576px at 72 DPI
		const width = 576;
		canvas.width = width;
		canvas.height = 1000; // Will adjust if needed

		// White background
		ctx.fillStyle = "#ffffff";
		ctx.fillRect(0, 0, canvas.width, canvas.height);

		// Setup drawing
		ctx.fillStyle = "#000000";
		ctx.direction = "rtl";

		let y = 40;
		const centerX = width / 2;
		const rightX = width - 40;

		// Helper to draw divider
		const drawDivider = () => {
			ctx.strokeStyle = "#000";
			ctx.setLineDash([5, 5]);
			ctx.beginPath();
			ctx.moveTo(20, y);
			ctx.lineTo(width - 20, y);
			ctx.stroke();
			ctx.setLineDash([]);
			y += 25;
		};

		// Header
		ctx.textAlign = "center";
		ctx.font = "bold 28px Arial, Tahoma, sans-serif";
		ctx.fillText(receiptData.header.storeName, centerX, y);
		y += 35;

		ctx.font = "16px Arial, Tahoma, sans-serif";
		ctx.fillText(receiptData.header.address, centerX, y);
		y += 30;

		drawDivider();

		// Items header
		ctx.font = "bold 20px Arial, Tahoma, sans-serif";
		ctx.fillText("الأصناف", centerX, y);
		y += 30;

		drawDivider();

		// Items
		ctx.textAlign = "right";
		receiptData.items.forEach((item) => {
			ctx.font = "bold 18px Arial, Tahoma, sans-serif";
			ctx.fillText(item.name, rightX, y);
			y += 25;

			ctx.font = "16px Arial, Tahoma, sans-serif";
			ctx.textAlign = "center";
			const itemLine = `${item.quantity}x @ ${item.price.toFixed(2)} ج.م = ${item.total.toFixed(2)} ج.م`;
			ctx.fillText(itemLine, centerX, y);
			y += 30;
			ctx.textAlign = "right";
		});

		y += 10;
		drawDivider();

		// Totals
		ctx.font = "16px Arial, Tahoma, sans-serif";
		ctx.fillText(
			`المجموع الفرعي: ${receiptData.totals.subtotal.toFixed(2)} ج.م`,
			rightX,
			y,
		);
		y += 25;
		ctx.fillText(
			`الضريبة (10٪): ${receiptData.totals.tax.toFixed(2)} ج.م`,
			rightX,
			y,
		);
		y += 30;

		drawDivider();

		ctx.font = "bold 22px Arial, Tahoma, sans-serif";
		ctx.fillText(
			`الإجمالي: ${receiptData.totals.total.toFixed(2)} ج.م`,
			rightX,
			y,
		);
		y += 35;

		drawDivider();

		// Footer
		ctx.textAlign = "center";
		ctx.font = "bold 18px Arial, Tahoma, sans-serif";
		ctx.fillText(receiptData.footer.thanks, centerX, y);
		y += 25;
		ctx.font = "16px Arial, Tahoma, sans-serif";
		ctx.fillText(receiptData.footer.comeback, centerX, y);
		y += 40;

		// Trim canvas height
		canvas.height = y;

		// Open in new window
		setMessage("✅ Preview generated! Opening in new window...");
		const dataUrl = canvas.toDataURL("image/png");
		const win = window.open("", "_blank");
		if (win) {
			win.document.write(`
				<!DOCTYPE html>
				<html>
					<head>
						<meta charset="UTF-8">
						<title>Receipt Preview - 80mm Width</title>
						<style>
							* { margin: 0; padding: 0; box-sizing: border-box; }
							body { 
								background: #2c3e50; 
								padding: 20px;
								display: flex;
								justify-content: center;
								align-items: flex-start;
								min-height: 100vh;
								font-family: system-ui, -apple-system, sans-serif;
							}
							.container {
								background: white;
								padding: 20px;
								border-radius: 8px;
								box-shadow: 0 10px 40px rgba(0,0,0,0.3);
								max-width: 616px;
							}
							img { 
								display: block;
								width: 100%;
								height: auto;
								border: 1px solid #ddd;
							}
							.info {
								margin-top: 15px;
								padding: 15px;
								background: #ecf0f1;
								border-radius: 4px;
								font-size: 14px;
								color: #2c3e50;
							}
							.info strong { color: #e74c3c; }
						</style>
					</head>
					<body>
						<div class="container">
							<img src="${dataUrl}" alt="Receipt Preview" />
							<div class="info">
								<strong>📏 80mm Thermal Printer</strong><br>
								Width: 576px (80mm @ 72 DPI)<br>
								This is how your receipt will look when printed.
							</div>
						</div>
					</body>
				</html>
			`);
			win.document.close();
		}
	};

	return (
		<main className="container">
			<h1>🧾 Thermal Receipt Printer</h1>
			<p style={{ color: "#666", fontSize: "14px", marginTop: "8px" }}>
				Arabic receipts for 80mm thermal printers
			</p>

			{message && (
				<div
					className={`message ${message.includes("❌") ? "error" : "success"}`}
				>
					{message}
				</div>
			)}

			<section style={{ marginTop: "30px" }}>
				<div style={{ display: "flex", gap: "12px", flexWrap: "wrap" }}>
					<button
						type="button"
						onClick={handlePrint}
						disabled={loading}
						className="print-btn primary"
						style={{
							padding: "16px 32px",
							fontSize: "16px",
							fontWeight: "bold",
							cursor: loading ? "not-allowed" : "pointer",
							opacity: loading ? 0.6 : 1,
						}}
					>
						{loading ? "⏳ Printing..." : "🖨️ Print Receipt"}
					</button>

					<button
						type="button"
						onClick={handlePreview}
						disabled={!receiptData}
						className="print-btn-small secondary"
						style={{
							padding: "16px 32px",
							fontSize: "16px",
							fontWeight: "bold",
							cursor: !receiptData ? "not-allowed" : "pointer",
							opacity: !receiptData ? 0.6 : 1,
						}}
					>
						🖼️ Preview Receipt
					</button>
				</div>

				<div
					style={{
						marginTop: "20px",
						padding: "16px",
						background: "#f8f9fa",
						borderRadius: "8px",
						fontSize: "14px",
					}}
				>
					<strong>ℹ️ Environment Variables:</strong>
					<ul style={{ marginTop: "8px", paddingLeft: "20px" }}>
						<li>
							<code>PRINTER_COM_PORT</code> - COM port (default: COM7)
						</li>
						<li>
							<code>PRINTER_BAUD_RATE</code> - Baud rate (default: 9600)
						</li>
					</ul>
				</div>
			</section>

			<canvas ref={canvasRef} style={{ display: "none" }} />
		</main>
	);
}

export default App;
