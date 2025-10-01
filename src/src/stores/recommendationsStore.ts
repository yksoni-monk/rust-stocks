import { createSignal } from 'solid-js';
import { recommendationsAPI } from '../services/api';

// Types
export interface Recommendation {
  rank: number;
  symbol: string;
  company_name: string;
  current_pe_ratio?: number;
  peg_ratio?: number;
  current_price?: number;
  eps_growth_rate_ttm?: number;
  eps_growth_rate_annual?: number;
  ttm_growth_rate?: number;
  annual_growth_rate?: number;
  net_profit_margin?: number;
  debt_to_equity_ratio?: number;
  garp_score?: number;
  quality_score?: number;
  passes_garp_screening?: boolean;
  market_cap?: number;
  enterprise_value?: number;
  ps_ratio_ttm?: number;
  z_score?: number;

  // Enhanced O'Shaughnessy metrics (all 6)
  pe_ratio?: number;
  pb_ratio?: number;
  ev_ebitda_ratio?: number;
  shareholder_yield?: number;
  reasoning: string;

  // Piotroski F-Score fields (when screening type is 'piotroski')
  stock_id?: number;
  f_score_complete?: number;
  data_completeness_score?: number;
  criterion_positive_net_income?: number;
  criterion_positive_operating_cash_flow?: number;
  criterion_improving_roa?: number;
  criterion_cash_flow_quality?: number;
  criterion_decreasing_debt_ratio?: number;
  criterion_improving_current_ratio?: number;
  criterion_no_dilution?: number;
  criterion_improving_net_margin?: number;
  criterion_improving_asset_turnover?: number;
  current_roa?: number;
  current_debt_ratio?: number;
  current_current_ratio?: number;
  current_net_margin?: number;
  current_asset_turnover?: number;
  current_operating_cash_flow?: number;

  // Simple Piotroski data availability (no fake confidence)
  criteria_met?: number;  // How many of the 9 criteria are actually met (0-9)
}

export interface RecommendationStats {
  total_sp500_stocks: number;
  stocks_with_pe_data: number;
  value_stocks_found: number;
  average_value_score?: number;
  average_risk_score?: number;
}

export type ScreeningType = 'garp_pe' | 'graham_value' | 'piotroski' | 'oshaughnessy';

export interface GarpCriteria {
  maxPegRatio: number;
  minRevenueGrowth: number;
  minProfitMargin: number;
  maxDebtToEquity: number;
  minMarketCap: number;
  minQualityScore: number;
  requirePositiveEarnings: boolean;
}

export interface PiotroskilCriteria {
  minFScore: number;
  minDataCompleteness: number;
  passesScreeningOnly: boolean;
  sectors?: string[];
}

export interface OShaughnessyCriteria {
  maxCompositePercentile: number;
  maxPsRatio: number;
  maxEvsRatio: number;
  minMarketCap: number;
  passesScreeningOnly: boolean;
  sectors?: string[];
}

export interface GrahamCriteria {
  max_pe_ratio: number;
  max_pb_ratio: number;
  max_pe_pb_product: number;
  min_dividend_yield: number;
  max_debt_to_equity: number;
  min_profit_margin: number;
  min_revenue_growth_1y: number;
  min_revenue_growth_3y: number;
  min_current_ratio: number;
  min_interest_coverage: number;
  min_roe: number;
  require_positive_earnings: boolean;
  require_dividend: boolean;
  min_market_cap: number;
  max_market_cap: number;
  excluded_sectors: string[];
}

export interface GrahamResult {
  stock_id: number;
  symbol: string;
  screening_date: string;
  pe_ratio?: number;
  pb_ratio?: number;
  pe_pb_product?: number;
  dividend_yield?: number;
  debt_to_equity?: number;
  profit_margin?: number;
  revenue_growth_1y?: number;
  revenue_growth_3y?: number;
  current_ratio?: number;
  quick_ratio?: number;
  interest_coverage_ratio?: number;
  return_on_equity?: number;
  return_on_assets?: number;
  passes_earnings_filter: boolean;
  passes_pe_filter: boolean;
  passes_pb_filter: boolean;
  passes_pe_pb_combined: boolean;
  passes_dividend_filter: boolean;
  passes_debt_filter: boolean;
  passes_quality_filter: boolean;
  passes_growth_filter: boolean;
  passes_all_filters: boolean;
  graham_score?: number;
  value_rank?: number;
  quality_score?: number;
  safety_score?: number;
  current_price?: number;
  market_cap?: number;
  shares_outstanding?: number;
  net_income?: number;
  total_equity?: number;
  total_debt?: number;
  revenue?: number;
  reasoning?: string;
  sector?: string;
  industry?: string;
  company_name?: string;
  is_sp500: boolean;
  exchange?: string;
  value_category: string;
  safety_category: string;
  recommendation: string;
  pe_percentile?: number;
  pb_percentile?: number;
  sector_pe_rank?: number;
  sector_pb_rank?: number;
}

