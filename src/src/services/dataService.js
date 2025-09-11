import { stockAPI, analysisAPI, recommendationsAPI, systemAPI, apiCall } from './api.js';

/**
 * Data service layer for complex business logic and data operations
 * Handles data transformation, caching, and complex queries
 */

// Stock data service
export const stockDataService = {
  // Load initial stock data with pagination
  async loadInitialStockData(stocksPerPage = 50) {
    const [stocksResult, totalStocksResult] = await Promise.all([
      apiCall(() => stockAPI.getPaginatedStocks(stocksPerPage, 0), 'load initial stocks'),
      apiCall(() => stockAPI.getAllStocksWithDataStatus(), 'get total stocks count')
    ]);

    return {
      stocks: stocksResult.success ? stocksResult.data : [],
      totalStocks: totalStocksResult.success ? totalStocksResult.data.length : 0,
      hasMore: stocksResult.success && stocksResult.data.length === stocksPerPage,
      error: stocksResult.success ? null : stocksResult.error
    };
  },

  // Load more stocks with pagination
  async loadMoreStocks(currentPage, stocksPerPage = 50) {
    const offset = currentPage * stocksPerPage;
    const result = await apiCall(
      () => stockAPI.getPaginatedStocks(stocksPerPage, offset),
      'load more stocks'
    );

    return {
      stocks: result.success ? result.data : [],
      hasMore: result.success && result.data.length === stocksPerPage,
      error: result.success ? null : result.error
    };
  },

  // Search stocks with error handling
  async searchStocks(query) {
    if (!query.trim()) {
      return { stocks: [], error: null };
    }

    const result = await apiCall(
      () => stockAPI.searchStocks(query),
      'search stocks'
    );

    return {
      stocks: result.success ? result.data : [],
      error: result.success ? null : result.error
    };
  },

  // Load S&P 500 symbols with caching
  async loadSp500Symbols() {
    const result = await apiCall(
      () => stockAPI.getSp500Symbols(),
      'load S&P 500 symbols'
    );

    return {
      symbols: result.success ? result.data : [],
      error: result.success ? null : result.error
    };
  },

  // Filter stocks by S&P 500
  filterStocksBySp500(stocks, sp500Symbols) {
    if (!sp500Symbols || sp500Symbols.length === 0) {
      return stocks;
    }
    
    const sp500Set = new Set(sp500Symbols);
    return stocks.filter(stock => sp500Set.has(stock.symbol));
  }
};

// Analysis data service
export const analysisDataService = {
  // Load complete analysis data for a stock
  async loadStockAnalysis(stockSymbol, startDate, endDate) {
    const [dateRangeResult, priceHistoryResult, valuationRatiosResult] = await Promise.all([
      apiCall(() => analysisAPI.getStockDateRange(stockSymbol), 'get stock date range'),
      apiCall(() => analysisAPI.getPriceHistory(stockSymbol, startDate, endDate), 'get price history'),
      apiCall(() => analysisAPI.getValuationRatios(stockSymbol), 'get valuation ratios')
    ]);

    return {
      dateRange: dateRangeResult.success ? dateRangeResult.data : null,
      priceHistory: priceHistoryResult.success ? priceHistoryResult.data : [],
      valuationRatios: valuationRatiosResult.success ? valuationRatiosResult.data : null,
      error: dateRangeResult.error || priceHistoryResult.error || valuationRatiosResult.error
    };
  },

  // Load P/S and EV/S history
  async loadPsEvsHistory(stockSymbol, startDate, endDate) {
    const result = await apiCall(
      () => analysisAPI.getPsEvsHistory(stockSymbol, startDate, endDate),
      'get P/S and EV/S history'
    );

    return {
      history: result.success ? result.data : [],
      error: result.success ? null : result.error
    };
  },

  // Export stock data
  async exportStockData(stockSymbol, format) {
    const result = await apiCall(
      () => analysisAPI.exportData(stockSymbol, format),
      'export stock data'
    );

    return {
      success: result.success,
      data: result.success ? result.data : null,
      error: result.success ? null : result.error
    };
  }
};

// Recommendations data service
export const recommendationsDataService = {
  // Load value recommendations with stats
  async loadValueRecommendations(limit = 20) {
    const result = await apiCall(
      () => recommendationsAPI.getValueRecommendationsWithStats(limit),
      'load value recommendations'
    );

    return {
      recommendations: result.success ? result.data.recommendations : [],
      stats: result.success ? result.data.stats : null,
      error: result.success ? null : result.error
    };
  },

  // Load undervalued stocks by P/S ratio
  async loadUndervaluedStocksByPs(maxPsRatio = 2.0, limit = 20, minMarketCap = 500_000_000) {
    const result = await apiCall(
      () => recommendationsAPI.getUndervaluedStocksByPs(maxPsRatio, limit, minMarketCap),
      'load undervalued stocks by P/S'
    );

    return {
      stocks: result.success ? result.data : [],
      error: result.success ? null : result.error
    };
  }
};

// System data service
export const systemDataService = {
  // Load system initialization status
  async loadInitializationStatus() {
    const result = await apiCall(
      () => systemAPI.getInitializationStatus(),
      'load initialization status'
    );

    return {
      status: result.success ? result.data : null,
      error: result.success ? null : result.error
    };
  },

  // Load database statistics
  async loadDatabaseStats() {
    const result = await apiCall(
      () => systemAPI.getDatabaseStats(),
      'load database statistics'
    );

    return {
      stats: result.success ? result.data : null,
      error: result.success ? null : result.error
    };
  },

  // Note: loadAvailableStockSymbols removed - command not registered in Tauri
};

// Note: Enhanced data service removed - these commands don't exist in the backend
