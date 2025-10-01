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
  passes_screening?: number;  // 1 if passes screening criteria, 0 otherwise
}

export interface RecommendationStats {
  total_sp500_stocks: number;
  stocks_with_pe_data: number;
  value_stocks_found: number;
  average_value_score?: number;
  average_risk_score?: number;
  // Piotroski-specific stats
  total_stocks?: number;
  avg_f_score?: number;
  avg_completeness?: number;
  high_quality_stocks?: number;
  excellent_stocks?: number;
  passing_stocks?: number;
}

export type ScreeningType = 'piotroski' | 'oshaughnessy';


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



// Recommendations store for managing stock recommendations
export function createRecommendationsStore() {
  // State signals
  const [recommendations, setRecommendations] = createSignal<Recommendation[]>([]);
  const [stats, setStats] = createSignal<RecommendationStats | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  
  // Screening configuration
  const [screeningType, setScreeningType] = createSignal<ScreeningType>('piotroski');
  const [limit, setLimit] = createSignal(10);
  

  const [piotroskilCriteria, setPiotroskilCriteria] = createSignal<PiotroskilCriteria>({
    minFScore: 7,
    minDataCompleteness: 80,
    passesScreeningOnly: true,
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


        default:
          throw new Error(`Unknown screening type: ${currentScreeningType}`);
      }
      
      // Transform data for all screening types
      if (result && result.length > 0) {
        const transformedRecommendations = result.map((stock: any, index: number) => {
          if (currentScreeningType === 'piotroski') {
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
              passes_screening: stock.passes_screening,

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
              criteria_met: stock.criteria_met,

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
          }
        });
        
        setRecommendations(transformedRecommendations);
        setStats(null); // O'Shaughnessy doesn't return stats
      } else {
        setRecommendations([]);
        setStats(null);
      }

      // Load statistics for Piotroski screening
      if (currentScreeningType === 'piotroski') {
        try {
          const piotroskiStats = await recommendationsAPI.getPiotroskilStatistics();
          setStats({
            total_sp500_stocks: 500,
            stocks_with_pe_data: 0,
            value_stocks_found: recommendations().length,
            total_stocks: piotroskiStats.total_stocks,
            avg_f_score: piotroskiStats.avg_f_score,
            avg_completeness: piotroskiStats.avg_completeness,
            high_quality_stocks: piotroskiStats.high_quality_stocks,
            excellent_stocks: piotroskiStats.excellent_stocks,
            passing_stocks: piotroskiStats.passing_stocks,
          });
        } catch (err) {
          console.warn('Failed to load Piotroski statistics:', err);
        }
      }
      
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load recommendations';
      setError(errorMessage);
      console.error('❌ Failed to load recommendations:', err);
    } finally {
      setLoading(false);
    }
  };


  // Update Piotroski criteria
  const updatePiotroskilCriteria = (updates: Partial<PiotroskilCriteria>) => {
    setPiotroskilCriteria(prev => ({ ...prev, ...updates }));
  };

  // Update O'Shaughnessy criteria
  const updateOshaughnessyCriteria = (updates: Partial<OShaughnessyCriteria>) => {
    setOshaughnessyCriteria(prev => ({ ...prev, ...updates }));
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
    piotroskilCriteria,
    oshaughnessyCriteria,

    // Actions
    loadRecommendations,
    updatePiotroskilCriteria,
    updateOshaughnessyCriteria,
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