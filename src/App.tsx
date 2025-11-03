// biome-ignore assist/source/organizeImports: disabled.
import { useState, useEffect } from "react";
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

	const handlePreview = async () => {
		if (!receiptData) {
			setMessage("⚠️ No receipt data loaded");
			return;
		}

		try {
			setMessage("🎨 Generating receipt image...");
			const result = await invoke<string>("generate_receipt_image");
			setMessage(result);
			console.log("Image generated successfully:", result);
		} catch (error) {
			setMessage(`❌ Failed to generate image: ${error}`);
			console.error("Generation error:", error);
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
		</main>
	);
}

export default App;
