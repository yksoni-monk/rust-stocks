// Stock related types
export interface Stock {
  id: number;
  symbol: string;
  company_name?: string;
  sector?: string;
  industry?: string;
  market_cap?: number;
  has_data?: boolean;
}

// Analysis related types
export interface PriceData {
  date: string;
  open_price: number;
  high_price: number;
  low_price: number;
  close_price: number;
  volume?: number;
  pe_ratio?: number;
}

export interface ValuationRatios {
  ps_ratio_ttm?: number;
  evs_ratio_ttm?: number;
  pe_ratio?: number;
  market_cap?: number;
}

export interface DateRange {
  min_date: string;
  max_date: string;
}

// Recommendation types
export interface GarpCriteria {
  maxPegRatio: number;
  minRevenueGrowth: number;
  minProfitMargin: number;
  maxDebtToEquity: number;
  minMarketCap: number;
  minQualityScore: number;
  requirePositiveEarnings: boolean;
}

export interface GarpScreeningResult {
  stock_id: number;
  symbol: string;
  sector?: string;
  current_pe_ratio: number;
  peg_ratio?: number;
  current_price: number;
  passes_positive_earnings: boolean;
  passes_peg_filter: boolean;
  current_eps_ttm?: number;
  current_eps_annual?: number;
  eps_growth_rate_ttm?: number;
  eps_growth_rate_annual?: number;
  current_ttm_revenue?: number;
  ttm_growth_rate?: number;
  current_annual_revenue?: number;
  annual_growth_rate?: number;
  passes_revenue_growth_filter: boolean;
  current_ttm_net_income?: number;
  net_profit_margin?: number;
  passes_profitability_filter: boolean;
  total_debt?: number;
  total_equity?: number;
  debt_to_equity_ratio?: number;
  passes_debt_filter: boolean;
  garp_score: number;
  quality_score: number;
  passes_garp_screening: boolean;
  market_cap: number;
  data_completeness_score: number;
}

export interface RecommendationStats {
  total_sp500_stocks: number;
  stocks_with_pe_data: number;
  value_stocks_found: number;
  average_value_score?: number;
  average_risk_score?: number;
}

export interface ValueRecommendation {
  rank: number;
  symbol: string;
  company_name: string;
  current_pe?: number;
  value_score?: number;
  risk_score?: number;
  reasoning: string;
}

// API response types
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// System types
export interface DatabaseStats {
  total_stocks: number;
  stocks_with_data: number;
  total_price_records: number;
  latest_data_date?: string;
}

export interface InitializationStatus {
  database_ready: boolean;
  stocks_loaded: boolean;
  price_data_available: boolean;
  message?: string;
}