import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

function DataFetchingPanel({ stock }) {
  const [startDate, setStartDate] = useState('2024-01-01');
  const [endDate, setEndDate] = useState('2024-12-31');
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState('');

  const handleFetchData = async () => {
    if (!stock?.symbol) return;

    setLoading(true);
    setMessage('');

    try {
      const result = await invoke('fetch_single_stock_data', {
        symbol: stock.symbol,
        startDate: startDate,
        endDate: endDate
      });

      setMessage(`✅ Successfully fetched data for ${stock.symbol}: ${result}`);
    } catch (error) {
      console.error('Fetch failed:', error);
      setMessage(`❌ Failed to fetch data: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="bg-white rounded-lg border p-6">
        <h3 className="text-lg font-semibold mb-4">Fetch Stock Data</h3>
        
        <div className="space-y-4">
          {/* Date Range Selection */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Start Date
              </label>
              <input
                type="date"
                value={startDate}
                onChange={(e) => setStartDate(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                End Date
              </label>
              <input
                type="date"
                value={endDate}
                onChange={(e) => setEndDate(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
          </div>

          {/* Fetch Button */}
          <div className="pt-4">
            <button
              onClick={handleFetchData}
              disabled={loading}
              className="w-full bg-blue-600 text-white py-3 px-4 rounded-lg hover:bg-blue-700 disabled:bg-gray-400 font-medium"
            >
              {loading ? (
                <div className="flex items-center justify-center space-x-2">
                  <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-white"></div>
                  <span>Fetching Data...</span>
                </div>
              ) : (
                'Fetch All Data'
              )}
            </button>
          </div>

          {/* Status Message */}
          {message && (
            <div className={`p-4 rounded-lg text-sm ${
              message.startsWith('✅') 
                ? 'bg-green-50 text-green-800 border border-green-200' 
                : 'bg-red-50 text-red-800 border border-red-200'
            }`}>
              {message}
            </div>
          )}

          {/* Info */}
          <div className="text-xs text-gray-500 mt-4">
            <p>This will fetch comprehensive stock data including:</p>
            <ul className="mt-2 space-y-1 list-disc list-inside">
              <li>Historical price data (OHLCV)</li>
              <li>Fundamental metrics (P/E, EPS, etc.)</li>
              <li>Market data (volume, market cap)</li>
              <li>Dividend information</li>
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
}

export default DataFetchingPanel;