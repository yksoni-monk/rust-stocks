import { useState, useEffect } from 'react';
import { recommendationsDataService, analysisDataService, stockDataService } from '../services/dataService.js';
import { analysisAPI } from '../services/api.js';

function RecommendationsPanel({ onClose, initialScreeningType = 'ps' }) {
  const [recommendations, setRecommendations] = useState([]);
  const [stats, setStats] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [limit, setLimit] = useState(20);
  const [screeningType, setScreeningType] = useState(initialScreeningType); // Use prop as default
  const [psRatio, setPsRatio] = useState(2.0);
  const [minMarketCap, setMinMarketCap] = useState(500_000_000); // Default $500M
  const [valuationExtremes, setValuationExtremes] = useState({});
  const [sp500Symbols, setSp500Symbols] = useState([]); // S&P 500 symbols for smart screening
  const [isFooterExpanded, setIsFooterExpanded] = useState(false);
  
  // GARP P/E screening criteria
  const [garpPeCriteria, setGarpPeCriteria] = useState({
    maxPegRatio: 1.0,
    minRevenueGrowth: 15.0,
    minProfitMargin: 5.0,
    maxDebtToEquity: 2.0,
    minMarketCap: 500_000_000,
    minQualityScore: 50,
    requirePositiveEarnings: true
  });

  // Load S&P 500 symbols for smart screening first
  useEffect(() => {
    loadSp500Symbols();
  }, []);

  // Update screening type when prop changes
  useEffect(() => {
    setScreeningType(initialScreeningType);
  }, [initialScreeningType]);

  // Load recommendations after S&P 500 symbols are loaded
  useEffect(() => {
    if (screeningType === 'ps' && sp500Symbols.length === 0) {
      // Don't load recommendations yet if we need S&P 500 symbols but they're not loaded
      return;
    }
    loadRecommendationsWithStats();
  }, [limit, screeningType, psRatio, minMarketCap, sp500Symbols, garpPeCriteria]);

  async function loadSp500Symbols() {
    try {
      const result = await stockDataService.loadSp500Symbols();
      
      if (result.success) {
        setSp500Symbols(result.symbols);
        setError(null); // Clear any previous errors
      } else {
        setError(`Failed to load S&P 500 symbols: ${result.error || 'Unknown error'}`);
      }
    } catch (err) {
      setError(`Failed to load S&P 500 symbols: ${err.message || err}`);
    }
  }

  async function loadRecommendationsWithStats() {
    try {
      setLoading(true);
      
      if (screeningType === 'ps') {
        // Use P/S screening with revenue growth requirements
        
        const result = await recommendationsDataService.loadPsScreeningWithRevenueGrowth(sp500Symbols, limit, minMarketCap);
        
        if (result.error) {
          setError(result.error);
          console.error('Error loading P/S screening with revenue growth:', result.error);
          return;
        }
        
        // Transform P/S screening with revenue growth data to match recommendations format
        const transformedRecommendations = result.stocks.map((stock, index) => ({
          rank: index + 1,
          symbol: stock.symbol,
          company_name: stock.symbol,
          current_pe: null,
          current_pe_date: null,
          historical_min_pe: 0,
          historical_max_pe: 0,
          value_score: Math.max(0, Math.min(100, (2.0 - (stock.current_ps || 0)) * 50)),
          risk_score: Math.min(100, (stock.current_ps || 0) * 20),
          data_points: stock.data_points || 0,
          reasoning: `P/S ${(stock.current_ps || 0).toFixed(2)} (Z-score: ${(stock.z_score || 0).toFixed(2)}) | Revenue Growth: ${stock.ttm_growth_rate ? stock.ttm_growth_rate.toFixed(1) + '%' : stock.annual_growth_rate ? stock.annual_growth_rate.toFixed(1) + '%' : 'N/A'}`,
          ps_ratio_ttm: stock.current_ps,
          evs_ratio_ttm: null,
          market_cap: stock.market_cap,
          revenue_ttm: stock.current_ttm_revenue,
          // P/S screening with revenue growth specific fields
          historical_mean: stock.historical_mean,
          historical_median: stock.historical_median,
          historical_stddev: stock.historical_stddev,
          historical_min: stock.historical_min,
          historical_max: stock.historical_max,
          current_ttm_revenue: stock.current_ttm_revenue,
          ttm_growth_rate: stock.ttm_growth_rate,
          current_annual_revenue: stock.current_annual_revenue,
          annual_growth_rate: stock.annual_growth_rate,
          z_score: stock.z_score,
          quality_score: stock.quality_score,
          is_undervalued: stock.undervalued_flag
        }));
        
        setRecommendations(transformedRecommendations);
        setStats({
          total_sp500_stocks: 503,
          stocks_with_pe_data: result.stocks.length,
          value_stocks_found: result.stocks.length,
          average_value_score: transformedRecommendations.reduce((sum, r) => sum + r.value_score, 0) / transformedRecommendations.length || 0,
          average_risk_score: transformedRecommendations.reduce((sum, r) => sum + r.risk_score, 0) / transformedRecommendations.length || 0
        });
        
        // Load valuation extremes for each stock
        await loadValuationExtremesForStocks(transformedRecommendations);
      } else if (screeningType === 'garp_pe') {
        // GARP P/E screening
        
        const result = await recommendationsDataService.loadGarpPeScreeningResults(sp500Symbols, garpPeCriteria, limit);
        
        if (result.error) {
          setError(result.error);
          console.error('Error loading GARP P/E screening:', result.error);
          return;
        }
        
        // Transform GARP P/E screening data to match recommendations format
        const transformedRecommendations = result.stocks.map((stock, index) => ({
          rank: index + 1,
          symbol: stock.symbol,
          company_name: stock.symbol,
          current_pe: stock.current_pe_ratio,
          current_pe_date: null,
          historical_min_pe: 0,
          historical_max_pe: 0,
          value_score: Math.max(0, Math.min(100, stock.garp_score * 10)), // Scale GARP score to 0-100
          risk_score: Math.min(100, (stock.current_pe_ratio || 0) * 2), // Higher P/E = higher risk
          data_points: stock.data_completeness_score || 0,
          reasoning: `P/E: ${(stock.current_pe_ratio || 0).toFixed(2)} | PEG: ${(stock.peg_ratio || 0).toFixed(2)} | Revenue Growth: ${stock.ttm_growth_rate ? stock.ttm_growth_rate.toFixed(1) + '%' : stock.annual_growth_rate ? stock.annual_growth_rate.toFixed(1) + '%' : 'N/A'} | Profit Margin: ${stock.net_profit_margin ? stock.net_profit_margin.toFixed(1) + '%' : 'N/A'}`,
          ps_ratio_ttm: null,
          evs_ratio_ttm: null,
          market_cap: stock.market_cap,
          revenue_ttm: stock.current_ttm_revenue,
          // GARP P/E specific fields
          current_pe_ratio: stock.current_pe_ratio,
          peg_ratio: stock.peg_ratio,
          eps_growth_rate_ttm: stock.eps_growth_rate_ttm,
          eps_growth_rate_annual: stock.eps_growth_rate_annual,
          ttm_growth_rate: stock.ttm_growth_rate,
          annual_growth_rate: stock.annual_growth_rate,
          net_profit_margin: stock.net_profit_margin,
          debt_to_equity_ratio: stock.debt_to_equity_ratio,
          garp_score: stock.garp_score,
          quality_score: stock.quality_score,
          passes_garp_screening: stock.passes_garp_screening,
          passes_positive_earnings: stock.passes_positive_earnings,
          passes_peg_filter: stock.passes_peg_filter,
          passes_revenue_growth_filter: stock.passes_revenue_growth_filter,
          passes_profitability_filter: stock.passes_profitability_filter,
          passes_debt_filter: stock.passes_debt_filter
        }));
        
        setRecommendations(transformedRecommendations);
        setStats({
          total_sp500_stocks: 503,
          stocks_with_pe_data: result.stocks.length,
          value_stocks_found: result.stocks.length,
          average_value_score: transformedRecommendations.reduce((sum, r) => sum + r.value_score, 0) / transformedRecommendations.length || 0,
          average_risk_score: transformedRecommendations.reduce((sum, r) => sum + r.risk_score, 0) / transformedRecommendations.length || 0
        });
        
        // Load valuation extremes for each stock
        await loadValuationExtremesForStocks(transformedRecommendations);
      } else {
        // Original P/E ratio screening
        const result = await recommendationsDataService.loadValueRecommendations(limit);
        
        if (result.error) {
          setError(result.error);
          console.error('Error loading P/E recommendations:', result.error);
          return;
        }
        
        setRecommendations(result.recommendations);
        setStats(result.stats);
        
        // Load valuation extremes for each stock
        await loadValuationExtremesForStocks(result.recommendations);
      }
    } catch (err) {
      setError(`Failed to load recommendations: ${err}`);
      console.error('Error loading recommendations:', err);
    } finally {
      setLoading(false);
    }
  }

  async function loadValuationExtremesForStocks(stocks) {
    const extremes = {};
    
    // Load extremes for each stock (limit to first 10 to avoid too many API calls)
    const stocksToLoad = stocks.slice(0, 10);
    
    // Run all API calls in parallel for better performance
    const promises = stocksToLoad.map(async (stock) => {
      try {
        const result = await analysisDataService.loadValuationExtremes(stock.symbol);
        if (result.extremes) {
          extremes[stock.symbol] = result.extremes;
        }
      } catch (err) {
        console.warn(`Failed to load extremes for ${stock.symbol}:`, err);
      }
    });
    
    await Promise.all(promises);
    setValuationExtremes(extremes);
  }

  const getValueScoreColor = (score) => {
    if (score >= 80) return 'text-green-600 bg-green-100';
    if (score >= 60) return 'text-yellow-600 bg-yellow-100';
    return 'text-red-600 bg-red-100';
  };

  const getRiskScoreColor = (score) => {
    if (score <= 30) return 'text-green-600 bg-green-100';
    if (score <= 60) return 'text-yellow-600 bg-yellow-100';
    return 'text-red-600 bg-red-100';
  };

  if (loading) {
    return (
      <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
        <div className="bg-white rounded-lg p-8 max-w-md w-full mx-4">
          <div className="text-center">
            <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto"></div>
            <p className="mt-4 text-gray-600">Analyzing S&P 500 stocks...</p>
          </div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
        <div className="bg-white rounded-lg p-8 max-w-md w-full mx-4">
          <div className="text-center">
            <div className="text-red-600 mb-4">
              <svg className="mx-auto h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
            </div>
            <h3 className="text-lg font-medium text-gray-900 mb-2">Analysis Error</h3>
            <p className="text-gray-600 mb-4">{error}</p>
            <div className="flex gap-2 justify-center">
              <button
                onClick={loadRecommendationsWithStats}
                className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700"
              >
                Retry
              </button>
              <button
                onClick={onClose}
                className="bg-gray-300 text-gray-700 px-4 py-2 rounded hover:bg-gray-400"
              >
                Close
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4">
      <div className="bg-white rounded-lg shadow-xl max-w-6xl w-full max-h-full overflow-hidden flex flex-col">
        {/* Header */}
        <div className="bg-blue-600 text-white p-6 flex justify-between items-center">
          <div>
            <h2 className="text-2xl font-bold">Stock Value Recommendations</h2>
            <p className="text-blue-100 mt-1">
              {screeningType === 'ps' 
                ? `P/S Screening with Revenue Growth: Statistical undervaluation + growth requirements (S&P 500 only, Market Cap > $${(minMarketCap / 1_000_000).toFixed(0)}M)` 
                : screeningType === 'garp_pe'
                ? `GARP P/E Screening: Growth at Reasonable Price using PEG ratios (S&P 500 only, PEG < ${garpPeCriteria.maxPegRatio}, Revenue Growth > ${garpPeCriteria.minRevenueGrowth}%)`
                : 'P/E ratio-based value screening for S&P 500 stocks'
              }
            </p>
          </div>
          <button
            onClick={onClose}
            className="text-white hover:text-gray-200 text-2xl font-bold"
          >
            ×
          </button>
        </div>

        {/* Stats Panel */}
        {stats && (
          <div className="bg-gray-50 p-4 border-b">
            <div className="grid grid-cols-2 md:grid-cols-5 gap-4 text-center">
              <div>
                <div className="text-2xl font-bold text-blue-600">{stats.total_sp500_stocks}</div>
                <div className="text-sm text-gray-600">Total S&P 500</div>
              </div>
              <div>
                <div className="text-2xl font-bold text-green-600">{stats.stocks_with_pe_data}</div>
                <div className="text-sm text-gray-600">With P/E Data</div>
              </div>
              <div>
                <div className="text-2xl font-bold text-purple-600">{stats.value_stocks_found}</div>
                <div className="text-sm text-gray-600">Value Stocks</div>
              </div>
              <div>
                <div className="text-2xl font-bold text-orange-600">{stats.average_value_score.toFixed(1)}</div>
                <div className="text-sm text-gray-600">Avg Value Score</div>
              </div>
              <div>
                <div className="text-2xl font-bold text-red-600">{stats.average_risk_score.toFixed(1)}</div>
                <div className="text-sm text-gray-600">Avg Risk Score</div>
              </div>
            </div>
          </div>
        )}

        {/* Controls */}
        <div className="p-4 bg-gray-50 border-b">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-6">
              {/* Screening Type Selection */}
              <div className="flex items-center gap-4">
                <label className="text-sm font-medium text-gray-700">
                  Screening Method:
                </label>
                <select
                  value={screeningType}
                  onChange={(e) => setScreeningType(e.target.value)}
                  className="border border-gray-300 rounded px-3 py-1 text-sm"
                >
                  <option value="pe">P/E Ratio (Historical)</option>
                  <option value="ps">P/S Ratio (TTM)</option>
                  <option value="garp_pe">GARP (P/E + PEG Based)</option>
                </select>
              </div>

              {/* Market Cap Filter (only show for P/S screening) */}
              {screeningType === 'ps' && (
                <div className="flex items-center gap-2">
                  <label className="text-sm font-medium text-gray-700">
                    Min Market Cap:
                  </label>
                  <select
                    value={minMarketCap}
                    onChange={(e) => setMinMarketCap(Number(e.target.value))}
                    className="border border-gray-300 rounded px-2 py-1 text-sm"
                  >
                    <option value={100_000_000}>$100M</option>
                    <option value={250_000_000}>$250M</option>
                    <option value={500_000_000}>$500M</option>
                    <option value={1_000_000_000}>$1B</option>
                    <option value={2_000_000_000}>$2B</option>
                    <option value={5_000_000_000}>$5B</option>
                    <option value={10_000_000_000}>$10B</option>
                  </select>
                </div>
              )}

              {/* GARP P/E Criteria Controls */}
              {screeningType === 'garp_pe' && (
                <div className="garp-pe-controls space-y-4">
                  <div className="grid grid-cols-2 gap-4">
                    <div>
                      <label className="text-sm font-medium text-gray-700">
                        Max PEG Ratio:
                      </label>
                      <select
                        value={garpPeCriteria.maxPegRatio}
                        onChange={(e) => setGarpPeCriteria(prev => ({
                          ...prev,
                          maxPegRatio: Number(e.target.value)
                        }))}
                        className="border border-gray-300 rounded px-2 py-1 text-sm w-full"
                      >
                        <option value={0.5}>0.5</option>
                        <option value={0.8}>0.8</option>
                        <option value={1.0}>1.0</option>
                        <option value={1.2}>1.2</option>
                        <option value={1.5}>1.5</option>
                        <option value={2.0}>2.0</option>
                      </select>
                    </div>
                    
                    <div>
                      <label className="text-sm font-medium text-gray-700">
                        Min Revenue Growth (%):
                      </label>
                      <select
                        value={garpPeCriteria.minRevenueGrowth}
                        onChange={(e) => setGarpPeCriteria(prev => ({
                          ...prev,
                          minRevenueGrowth: Number(e.target.value)
                        }))}
                        className="border border-gray-300 rounded px-2 py-1 text-sm w-full"
                      >
                        <option value={5}>5%</option>
                        <option value={10}>10%</option>
                        <option value={15}>15%</option>
                        <option value={20}>20%</option>
                        <option value={25}>25%</option>
                      </select>
                    </div>
                    
                    <div>
                      <label className="text-sm font-medium text-gray-700">
                        Min Profit Margin (%):
                      </label>
                      <select
                        value={garpPeCriteria.minProfitMargin}
                        onChange={(e) => setGarpPeCriteria(prev => ({
                          ...prev,
                          minProfitMargin: Number(e.target.value)
                        }))}
                        className="border border-gray-300 rounded px-2 py-1 text-sm w-full"
                      >
                        <option value={1}>1%</option>
                        <option value={3}>3%</option>
                        <option value={5}>5%</option>
                        <option value={7}>7%</option>
                        <option value={10}>10%</option>
                      </select>
                    </div>
                    
                    <div>
                      <label className="text-sm font-medium text-gray-700">
                        Max Debt-to-Equity:
                      </label>
                      <select
                        value={garpPeCriteria.maxDebtToEquity}
                        onChange={(e) => setGarpPeCriteria(prev => ({
                          ...prev,
                          maxDebtToEquity: Number(e.target.value)
                        }))}
                        className="border border-gray-300 rounded px-2 py-1 text-sm w-full"
                      >
                        <option value={1}>1.0</option>
                        <option value={2}>2.0</option>
                        <option value={3}>3.0</option>
                        <option value={5}>5.0</option>
                      </select>
                    </div>
                  </div>
                  
                  <div className="flex items-center gap-4">
                    <label className="flex items-center gap-2">
                      <input
                        type="checkbox"
                        checked={garpPeCriteria.requirePositiveEarnings}
                        onChange={(e) => setGarpPeCriteria(prev => ({
                          ...prev,
                          requirePositiveEarnings: e.target.checked
                        }))}
                        className="rounded"
                      />
                      <span className="text-sm text-gray-700">Require Positive Earnings (Net Income &gt; 0)</span>
                    </label>
                  </div>
                </div>
              )}

              {/* Limit Selection */}
              <div className="flex items-center gap-2">
                <label className="text-sm font-medium text-gray-700">
                  Show top:
                </label>
                <select
                  value={limit}
                  onChange={(e) => setLimit(Number(e.target.value))}
                  className="border border-gray-300 rounded px-3 py-1 text-sm"
                >
                  <option value={10}>10 stocks</option>
                  <option value={20}>20 stocks</option>
                  <option value={50}>50 stocks</option>
                  <option value={100}>100 stocks</option>
                </select>
              </div>
            </div>
          </div>
        </div>

        {/* Recommendations List */}
        <div className="flex-1 overflow-auto">
          {recommendations.length === 0 ? (
            <div className="p-8 text-center">
              <div className="text-gray-400 mb-4">
                <svg className="mx-auto h-16 w-16" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                </svg>
              </div>
              <h3 className="text-xl font-medium text-gray-900 mb-2">No Value Stocks Found</h3>
              <p className="text-gray-600">
                {screeningType === 'ps' 
                  ? `No S&P 500 stocks currently meet our smart P/S criteria (Historical analysis + Z-score screening). This could indicate the market is fairly valued or overvalued.`
                  : 'No S&P 500 stocks currently meet our value criteria (P/E ≤ 20% above historical minimum). This could indicate the market is fairly valued or overvalued.'
                }
              </p>
            </div>
          ) : (
            <div className="divide-y divide-gray-200">
              {recommendations.map((rec) => (
                <div key={`${rec.symbol}-${rec.rank}`} className="p-4 hover:bg-gray-50">
                  <div className="flex items-center justify-between">
                    {/* Stock Info */}
                    <div className="flex-1">
                      <div className="flex items-center gap-3 mb-2">
                        <span className="text-lg font-bold text-gray-900">#{rec.rank}</span>
                        <span className="text-xl font-bold text-blue-600">{rec.symbol}</span>
                        <span className="text-gray-600">{rec.company_name}</span>
                      </div>
                      <div className="text-sm text-gray-600 mb-2">{rec.reasoning}</div>
                    </div>

                    {/* Metrics */}
                    <div className="flex gap-4 items-center">
                      {screeningType === 'ps' ? (
                        <>
                          {/* Current P/S Ratio */}
                          <div className="text-center">
                            <div className="text-lg font-bold text-gray-900">
                              {rec.ps_ratio_ttm ? rec.ps_ratio_ttm.toFixed(2) : 'N/A'}
                            </div>
                            <div className="text-xs text-gray-500">Current P/S</div>
                          </div>

                          {/* Z-Score */}
                          <div className="text-center">
                            <div className="text-sm font-bold text-gray-700">
                              {rec.z_score ? rec.z_score.toFixed(2) : 'N/A'}
                            </div>
                            <div className="text-xs text-gray-500">Z-Score</div>
                          </div>

                          {/* Market Cap */}
                          <div className="text-center">
                            <div className="text-sm text-gray-700">
                              {rec.market_cap ? `$${(rec.market_cap / 1e9).toFixed(1)}B` : 'N/A'}
                            </div>
                            <div className="text-xs text-gray-500">Market Cap</div>
                          </div>

                          {/* Historical Range */}
                          {valuationExtremes[rec.symbol] ? (
                            <div className="text-center">
                              <div className="text-xs text-gray-500 mb-1">Historical Range</div>
                              <div className="text-xs text-gray-600">
                                P/S: {rec.historical_min?.toFixed(2) || 'N/A'} - {rec.historical_max?.toFixed(2) || 'N/A'}
                              </div>
                              <div className="text-xs text-gray-500">
                                Mean: {rec.historical_mean?.toFixed(2) || 'N/A'} (±{rec.historical_variance ? Math.sqrt(rec.historical_variance).toFixed(2) : 'N/A'})
                              </div>
                            </div>
                          ) : (
                            <div className="text-center">
                              <div className="text-xs text-gray-400">Loading ranges...</div>
                            </div>
                          )}
                        </>
                      ) : screeningType === 'garp_pe' ? (
                        <>
                          {/* Current P/E Ratio */}
                          <div className="text-center">
                            <div className="text-lg font-bold text-gray-900">
                              {rec.current_pe_ratio ? rec.current_pe_ratio.toFixed(2) : 'N/A'}
                            </div>
                            <div className="text-xs text-gray-500">Current P/E</div>
                          </div>

                          {/* PEG Ratio */}
                          <div className="text-center">
                            <div className="text-sm font-bold text-gray-700">
                              {rec.peg_ratio ? rec.peg_ratio.toFixed(2) : 'N/A'}
                            </div>
                            <div className="text-xs text-gray-500">PEG Ratio</div>
                          </div>

                          {/* Revenue Growth */}
                          <div className="text-center">
                            <div className="text-sm font-bold text-gray-700">
                              {rec.ttm_growth_rate ? rec.ttm_growth_rate.toFixed(1) + '%' : rec.annual_growth_rate ? rec.annual_growth_rate.toFixed(1) + '%' : 'N/A'}
                            </div>
                            <div className="text-xs text-gray-500">Revenue Growth</div>
                          </div>

                          {/* Profit Margin */}
                          <div className="text-center">
                            <div className="text-sm font-bold text-gray-700">
                              {rec.net_profit_margin ? rec.net_profit_margin.toFixed(1) + '%' : 'N/A'}
                            </div>
                            <div className="text-xs text-gray-500">Profit Margin</div>
                          </div>

                          {/* GARP Score */}
                          <div className="text-center">
                            <div className="text-sm font-bold text-gray-700">
                              {rec.garp_score ? rec.garp_score.toFixed(2) : 'N/A'}
                            </div>
                            <div className="text-xs text-gray-500">GARP Score</div>
                          </div>
                        </>
                      ) : (
                        <>
                          {/* Current P/E */}
                          <div className="text-center">
                            <div className="text-lg font-bold text-gray-900">
                              {rec.current_pe ? rec.current_pe.toFixed(1) : 'N/A'}
                            </div>
                            <div className="text-xs text-gray-500">Current P/E</div>
                            {rec.current_pe_date && (
                              <div className="text-xs text-gray-400">
                                {new Date(rec.current_pe_date).toLocaleDateString()}
                              </div>
                            )}
                          </div>

                          {/* Historical Range */}
                          <div className="text-center">
                            <div className="text-sm text-gray-700">
                              {rec.historical_min_pe.toFixed(1)} - {rec.historical_max_pe.toFixed(1)}
                            </div>
                            <div className="text-xs text-gray-500">Historical Range</div>
                          </div>
                        </>
                      )}

                      {/* Value Score */}
                      <div className="text-center">
                        <div className={`text-lg font-bold px-2 py-1 rounded ${getValueScoreColor(rec.value_score)}`}>
                          {rec.value_score.toFixed(0)}
                        </div>
                        <div className="text-xs text-gray-500">Value Score</div>
                      </div>

                      {/* Risk Score */}
                      <div className="text-center">
                        <div className={`text-lg font-bold px-2 py-1 rounded ${getRiskScoreColor(rec.risk_score)}`}>
                          {rec.risk_score.toFixed(0)}
                        </div>
                        <div className="text-xs text-gray-500">Risk Score</div>
                      </div>

                      {/* Data Points */}
                      <div className="text-center">
                        <div className="text-sm text-gray-700">{rec.data_points}</div>
                        <div className="text-xs text-gray-500">Data Points</div>
                      </div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Collapsible Footer */}
        <div className="bg-gray-50 border-t">
          {/* Footer Toggle Button */}
          <button
            onClick={() => setIsFooterExpanded(!isFooterExpanded)}
            className="w-full px-4 py-2 text-left text-xs text-gray-600 hover:bg-gray-100 flex items-center justify-between transition-colors"
          >
            <span className="font-medium">Algorithm Details & Disclaimer</span>
            <svg 
              className={`w-4 h-4 transition-transform ${isFooterExpanded ? 'rotate-180' : ''}`}
              fill="none" 
              stroke="currentColor" 
              viewBox="0 0 24 24"
            >
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
            </svg>
          </button>
          
          {/* Collapsible Content */}
          {isFooterExpanded && (
            <div className="px-4 pb-4">
              <div className="text-xs text-gray-600 space-y-1">
                {screeningType === 'ps' ? (
                  <>
                    <p><strong>P/S Screening with Revenue Growth:</strong> Statistical undervaluation + growth requirements (S&P 500 only, Market Cap &gt; $${(minMarketCap / 1_000_000).toFixed(0)}M)</p>
                    <p><strong>Criteria:</strong> Current P/S &lt; (Historical Median - 1.0 × Std Dev) AND Revenue Growth &gt; 0% (TTM OR Annual) AND Quality Score ≥ 50</p>
                    <p><strong>Value Score:</strong> Higher is better (0-100). Based on how low the P/S ratio is relative to historical average.</p>
                    <p><strong>Risk Score:</strong> Lower is better (0-100). Higher P/S ratios indicate higher valuation risk.</p>
                    <p><strong>Z-Score:</strong> How many standard deviations below/above the historical P/S mean.</p>
                    <p><strong>Revenue Growth:</strong> TTM or Annual revenue growth rate (percentage).</p>
                  </>
                ) : screeningType === 'garp_pe' ? (
                  <>
                    <p><strong>GARP P/E Screening:</strong> Growth at Reasonable Price using PEG ratios (S&P 500 only)</p>
                    <p><strong>Criteria:</strong> PEG &lt; {garpPeCriteria.maxPegRatio} AND Revenue Growth &gt; {garpPeCriteria.minRevenueGrowth}% AND Profit Margin &gt; {garpPeCriteria.minProfitMargin}% AND Debt-to-Equity &lt; {garpPeCriteria.maxDebtToEquity} AND Net Income &gt; 0</p>
                    <p><strong>Value Score:</strong> Higher is better (0-100). Based on GARP score (Revenue Growth % / PEG Ratio).</p>
                    <p><strong>Risk Score:</strong> Lower is better (0-100). Higher P/E ratios indicate higher valuation risk.</p>
                    <p><strong>PEG Ratio:</strong> Price/Earnings to Growth ratio. Lower values indicate better value relative to growth.</p>
                    <p><strong>GARP Score:</strong> Revenue Growth % divided by PEG Ratio. Higher values indicate better growth-to-value balance.</p>
                    <p><strong>Revenue Growth:</strong> TTM or Annual revenue growth rate (percentage).</p>
                    <p><strong>Profit Margin:</strong> Net profit margin (Net Income / Revenue).</p>
                  </>
                ) : (
                  <>
                    <p><strong>P/E Screening Criteria:</strong> Current P/E ≤ Historical Minimum × 1.20 (20% above historical low)</p>
                    <p><strong>Value Score:</strong> Higher is better (0-120). Based on position in historical P/E range with bonuses for near-minimum values.</p>
                    <p><strong>Risk Score:</strong> Lower is better (0-100). Based on P/E volatility, extreme values, and data quality.</p>
                  </>
                )}
                <p><strong>Disclaimer:</strong> This analysis is for educational purposes only. Past performance does not predict future results. Consider additional factors before investing.</p>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export default RecommendationsPanel;