import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
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

			<div className="button-group">
				<button
					type="button"
					onClick={async () => {
						if (!selectedPrinter) return
						setLoading(true)
						setMessage("")
						try {
							const result = await invoke("test_bitmap_square", {
								printerName: selectedPrinter,
							})
							setMessage(`✅ ${result}`)
						} catch (error) {
							setMessage(`❌ Error: ${error}`)
						} finally {
							setLoading(false)
						}
					}}
					disabled={loading || !selectedPrinter}
					className="test-btn"
				>
					{loading ? "Testing..." : "🔲 Test Square"}
				</button>
				
				<button
					type="button"
					onClick={printReceipt}
					disabled={loading || !selectedPrinter}
					className="print-btn"
				>
					{loading ? "Printing..." : "🧾 Print Sample Receipt"}
				</button>
			</div>

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