// Recommendations store for managing stock recommendations
export function createRecommendationsStore() {
  // State signals
  const [recommendations, setRecommendations] = createSignal<Recommendation[]>([]);
  const [stats, setStats] = createSignal<RecommendationStats | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  
  // Screening configuration
  const [screeningType, setScreeningType] = createSignal<ScreeningType>('garp_pe');
  const [limit, setLimit] = createSignal(20);
  
  // Criteria for different screening types
  const [garpCriteria, setGarpCriteria] = createSignal<GarpCriteria>({
    maxPegRatio: 1.0,
    minRevenueGrowth: 15.0,
    minProfitMargin: 5.0,
    maxDebtToEquity: 2.0,
    minMarketCap: 500_000_000,
    minQualityScore: 50,
    requirePositiveEarnings: true
  });

  const [piotroskilCriteria, setPiotroskilCriteria] = createSignal<PiotroskilCriteria>({
    minFScore: 3,
    minDataCompleteness: 50,
    passesScreeningOnly: false,
    sectors: []
  });

  const [oshaughnessyCriteria, setOshaughnessyCriteria] = createSignal<OShaughnessyCriteria>({
    maxCompositePercentile: 20.0,
    maxPsRatio: 2.0,
    maxEvsRatio: 2.0,
    minMarketCap: 200_000_000,
    passesScreeningOnly: false,
    sectors: []
  });
  
  const [grahamCriteria, setGrahamCriteria] = createSignal<GrahamCriteria>({
    max_pe_ratio: 25.0,           // More relaxed for modern market
    max_pb_ratio: 3.0,            // More relaxed for modern market  
    max_pe_pb_product: 40.0,      // Adjusted for higher ratios
    min_dividend_yield: 0.0,      // Allow non-dividend stocks
    max_debt_to_equity: 2.0,      // More realistic debt tolerance
    min_profit_margin: 5.0,
    min_revenue_growth_1y: 0.0,
    min_revenue_growth_3y: 0.0,
    min_current_ratio: 1.2,       // Slightly more relaxed
    min_interest_coverage: 2.5,
    min_roe: 8.0,                 // Slightly more relaxed
    require_positive_earnings: true,
    require_dividend: false,      // Don't require dividends
    min_market_cap: 100_000_000,
    max_market_cap: 1_000_000_000_000,
    excluded_sectors: []
  });

  // Load recommendations based on current screening type
  const loadRecommendations = async (stockTickers: string[]) => {
    if (stockTickers.length === 0) {
      setError('No stock symbols provided');
      return;
    }

    setLoading(true);
    setError(null);
    
    try {
      let result;
      const currentScreeningType = screeningType();
      const currentLimit = limit();
      
      switch (currentScreeningType) {
        case 'garp_pe':
          console.log('🎯 Loading GARP P/E screening results...');
          result = await recommendationsAPI.getGarpPeScreeningResults(
            stockTickers,
            garpCriteria(),
            currentLimit
          );
          break;

        case 'piotroski':
          console.log('🎯 Loading Piotroski F-Score screening results...');
          result = await recommendationsAPI.getPiotroskilScreeningResults(
            stockTickers,
            piotroskilCriteria(),
            currentLimit
          );
          break;

        case 'oshaughnessy':
          console.log('🎯 Loading O\'Shaughnessy Value Composite screening results...');
          result = await recommendationsAPI.getOShaughnessyScreeningResults(
            stockTickers,
            oshaughnessyCriteria(),
            currentLimit
          );
          break;

        case 'graham_value':
          console.log('🎯 Loading Graham value screening results...');
          result = await recommendationsAPI.runGrahamScreening(grahamCriteria());
          break;

        default:
          throw new Error(`Unknown screening type: ${currentScreeningType}`);
      }
      
      // Transform data for all screening types
      if (result && result.length > 0) {
        const transformedRecommendations = result.map((stock: any, index: number) => {
          if (currentScreeningType === 'graham_value') {
            // Graham-specific transformation
            return {
              rank: index + 1,
              symbol: stock.result?.symbol || stock.symbol,
              company_name: stock.company_name || stock.result?.symbol || stock.symbol,

              // Graham-specific fields
              current_pe_ratio: stock.result?.pe_ratio,
              current_price: stock.result?.current_price,
              market_cap: stock.result?.market_cap,
              debt_to_equity_ratio: stock.result?.debt_to_equity,
              net_profit_margin: stock.result?.profit_margin,
              garp_score: stock.result?.graham_score, // Use graham_score as garp_score for compatibility
              quality_score: stock.result?.quality_score,
              passes_garp_screening: stock.result?.passes_all_filters,

              // Graham-specific reasoning
              reasoning: `${stock.recommendation} | Graham Score: ${stock.result?.graham_score?.toFixed(1) || 'N/A'} | ${stock.value_category} | ${stock.safety_category} | P/E: ${stock.result?.pe_ratio?.toFixed(2) || 'N/A'} | P/B: ${stock.result?.pb_ratio?.toFixed(2) || 'N/A'} | Debt/Equity: ${stock.result?.debt_to_equity?.toFixed(2) || 'N/A'}`
            };
          } else if (currentScreeningType === 'piotroski') {
            // Piotroski F-Score transformation
            return {
              rank: index + 1,
              symbol: stock.symbol,
              company_name: stock.symbol,
              current_pe_ratio: null,
              current_price: null,
              market_cap: null,
              garp_score: stock.f_score_complete, // Fixed: use correct field name
              quality_score: stock.data_completeness_score,
              passes_garp_screening: stock.passes_screening === 1,

              // ✅ ADD ALL MISSING PIOTROSKI CRITERION FIELDS
              stock_id: stock.stock_id,
              f_score_complete: stock.f_score_complete,
              data_completeness_score: stock.data_completeness_score,
              criterion_positive_net_income: stock.criterion_positive_net_income,
              criterion_positive_operating_cash_flow: stock.criterion_positive_operating_cash_flow,
              criterion_improving_roa: stock.criterion_improving_roa,
              criterion_cash_flow_quality: stock.criterion_cash_flow_quality,
              criterion_decreasing_debt_ratio: stock.criterion_decreasing_debt_ratio,
              criterion_improving_current_ratio: stock.criterion_improving_current_ratio,
              criterion_no_dilution: stock.criterion_no_dilution,
              criterion_improving_net_margin: stock.criterion_improving_net_margin,
              criterion_improving_asset_turnover: stock.criterion_improving_asset_turnover,
              current_roa: stock.current_roa,
              current_debt_ratio: stock.current_debt_ratio,
              current_current_ratio: stock.current_current_ratio,
              current_net_margin: stock.current_net_margin,
              current_asset_turnover: stock.current_asset_turnover,
              current_operating_cash_flow: stock.current_operating_cash_flow,

              reasoning: `F-Score: ${stock.f_score_complete}/9 | Data Quality: ${stock.data_completeness_score}% | Income: ${stock.criterion_positive_net_income ? '✓' : '✗'} | ROA: ${stock.criterion_improving_roa ? '✓' : '✗'} | Debt: ${stock.criterion_decreasing_debt_ratio ? '✓' : '✗'}`
            };
          } else if (currentScreeningType === 'oshaughnessy') {
            // Enhanced O'Shaughnessy Value Composite transformation (6 metrics)
            return {
              rank: index + 1,
              symbol: stock.symbol,
              company_name: stock.symbol,
              current_pe_ratio: stock.pe_ratio,
              current_price: stock.current_price,
              market_cap: stock.market_cap,
              enterprise_value: stock.enterprise_value,

              // Enhanced O'Shaughnessy metrics
              ps_ratio_ttm: stock.ps_ratio,
              pe_ratio: stock.pe_ratio,
              pb_ratio: stock.pb_ratio,
              ev_ebitda_ratio: stock.ev_ebitda_ratio,
              shareholder_yield: stock.shareholder_yield,

              garp_score: 100 - stock.composite_percentile, // Invert percentile for scoring
              quality_score: stock.data_completeness_score,
              passes_garp_screening: stock.passes_screening === 1,
              reasoning: `Value Rank: ${stock.overall_rank} (${stock.composite_percentile}th percentile) | Metrics: ${stock.metrics_available}/6 | P/S: ${stock.ps_ratio?.toFixed(2) || 'N/A'} | P/E: ${stock.pe_ratio?.toFixed(2) || 'N/A'} | P/B: ${stock.pb_ratio?.toFixed(2) || 'N/A'} | EV/EBITDA: ${stock.ev_ebitda_ratio?.toFixed(2) || 'N/A'} | EV/S: ${stock.evs_ratio?.toFixed(2) || 'N/A'} | Yield: ${stock.shareholder_yield?.toFixed(1) || 'N/A'}%`
            };
          } else {
            // GARP transformation
            return {
              rank: index + 1,
              symbol: stock.symbol,
              company_name: stock.symbol,

              // GARP-specific fields
              current_pe_ratio: stock.current_pe_ratio,
              peg_ratio: stock.peg_ratio,
              current_price: stock.current_price,
              eps_growth_rate_ttm: stock.eps_growth_rate_ttm,
              eps_growth_rate_annual: stock.eps_growth_rate_annual,
              ttm_growth_rate: stock.ttm_growth_rate,
              annual_growth_rate: stock.annual_growth_rate,
              net_profit_margin: stock.net_profit_margin,
              debt_to_equity_ratio: stock.debt_to_equity_ratio,
              garp_score: stock.garp_score,
              quality_score: stock.quality_score,
              passes_garp_screening: stock.passes_garp_screening,
              market_cap: stock.market_cap,

              // GARP-specific reasoning
              reasoning: `GARP Score: ${stock.garp_score?.toFixed(2) || 'N/A'} | P/E: ${stock.current_pe_ratio?.toFixed(2) || 'N/A'} | PEG: ${stock.peg_ratio?.toFixed(2) || 'N/A'} | Growth: ${stock.ttm_growth_rate?.toFixed(1) || stock.annual_growth_rate?.toFixed(1) || 'N/A'}% | Quality: ${stock.quality_score || 0}/100`
            };
          }
        });
        
        setRecommendations(transformedRecommendations);
        setStats(null); // GARP, P/S, and Graham don't return stats
      } else {
        setRecommendations([]);
        setStats(null);
      }
      
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load recommendations';
      setError(errorMessage);
      console.error('❌ Failed to load recommendations:', err);
    } finally {
      setLoading(false);
    }
  };

  // Update GARP criteria
  const updateGarpCriteria = (updates: Partial<GarpCriteria>) => {
    setGarpCriteria(prev => ({ ...prev, ...updates }));
  };

  // Update Piotroski criteria
  const updatePiotroskilCriteria = (updates: Partial<PiotroskilCriteria>) => {
    setPiotroskilCriteria(prev => ({ ...prev, ...updates }));
  };

  // Update O'Shaughnessy criteria
  const updateOshaughnessyCriteria = (updates: Partial<OShaughnessyCriteria>) => {
    setOshaughnessyCriteria(prev => ({ ...prev, ...updates }));
  };

  // Update Graham criteria
  const updateGrahamCriteria = (updates: Partial<GrahamCriteria>) => {
    setGrahamCriteria(prev => ({ ...prev, ...updates }));
  };

  // Clear recommendations
  const clearRecommendations = () => {
    setRecommendations([]);
    setStats(null);
    setError(null);
  };

  return {
    // State
    recommendations,
    stats,
    loading,
    error,
    screeningType,
    limit,
    garpCriteria,
    piotroskilCriteria,
    oshaughnessyCriteria,
    grahamCriteria,

    // Actions
    loadRecommendations,
    updateGarpCriteria,
    updatePiotroskilCriteria,
    updateOshaughnessyCriteria,
    updateGrahamCriteria,
    clearRecommendations,

    // Setters
    setScreeningType,
    setLimit,
    setError,
    setLoading
  };
}

// Create global store instance
export const recommendationsStore = createRecommendationsStore();