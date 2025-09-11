import { invoke } from '@tauri-apps/api/core';

/**
 * Centralized API service layer for all backend operations
 * Separates backend logic from UI components
 */

// Stock Data API
export const stockAPI = {
  // Get paginated stocks
  async getPaginatedStocks(limit, offset) {
    return await invoke('get_stocks_paginated', { limit, offset });
  },

  // Get all stocks with data status
  async getAllStocksWithDataStatus() {
    return await invoke('get_stocks_with_data_status');
  },

  // Search stocks
  async searchStocks(query) {
    return await invoke('search_stocks', { query });
  },

  // Get S&P 500 symbols
  async getSp500Symbols() {
    return await invoke('get_sp500_symbols');
  }
};

// Analysis API
export const analysisAPI = {
  // Get stock date range
  async getStockDateRange(symbol) {
    return await invoke('get_stock_date_range', { symbol });
  },

  // Get price history
  async getPriceHistory(symbol, startDate, endDate) {
    return await invoke('get_price_history', { symbol, startDate, endDate });
  },

  // Get valuation ratios
  async getValuationRatios(symbol) {
    return await invoke('get_valuation_ratios', { symbol });
  },

  // Get P/S and EV/S history
  async getPsEvsHistory(symbol, startDate, endDate) {
    return await invoke('get_ps_evs_history', { symbol, startDate, endDate });
  },

  // Export data
  async exportData(symbol, format) {
    return await invoke('export_data', { symbol, format });
  }
};

// Recommendations API
export const recommendationsAPI = {
  // Get undervalued stocks by P/S ratio
  async getUndervaluedStocksByPs(maxPsRatio, limit, minMarketCap) {
    return await invoke('get_undervalued_stocks_by_ps', { maxPsRatio, limit, minMarketCap });
  },

  // Get value recommendations with stats
  async getValueRecommendationsWithStats(limit) {
    return await invoke('get_value_recommendations_with_stats', { limit });
  }
};

// Note: Enhanced Data API removed - these commands don't exist in the backend

// System API
export const systemAPI = {
  // Get initialization status
  async getInitializationStatus() {
    return await invoke('get_initialization_status');
  },

  // Get database stats
  async getDatabaseStats() {
    return await invoke('get_database_stats');
  },

  // Note: get_available_stock_symbols and get_database_migration_status removed - not registered in Tauri
};

// Centralized error handling
export const handleAPIError = (error, context = 'API call') => {
  console.error(`Error in ${context}:`, error);
  return {
    success: false,
    error: error.toString(),
    message: `Failed to ${context.toLowerCase()}`
  };
};

// API response wrapper for consistent error handling
export const apiCall = async (apiFunction, context) => {
  try {
    const result = await apiFunction();
    return {
      success: true,
      data: result
    };
  } catch (error) {
    return handleAPIError(error, context);
  }
};
