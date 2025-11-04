import { useState,  } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type Item = { name: string; qty: number; price: number, total: number };

  const items: Item[] = [
    { name: "عرض تفاح", qty: 0.96, price: 70.0, total: 67.2 },
    { name: "تفاح", qty: 1.95, price: 30.0, total: 58.5 },
    { name: "خيار", qty: 1.02, price: 25.0, total: 25.5 },
    { name: "ليمون بلدي", qty: 0.44, price: 30.0, total: 13.2 },
    { name: "بطاطا", qty: 2.16, price: 20.0, total: 43.2 },
    { name: "ربطة جرجير", qty: 4.0, price: 3.0, total: 12.0 },
    { name: "نعناع فريش", qty: 1.0, price: 5.0, total: 5.0 },
    // Mixed Arabic + English digits
    { name: "بسكوت بسكرم 24 قطعه", qty: 5, price: 12.5, total: 62.5 },
    { name: "بسكوت شوفان 30 قطعه", qty: 7, price: 18.75, total: 131.25 },
    { name: "كوكاكولا لمون نعناع 250 جم", qty: 25, price: 40.0, total: 1000.0 },
  ];

  const SumOfAllItems = items.reduce((sum, item) => sum + item.total, 0); 




function App() {
	const [message, setMessage] = useState<string>("");
	const [loading, setLoading] = useState<boolean>(false);


	const handlePrint = async () => {
		try {
			setLoading(true);
			setMessage("Printing receipt...");
  const result = await invoke<string>("print_receipt", {
    title: "اسواق ابو عمر",
    time: "٤ نوفمبر - ٤:٠٩ صباحا",
    number: "123456",
    items,
    total: SumOfAllItems,                // <- printed exactly as provided
    discount: 0,             // optional; shown if > 0
    footer: {
      address: "دمياط الجديدة - المركزية - مقابل البنك الأهلي القديم",
      "last line": "خدمة توصيل للمنازل ٢٤ ساعة",
      // phones: "01533333161 - 01533333262",
    },
  });
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
