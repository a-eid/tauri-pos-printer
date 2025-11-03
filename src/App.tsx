// biome-ignore assist/source/organizeImports: disabled.
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
			console.log("Loading receipt data...");
			const data = await invoke<ReceiptData>("get_receipt_data");
			console.log("Receipt data loaded:", data);
			setReceiptData(data);
		} catch (error) {
			console.error("Failed to load receipt data:", error);
			setMessage(`❌ Failed to load receipt data: ${error}`);
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
		if (!receiptData) {
			setMessage("⚠️ No receipt data loaded");
			return;
		}

		const canvas = canvasRef.current;
		if (!canvas) {
			setMessage("⚠️ Canvas not available");
			return;
		}

		const ctx = canvas.getContext("2d");
		if (!ctx) {
			setMessage("⚠️ Canvas context not available");
			return;
		}

		console.log("Receipt data:", receiptData);
		
		// Test Arabic rendering
		console.log("Store name:", receiptData.header.storeName);
		console.log("Item names:", receiptData.items.map(i => i.name));

		// 80mm width = 576px at 72 DPI
		const width = 576;
		canvas.width = width;
		canvas.height = 1200; // Increased initial height

		// White background
		ctx.fillStyle = "#ffffff";
		ctx.fillRect(0, 0, canvas.width, canvas.height);

		// Setup drawing with proper font that supports Arabic
		ctx.fillStyle = "#000000";

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
		ctx.font = "bold 28px 'Segoe UI', Tahoma, Arial, sans-serif";
		ctx.fillText(receiptData.header.storeName, centerX, y);
		y += 35;

		ctx.font = "16px 'Segoe UI', Tahoma, Arial, sans-serif";
		ctx.fillText(receiptData.header.address, centerX, y);
		y += 30;

		drawDivider();

		// Items header
		ctx.font = "bold 20px 'Segoe UI', Tahoma, Arial, sans-serif";
		ctx.fillText("الأصناف", centerX, y);
		y += 30;

		drawDivider();

		// Items
		ctx.textAlign = "right";
		receiptData.items.forEach((item) => {
			ctx.font = "bold 18px 'Segoe UI', Tahoma, Arial, sans-serif";
			ctx.fillText(item.name, rightX, y);
			y += 25;

			ctx.font = "16px 'Segoe UI', Tahoma, Arial, sans-serif";
			ctx.textAlign = "center";
			const itemLine = `${item.quantity}x @ ${item.price.toFixed(2)} ج.م = ${item.total.toFixed(2)} ج.م`;
			ctx.fillText(itemLine, centerX, y);
			y += 30;
			ctx.textAlign = "right";
		});

		y += 10;
		drawDivider();

		// Totals
		ctx.font = "16px 'Segoe UI', Tahoma, Arial, sans-serif";
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

		ctx.font = "bold 22px 'Segoe UI', Tahoma, Arial, sans-serif";
		ctx.fillText(
			`الإجمالي: ${receiptData.totals.total.toFixed(2)} ج.م`,
			rightX,
			y,
		);
		y += 35;

		drawDivider();

		// Footer
		ctx.textAlign = "center";
		ctx.font = "bold 18px 'Segoe UI', Tahoma, Arial, sans-serif";
		ctx.fillText(receiptData.footer.thanks, centerX, y);
		y += 25;
		ctx.font = "16px 'Segoe UI', Tahoma, Arial, sans-serif";
		ctx.fillText(receiptData.footer.comeback, centerX, y);
		y += 40;

		// Trim canvas height
		canvas.height = y;

		console.log("Canvas drawn successfully. Height:", y);

		// Convert to data URL
		const dataUrl = canvas.toDataURL("image/png");
		console.log("Data URL length:", dataUrl.length);

		// Save to desktop
		setMessage("💾 Saving receipt to Desktop...");
		invoke<string>("save_receipt_image", { imageData: dataUrl })
			.then((result) => {
				setMessage(result);
				console.log("Image saved successfully");
			})
			.catch((error) => {
				setMessage(`❌ Failed to save image: ${error}`);
				console.error("Save error:", error);
			});
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
						� Save Receipt to Desktop
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
