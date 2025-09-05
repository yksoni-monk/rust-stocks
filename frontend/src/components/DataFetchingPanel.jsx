import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

function DataFetchingPanel({ stock }) {
  const [startDate, setStartDate] = useState('2024-01-01');
  const [endDate, setEndDate] = useState('2024-12-31');
  const [includeFundamentals, setIncludeFundamentals] = useState(true);
  const [includeRealTime, setIncludeRealTime] = useState(false);
  const [includeOptions, setIncludeOptions] = useState(false);
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState('');
  const [fetchStatus, setFetchStatus] = useState(null);

  useEffect(() => {
    // Load current fetch status
    loadFetchStatus();
  }, [stock.symbol]);

  const loadFetchStatus = async () => {
    if (!stock?.symbol) return;

    try {
      const status = await invoke('get_stock_data_status', {
        symbol: stock.symbol
      });
      setFetchStatus(status);
    } catch (err) {
      console.error('Failed to load fetch status:', err);
    }
  };

  const handleFetchData = async () => {
    if (!stock?.symbol) return;

    setLoading(true);
    setMessage('');

    try {
      const result = await invoke('populate_enhanced_stock_data', {
        symbol: stock.symbol,
        startDate,
        endDate,
        fetchFundamentals: includeFundamentals
      });

      if (result.success) {
        setMessage(`✅ Successfully fetched data for ${stock.symbol}`);
        await loadFetchStatus(); // Refresh status
      } else {
        setMessage(`❌ Error: ${result.error}`);
      }
    } catch (error) {
      console.error('Fetch failed:', error);
      setMessage(`❌ Failed to fetch data: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleComprehensiveFetch = async () => {
    if (!stock?.symbol) return;

    setLoading(true);
    setMessage('');

    try {
      const result = await invoke('fetch_comprehensive_data', {
        symbol: stock.symbol,
        startDate,
        endDate,
        includeFundamentals,
        includeRealTime,
        includeOptions
      });

      if (result.success) {
        setMessage(`✅ Comprehensive data fetched successfully for ${stock.symbol}`);
        await loadFetchStatus(); // Refresh status
      } else {
        setMessage(`❌ Error: ${result.error}`);
      }
    } catch (error) {
      console.error('Comprehensive fetch failed:', error);
      setMessage(`❌ Failed to fetch comprehensive data: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const testFundamentalsAPI = async () => {
    setLoading(true);
    try {
      const result = await invoke('get_fundamentals', {
        symbol: stock.symbol
      });

      if (result.success) {
        setMessage(`✅ Fundamentals API test successful for ${stock.symbol}`);
        console.log('Fundamentals data:', result.data);
      } else {
        setMessage(`❌ Fundamentals API test failed: ${result.error}`);
      }
    } catch (error) {
      setMessage(`❌ Fundamentals API test error: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const getStatusColor = (hasData) => {
    if (hasData === true) return 'text-green-600';
    if (hasData === false) return 'text-red-600';
    return 'text-gray-600';
  };

  const getStatusIcon = (hasData) => {
    if (hasData === true) return '✅';
    if (hasData === false) return '❌';
    return '⏳';
  };

  return (
    <div className="space-y-6">
      {/* Current Data Status */}
      <div className="bg-white p-4 rounded-lg border">
        <h3 className="text-lg font-semibold mb-3">Data Status for {stock.symbol}</h3>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="flex items-center justify-between p-3 bg-gray-50 rounded">
            <span className="text-sm font-medium">Price History</span>
            <span className={`flex items-center gap-1 ${getStatusColor(stock.has_data)}`}>
              <span>{getStatusIcon(stock.has_data)}</span>
              <span className="text-sm">
                {stock.has_data ? `${stock.data_count || 0} records` : 'No data'}
              </span>
            </span>
          </div>
          <div className="flex items-center justify-between p-3 bg-gray-50 rounded">
            <span className="text-sm font-medium">Fundamentals</span>
            <span className={`flex items-center gap-1 ${getStatusColor(fetchStatus?.has_fundamentals)}`}>
              <span>{getStatusIcon(fetchStatus?.has_fundamentals)}</span>
              <span className="text-sm">
                {fetchStatus?.has_fundamentals ? 'Available' : 'Not fetched'}
              </span>
            </span>
          </div>
          <div className="flex items-center justify-between p-3 bg-gray-50 rounded">
            <span className="text-sm font-medium">Last Updated</span>
            <span className="text-sm text-gray-600">
              {fetchStatus?.last_updated || 'Never'}
            </span>
          </div>
        </div>
      </div>

      {/* Date Range Selection */}
      <div className="bg-white p-4 rounded-lg border">
        <h3 className="text-lg font-semibold mb-3">Date Range</h3>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              Start Date
            </label>
            <input
              type="date"
              value={startDate}
              onChange={(e) => setStartDate(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              disabled={loading}
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              End Date
            </label>
            <input
              type="date"
              value={endDate}
              onChange={(e) => setEndDate(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              disabled={loading}
            />
          </div>
        </div>
      </div>

      {/* Data Type Selection */}
      <div className="bg-white p-4 rounded-lg border">
        <h3 className="text-lg font-semibold mb-3">Data Types to Include</h3>
        <div className="space-y-3">
          <label className="flex items-center">
            <input
              type="checkbox"
              checked={includeFundamentals}
              onChange={(e) => setIncludeFundamentals(e.target.checked)}
              className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
              disabled={loading}
            />
            <span className="ml-3 text-sm">
              <span className="font-medium">Fundamental Data</span>
              <span className="text-gray-600 block">P/E, EPS, Market Cap, Dividend Yield, etc.</span>
            </span>
          </label>
          <label className="flex items-center">
            <input
              type="checkbox"
              checked={includeRealTime}
              onChange={(e) => setIncludeRealTime(e.target.checked)}
              className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
              disabled={loading}
            />
            <span className="ml-3 text-sm">
              <span className="font-medium">Real-time Quotes</span>
              <span className="text-gray-600 block">Live bid/ask prices and market data</span>
            </span>
          </label>
          <label className="flex items-center">
            <input
              type="checkbox"
              checked={includeOptions}
              onChange={(e) => setIncludeOptions(e.target.checked)}
              className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
              disabled={loading}
            />
            <span className="ml-3 text-sm">
              <span className="font-medium">Options Data</span>
              <span className="text-gray-600 block">Option chains and Greeks (if available)</span>
            </span>
          </label>
        </div>
      </div>

      {/* Action Buttons */}
      <div className="bg-white p-4 rounded-lg border">
        <h3 className="text-lg font-semibold mb-3">Actions</h3>
        <div className="flex flex-wrap gap-3">
          <button
            onClick={handleFetchData}
            disabled={loading}
            className="flex items-center space-x-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {loading ? (
              <>
                <svg className="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                  <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                  <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                <span>Fetching...</span>
              </>
            ) : (
              <>
                <span>📊</span>
                <span>Fetch Enhanced Data</span>
              </>
            )}
          </button>

          <button
            onClick={handleComprehensiveFetch}
            disabled={loading}
            className="flex items-center space-x-2 px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {loading ? (
              <>
                <svg className="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                  <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                  <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                <span>Fetching...</span>
              </>
            ) : (
              <>
                <span>⚡</span>
                <span>Comprehensive Fetch</span>
              </>
            )}
          </button>

          <button
            onClick={testFundamentalsAPI}
            disabled={loading}
            className="flex items-center space-x-2 px-4 py-2 bg-orange-600 text-white rounded-lg hover:bg-orange-700 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <span>🧪</span>
            <span>Test API</span>
          </button>
        </div>
      </div>

      {/* Status Message */}
      {message && (
        <div className={`p-4 rounded-lg border ${
          message.includes('✅') 
            ? 'bg-green-50 text-green-800 border-green-200' 
            : message.includes('❌')
            ? 'bg-red-50 text-red-800 border-red-200'
            : 'bg-blue-50 text-blue-800 border-blue-200'
        }`}>
          <pre className="whitespace-pre-wrap font-mono text-sm">{message}</pre>
        </div>
      )}

      {/* Instructions */}
      <div className="bg-gray-50 p-4 rounded-lg border">
        <h4 className="font-semibold text-gray-800 mb-2">Unified Data Fetching</h4>
        <ul className="text-sm text-gray-600 space-y-1">
          <li>• <strong>Enhanced Data:</strong> Fetch price history with fundamental metrics</li>
          <li>• <strong>Comprehensive Fetch:</strong> All available data types including real-time and options</li>
          <li>• <strong>Test API:</strong> Verify Schwab API connection and data availability</li>
          <li>• All data is comprehensive - no artificial "basic vs enhanced" tiers</li>
        </ul>
      </div>
    </div>
  );
}

export default DataFetchingPanel;