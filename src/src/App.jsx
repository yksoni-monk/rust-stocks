import { useEffect, useState } from 'react';
import StockRow from './components/StockRow';
import RecommendationsPanel from './components/RecommendationsPanel';
import { stockDataService, systemDataService } from './services/dataService.js';

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
  const [sp500Filter, setSp500Filter] = useState(false);
  const [sp500Symbols, setSp500Symbols] = useState([]);
  const [showRecommendations, setShowRecommendations] = useState(false);
  const [selectedScreeningType, setSelectedScreeningType] = useState('ps'); // Default to P/S
  
  const STOCKS_PER_PAGE = 50;

  useEffect(() => {
    fetchInitialData();
    loadInitializationStatus();
    loadSp500Symbols();
  }, []);

  async function fetchInitialData() {
    try {
      setLoading(true);
      setCurrentPage(0);
      setStocks([]);
      
      const result = await stockDataService.loadInitialStockData(STOCKS_PER_PAGE);
      
      if (result.error) {
        setError(result.error);
        console.error('Error fetching data:', result.error);
      } else {
        setStocks(result.stocks);
        setHasMoreStocks(result.hasMore);
        setTotalStocks(result.totalStocks);
      }
      
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
      
      const result = await stockDataService.loadMoreStocks(nextPage, STOCKS_PER_PAGE);
      
      if (result.error) {
        setError(result.error);
        console.error('Error loading more stocks:', result.error);
        return;
      }
      
      // Apply S&P 500 filter if active
      let filteredStocks = result.stocks;
      if (sp500Filter) {
        // Ensure we have S&P 500 symbols loaded
        let currentSp500Symbols = sp500Symbols;
        if (sp500Symbols.length === 0) {
          console.log('🔄 Loading S&P 500 symbols for pagination...');
          const sp500Result = await stockDataService.loadSp500Symbols();
          if (sp500Result.error) {
            setError(sp500Result.error);
            return;
          }
          currentSp500Symbols = sp500Result.symbols;
          setSp500Symbols(currentSp500Symbols);
        }
        filteredStocks = stockDataService.filterStocksBySp500(result.stocks, currentSp500Symbols);
      }
      
      setStocks(prev => [...prev, ...filteredStocks]);
      setCurrentPage(nextPage);
      setHasMoreStocks(result.hasMore);
      
    } catch (err) {
      setError(`Failed to load more stocks: ${err}`);
    } finally {
      setLoading(false);
    }
  }

  async function loadInitializationStatus() {
    try {
      const result = await systemDataService.loadInitializationStatus();
      if (result.error) {
        console.error('Failed to load initialization status:', result.error);
      } else {
        setInitStatus(result.status);
      }
    } catch (err) {
      console.error('Failed to load initialization status:', err);
    }
  }

  async function loadSp500Symbols() {
    try {
      console.log('🔄 Loading S&P 500 symbols...');
      const result = await stockDataService.loadSp500Symbols();
      
      if (result.error) {
        console.error('❌ Failed to load S&P 500 symbols:', result.error);
      } else {
        console.log('✅ Loaded S&P 500 symbols:', result.symbols.length, 'symbols');
        console.log('📋 First 10 symbols:', result.symbols.slice(0, 10));
        setSp500Symbols(result.symbols);
      }
    } catch (err) {
      console.error('❌ Failed to load S&P 500 symbols:', err);
    }
  }

  async function handleSp500Filter() {
    const newFilterState = !sp500Filter;
    console.log('🔍 S&P 500 Filter clicked. New state:', newFilterState);
    console.log('📊 Current sp500Symbols length:', sp500Symbols.length);
    console.log('📋 First 5 sp500Symbols:', sp500Symbols.slice(0, 5));
    
    setSp500Filter(newFilterState);
    setCurrentPage(0);
    setStocks([]);
    
    try {
      setLoading(true);
      
      // Ensure S&P 500 symbols are loaded if filter is being turned ON
      let currentSp500Symbols = sp500Symbols;
      if (newFilterState && sp500Symbols.length === 0) {
        console.log('🔄 S&P 500 symbols not loaded, loading now...');
        const sp500Result = await stockDataService.loadSp500Symbols();
        if (sp500Result.error) {
          setError(sp500Result.error);
          return;
        }
        currentSp500Symbols = sp500Result.symbols;
        setSp500Symbols(currentSp500Symbols);
        console.log('✅ Loaded S&P 500 symbols:', currentSp500Symbols.length);
      }
      
      // Load first page of stocks
      const result = await stockDataService.loadInitialStockData(STOCKS_PER_PAGE);
      
      if (result.error) {
        setError(result.error);
        console.error('Error loading stocks:', result.error);
        return;
      }
      
      console.log('📈 Loaded stocks data:', result.stocks.length, 'stocks');
      console.log('📋 First 5 stock symbols:', result.stocks.slice(0, 5).map(s => s.symbol));
      
      // Apply S&P 500 filter if enabled
      let filteredStocks = result.stocks;
      if (newFilterState) { // If filter is now ON
        console.log('🔍 Applying S&P 500 filter...');
        console.log('📊 Using S&P 500 symbols:', currentSp500Symbols.length);
        filteredStocks = stockDataService.filterStocksBySp500(result.stocks, currentSp500Symbols);
        console.log('✅ Filtered stocks:', filteredStocks.length, 'stocks');
        console.log('📋 Filtered stock symbols:', filteredStocks.map(s => s.symbol));
      }
      
      setStocks(filteredStocks);
      setHasMoreStocks(result.hasMore);
      
      // Calculate total count for display
      let totalCount = result.totalStocks;
      if (newFilterState) { // If filter is now ON
        console.log('🔍 Calculating total S&P 500 count...');
        console.log('📊 All stocks count:', result.totalStocks);
        console.log('📊 S&P 500 symbols count:', currentSp500Symbols.length);
        // We need to get all stocks to calculate S&P 500 total
        const allStocksResult = await stockDataService.loadInitialStockData(10000); // Large limit to get all
        if (allStocksResult.success) {
          const allStocks = allStocksResult.stocks;
          totalCount = stockDataService.filterStocksBySp500(allStocks, currentSp500Symbols).length;
          console.log('✅ Total S&P 500 stocks:', totalCount);
        }
      }
      setTotalStocks(totalCount);
      
    } catch (err) {
      setError(`Failed to apply filter: ${err}`);
      console.error('Error applying filter:', err);
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
      setLoading(true);
      const result = await stockDataService.searchStocks(searchQuery);
      
      if (result.error) {
        setError(result.error);
        console.error('Search failed:', result.error);
      } else {
        setStocks(result.stocks);
      }
    } catch (err) {
      setError(`Search failed: ${err}`);
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
        {/* S&P 500 Filter */}
        <div className="bg-blue-50 p-4 rounded-lg border border-blue-200 mb-6">
          <div className="flex items-center justify-between">
            <div>
              <h3 className="font-semibold text-blue-800">S&P 500 Filter</h3>
              <p className="text-sm text-blue-700">
                {sp500Filter ? 'Showing only S&P 500 stocks' : 'Showing all stocks'}
              </p>
              <p className="text-xs text-blue-600">
                {sp500Symbols.length} S&P 500 symbols loaded
              </p>
            </div>
            <button
              onClick={handleSp500Filter}
              className={`px-4 py-2 rounded text-sm font-medium ${
                sp500Filter 
                  ? 'bg-blue-600 text-white hover:bg-blue-700' 
                  : 'bg-white text-blue-600 border border-blue-600 hover:bg-blue-50'
              }`}
            >
              {sp500Filter ? 'Show All Stocks' : 'Filter S&P 500'}
            </button>
          </div>
        </div>

        {/* Value Picks Button */}
        <div className="bg-green-50 p-4 rounded-lg border border-green-200 mb-6">
          <div className="flex items-center justify-between">
            <div className="flex-1">
              <h3 className="font-semibold text-green-800">💎 Value Stock Analysis</h3>
              <p className="text-sm text-green-700">
                Find undervalued stocks using P/E and P/S ratio screening
              </p>
              <p className="text-xs text-green-600">
                Advanced screening with historical context and quality filters
              </p>
              
              {/* Filter Selection */}
              <div className="mt-3 flex items-center gap-4">
                <label className="text-sm font-medium text-green-800">
                  Screening Method:
                </label>
                <select
                  value={selectedScreeningType}
                  onChange={(e) => setSelectedScreeningType(e.target.value)}
                  className="border border-green-300 rounded px-3 py-1 text-sm bg-white focus:outline-none focus:ring-2 focus:ring-green-500"
                >
                  <option value="ps">P/S Ratio (Smart Algorithm) - Default</option>
                  <option value="pe">P/E Ratio (Historical)</option>
                  <option value="garp_pe">GARP (P/E + PEG Based)</option>
                </select>
              </div>
            </div>
            <button
              onClick={() => setShowRecommendations(true)}
              className="bg-green-600 text-white px-4 py-2 rounded text-sm font-medium hover:bg-green-700 ml-4"
            >
              Get Value Stocks
            </button>
          </div>
        </div>

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
                : sp500Filter 
                  ? 'No S&P 500 stocks found. Try adjusting your filter or search.'
                  : 'No stocks available in the database.'
              }
            </p>
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

        {/* Recommendations Panel */}
        {showRecommendations && (
          <RecommendationsPanel 
            onClose={() => setShowRecommendations(false)}
            initialScreeningType={selectedScreeningType}
          />
        )}
      </div>
    </div>
  );
}

export default App;