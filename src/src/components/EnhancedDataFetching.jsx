import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

function EnhancedDataFetching() {
  const [availableStocks, setAvailableStocks] = useState([]);
  const [selectedStock, setSelectedStock] = useState('');
  const [startDate, setStartDate] = useState('2024-01-01');
  const [endDate, setEndDate] = useState('2024-12-31');
  const [fetchMode, setFetchMode] = useState('single');
  const [message, setMessage] = useState('');
  const [loading, setLoading] = useState(false);
  const [includeRealTime, setIncludeRealTime] = useState(false);
  const [includeFundamentals, setIncludeFundamentals] = useState(true);
  const [includeOptions, setIncludeOptions] = useState(false);
  const [migrationStatus, setMigrationStatus] = useState(null);

  useEffect(() => {
    loadInitialData();
  }, []);

  async function loadInitialData() {
    try {
      const [stocks, status] = await Promise.all([
        invoke('get_available_stock_symbols'),
        invoke('get_database_migration_status')
      ]);
      
      setAvailableStocks(stocks);
      setMigrationStatus(status);
      
      if (stocks.length > 0) {
        setSelectedStock(stocks[0].symbol);
      }
    } catch (error) {
      console.error('Failed to load initial data:', error);
      setMessage(`Error loading data: ${error}`);
    }
  }

  async function handleSingleStockFetch() {
    if (!selectedStock) {
      setMessage('Please select a stock first');
      return;
    }

    setLoading(true);
    setMessage('');

    try {
      const result = await invoke('populate_enhanced_stock_data', {
        symbol: selectedStock,
        startDate,
        endDate,
        fetchFundamentals: includeFundamentals
      });

      if (result.success) {
        setMessage(`✅ ${result.data}`);
      } else {
        setMessage(`❌ Error: ${result.error}`);
      }
    } catch (error) {
      console.error('Fetch failed:', error);
      setMessage(`❌ Failed to fetch data: ${error}`);
    } finally {
      setLoading(false);
    }
  }

  async function handleComprehensiveFetch() {
    if (!selectedStock) {
      setMessage('Please select a stock first');
      return;
    }

    setLoading(true);
    setMessage('');

    try {
      const result = await invoke('fetch_comprehensive_data', {
        symbol: selectedStock,
        startDate,
        endDate,
        includeFundamentals,
        includeRealTime,
        includeOptions
      });

      if (result.success) {
        setMessage(`✅ Comprehensive data fetched successfully`);
      } else {
        setMessage(`❌ Error: ${result.error}`);
      }
    } catch (error) {
      console.error('Comprehensive fetch failed:', error);
      setMessage(`❌ Failed to fetch comprehensive data: ${error}`);
    } finally {
      setLoading(false);
    }
  }

  async function handleBulkFetch() {
    if (availableStocks.length === 0) {
      setMessage('No stocks available for bulk fetch');
      return;
    }

    setLoading(true);
    setMessage('Starting bulk fetch for all stocks...');

    let successCount = 0;
    let errorCount = 0;

    for (let i = 0; i < availableStocks.length; i++) {
      const stock = availableStocks[i];
      setMessage(`Fetching ${i + 1}/${availableStocks.length}: ${stock.symbol}...`);

      try {
        const result = await invoke('populate_enhanced_stock_data', {
          symbol: stock.symbol,
          startDate,
          endDate,
          fetchFundamentals: includeFundamentals
        });

        if (result.success) {
          successCount++;
        } else {
          errorCount++;
          console.warn(`Failed to fetch ${stock.symbol}:`, result.error);
        }
      } catch (error) {
        errorCount++;
        console.error(`Error fetching ${stock.symbol}:`, error);
      }

      // Small delay to avoid overwhelming the API
      await new Promise(resolve => setTimeout(resolve, 100));
    }

    setMessage(`✅ Bulk fetch completed: ${successCount} successful, ${errorCount} errors`);
    setLoading(false);
  }

  async function testFundamentalsAPI() {
    if (!selectedStock) {
      setMessage('Please select a stock first');
      return;
    }

    setLoading(true);
    try {
      const result = await invoke('get_fundamentals', {
        symbol: selectedStock
      });

      if (result.success) {
        setMessage(`✅ Fundamentals API test successful for ${selectedStock}`);
        console.log('Fundamentals data:', result.data);
      } else {
        setMessage(`❌ Fundamentals API test failed: ${result.error}`);
      }
    } catch (error) {
      setMessage(`❌ Fundamentals API test error: ${error}`);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="max-w-4xl mx-auto space-y-6">
      <div className="bg-white p-6 rounded-lg shadow">
        <h2 className="text-2xl font-bold mb-6">Enhanced Data Fetching</h2>

        {/* Migration Status */}
        {migrationStatus && (
          <div className="mb-6 p-4 bg-blue-50 rounded-lg">
            <h3 className="font-semibold text-blue-800">Database Status</h3>
            <p className="text-blue-700">
              {migrationStatus.success ? migrationStatus.data : migrationStatus.error}
            </p>
          </div>
        )}

        {/* Stock Selection */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-6">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Select Stock
            </label>
            <select
              value={selectedStock}
              onChange={(e) => setSelectedStock(e.target.value)}
              className="w-full p-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              disabled={loading}
            >
              <option value="">Choose a stock...</option>
              {availableStocks.map(stock => (
                <option key={stock.symbol} value={stock.symbol}>
                  {stock.symbol} - {stock.company_name}
                </option>
              ))}
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Fetch Mode
            </label>
            <select
              value={fetchMode}
              onChange={(e) => setFetchMode(e.target.value)}
              className="w-full p-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              disabled={loading}
            >
              <option value="single">Single Stock</option>
              <option value="comprehensive">Comprehensive Data</option>
              <option value="bulk">All Stocks (Bulk)</option>
            </select>
          </div>
        </div>

        {/* Date Range */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-6">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Start Date
            </label>
            <input
              type="date"
              value={startDate}
              onChange={(e) => setStartDate(e.target.value)}
              className="w-full p-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              disabled={loading}
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
              className="w-full p-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              disabled={loading}
            />
          </div>
        </div>

        {/* Data Options */}
        <div className="mb-6">
          <label className="block text-sm font-medium text-gray-700 mb-3">
            Data to Include
          </label>
          <div className="space-y-2">
            <label className="flex items-center">
              <input
                type="checkbox"
                checked={includeFundamentals}
                onChange={(e) => setIncludeFundamentals(e.target.checked)}
                className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                disabled={loading}
              />
              <span className="ml-2 text-sm text-gray-700">
                Fundamental Data (P/E, EPS, Market Cap, etc.)
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
              <span className="ml-2 text-sm text-gray-700">
                Real-time Quotes (if available)
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
              <span className="ml-2 text-sm text-gray-700">
                Options Data (if available)
              </span>
            </label>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex flex-wrap gap-3 mb-6">
          {fetchMode === 'single' && (
            <button
              onClick={handleSingleStockFetch}
              disabled={loading || !selectedStock}
              className="bg-blue-600 text-white px-4 py-2 rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed flex items-center"
            >
              {loading ? (
                <>
                  <svg className="animate-spin -ml-1 mr-3 h-4 w-4 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                    <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                    <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                  </svg>
                  Fetching...
                </>
              ) : (
                'Fetch Enhanced Data'
              )}
            </button>
          )}

          {fetchMode === 'comprehensive' && (
            <button
              onClick={handleComprehensiveFetch}
              disabled={loading || !selectedStock}
              className="bg-green-600 text-white px-4 py-2 rounded-lg hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed flex items-center"
            >
              {loading ? (
                <>
                  <svg className="animate-spin -ml-1 mr-3 h-4 w-4 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                    <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                    <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                  </svg>
                  Fetching...
                </>
              ) : (
                'Fetch Comprehensive Data'
              )}
            </button>
          )}

          {fetchMode === 'bulk' && (
            <button
              onClick={handleBulkFetch}
              disabled={loading}
              className="bg-purple-600 text-white px-4 py-2 rounded-lg hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed flex items-center"
            >
              {loading ? (
                <>
                  <svg className="animate-spin -ml-1 mr-3 h-4 w-4 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                    <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                    <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                  </svg>
                  Processing...
                </>
              ) : (
                `Fetch All Stocks (${availableStocks.length})`
              )}
            </button>
          )}

          <button
            onClick={testFundamentalsAPI}
            disabled={loading || !selectedStock}
            className="bg-orange-600 text-white px-4 py-2 rounded-lg hover:bg-orange-700 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Test Fundamentals API
          </button>
        </div>

        {/* Status Message */}
        {message && (
          <div className={`p-4 rounded-lg ${
            message.includes('✅') 
              ? 'bg-green-100 text-green-800 border border-green-200' 
              : message.includes('❌')
              ? 'bg-red-100 text-red-800 border border-red-200'
              : 'bg-blue-100 text-blue-800 border border-blue-200'
          }`}>
            <pre className="whitespace-pre-wrap font-mono text-sm">{message}</pre>
          </div>
        )}

        {/* Instructions */}
        <div className="mt-6 p-4 bg-gray-50 rounded-lg">
          <h3 className="font-semibold text-gray-800 mb-2">Instructions</h3>
          <ul className="text-sm text-gray-600 space-y-1">
            <li>• <strong>Single Stock:</strong> Fetch enhanced data with fundamentals for one stock</li>
            <li>• <strong>Comprehensive Data:</strong> Fetch all available data types for detailed analysis</li>
            <li>• <strong>All Stocks (Bulk):</strong> Populate enhanced data for all available stocks</li>
            <li>• <strong>Test Fundamentals API:</strong> Verify Schwab API connection and data retrieval</li>
          </ul>
        </div>
      </div>
    </div>
  );
}

export default EnhancedDataFetching;