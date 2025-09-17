import { createSignal, createEffect, onMount, For, Show } from 'solid-js';
import { stockStore } from '../stores/stockStore';
import StockRow from './StockRow';

export default function StockBrowser() {
  const [isExpanded, setIsExpanded] = createSignal(false);
  const [searchInput, setSearchInput] = createSignal('');

  // Initialize with S&P 500 filter enabled by default
  onMount(() => {
    if (!stockStore.sp500Filter()) {
      stockStore.filterBySp500(true);
    }
  });

  // Search handler
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

  const handleToggleExpansion = () => {
    setIsExpanded(!isExpanded());
    // Load data when first expanded
    if (!isExpanded() && stockStore.stocks().length === 0) {
      stockStore.loadInitialStocks();
    }
  };

  return (
    <div class="bg-white rounded-lg shadow-sm border border-gray-200 overflow-hidden">
      {/* Collapsible Header */}
      <div 
        class="p-4 cursor-pointer hover:bg-gray-50 transition-colors border-b border-gray-100"
        onClick={handleToggleExpansion}
      >
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <span class="text-lg">
              {isExpanded() ? '▼' : '▶'}
            </span>
            <h3 class="text-lg font-semibold text-gray-900">
              Browse Individual Stocks
            </h3>
            <Show when={stockStore.sp500Filter()}>
              <span class="bg-blue-100 text-blue-800 text-xs font-medium px-2 py-1 rounded">
                S&P 500 Only
              </span>
            </Show>
          </div>
          <div class="text-sm text-gray-500">
            {isExpanded() ? 'Click to collapse' : 'Click to expand'}
          </div>
        </div>
      </div>

      {/* Expandable Content */}
      <Show when={isExpanded()}>
        <div class="p-4 bg-gray-50 border-b">
          {/* Search and Filters */}
          <div class="flex flex-col sm:flex-row gap-4 mb-4">
            <div class="flex-1">
              <div class="relative">
                <input
                  type="text"
                  placeholder="Search by symbol or company name (e.g., AAPL, Microsoft)..."
                  value={searchInput()}
                  onInput={(e) => setSearchInput(e.target.value)}
                  onKeyPress={(e) => e.key === 'Enter' && handleSearch()}
                  class="w-full pl-10 pr-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 text-sm"
                />
                <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                  <svg class="h-4 w-4 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                  </svg>
                </div>
              </div>
            </div>
            
            <div class="flex gap-2">
              <button
                onClick={handleSearch}
                disabled={stockStore.loading()}
                class="bg-blue-600 text-white px-4 py-2 rounded-lg hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed text-sm"
              >
                {stockStore.loading() ? 'Searching...' : 'Search'}
              </button>
              
              <Show when={stockStore.searchQuery()}>
                <button
                  onClick={handleClearSearch}
                  class="bg-gray-500 text-white px-3 py-2 rounded-lg hover:bg-gray-600 text-sm"
                >
                  Clear
                </button>
              </Show>
            </div>
          </div>

          {/* Filters */}
          <div class="flex items-center gap-4 text-sm">
            <label class="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={stockStore.sp500Filter()}
                onChange={(e) => stockStore.filterBySp500(e.target.checked)}
                class="rounded border-gray-300"
              />
              <span class="text-gray-700">S&P 500 Only</span>
              <Show when={stockStore.sp500Symbols().length > 0}>
                <span class="text-xs text-gray-500 bg-gray-100 px-2 py-1 rounded">
                  {stockStore.sp500Symbols().length} symbols
                </span>
              </Show>
            </label>
          </div>
        </div>

        {/* Stock List Content */}
        <div class="max-h-96 overflow-y-auto">
          {/* Loading State */}
          <Show when={stockStore.loading()}>
            <div class="p-8 text-center">
              <div class="flex items-center justify-center">
                <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600 mr-2"></div>
                <span class="text-gray-600 text-sm">Loading stocks...</span>
              </div>
            </div>
          </Show>

          {/* Error State */}
          <Show when={stockStore.error()}>
            <div class="p-4 bg-red-50 border border-red-200 rounded-lg m-4">
              <div class="flex items-center">
                <svg class="w-4 h-4 text-red-400 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                <span class="text-red-800 text-sm">{stockStore.error()}</span>
              </div>
            </div>
          </Show>

          {/* Stats */}
          <Show when={!stockStore.loading() && stockStore.stocks().length > 0}>
            <div class="p-3 bg-blue-50 border-b text-sm text-gray-600">
              <div class="flex items-center justify-between">
                <span>
                  📊 Showing {stockStore.stocks().length} of {stockStore.totalStocks()} stocks
                  {stockStore.sp500Filter() && ' (S&P 500 filtered)'}
                  {stockStore.searchQuery() && ` (search: "${stockStore.searchQuery()}")`}
                </span>
                <span class="text-xs text-gray-500">Click any stock to analyze</span>
              </div>
            </div>
          </Show>

          {/* Stock List */}
          <Show when={!stockStore.loading() && !stockStore.error()}>
            <div class="divide-y divide-gray-100">
              <Show 
                when={stockStore.stocks().length > 0}
                fallback={
                  <div class="p-8 text-center">
                    <div class="text-gray-400 mb-2">
                      <svg class="mx-auto h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width={2} d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
                      </svg>
                    </div>
                    <h3 class="text-lg font-medium text-gray-900 mb-2">No Stocks Found</h3>
                    <p class="text-gray-600 text-sm">
                      {stockStore.searchQuery() 
                        ? `No stocks match "${stockStore.searchQuery()}". Try a different search term.`
                        : stockStore.sp500Filter() 
                          ? 'No S&P 500 stocks found. Try adjusting your filters.'
                          : 'No stocks available in the database.'}
                    </p>
                  </div>
                }
              >
                <For each={stockStore.stocks().slice(0, 10)}>
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
                
                {/* Show More Button */}
                <Show when={stockStore.stocks().length > 10}>
                  <div class="p-4 text-center border-t bg-gray-50">
                    <button
                      onClick={stockStore.loadMoreStocks}
                      disabled={stockStore.loading() || !stockStore.hasMoreStocks()}
                      class="text-blue-600 hover:text-blue-700 text-sm font-medium disabled:text-gray-400"
                    >
                      {stockStore.loading() 
                        ? 'Loading...' 
                        : stockStore.hasMoreStocks()
                          ? `Show More (${stockStore.totalStocks() - stockStore.stocks().length} remaining)`
                          : 'All stocks loaded'
                      }
                    </button>
                  </div>
                </Show>
              </Show>
            </div>
          </Show>
        </div>
      </Show>
    </div>
  );
}