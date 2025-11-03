import { useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

function App() {
	const [message, setMessage] = useState<string>("");
	const [loading, setLoading] = useState<boolean>(false);
	const [serialPort, setSerialPort] = useState<string>("");
	const [serialBaud, setSerialBaud] = useState<number>(9600);
	const [serialCodepage, setSerialCodepage] = useState<number>(28);
	const [serialContextual, setSerialContextual] = useState<string>("5");
	const canvasRef = useRef<HTMLCanvasElement>(null);

	const escposSerialCustom = async () => {
		try {
			setLoading(true);
			setMessage("Sending serial Arabic test...");
			const contextualVal = serialContextual.trim() === "" ? null : Number(serialContextual);
			const result = await invoke<string>("escpos_print_text_ar_custom_serial", {
				port: serialPort || null,
				baud: serialBaud || null,
				codepage: serialCodepage,
				contextual: contextualVal,
			});
			setMessage(result);
		} catch (error) {
			setMessage(`Error (serial custom): ${error}`);
		} finally {
			setLoading(false);
		}
	};

	const escposSerialSweep = async () => {
		try {
			setLoading(true);
			setMessage("Running serial Arabic sweep...");
			const result = await invoke<string>("escpos_arabic_sweep_serial", {
				port: serialPort || null,
				baud: serialBaud || null,
				tryContextual: true,
			});
			setMessage(result);
		} catch (error) {
			setMessage(`Error (serial sweep): ${error}`);
		} finally {
			setLoading(false);
		}
	};

	const generateReceiptPreview = () => {
		const canvas = canvasRef.current;
		if (!canvas) return;

		const ctx = canvas.getContext("2d");
		if (!ctx) return;

		canvas.width = 576;
		canvas.height = 800;

		ctx.fillStyle = "#ffffff";
		ctx.fillRect(0, 0, canvas.width, canvas.height);
		ctx.fillStyle = "#000000";
		ctx.textAlign = "center";
		ctx.direction = "rtl";

		let y = 30;

		// Header
		ctx.font = "bold 24px Arial, Tahoma";
		ctx.fillText("متجر عينة", canvas.width / 2, y);
		y += 30;

		ctx.font = "14px Arial, Tahoma";
		ctx.fillText("123 شارع الرئيسي", canvas.width / 2, y);
		y += 25;

		// Divider
		const drawDivider = () => {
			ctx.strokeStyle = "#000";
			ctx.setLineDash([5, 5]);
			ctx.beginPath();
			ctx.moveTo(20, y);
			ctx.lineTo(canvas.width - 20, y);
			ctx.stroke();
			ctx.setLineDash([]);
			y += 25;
		};

		drawDivider();

		// Items header
		ctx.font = "bold 16px Arial, Tahoma";
		ctx.fillText("الأصناف", canvas.width / 2, y);
		y += 25;

		drawDivider();
		y += 5;

		// Items
		ctx.textAlign = "right";
		const items = [
			{ name: "تفاح", desc: "2x @ 2.50 ج.م = 5.00 ج.م" },
			{ name: "موز", desc: "3x @ 1.50 ج.م = 4.50 ج.م" },
			{ name: "برتقال", desc: "1x @ 3.00 ج.م = 3.00 ج.م" },
		];

		for (const item of items) {
			ctx.font = "bold 16px Arial, Tahoma";
			ctx.fillText(item.name, canvas.width - 40, y);
			y += 22;
			ctx.font = "14px Arial, Tahoma";
			ctx.textAlign = "center";
			ctx.fillText(item.desc, canvas.width / 2, y);
			ctx.textAlign = "right";
			y += 25;
		}

		y += 10;
		drawDivider();

		// Totals
		ctx.font = "14px Arial, Tahoma";
		ctx.fillText("المجموع الفرعي: 7.00 ج.م", canvas.width - 40, y);
		y += 22;
		ctx.fillText("الضريبة (10٪): 0.70 ج.م", canvas.width - 40, y);
		y += 25;

		drawDivider();

		ctx.font = "bold 20px Arial, Tahoma";
		ctx.fillText("الإجمالي: 7.70 ج.م", canvas.width - 40, y);
		y += 30;

		drawDivider();
		y += 5;

		// Footer
		ctx.textAlign = "center";
		ctx.font = "bold 16px Arial, Tahoma";
		ctx.fillText("شكراً لك على الشراء!", canvas.width / 2, y);
		y += 22;
		ctx.font = "14px Arial, Tahoma";
		ctx.fillText("نتمنى رؤيتك مرة أخرى", canvas.width / 2, y);

		// Open preview
		setMessage("✅ Receipt preview generated! Opening in new window...");
		const dataUrl = canvas.toDataURL("image/png");
		const win = window.open();
		if (win) {
			win.document.write(`
				<html>
					<head>
						<title>Receipt Preview</title>
						<style>
							body { 
								margin: 0; 
								padding: 20px; 
								background: #f0f0f0; 
								display: flex; 
								justify-content: center; 
								align-items: center; 
								min-height: 100vh; 
							}
							img { 
								max-width: 100%; 
								height: auto; 
								box-shadow: 0 4px 8px rgba(0,0,0,0.2); 
								background: white; 
							}
						</style>
					</head>
					<body>
						<img src="${dataUrl}" alt="Receipt Preview" />
					</body>
				</html>
			`);
		}
	};


	return (
		<main className="container">
			<h1>Thermal POS Printer (ESC/POS Serial)</h1>

			{message && (
				<div className={`message ${message.includes("Error") ? "error" : "success"}`}>
					{message}
				</div>
			)}

			<section style={{ marginTop: 24, paddingTop: 16, borderTop: '1px solid #eee' }}>
				<h2 style={{ fontSize: 18, margin: '0 0 8px' }}>ESC/POS (Serial / COM)</h2>
				<p style={{ color: '#666', marginTop: 0, fontSize: 12 }}>
					Printer connected via USB→COM. Leave fields empty to use env vars (ACC_PRINTER_COM/ACC_PRINTER_BAUD) or autodetect.
				</p>
				
				<div style={{ display: 'grid', gap: 8, gridTemplateColumns: '1fr 120px 120px 120px', alignItems: 'center', marginBottom: 8 }}>
					<input
						type="text"
						placeholder="COM port (e.g., COM6)"
						value={serialPort}
						onChange={(e) => setSerialPort(e.target.value)}
						disabled={loading}
						style={{ width: '100%' }}
					/>
					<input
						type="number"
						placeholder="Baud"
						value={serialBaud}
						onChange={(e) => setSerialBaud(Number(e.target.value))}
						disabled={loading}
						style={{ width: '100%' }}
					/>
					<input
						type="number"
						placeholder="Codepage"
						value={serialCodepage}
						onChange={(e) => setSerialCodepage(Number(e.target.value))}
						disabled={loading}
						style={{ width: '100%' }}
					/>
					<input
						type="text"
						placeholder="Contextual"
						value={serialContextual}
						onChange={(e) => setSerialContextual(e.target.value)}
						disabled={loading}
						style={{ width: '100%' }}
					/>
				</div>
				
				<div className="secondary-buttons" style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
					<button 
						type="button" 
						className="print-btn-small secondary" 
						disabled={loading}
						onClick={escposSerialCustom}
						title="Send Windows-1256 Arabic sample over COM with codepage/contextual"
					>
						{loading ? "..." : "🔌 Print Arabic Receipt"}
					</button>
					<button 
						type="button" 
						className="print-btn-small secondary" 
						disabled={loading}
						onClick={escposSerialSweep}
						title="Try multiple ESC t code pages and contextual mode over Serial"
					>
						{loading ? "..." : "🔍 Codepage Sweep"}
					</button>
					<button 
						type="button" 
						className="print-btn-small secondary"
						onClick={generateReceiptPreview}
						title="Generate and open a preview image of the receipt"
					>
						🖼️ Preview Receipt
					</button>
				</div>
			</section>

			<canvas ref={canvasRef} style={{ display: 'none' }} />
		</main>
	);
}

export default App;