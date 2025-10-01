import { invoke } from '@tauri-apps/api/core';
import type {
  RefreshRequestDto,
  RefreshProgressDto,
  SystemFreshnessReport
} from '../bindings';
import type {
  Stock,
  PriceData,
  ValuationRatios,
  DateRange,
  RecommendationStats,
  ValueRecommendation,
  DatabaseStats,
  InitializationStatus,
  RefreshResult,
  RefreshDurationEstimates
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
    return await invoke('get_price_history', { symbol, start_date: startDate, end_date: endDate });
  },

  // Get valuation ratios
  async getValuationRatios(symbol: string): Promise<ValuationRatios> {
    return await invoke('get_valuation_ratios', { symbol });
  },

  // Get P/S and EV/S history
  async getPsEvsHistory(symbol: string, startDate: string, endDate: string): Promise<any[]> {
    return await invoke('get_ps_evs_history', { symbol, start_date: startDate, end_date: endDate });
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

  // Get Piotroski F-Score screening results
  async getPiotroskilScreeningResults(stockTickers: string[], criteria?: any, limit?: number): Promise<any[]> {
    return await invoke('get_piotroski_screening_results', {
      stockTickers,
      criteria: criteria || {
        minFScore: 6,
        minDataCompleteness: 80,
        passesScreeningOnly: true
      },
      limit: limit || 10
    });
  },

  // Get Piotroski statistics
  async getPiotroskilStatistics(): Promise<any> {
    return await invoke('get_piotroski_statistics');
  },

  // Get O'Shaughnessy Value Composite screening results
  async getOShaughnessyScreeningResults(stockTickers: string[], criteria?: any, limit?: number): Promise<any[]> {
    return await invoke('get_oshaughnessy_screening_results', {
      stockTickers,
      criteria: criteria || {
        maxCompositePercentile: 20.0,
        maxPsRatio: 2.0,
        maxEvsRatio: 2.0,
        passesScreeningOnly: false
      },
      limit: limit || 50
    });
  },

  // Get O'Shaughnessy statistics
  async getOShaughnessyStatistics(): Promise<any> {
    return await invoke('get_oshaughnessy_statistics');
  },

};

// Note: Enhanced Data API removed - these commands don't exist in the backend

// Data Refresh API
export const dataRefreshAPI = {
  // Get current data freshness status
  async getDataFreshnessStatus(): Promise<SystemFreshnessReport> {
    console.log('🔄 API: Calling get_data_freshness_status...');
    try {
      const result = await invoke('get_data_freshness_status');
      console.log('✅ API: get_data_freshness_status result:', result);
      return result;
    } catch (error) {
      console.error('❌ API: get_data_freshness_status failed:', error);
      throw error;
    }
  },

  // Check if specific screening features are ready
  async checkScreeningReadiness(feature: string): Promise<boolean> {
    return await invoke('check_screening_readiness', { feature });
  },

  // Start data refresh operation
  async startDataRefresh(request: RefreshRequestDto): Promise<string> {
    return await invoke('start_data_refresh', { request });
  },

  // Get refresh progress
  async getRefreshProgress(sessionId: string): Promise<RefreshProgressDto | null> {
    return await invoke('get_refresh_progress', { sessionId });
  },

  // Get last refresh result
  async getLastRefreshResult(): Promise<RefreshResult | null> {
    return await invoke('get_last_refresh_result');
  },

  // Cancel refresh operation
  async cancelRefreshOperation(sessionId: string): Promise<boolean> {
    return await invoke('cancel_refresh_operation', { sessionId });
  },

  // Get refresh duration estimates
  async getRefreshDurationEstimates(): Promise<RefreshDurationEstimates> {
    return await invoke('get_refresh_duration_estimates');
  }
};

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
