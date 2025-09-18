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
  ps_ratio_ttm?: number;
  z_score?: number;
  reasoning: string;
}

export interface RecommendationStats {
  total_sp500_stocks: number;
  stocks_with_pe_data: number;
  value_stocks_found: number;
  average_value_score?: number;
  average_risk_score?: number;
}

export type ScreeningType = 'pe' | 'ps' | 'garp_pe' | 'graham_value';

export interface GarpCriteria {
  maxPegRatio: number;
  minRevenueGrowth: number;
  minProfitMargin: number;
  maxDebtToEquity: number;
  minMarketCap: number;
  minQualityScore: number;
  requirePositiveEarnings: boolean;
}

export interface PsCriteria {
  psRatio: number;
  minMarketCap: number;
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
  
  const [psCriteria, setPsCriteria] = createSignal<PsCriteria>({
    psRatio: 2.0,
    minMarketCap: 500_000_000
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
          
        case 'ps':
          console.log('🎯 Loading P/S screening results...');
          result = await recommendationsAPI.getPsScreeningWithRevenueGrowth(
            stockTickers, 
            currentLimit, 
            psCriteria().minMarketCap
          );
          break;
          
        case 'pe':
          console.log('🎯 Loading P/E screening results...');
          const peResult = await recommendationsAPI.getValueRecommendationsWithStats(currentLimit);
          setRecommendations(peResult.recommendations);
          setStats(peResult.stats);
          return;
          
        case 'graham_value':
          console.log('🎯 Loading Graham value screening results...');
          result = await recommendationsAPI.runGrahamScreening(grahamCriteria());
          break;
          
        default:
          throw new Error(`Unknown screening type: ${currentScreeningType}`);
      }
      
      // Transform data for GARP, P/S, and Graham screening
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
          } else {
            // GARP and P/S transformation
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
              
              // P/S-specific fields
              ps_ratio_ttm: stock.ps_ratio_ttm,
              z_score: stock.z_score,
              
              // Generate reasoning based on screening type
              reasoning: currentScreeningType === 'garp_pe' 
                ? `GARP Score: ${stock.garp_score?.toFixed(2) || 'N/A'} | P/E: ${stock.current_pe_ratio?.toFixed(2) || 'N/A'} | PEG: ${stock.peg_ratio?.toFixed(2) || 'N/A'} | Growth: ${stock.ttm_growth_rate?.toFixed(1) || stock.annual_growth_rate?.toFixed(1) || 'N/A'}% | Quality: ${stock.quality_score || 0}/100`
                : `P/S ${stock.ps_ratio_ttm?.toFixed(2) || 'N/A'} (Z-score: ${stock.z_score?.toFixed(2) || 'N/A'})`
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

  // Update P/S criteria
  const updatePsCriteria = (updates: Partial<PsCriteria>) => {
    setPsCriteria(prev => ({ ...prev, ...updates }));
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
    psCriteria,
    grahamCriteria,
    
    // Actions
    loadRecommendations,
    updateGarpCriteria,
    updatePsCriteria,
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