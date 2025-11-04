import { useState,  } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

function App() {
	const [message, setMessage] = useState<string>("");
	const [loading, setLoading] = useState<boolean>(false);

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
				</div>
			</section>
		</main>
	);
}

export default App;
