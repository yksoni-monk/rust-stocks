import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import StockRow from './components/StockRow';

function App() {
  const [stocks, setStocks] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [expandedPanels, setExpandedPanels] = useState({});
  const [initStatus, setInitStatus] = useState(null);
  const [initializing, setInitializing] = useState(false);
  const [currentPage, setCurrentPage] = useState(0);
  const [hasMoreStocks, setHasMoreStocks] = useState(true);
  const [totalStocks, setTotalStocks] = useState(0);
  
  const STOCKS_PER_PAGE = 50;

  useEffect(() => {
    fetchInitialData();
    loadInitializationStatus();
  }, []);

  async function fetchInitialData() {
    try {
      setLoading(true);
      setCurrentPage(0);
      setStocks([]);
      
      // Load first page of stocks
      const stocksData = await invoke('get_stocks_paginated', { 
        limit: STOCKS_PER_PAGE, 
        offset: 0 
      });
      
      setStocks(stocksData);
      setHasMoreStocks(stocksData.length === STOCKS_PER_PAGE);
      
      // Get total count for display
      const allStocks = await invoke('get_stocks_with_data_status');
      setTotalStocks(allStocks.length);
      
    } catch (err) {
      setError(`Failed to fetch data: ${err}`);
      console.error('Error fetching data:', err);
    } finally {
      setLoading(false);
    }
  }

  async function loadMoreStocks() {
    if (loading || !hasMoreStocks) return;
    
    try {
      setLoading(true);
      const nextPage = currentPage + 1;
      const stocksData = await invoke('get_stocks_paginated', { 
        limit: STOCKS_PER_PAGE, 
        offset: nextPage * STOCKS_PER_PAGE 
      });
      
      setStocks(prev => [...prev, ...stocksData]);
      setCurrentPage(nextPage);
      setHasMoreStocks(stocksData.length === STOCKS_PER_PAGE);
      
    } catch (err) {
      setError(`Failed to load more stocks: ${err}`);
    } finally {
      setLoading(false);
    }
  }

  async function loadInitializationStatus() {
    try {
      const status = await invoke('get_initialization_status');
      setInitStatus(status);
    } catch (err) {
      console.error('Failed to load initialization status:', err);
    }
  }

  async function handleSearch() {
    if (!searchQuery.trim()) {
      fetchInitialData();
      return;
    }
    
    try {
      setLoading(true);
      const results = await invoke('search_stocks', { query: searchQuery });
      setStocks(results);
    } catch (err) {
      setError(`Search failed: ${err}`);
    } finally {
      setLoading(false);
    }
  }

  async function handleInitializeStocks() {
    setInitializing(true);
    
    try {
      const result = await invoke('initialize_sp500_stocks');
      console.log('Initialization result:', result);
      
      // Reload stocks and status after initialization
      await fetchInitialData();
      await loadInitializationStatus();
    } catch (error) {
      setError(`Initialization failed: ${error}`);
    } finally {
      setInitializing(false);
    }
  }

  async function handleBulkFetch(fetchMode = 'compact') {
    if (stocks.length === 0) return;

    const confirmed = confirm(
      `This will fetch ${fetchMode} data for all ${stocks.length} stocks.\n\n` +
      `⏱️ Estimated time: ${Math.ceil(stocks.length * 0.5)} minutes\n` +
      `📊 Data: ${fetchMode === 'compact' ? '100 days per stock' : '20+ years per stock'}\n\n` +
      `Continue?`
    );

    if (!confirmed) return;

    setLoading(true);

    try {
      const result = await invoke('fetch_all_stocks_comprehensive', {
        fetchMode: fetchMode
      });

      alert(`✅ ${result}`);
      await fetchInitialData(); // Refresh data
    } catch (error) {
      console.error('Bulk fetch failed:', error);
      alert(`❌ Bulk fetch failed: ${error}`);
    } finally {
      setLoading(false);
    }
  }

  const handleToggleExpansion = (stockId, panelType) => {
    console.log('handleToggleExpansion called:', { stockId, panelType });
    setExpandedPanels(prev => {
      const newState = { ...prev };
      console.log('Previous expandedPanels:', prev);
      
      if (newState[stockId] === panelType) {
        // Collapse if same panel is clicked
        console.log('Collapsing panel');
        delete newState[stockId];
      } else {
        // Expand new panel (or switch to different panel)
        console.log('Expanding panel');
        newState[stockId] = panelType;
      }
      
      console.log('New expandedPanels:', newState);
      return newState;
    });
  };

  if (loading && stocks.length === 0) {
    return (
      <div className="min-h-screen bg-gray-100 flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-32 w-32 border-b-2 border-blue-600 mx-auto"></div>
          <p className="mt-4 text-gray-600">Loading stock data...</p>
        </div>
      </div>
    );
  }

  if (error && stocks.length === 0) {
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
            <div className="flex items-center space-x-4">
              <input
                type="text"
                placeholder="Search stocks..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                onKeyPress={(e) => e.key === 'Enter' && handleSearch()}
                className="px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900"
              />
              <button 
                onClick={handleSearch}
                className="bg-blue-700 text-white px-4 py-2 rounded-lg hover:bg-blue-800"
              >
                Search
              </button>
            </div>
          </div>
        </div>
      </header>

      <div className="container mx-auto px-4 py-8">
        {/* S&P 500 Initialization Status */}
        {initStatus && (
          <div className="bg-blue-50 p-4 rounded-lg border border-blue-200 mb-6">
            <div className="flex items-center justify-between">
              <div>
                <h3 className="font-semibold text-blue-800">S&P 500 Database Status</h3>
                <p className="text-sm text-blue-700">{initStatus.status}</p>
                <p className="text-xs text-blue-600">
                  Companies: {initStatus.companies_processed} / {initStatus.total_companies}
                </p>
              </div>
              <div className="flex space-x-2">
                <button
                  onClick={handleInitializeStocks}
                  disabled={initializing}
                  className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700 disabled:bg-gray-400 text-sm"
                >
                  {initializing ? 'Initializing...' : 'Initialize S&P 500'}
                </button>
                <div className="flex space-x-2">
                  <button
                    onClick={() => handleBulkFetch('compact')}
                    disabled={loading || stocks.length === 0}
                    className="bg-green-600 text-white px-3 py-2 rounded hover:bg-green-700 disabled:bg-gray-400 text-sm"
                  >
                    {loading ? 'Fetching...' : 'Bulk: Compact'}
                  </button>
                  <button
                    onClick={() => handleBulkFetch('full')}
                    disabled={loading || stocks.length === 0}
                    className="bg-purple-600 text-white px-3 py-2 rounded hover:bg-purple-700 disabled:bg-gray-400 text-sm"
                  >
                    {loading ? 'Fetching...' : 'Bulk: Full'}
                  </button>
                </div>
              </div>
            </div>
            {initializing && (
              <div className="mt-3 flex items-center gap-2">
                <div className="w-4 h-4 border-2 border-blue-600 border-t-transparent rounded-full animate-spin"></div>
                <span className="text-sm text-blue-700">Fetching S&P 500 companies...</span>
              </div>
            )}
          </div>
        )}

        {/* Stock Count and Status */}
        <div className="mb-6 flex justify-between items-center">
          <div className="text-gray-600">
            <span className="text-lg font-medium">{stocks.length}</span> of <span className="text-lg font-medium">{totalStocks}</span> stocks loaded
            {stocks.filter(s => s.has_data).length > 0 && (
              <span className="ml-2 text-sm">
                • {stocks.filter(s => s.has_data).length} with data
              </span>
            )}
          </div>
          
          {/* Legend */}
          <div className="flex items-center gap-4 text-sm text-gray-600">
            <div className="flex items-center gap-1">
              <span>📊</span>
              <span>Has data</span>
            </div>
            <div className="flex items-center gap-1">
              <span>📋</span>
              <span>No data</span>
            </div>
            <div className="flex items-center gap-1">
              <span>🔍</span>
              <span>Checking...</span>
            </div>
          </div>
        </div>

        {/* Expandable Stock List */}
        <div className="space-y-2">
          {stocks.map((stock) => (
            <StockRow
              key={stock.id || stock.symbol}
              stock={stock}
              isExpanded={!!expandedPanels[stock.id || stock.symbol]}
              expandedPanel={expandedPanels[stock.id || stock.symbol]}
              onToggleExpansion={handleToggleExpansion}
            />
          ))}
        </div>

        {/* Load More Button */}
        {hasMoreStocks && stocks.length > 0 && (
          <div className="mt-6 text-center">
            <button
              onClick={loadMoreStocks}
              disabled={loading}
              className="bg-blue-600 text-white px-6 py-3 rounded-lg hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed"
            >
              {loading ? 'Loading...' : `Load More Stocks (${totalStocks - stocks.length} remaining)`}
            </button>
          </div>
        )}

        {/* Empty State */}
        {stocks.length === 0 && !loading && (
          <div className="bg-white rounded-lg shadow p-12 text-center">
            <div className="text-gray-400 mb-4">
              <svg className="mx-auto h-16 w-16" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
              </svg>
            </div>
            <h3 className="text-xl font-medium text-gray-900 mb-2">No Stocks Found</h3>
            <p className="text-gray-600 mb-4">
              {searchQuery 
                ? `No stocks match "${searchQuery}". Try a different search term.`
                : 'No stocks available. Initialize the S&P 500 database to get started.'
              }
            </p>
            {!searchQuery && (
              <button
                onClick={handleInitializeStocks}
                disabled={initializing}
                className="bg-blue-600 text-white px-6 py-3 rounded-lg hover:bg-blue-700 disabled:bg-gray-400"
              >
                {initializing ? 'Initializing...' : 'Initialize S&P 500 Stocks'}
              </button>
            )}
          </div>
        )}

        {/* Loading Overlay for Bulk Operations */}
        {loading && stocks.length > 0 && (
          <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
            <div className="bg-white p-6 rounded-lg shadow-xl">
              <div className="flex items-center space-x-3">
                <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
                <span className="text-lg">Processing stocks...</span>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default App;