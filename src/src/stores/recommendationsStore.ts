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

export type ScreeningType = 'pe' | 'ps' | 'garp_pe';

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
          
        default:
          throw new Error(`Unknown screening type: ${currentScreeningType}`);
      }
      
      // Transform data for GARP and P/S screening
      if (result && result.length > 0) {
        const transformedRecommendations = result.map((stock: any, index: number) => ({
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
        }));
        
        setRecommendations(transformedRecommendations);
        setStats(null); // GARP and P/S don't return stats
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
    
    // Actions
    loadRecommendations,
    updateGarpCriteria,
    updatePsCriteria,
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