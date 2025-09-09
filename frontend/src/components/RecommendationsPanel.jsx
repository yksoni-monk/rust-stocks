import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

function RecommendationsPanel({ onClose }) {
  const [recommendations, setRecommendations] = useState([]);
  const [stats, setStats] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [limit, setLimit] = useState(20);

  useEffect(() => {
    loadRecommendationsWithStats();
  }, [limit]);

  async function loadRecommendationsWithStats() {
    try {
      setLoading(true);
      const response = await invoke('get_value_recommendations_with_stats', { limit });
      setRecommendations(response.recommendations);
      setStats(response.stats);
    } catch (err) {
      setError(`Failed to load recommendations: ${err}`);
      console.error('Error loading recommendations:', err);
    } finally {
      setLoading(false);
    }
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
            <p className="text-blue-100 mt-1">P/E ratio-based value screening for S&P 500 stocks</p>
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
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
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
            <div className="text-sm text-gray-600">
              Found {recommendations.length} value opportunities
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
                No S&P 500 stocks currently meet our value criteria (P/E ≤ 20% above historical minimum).
                This could indicate the market is fairly valued or overvalued.
              </p>
            </div>
          ) : (
            <div className="divide-y divide-gray-200">
              {recommendations.map((rec) => (
                <div key={rec.symbol} className="p-4 hover:bg-gray-50">
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

        {/* Footer */}
        <div className="bg-gray-50 p-4 border-t">
          <div className="text-xs text-gray-600 space-y-1">
            <p><strong>Value Criteria:</strong> Current P/E ≤ Historical Minimum × 1.20 (20% above historical low)</p>
            <p><strong>Value Score:</strong> Higher is better (0-120). Based on position in historical P/E range with bonuses for near-minimum values.</p>
            <p><strong>Risk Score:</strong> Lower is better (0-100). Based on P/E volatility, extreme values, and data quality.</p>
            <p><strong>Disclaimer:</strong> This analysis is for educational purposes only. Past performance does not predict future results. Consider additional factors before investing.</p>
          </div>
        </div>
      </div>
    </div>
  );
}

export default RecommendationsPanel;