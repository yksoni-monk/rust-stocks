import { invoke } from '@tauri-apps/api/core';
import type { 
  Stock, 
  PriceData, 
  ValuationRatios, 
  DateRange, 
  GarpCriteria, 
  GarpScreeningResult,
  RecommendationStats,
  ValueRecommendation,
  DatabaseStats,
  InitializationStatus 
} from '../utils/types';

/**
 * Centralized API service layer for all backend operations
 * Separates backend logic from UI components
 */

// Stock Data API
export const stockAPI = {
  // Get paginated stocks
  async getPaginatedStocks(limit: number, offset: number): Promise<Stock[]> {
    return await invoke('get_stocks_paginated', { limit, offset });
  },

  // Get all stocks with data status
  async getAllStocksWithDataStatus(): Promise<Stock[]> {
    return await invoke('get_stocks_with_data_status');
  },

  // Search stocks
  async searchStocks(query: string): Promise<Stock[]> {
    return await invoke('search_stocks', { query });
  },

  // Get S&P 500 symbols
  async getSp500Symbols(): Promise<string[]> {
    return await invoke('get_sp500_symbols');
  }
};

// Analysis API
export const analysisAPI = {
  // Get stock date range
  async getStockDateRange(symbol: string): Promise<DateRange> {
    return await invoke('get_stock_date_range', { symbol });
  },

  // Get price history
  async getPriceHistory(symbol: string, startDate: string, endDate: string): Promise<PriceData[]> {
    return await invoke('get_price_history', { symbol, startDate, endDate });
  },

  // Get valuation ratios
  async getValuationRatios(symbol: string): Promise<ValuationRatios> {
    return await invoke('get_valuation_ratios', { symbol });
  },

  // Get P/S and EV/S history
  async getPsEvsHistory(symbol: string, startDate: string, endDate: string): Promise<any[]> {
    return await invoke('get_ps_evs_history', { symbol, startDate, endDate });
  },

  // Get valuation extremes (all-time high/low P/E and P/S ratios)
  async getValuationExtremes(symbol: string): Promise<any> {
    return await invoke('get_valuation_extremes', { symbol });
  },

  // Export data
  async exportData(symbol: string, format: string): Promise<string> {
    return await invoke('export_data', { symbol, format });
  }
};

// Recommendations API
export const recommendationsAPI = {
  // Get undervalued stocks by P/S ratio (smart algorithm)
  async getUndervaluedStocksByPs(stockTickers: string[], limit: number, minMarketCap: number): Promise<any[]> {
    return await invoke('get_undervalued_stocks_by_ps', { stockTickers, limit, minMarketCap });
  },

  // Get P/S screening with revenue growth requirements
  async getPsScreeningWithRevenueGrowth(stockTickers: string[], limit: number, minMarketCap: number): Promise<any[]> {
    return await invoke('get_ps_screening_with_revenue_growth', { stockTickers, limit, minMarketCap });
  },

  // Get value recommendations with stats
  async getValueRecommendationsWithStats(limit: number): Promise<{ recommendations: ValueRecommendation[], stats: RecommendationStats }> {
    return await invoke('get_value_recommendations_with_stats', { limit });
  },

  // Get GARP P/E screening results
  async getGarpPeScreeningResults(stockTickers: string[], criteria?: GarpCriteria, limit?: number): Promise<GarpScreeningResult[]> {
    return await invoke('get_garp_pe_screening_results', { 
      stockTickers, 
      criteria: criteria || {
        maxPegRatio: 1.0,
        minRevenueGrowth: 15.0,
        minProfitMargin: 5.0,
        maxDebtToEquity: 2.0,
        minMarketCap: 500_000_000,
        minQualityScore: 50,
        requirePositiveEarnings: true
      },
      limit: limit || 50
    });
  }
};

// Note: Enhanced Data API removed - these commands don't exist in the backend

// System API
export const systemAPI = {
  // Get initialization status
  async getInitializationStatus(): Promise<InitializationStatus> {
    return await invoke('get_initialization_status');
  },

  // Get database stats
  async getDatabaseStats(): Promise<DatabaseStats> {
    return await invoke('get_database_stats');
  },

  // Note: get_available_stock_symbols and get_database_migration_status removed - not registered in Tauri
};

// Centralized error handling
export const handleAPIError = (error: any, context = 'API call') => {
  console.error(`Error in ${context}:`, error);
  return {
    success: false,
    error: error.toString(),
    message: `Failed to ${context.toLowerCase()}`
  };
};

// API response wrapper for consistent error handling
export const apiCall = async <T>(apiFunction: () => Promise<T>, context: string) => {
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
