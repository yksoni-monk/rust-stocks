import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import EnhancedStockDetails from './components/EnhancedStockDetails';
import EnhancedDataFetching from './components/EnhancedDataFetching';

// Simple AnalysisView component
function AnalysisView({ stocks }) {
  const [selectedStock, setSelectedStock] = useState(null);
  const [priceHistory, setPriceHistory] = useState([]);
  const [loading, setLoading] = useState(false);
  const [startDate, setStartDate] = useState('2024-01-01');
  const [endDate, setEndDate] = useState('2024-12-31');

  async function loadPriceHistory() {
    if (!selectedStock) return;
    
    setLoading(true);
    try {
      const history = await invoke('get_price_history', {
        symbol: selectedStock.symbol,
        startDate,
        endDate
      });
      setPriceHistory(history);
    } catch (err) {
      console.error('Failed to load price history:', err);
      alert(`Failed to load price history: ${err}`);
    } finally {
      setLoading(false);
    }
  }

  async function handleExport(format) {
    if (!selectedStock) return;
    
    try {
      const result = await invoke('export_data', {
        symbol: selectedStock.symbol,
        format
      });
      alert(result);
    } catch (err) {
      alert(`Export failed: ${err}`);
    }
  }

  return (
    <div>
      <h2 className="text-3xl font-bold mb-6">Stock Analysis</h2>
      
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 mb-6">
        {/* Stock Selection */}
        <div className="bg-white p-6 rounded-lg shadow">
          <h3 className="text-lg font-semibold mb-4">Select Stock</h3>
          <select 
            className="w-full p-2 border border-gray-300 rounded"
            value={selectedStock?.symbol || ''}
            onChange={(e) => {
              const stock = stocks.find(s => s.symbol === e.target.value);
              setSelectedStock(stock);
            }}
          >
            <option value="">Choose a stock...</option>
            {stocks.map(stock => (
              <option key={stock.symbol || stock.id} value={stock.symbol || stock.id}>
                {stock.has_data !== undefined ? (stock.has_data ? '📊' : '📋') : '🔍'} {stock.symbol} - {stock.company_name} 
                {stock.has_data ? ` (${stock.data_count} records)` : stock.has_data === false ? ' (no data)' : ' (checking...)'}
              </option>
            ))}
          </select>
          
          <div className="mt-3 text-sm text-gray-600">
            <div className="flex items-center gap-4">
              <div className="flex items-center gap-1">
                <span>📊</span>
                <span>Has price data</span>
              </div>
              <div className="flex items-center gap-1">
                <span>📋</span>
                <span>No data yet</span>
              </div>
              <div className="flex items-center gap-1">
                <span>🔍</span>
                <span>Checking data...</span>
              </div>
            </div>
            <div className="mt-2 p-2 bg-gray-100 rounded text-xs">
              Debug: {stocks.length} stocks loaded. First stock: {stocks[0] ? `${stocks[0].symbol} (has_data: ${stocks[0].has_data}, data_count: ${stocks[0].data_count})` : 'none'}
            </div>
          </div>
          
          {selectedStock && (
            <div className="mt-4 p-3 bg-blue-50 rounded">
              <p className="font-medium">{selectedStock.symbol}</p>
              <p className="text-sm text-gray-600">{selectedStock.company_name}</p>
              {selectedStock.has_data ? (
                <p className="text-sm text-green-600 font-medium">✓ {selectedStock.data_count} price records available</p>
              ) : (
                <p className="text-sm text-orange-600">⚠ No price data - fetch data first</p>
              )}
            </div>
          )}
        </div>

        {/* Date Range */}
        <div className="bg-white p-6 rounded-lg shadow">
          <h3 className="text-lg font-semibold mb-4">Date Range</h3>
          <div className="space-y-3">
            <div>
              <label className="block text-sm font-medium text-gray-700">Start Date</label>
              <input 
                type="date" 
                value={startDate}
                onChange={(e) => setStartDate(e.target.value)}
                className="mt-1 w-full p-2 border border-gray-300 rounded"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700">End Date</label>
              <input 
                type="date" 
                value={endDate}
                onChange={(e) => setEndDate(e.target.value)}
                className="mt-1 w-full p-2 border border-gray-300 rounded"
              />
            </div>
          </div>
        </div>

        {/* Actions */}
        <div className="bg-white p-6 rounded-lg shadow">
          <h3 className="text-lg font-semibold mb-4">Actions</h3>
          <div className="space-y-3">
            <button 
              onClick={loadPriceHistory}
              disabled={!selectedStock || loading}
              className="w-full bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700 disabled:bg-gray-400"
            >
              {loading ? 'Loading...' : 'Load Price History'}
            </button>
            
            <div className="flex space-x-2">
              <button 
                onClick={() => handleExport('csv')}
                disabled={!selectedStock}
                className="flex-1 bg-green-600 text-white px-3 py-2 text-sm rounded hover:bg-green-700 disabled:bg-gray-400"
              >
                Export CSV
              </button>
              <button 
                onClick={() => handleExport('json')}
                disabled={!selectedStock}
                className="flex-1 bg-purple-600 text-white px-3 py-2 text-sm rounded hover:bg-purple-700 disabled:bg-gray-400"
              >
                Export JSON
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Price History Display */}
      {priceHistory.length > 0 && (
        <div className="bg-white rounded-lg shadow p-6">
          <h3 className="text-xl font-semibold mb-4">
            Price History - {selectedStock?.symbol} 
            <span className="text-sm text-gray-500 ml-2">({priceHistory.length} records)</span>
          </h3>
          
          {/* Simple Price Chart (Text-based for now) */}
          <div className="mb-6">
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
              <div className="text-center p-3 bg-blue-50 rounded">
                <div className="text-sm text-gray-500">Latest Close</div>
                <div className="text-xl font-bold text-blue-600">
                  ${priceHistory[priceHistory.length - 1]?.close.toFixed(2)}
                </div>
              </div>
              <div className="text-center p-3 bg-green-50 rounded">
                <div className="text-sm text-gray-500">Highest</div>
                <div className="text-xl font-bold text-green-600">
                  ${Math.max(...priceHistory.map(p => p.high)).toFixed(2)}
                </div>
              </div>
              <div className="text-center p-3 bg-red-50 rounded">
                <div className="text-sm text-gray-500">Lowest</div>
                <div className="text-xl font-bold text-red-600">
                  ${Math.min(...priceHistory.map(p => p.low)).toFixed(2)}
                </div>
              </div>
              <div className="text-center p-3 bg-yellow-50 rounded">
                <div className="text-sm text-gray-500">Avg Volume</div>
                <div className="text-xl font-bold text-yellow-600">
                  {Math.round(priceHistory.reduce((sum, p) => sum + p.volume, 0) / priceHistory.length).toLocaleString()}
                </div>
              </div>
            </div>
          </div>

          {/* Price History Table */}
          <div className="overflow-x-auto">
            <table className="min-w-full divide-y divide-gray-200">
              <thead className="bg-gray-50">
                <tr>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Date</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Open</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">High</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Low</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Close</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Volume</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">P/E</th>
                </tr>
              </thead>
              <tbody className="bg-white divide-y divide-gray-200">
                {priceHistory.slice(-10).reverse().map((price, index) => (
                  <tr key={index} className="hover:bg-gray-50">
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{price.date}</td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">${price.open.toFixed(2)}</td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-green-600">${price.high.toFixed(2)}</td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-red-600">${price.low.toFixed(2)}</td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">${price.close.toFixed(2)}</td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{price.volume.toLocaleString()}</td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                      {price.pe_ratio ? price.pe_ratio.toFixed(2) : 'N/A'}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {priceHistory.length > 10 && (
            <div className="mt-4 text-center">
              <p className="text-sm text-gray-500">
                Showing latest 10 records out of {priceHistory.length} total
              </p>
            </div>
          )}
        </div>
      )}

      {/* No Data State */}
      {!selectedStock && (
        <div className="bg-white rounded-lg shadow p-12 text-center">
          <div className="text-gray-400 mb-4">
            <svg className="mx-auto h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
            </svg>
          </div>
          <h3 className="text-lg font-medium text-gray-900 mb-2">No Stock Selected</h3>
          <p className="text-gray-600">Select a stock from the dropdown above to begin analysis</p>
        </div>
      )}
    </div>
  );
}

function DataFetchingView() {
  const [availableStocks, setAvailableStocks] = useState([]);
  const [selectedStock, setSelectedStock] = useState('');
  const [startDate, setStartDate] = useState('2024-01-01');
  const [endDate, setEndDate] = useState('2024-12-31');
  const [fetchMode, setFetchMode] = useState('single'); // 'single' or 'concurrent'
  const [message, setMessage] = useState('');
  const [loading, setLoading] = useState(false);
  const [initStatus, setInitStatus] = useState(null);
  const [initializing, setInitializing] = useState(false);

  // Load available stocks and initialization status on component mount
  useEffect(() => {
    async function loadData() {
      try {
        try {
          const [stocks, stocksWithData, status] = await Promise.all([
            invoke('get_available_stock_symbols'),
            invoke('get_stocks_with_data_status'),
            invoke('get_initialization_status')
          ]);
          setAvailableStocks(stocksWithData); // Use enhanced data with has_data info
          console.log('Loaded stocksWithData:', stocksWithData.slice(0, 3)); // Debug log
          setInitStatus(status);
        } catch (dataError) {
          console.error('Error calling get_stocks_with_data_status:', dataError);
          // Fallback to available symbols only
          const [stocks, status] = await Promise.all([
            invoke('get_available_stock_symbols'),
            invoke('get_initialization_status')
          ]);
          setAvailableStocks(stocks); // Use old data structure as fallback
          setInitStatus(status);
        }
        if (stocks.length > 0 && stocks[0].symbol !== 'INIT') {
          setSelectedStock(stocks[0].symbol);
        }
      } catch (error) {
        console.error('Failed to load data:', error);
      }
    }
    loadData();
  }, []);

  async function handleInitializeStocks() {
    setInitializing(true);
    setMessage('');
    
    try {
      const result = await invoke('initialize_sp500_stocks');
      setMessage(result);
      
      // Reload stocks and status after initialization
      const [stocks, stocksWithData, status] = await Promise.all([
        invoke('get_available_stock_symbols'),
        invoke('get_stocks_with_data_status'),
        invoke('get_initialization_status')
      ]);
      setAvailableStocks(stocksWithData);
      setInitStatus(status);
      if (stocks.length > 0) {
        setSelectedStock(stocks[0].symbol);
      }
    } catch (error) {
      setMessage(`Initialization failed: ${error}`);
    } finally {
      setInitializing(false);
    }
  }

  async function handleSingleStockFetch() {
    if (!selectedStock) {
      setMessage('Please select a stock symbol');
      return;
    }

    setLoading(true);
    setMessage('');
    
    try {
      const result = await invoke('fetch_single_stock_data', {
        symbol: selectedStock,
        startDate,
        endDate
      });
      setMessage(result);
    } catch (error) {
      setMessage(`Error: ${error}`);
    } finally {
      setLoading(false);
    }
  }

  async function handleConcurrentFetch() {
    setLoading(true);
    setMessage('');
    
    try {
      const result = await invoke('fetch_all_stocks_concurrent', {
        startDate,
        endDate
      });
      setMessage(result);
    } catch (error) {
      setMessage(`Error: ${error}`);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div>
      <h2 className="text-2xl font-bold mb-6">Data Fetching</h2>
      
      <div className="space-y-6">
        {/* S&P 500 Initialization Status */}
        {initStatus && (
          <div className="bg-blue-50 p-4 rounded-lg border border-blue-200">
            <h3 className="font-semibold mb-3 text-blue-800">S&P 500 Stock Database</h3>
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-gray-700 mb-1">{initStatus.status}</p>
                <p className="text-xs text-gray-500">
                  Companies: {initStatus.companies_processed} / {initStatus.total_companies}
                </p>
              </div>
              <button
                onClick={handleInitializeStocks}
                disabled={initializing}
                className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700 disabled:bg-gray-400 text-sm"
              >
                {initializing ? 'Initializing...' : 'Initialize S&P 500 Stocks'}
              </button>
            </div>
            {initializing && (
              <div className="mt-3 flex items-center gap-2">
                <div className="w-4 h-4 border-2 border-blue-600 border-t-transparent rounded-full animate-spin"></div>
                <span className="text-sm text-blue-700">Fetching 503 S&P 500 companies from GitHub...</span>
              </div>
            )}
          </div>
        )}

        {/* Date Range Selection */}
        <div className="bg-gray-50 p-4 rounded-lg">
          <h3 className="font-semibold mb-3">Date Range</h3>
          <div className="flex gap-4">
            <div>
              <label className="block text-sm font-medium mb-1">Start Date</label>
              <input
                type="date"
                value={startDate}
                onChange={(e) => setStartDate(e.target.value)}
                className="border rounded px-3 py-2"
              />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">End Date</label>
              <input
                type="date"
                value={endDate}
                onChange={(e) => setEndDate(e.target.value)}
                className="border rounded px-3 py-2"
              />
            </div>
          </div>
        </div>

        {/* Fetch Mode Selection */}
        <div className="bg-gray-50 p-4 rounded-lg">
          <h3 className="font-semibold mb-3">Fetch Mode</h3>
          <div className="flex gap-4">
            <label className="flex items-center">
              <input
                type="radio"
                name="fetchMode"
                value="single"
                checked={fetchMode === 'single'}
                onChange={(e) => setFetchMode(e.target.value)}
                className="mr-2"
              />
              Single Stock
            </label>
            <label className="flex items-center">
              <input
                type="radio"
                name="fetchMode"
                value="concurrent"
                checked={fetchMode === 'concurrent'}
                onChange={(e) => setFetchMode(e.target.value)}
                className="mr-2"
              />
              All Stocks (Concurrent)
            </label>
          </div>
        </div>

        {/* Single Stock Selection */}
        {fetchMode === 'single' && (
          <div className="bg-gray-50 p-4 rounded-lg">
            <h3 className="font-semibold mb-3">Stock Selection</h3>
            <select
              value={selectedStock}
              onChange={(e) => setSelectedStock(e.target.value)}
              className="border rounded px-3 py-2 min-w-48"
            >
              {availableStocks.map((stock) => (
                <option key={stock.symbol} value={stock.symbol}>
                  {stock.has_data !== undefined ? (stock.has_data ? '📊' : '📋') : '🔍'} {stock.symbol} - {stock.company_name}
                  {stock.has_data ? ` (${stock.data_count} records)` : stock.has_data === false ? ' (no data)' : ' (checking...)'}
                </option>
              ))}
            </select>
          </div>
        )}

        {/* Action Buttons */}
        <div className="flex gap-4">
          {fetchMode === 'single' ? (
            <button
              onClick={handleSingleStockFetch}
              disabled={loading}
              className="bg-blue-500 text-white px-6 py-3 rounded hover:bg-blue-600 disabled:bg-gray-400"
            >
              {loading ? 'Fetching...' : `Fetch ${selectedStock} Data`}
            </button>
          ) : (
            <button
              onClick={handleConcurrentFetch}
              disabled={loading}
              className="bg-green-500 text-white px-6 py-3 rounded hover:bg-green-600 disabled:bg-gray-400"
            >
              {loading ? 'Fetching All Stocks...' : 'Fetch All Stocks (Concurrent)'}
            </button>
          )}
        </div>

        {/* Progress Display */}
        {loading && (
          <div className="bg-blue-50 p-4 rounded-lg border border-blue-200">
            <div className="flex items-center gap-2 mb-2">
              <div className="w-4 h-4 border-2 border-blue-600 border-t-transparent rounded-full animate-spin"></div>
              <span className="font-medium">
                {fetchMode === 'single' ? `Fetching ${selectedStock}...` : 'Fetching all stocks...'}
              </span>
            </div>
            <div className="text-sm text-gray-600">
              Please wait while we retrieve stock data for the selected date range.
            </div>
          </div>
        )}

        {/* Results Display */}
        {message && (
          <div className="mt-4 p-4 bg-gray-100 border rounded-lg">
            <h4 className="font-medium mb-2">Results:</h4>
            <p className="text-sm whitespace-pre-wrap">{message}</p>
          </div>
        )}
      </div>
    </div>
  );
}

function App() {
  const [stocks, setStocks] = useState([]);
  const [dbStats, setDbStats] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [currentView, setCurrentView] = useState('dashboard');
  const [selectedStockForDetails, setSelectedStockForDetails] = useState(null);

  useEffect(() => {
    fetchInitialData();
  }, []);

  async function fetchInitialData() {
    try {
      setLoading(true);
      const [stocksData, statsData] = await Promise.all([
        invoke('get_stocks_with_data_status'),
        invoke('get_database_stats')
      ]);
      setStocks(stocksData);
      setDbStats(statsData);
    } catch (err) {
      setError(`Failed to fetch data: ${err}`);
      console.error('Error fetching data:', err);
    } finally {
      setLoading(false);
    }
  }

  async function handleSearch() {
    if (!searchQuery.trim()) {
      fetchInitialData();
      return;
    }
    
    try {
      const results = await invoke('search_stocks', { query: searchQuery });
      setStocks(results);
    } catch (err) {
      setError(`Search failed: ${err}`);
    }
  }

  async function handleFetchData() {
    try {
      const result = await invoke('fetch_stock_data', {
        stockSymbols: stocks.slice(0, 5).map(s => s.symbol), // First 5 stocks as example
        startDate: '2024-01-01',
        endDate: '2024-12-31'
      });
      alert(result);
    } catch (err) {
      setError(`Data fetch failed: ${err}`);
    }
  }

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-100 flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-32 w-32 border-b-2 border-blue-600 mx-auto"></div>
          <p className="mt-4 text-gray-600">Loading stock data...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen bg-gray-100 flex items-center justify-center">
        <div className="bg-red-50 border border-red-200 rounded-lg p-6 max-w-md">
          <h2 className="text-red-800 font-semibold mb-2">Error</h2>
          <p className="text-red-600">{error}</p>
          <button 
            onClick={() => { setError(null); fetchInitialData(); }}
            className="mt-4 bg-red-600 text-white px-4 py-2 rounded hover:bg-red-700"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-100">
      {/* Header */}
      <header className="bg-blue-600 text-white shadow-lg">
        <div className="container mx-auto px-4 py-4">
          <div className="flex justify-between items-center">
            <h1 className="text-2xl font-bold">Stock Analysis System</h1>
            <nav className="flex space-x-4">
              <button 
                onClick={() => setCurrentView('dashboard')}
                className={`px-4 py-2 rounded ${currentView === 'dashboard' ? 'bg-blue-800' : 'hover:bg-blue-700'}`}
              >
                Dashboard
              </button>
              <button 
                onClick={() => setCurrentView('stocks')}
                className={`px-4 py-2 rounded ${currentView === 'stocks' ? 'bg-blue-800' : 'hover:bg-blue-700'}`}
              >
                Stocks
              </button>
              <button 
                onClick={() => setCurrentView('analysis')}
                className={`px-4 py-2 rounded ${currentView === 'analysis' ? 'bg-blue-800' : 'hover:bg-blue-700'}`}
              >
                Analysis
              </button>
              <button 
                onClick={() => setCurrentView('fetching')}
                className={`px-4 py-2 rounded ${currentView === 'fetching' ? 'bg-blue-800' : 'hover:bg-blue-700'}`}
              >
                Data Fetching
              </button>
              <button 
                onClick={() => setCurrentView('enhanced-fetching')}
                className={`px-4 py-2 rounded ${currentView === 'enhanced-fetching' ? 'bg-blue-800' : 'hover:bg-blue-700'}`}
              >
                Enhanced Fetching
              </button>
            </nav>
          </div>
        </div>
      </header>

      <div className="container mx-auto px-4 py-8">
        {currentView === 'dashboard' && (
          <div>
            <h2 className="text-3xl font-bold mb-6">Dashboard</h2>
            
            {/* Database Stats */}
            {dbStats && (
              <div className="grid grid-cols-1 md:grid-cols-4 gap-6 mb-8">
                <div className="bg-white p-6 rounded-lg shadow">
                  <h3 className="text-gray-500 text-sm font-medium">Total Stocks</h3>
                  <p className="text-2xl font-bold text-blue-600">{dbStats.total_stocks.toLocaleString()}</p>
                </div>
                <div className="bg-white p-6 rounded-lg shadow">
                  <h3 className="text-gray-500 text-sm font-medium">Price Records</h3>
                  <p className="text-2xl font-bold text-green-600">{dbStats.total_price_records.toLocaleString()}</p>
                </div>
                <div className="bg-white p-6 rounded-lg shadow">
                  <h3 className="text-gray-500 text-sm font-medium">Data Coverage</h3>
                  <p className="text-2xl font-bold text-purple-600">{dbStats.data_coverage_percentage}%</p>
                </div>
                <div className="bg-white p-6 rounded-lg shadow">
                  <h3 className="text-gray-500 text-sm font-medium">Last Update</h3>
                  <p className="text-lg font-semibold text-gray-700">{dbStats.last_update}</p>
                </div>
              </div>
            )}

            {/* Quick Actions */}
            <div className="bg-white rounded-lg shadow p-6">
              <h3 className="text-xl font-semibold mb-4">Quick Actions</h3>
              <div className="flex space-x-4">
                <button 
                  onClick={handleFetchData}
                  className="bg-blue-600 text-white px-6 py-2 rounded hover:bg-blue-700 transition-colors"
                >
                  Fetch Recent Data
                </button>
                <button 
                  onClick={() => setCurrentView('stocks')}
                  className="bg-green-600 text-white px-6 py-2 rounded hover:bg-green-700 transition-colors"
                >
                  View Stocks
                </button>
                <button 
                  onClick={() => setCurrentView('analysis')}
                  className="bg-purple-600 text-white px-6 py-2 rounded hover:bg-purple-700 transition-colors"
                >
                  Start Analysis
                </button>
              </div>
            </div>
          </div>
        )}

        {currentView === 'stocks' && (
          <div>
            <div className="flex justify-between items-center mb-6">
              <h2 className="text-3xl font-bold">Stocks</h2>
              <div className="flex space-x-4">
                <input
                  type="text"
                  placeholder="Search stocks..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  onKeyPress={(e) => e.key === 'Enter' && handleSearch()}
                  className="px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
                <button 
                  onClick={handleSearch}
                  className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700"
                >
                  Search
                </button>
              </div>
            </div>

            <div className="bg-white rounded-lg shadow overflow-hidden">
              <div className="overflow-x-auto">
                <table className="min-w-full divide-y divide-gray-200">
                  <thead className="bg-gray-50">
                    <tr>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Symbol</th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Company</th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Sector</th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Actions</th>
                    </tr>
                  </thead>
                  <tbody className="bg-white divide-y divide-gray-200">
                    {stocks.map((stock) => (
                      <tr key={stock.id} className="hover:bg-gray-50">
                        <td className="px-6 py-4 whitespace-nowrap">
                          <div className="text-sm font-medium text-gray-900">{stock.symbol}</div>
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap">
                          <div className="text-sm text-gray-900">{stock.company_name}</div>
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap">
                          <span className="inline-flex px-2 py-1 text-xs font-semibold rounded-full bg-blue-100 text-blue-800">
                            {stock.sector || 'N/A'}
                          </span>
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm font-medium">
                          <div className="flex space-x-2">
                            <button 
                              onClick={() => setCurrentView('analysis')}
                              className="text-blue-600 hover:text-blue-900"
                            >
                              Basic Analysis
                            </button>
                            <button 
                              onClick={() => {
                                setSelectedStockForDetails(stock);
                                setCurrentView('enhanced-details');
                              }}
                              className="text-green-600 hover:text-green-900"
                            >
                              Enhanced View
                            </button>
                          </div>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        )}

        {currentView === 'analysis' && <AnalysisView stocks={stocks} />}
        
        {currentView === 'fetching' && <DataFetchingView />}

        {currentView === 'enhanced-fetching' && <EnhancedDataFetching />}

        {currentView === 'enhanced-details' && selectedStockForDetails && (
          <EnhancedStockDetails 
            selectedStock={selectedStockForDetails}
            onBack={() => {
              setCurrentView('stocks');
              setSelectedStockForDetails(null);
            }}
          />
        )}
      </div>
    </div>
  );
}

export default App;