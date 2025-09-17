import { createSignal } from 'solid-js';
import { stockAPI } from '../services/api';

// Types
export interface Stock {
  id: number;
  symbol: string;
  company_name?: string;
  sector?: string;
  industry?: string;
  market_cap?: number;
  has_data?: boolean;
}

// Stock store for managing stock data and pagination
export function createStockStore() {
  // Signals for stock data
  const [stocks, setStocks] = createSignal<Stock[]>([]);
  const [totalStocks, setTotalStocks] = createSignal(0);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  
  // Pagination state
  const [currentPage, setCurrentPage] = createSignal(0);
  const [hasMoreStocks, setHasMoreStocks] = createSignal(true);
  const stocksPerPage = 50;
  
  // Search and filter state
  const [searchQuery, setSearchQuery] = createSignal('');
  const [sp500Filter, setSp500Filter] = createSignal(false);
  const [sp500Symbols, setSp500Symbols] = createSignal<string[]>([]);
  
  // Expanded panels state
  const [expandedPanels, setExpandedPanels] = createSignal<Record<string, string>>({});

  // Load initial stock data
  const loadInitialStocks = async () => {
    setLoading(true);
    setError(null);
    
    try {
      const [stocksResult, totalStocksResult] = await Promise.all([
        stockAPI.getPaginatedStocks(stocksPerPage, 0),
        stockAPI.getAllStocksWithDataStatus()
      ]);

      setStocks(stocksResult);
      setTotalStocks(totalStocksResult.length);
      setHasMoreStocks(stocksResult.length === stocksPerPage);
      setCurrentPage(0);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load stocks');
    } finally {
      setLoading(false);
    }
  };

  // Load more stocks
  const loadMoreStocks = async () => {
    if (loading() || !hasMoreStocks()) return;
    
    setLoading(true);
    
    try {
      const nextPage = currentPage() + 1;
      const offset = nextPage * stocksPerPage;
      const newStocks = await stockAPI.getPaginatedStocks(stocksPerPage, offset);
      
      setStocks(prev => [...prev, ...newStocks]);
      setCurrentPage(nextPage);
      setHasMoreStocks(newStocks.length === stocksPerPage);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load more stocks');
    } finally {
      setLoading(false);
    }
  };

  // Search stocks
  const searchStocks = async (query: string) => {
    if (!query.trim()) {
      loadInitialStocks();
      return;
    }
    
    setLoading(true);
    setError(null);
    
    try {
      const results = await stockAPI.searchStocks(query);
      setStocks(results);
      setHasMoreStocks(false);
      setSearchQuery(query);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Search failed');
    } finally {
      setLoading(false);
    }
  };

  // Load S&P 500 symbols
  const loadSp500Symbols = async () => {
    try {
      const symbols = await stockAPI.getSp500Symbols();
      setSp500Symbols(symbols);
    } catch (err) {
      console.error('Failed to load S&P 500 symbols:', err);
    }
  };

  // Filter by S&P 500
  const filterBySp500 = (enabled: boolean) => {
    setSp500Filter(enabled);
    // Reload stocks with filter
    loadInitialStocks();
  };

  // Toggle panel expansion
  const togglePanelExpansion = (stockKey: string, panelType?: string) => {
    setExpandedPanels(prev => {
      const newPanels = { ...prev };
      if (newPanels[stockKey]) {
        delete newPanels[stockKey];
      } else {
        newPanels[stockKey] = panelType || 'analysis';
      }
      return newPanels;
    });
  };

  // Get filtered stocks (for S&P 500 filter)
  const filteredStocks = () => {
    if (!sp500Filter() || sp500Symbols().length === 0) {
      return stocks();
    }
    
    const sp500Set = new Set(sp500Symbols());
    return stocks().filter(stock => sp500Set.has(stock.symbol));
  };

  return {
    // State
    stocks: filteredStocks,
    totalStocks,
    loading,
    error,
    currentPage,
    hasMoreStocks,
    searchQuery,
    sp500Filter,
    sp500Symbols,
    expandedPanels,
    
    // Actions
    loadInitialStocks,
    loadMoreStocks,
    searchStocks,
    loadSp500Symbols,
    filterBySp500,
    togglePanelExpansion,
    
    // Setters for external use
    setError,
    setLoading
  };
}

// Create global store instance
export const stockStore = createStockStore();