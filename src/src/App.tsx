import { createSignal, createEffect, onMount, For, Show } from 'solid-js';
import { stockStore } from './stores/stockStore';
import { recommendationsStore } from './stores/recommendationsStore';
import { uiStore } from './stores/uiStore';
import StockRow from './components/StockRow';
import RecommendationsPanel from './components/RecommendationsPanel';
import DataFetchingPanel from './components/DataFetchingPanel';

function App() {
  console.log('🚀 SolidJS App starting...');
  
  // Local signals for search and filtering
  const [searchInput, setSearchInput] = createSignal('');
  
  // Initialize data on mount
  onMount(async () => {
    console.log('🔄 Loading initial data...');
    await Promise.all([
      stockStore.loadInitialStocks(),
      stockStore.loadSp500Symbols()
    ]);
    console.log('✅ Initial data loaded');
  });

  // Search handler with debouncing
  const handleSearch = async () => {
    const query = searchInput().trim();
    if (query) {
      await stockStore.searchStocks(query);
    } else {
      await stockStore.loadInitialStocks();
    }
  };

  const handleClearSearch = async () => {
    setSearchInput('');
    await stockStore.loadInitialStocks();
  };

  // Panel handlers
  const handleGetValueStocks = (screeningType: string) => {
    console.log('🔘 Get Value Stocks button clicked!');
    console.log('📋 Selected screening type:', screeningType);
    
    recommendationsStore.setScreeningType(screeningType as any);
    uiStore.openRecommendations();
  };

  const handleDataFetching = () => {
    console.log('📊 Data Fetching button clicked!');
    uiStore.openDataFetching();
  };

  return (
    <div class="min-h-screen bg-gray-50">
      <div class="container mx-auto px-4 py-8">
        {/* Header */}
        <div class="mb-8">
          <h1 class="text-4xl font-bold text-gray-900 mb-2">
            📊 Stock Analysis Dashboard
          </h1>
          <p class="text-gray-600">
            Analyze stocks with comprehensive financial data and screening tools
          </p>
        </div>

        {/* Controls */}
        <div class="bg-white rounded-lg shadow-lg p-6 mb-8">
          {/* Search Section */}
          <div class="mb-6">
            <div class="flex flex-col sm:flex-row gap-4">
              <div class="flex-1">
                <div class="relative">
                  <input
                    type="text"
                    placeholder="Search stocks by symbol or company name..."
                    value={searchInput()}
                    onInput={(e) => setSearchInput(e.target.value)}
                    onKeyPress={(e) => e.key === 'Enter' && handleSearch()}
                    class="w-full pl-10 pr-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                  />
                  <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                    <svg class="h-5 w-5 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                    </svg>
                  </div>
                </div>
              </div>
              <button
                onClick={handleSearch}
                disabled={stockStore.loading()}
                class="bg-blue-600 text-white px-6 py-2 rounded-lg hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed"
              >
                {stockStore.loading() ? 'Searching...' : 'Search'}
              </button>
              <Show when={stockStore.searchQuery()}>
                <button
                  onClick={handleClearSearch}
                  class="bg-gray-500 text-white px-4 py-2 rounded-lg hover:bg-gray-600"
                >
                  Clear
                </button>
              </Show>
            </div>
          </div>

          {/* Filters and Actions */}
          <div class="flex flex-col lg:flex-row gap-6 items-start lg:items-center justify-between">
            {/* Filters */}
            <div class="flex flex-col sm:flex-row gap-4">
              <div class="flex items-center gap-3">
                <label class="text-sm font-medium text-gray-700">Filters:</label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={stockStore.sp500Filter()}
                    onChange={(e) => stockStore.filterBySp500(e.target.checked)}
                    class="rounded border-gray-300"
                  />
                  <span class="text-sm text-gray-600">S&P 500 Only</span>
                  <Show when={stockStore.sp500Symbols().length > 0}>
                    <span class="text-xs text-gray-500 bg-gray-100 px-2 py-1 rounded">
                      {stockStore.sp500Symbols().length} symbols
                    </span>
                  </Show>
                </label>
              </div>
            </div>

            {/* Action Buttons */}
            <div class="flex flex-col sm:flex-row gap-3">
              <div class="flex items-center gap-2">
                <label class="text-sm font-medium text-gray-700">Screening:</label>
                <select
                  class="border border-gray-300 rounded px-3 py-1 text-sm"
                  onChange={(e) => recommendationsStore.setScreeningType(e.target.value as any)}
                >
                  <option value="garp_pe">GARP (P/E + PEG Based)</option>
                  <option value="ps">P/S Ratio (TTM)</option>
                  <option value="pe">P/E Ratio (Historical)</option>
                </select>
              </div>
              <button
                onClick={() => handleGetValueStocks(recommendationsStore.screeningType())}
                class="bg-green-600 text-white px-4 py-2 rounded-lg hover:bg-green-700 text-sm font-medium"
              >
                Get Value Stocks
              </button>
              <button
                onClick={handleDataFetching}
                class="bg-purple-600 text-white px-4 py-2 rounded-lg hover:bg-purple-700 text-sm font-medium"
              >
                Data Fetching
              </button>
            </div>
          </div>
        </div>

        {/* Stats */}
        <Show when={!stockStore.loading() && stockStore.stocks().length > 0}>
          <div class="bg-white rounded-lg shadow p-4 mb-6">
            <div class="flex items-center justify-between text-sm text-gray-600">
              <div class="flex items-center gap-1">
                <span>📊</span>
                <span>
                  Showing {stockStore.stocks().length} of {stockStore.totalStocks()} stocks
                  {stockStore.sp500Filter() && ' (S&P 500 filtered)'}
                  {stockStore.searchQuery() && ` (search: "${stockStore.searchQuery()}")`}
                </span>
              </div>
              <div class="flex items-center gap-1">
                <span>🔍</span>
                <span>Ready for analysis</span>
              </div>
            </div>
          </div>
        </Show>

        {/* Loading State */}
        <Show when={stockStore.loading()}>
          <div class="bg-white rounded-lg shadow p-8 mb-6">
            <div class="flex items-center justify-center">
              <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 mr-3"></div>
              <span class="text-gray-600">Loading stocks...</span>
            </div>
          </div>
        </Show>

        {/* Error State */}
        <Show when={stockStore.error()}>
          <div class="bg-red-50 border border-red-200 rounded-lg p-4 mb-6">
            <div class="flex items-center">
              <svg class="w-5 h-5 text-red-400 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <span class="text-red-800">{stockStore.error()}</span>
            </div>
          </div>
        </Show>

        {/* Recommendations Panel */}
        <Show when={uiStore.showRecommendations()}>
          <div class="mb-6">
            <RecommendationsPanel onClose={uiStore.closeAllPanels} />
          </div>
        </Show>

        {/* Data Fetching Panel */}
        <Show when={uiStore.showDataFetching()}>
          <div class="mb-6">
            <DataFetchingPanel onClose={uiStore.closeAllPanels} />
          </div>
        </Show>

        {/* Stock List */}
        <div class="space-y-2">
          <For each={stockStore.stocks()}>
            {(stock) => (
              <StockRow
                stock={stock}
                isExpanded={!!stockStore.expandedPanels()[stock.id?.toString() || stock.symbol]}
                expandedPanel={stockStore.expandedPanels()[stock.id?.toString() || stock.symbol]}
                onToggleExpansion={(panelType) => 
                  stockStore.togglePanelExpansion(
                    stock.id?.toString() || stock.symbol, 
                    panelType
                  )
                }
              />
            )}
          </For>
        </div>

        {/* Load More Button */}
        <Show when={stockStore.hasMoreStocks() && stockStore.stocks().length > 0}>
          <div class="mt-6 text-center">
            <button
              onClick={stockStore.loadMoreStocks}
              disabled={stockStore.loading()}
              class="bg-blue-600 text-white px-6 py-3 rounded-lg hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed"
            >
              {stockStore.loading() 
                ? 'Loading...' 
                : `Load More Stocks (${stockStore.totalStocks() - stockStore.stocks().length} remaining)`}
            </button>
          </div>
        </Show>

        {/* Empty State */}
        <Show when={stockStore.stocks().length === 0 && !stockStore.loading()}>
          <div class="bg-white rounded-lg shadow p-12 text-center">
            <div class="text-gray-400 mb-4">
              <svg class="mx-auto h-16 w-16" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width={2} d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
              </svg>
            </div>
            <h3 class="text-xl font-medium text-gray-900 mb-2">No Stocks Found</h3>
            <p class="text-gray-600 mb-4">
              {stockStore.searchQuery() 
                ? `No stocks match "${stockStore.searchQuery()}". Try a different search term.`
                : stockStore.sp500Filter() 
                  ? 'No S&P 500 stocks found. Try adjusting your filter or search.'
                  : 'No stocks available in the database.'}
            </p>
          </div>
        </Show>
      </div>
    </div>
  );
}

export default App;