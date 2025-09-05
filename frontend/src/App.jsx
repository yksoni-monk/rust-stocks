import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

function App() {
  const [stockData, setStockData] = useState(null);

  useEffect(() => {
    async function fetchStockData() {
      try {
        const result = await invoke('get_stock_data', {});
        setStockData(result);
      } catch (error) {
        console.error('Error fetching stock data:', error);
      }
    }
    fetchStockData();
  }, []);

  return (
    <div className="min-h-screen bg-gray-100 flex flex-col items-center justify-center p-4">
      <h1 className="text-3xl font-bold text-blue-600 mb-4">Rust Stocks</h1>
      <div className="bg-white p-6 rounded-lg shadow-lg">
        {stockData ? (
          <pre className="text-sm">{JSON.stringify(stockData, null, 2)}</pre>
        ) : (
          <p>Loading stock data...</p>
        )}
      </div>
    </div>
  );
}

export default App;
