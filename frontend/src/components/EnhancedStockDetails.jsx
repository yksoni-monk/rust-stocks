import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

// Enhanced fundamental metrics card component
function FundamentalMetricCard({ title, value, prefix = '', suffix = '', isPositive = null, description = '' }) {
  const getValueColor = (value, isPositive) => {
    if (isPositive === null || value === null || value === undefined || value === 'N/A') return 'text-gray-600';
    return isPositive ? 'text-green-600' : 'text-red-600';
  };

  const formatValue = (val) => {
    if (val === null || val === undefined) return 'N/A';
    if (typeof val === 'number') {
      if (val > 1000000000) return `${(val / 1000000000).toFixed(2)}B`;
      if (val > 1000000) return `${(val / 1000000).toFixed(2)}M`;
      if (val > 1000) return `${(val / 1000).toFixed(2)}K`;
      return val.toFixed(2);
    }
    return val;
  };

  return (
    <div className="bg-white p-4 rounded-lg shadow border hover:shadow-md transition-shadow">
      <div className="flex items-start justify-between">
        <div>
          <h4 className="text-sm font-medium text-gray-500 mb-1">{title}</h4>
          <div className={`text-2xl font-bold ${getValueColor(value, isPositive)}`}>
            {prefix}{formatValue(value)}{suffix}
          </div>
          {description && (
            <p className="text-xs text-gray-400 mt-1">{description}</p>
          )}
        </div>
      </div>
    </div>
  );
}

// Enhanced stock details component
function EnhancedStockDetails({ selectedStock, onBack }) {
  const [stockInfo, setStockInfo] = useState(null);
  const [fundamentals, setFundamentals] = useState(null);
  const [priceHistory, setPriceHistory] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);
  const [activeTab, setActiveTab] = useState('overview');

  useEffect(() => {
    if (selectedStock) {
      loadEnhancedData();
    }
  }, [selectedStock]);

  async function loadEnhancedData() {
    if (!selectedStock) return;

    setLoading(true);
    setError(null);

    try {
      // Load enhanced stock information
      const stockInfoResponse = await invoke('get_enhanced_stock_info', {
        symbol: selectedStock.symbol
      });

      if (stockInfoResponse.success) {
        setStockInfo(stockInfoResponse.data);
      }

      // Load fundamentals from Schwab API
      const fundamentalsResponse = await invoke('get_fundamentals', {
        symbol: selectedStock.symbol
      });

      if (fundamentalsResponse.success) {
        setFundamentals(fundamentalsResponse.data);
      }

      // Load enhanced price history
      const priceResponse = await invoke('get_enhanced_price_history', {
        symbol: selectedStock.symbol,
        startDate: '2024-01-01',
        endDate: '2024-12-31'
      });

      if (priceResponse.success) {
        setPriceHistory(priceResponse.data || []);
      }

    } catch (err) {
      console.error('Failed to load enhanced data:', err);
      setError(err.toString());
    } finally {
      setLoading(false);
    }
  }

  async function populateEnhancedData(withFundamentals = true) {
    setLoading(true);
    try {
      const result = await invoke('populate_enhanced_stock_data', {
        symbol: selectedStock.symbol,
        startDate: '2024-01-01',
        endDate: '2024-12-31',
        fetchFundamentals: withFundamentals
      });

      if (result.success) {
        alert(`Success: ${result.data}`);
        // Reload data after population
        await loadEnhancedData();
      } else {
        alert(`Error: ${result.error}`);
      }
    } catch (err) {
      console.error('Failed to populate enhanced data:', err);
      alert(`Failed to populate data: ${err}`);
    } finally {
      setLoading(false);
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600"></div>
        <span className="ml-3 text-lg">Loading enhanced stock data...</span>
      </div>
    );
  }

  return (
    <div className="max-w-6xl mx-auto">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center">
          <button
            onClick={onBack}
            className="mr-4 p-2 rounded-lg hover:bg-gray-100 transition-colors"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
            </svg>
          </button>
          <div>
            <h1 className="text-3xl font-bold">{selectedStock.symbol}</h1>
            <p className="text-gray-600">{stockInfo?.company_name || selectedStock.company_name}</p>
          </div>
        </div>
        <div className="flex gap-2">
          <button
            onClick={() => populateEnhancedData(true)}
            className="bg-blue-600 text-white px-4 py-2 rounded-lg hover:bg-blue-700 transition-colors"
            disabled={loading}
          >
            Fetch with Fundamentals
          </button>
          <button
            onClick={() => populateEnhancedData(false)}
            className="bg-gray-600 text-white px-4 py-2 rounded-lg hover:bg-gray-700 transition-colors"
            disabled={loading}
          >
            Fetch Price Only
          </button>
        </div>
      </div>

      {error && (
        <div className="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-6">
          Error: {error}
        </div>
      )}

      {/* Tabs */}
      <div className="border-b border-gray-200 mb-6">
        <nav className="-mb-px flex">
          {[
            { id: 'overview', label: 'Overview' },
            { id: 'fundamentals', label: 'Fundamentals' },
            { id: 'price-history', label: 'Price History' }
          ].map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`py-2 px-4 border-b-2 font-medium text-sm ${
                activeTab === tab.id
                  ? 'border-blue-500 text-blue-600'
                  : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
              }`}
            >
              {tab.label}
            </button>
          ))}
        </nav>
      </div>

      {/* Tab Content */}
      {activeTab === 'overview' && (
        <div className="space-y-6">
          {/* Company Information */}
          {stockInfo && (
            <div className="bg-white p-6 rounded-lg shadow">
              <h2 className="text-xl font-semibold mb-4">Company Information</h2>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                  <p className="text-sm text-gray-600">Exchange</p>
                  <p className="font-medium">{stockInfo.exchange || 'N/A'}</p>
                </div>
                <div>
                  <p className="text-sm text-gray-600">Sector</p>
                  <p className="font-medium">{stockInfo.sector || 'N/A'}</p>
                </div>
                <div>
                  <p className="text-sm text-gray-600">Industry</p>
                  <p className="font-medium">{stockInfo.industry || 'N/A'}</p>
                </div>
                <div>
                  <p className="text-sm text-gray-600">Employees</p>
                  <p className="font-medium">{stockInfo.employees?.toLocaleString() || 'N/A'}</p>
                </div>
                {stockInfo.description && (
                  <div className="md:col-span-2">
                    <p className="text-sm text-gray-600">Description</p>
                    <p className="font-medium">{stockInfo.description}</p>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Key Metrics */}
          {fundamentals && (
            <div>
              <h2 className="text-xl font-semibold mb-4">Key Metrics</h2>
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
                <FundamentalMetricCard
                  title="Market Cap"
                  value={fundamentals.market_cap}
                  prefix="$"
                  description="Total market value of shares"
                />
                <FundamentalMetricCard
                  title="P/E Ratio"
                  value={fundamentals.pe_ratio}
                  description="Price to earnings ratio"
                />
                <FundamentalMetricCard
                  title="Dividend Yield"
                  value={fundamentals.dividend_yield}
                  suffix="%"
                  description="Annual dividend as % of price"
                />
                <FundamentalMetricCard
                  title="EPS"
                  value={fundamentals.eps}
                  prefix="$"
                  description="Earnings per share (TTM)"
                />
              </div>
            </div>
          )}
        </div>
      )}

      {activeTab === 'fundamentals' && (
        <div className="space-y-6">
          {fundamentals ? (
            <>
              {/* Valuation Metrics */}
              <div>
                <h2 className="text-xl font-semibold mb-4">Valuation Metrics</h2>
                <div className="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-4 gap-4">
                  <FundamentalMetricCard
                    title="P/E Ratio"
                    value={fundamentals.pe_ratio}
                    description="Current P/E ratio"
                  />
                  <FundamentalMetricCard
                    title="Forward P/E"
                    value={fundamentals.pe_ratio_forward}
                    description="Forward P/E estimate"
                  />
                  <FundamentalMetricCard
                    title="P/B Ratio"
                    value={fundamentals.pb_ratio}
                    description="Price to book ratio"
                  />
                  <FundamentalMetricCard
                    title="P/S Ratio"
                    value={fundamentals.ps_ratio}
                    description="Price to sales ratio"
                  />
                </div>
              </div>

              {/* Growth & Profitability */}
              <div>
                <h2 className="text-xl font-semibold mb-4">Growth & Profitability</h2>
                <div className="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-4 gap-4">
                  <FundamentalMetricCard
                    title="EPS"
                    value={fundamentals.eps}
                    prefix="$"
                    description="Earnings per share (TTM)"
                  />
                  <FundamentalMetricCard
                    title="Forward EPS"
                    value={fundamentals.eps_forward}
                    prefix="$"
                    description="Forward EPS estimate"
                  />
                  <FundamentalMetricCard
                    title="Revenue (TTM)"
                    value={fundamentals.revenue_ttm}
                    prefix="$"
                    description="Trailing twelve months"
                  />
                  <FundamentalMetricCard
                    title="Profit Margin"
                    value={fundamentals.profit_margin}
                    suffix="%"
                    isPositive={fundamentals.profit_margin > 0}
                    description="Net profit margin"
                  />
                </div>
              </div>

              {/* Risk & Returns */}
              <div>
                <h2 className="text-xl font-semibold mb-4">Risk & Returns</h2>
                <div className="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-4 gap-4">
                  <FundamentalMetricCard
                    title="Beta"
                    value={fundamentals.beta}
                    description="Market risk relative to S&P 500"
                  />
                  <FundamentalMetricCard
                    title="52W High"
                    value={fundamentals.week_52_high}
                    prefix="$"
                    description="52-week high price"
                  />
                  <FundamentalMetricCard
                    title="52W Low"
                    value={fundamentals.week_52_low}
                    prefix="$"
                    description="52-week low price"
                  />
                  <FundamentalMetricCard
                    title="ROE"
                    value={fundamentals.return_on_equity}
                    suffix="%"
                    isPositive={fundamentals.return_on_equity > 0}
                    description="Return on equity"
                  />
                </div>
              </div>

              {/* Balance Sheet */}
              <div>
                <h2 className="text-xl font-semibold mb-4">Balance Sheet</h2>
                <div className="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-4 gap-4">
                  <FundamentalMetricCard
                    title="Debt to Equity"
                    value={fundamentals.debt_to_equity}
                    isPositive={fundamentals.debt_to_equity < 0.5}
                    description="Total debt / shareholder equity"
                  />
                  <FundamentalMetricCard
                    title="ROA"
                    value={fundamentals.return_on_assets}
                    suffix="%"
                    isPositive={fundamentals.return_on_assets > 0}
                    description="Return on assets"
                  />
                  <FundamentalMetricCard
                    title="Operating Margin"
                    value={fundamentals.operating_margin}
                    suffix="%"
                    isPositive={fundamentals.operating_margin > 0}
                    description="Operating income / revenue"
                  />
                  <FundamentalMetricCard
                    title="Shares Outstanding"
                    value={fundamentals.shares_outstanding}
                    description="Total shares outstanding"
                  />
                </div>
              </div>
            </>
          ) : (
            <div className="bg-gray-100 p-8 rounded-lg text-center">
              <p className="text-gray-600">No fundamental data available. Fetch data to see comprehensive metrics.</p>
            </div>
          )}
        </div>
      )}

      {activeTab === 'price-history' && (
        <div className="space-y-6">
          {priceHistory.length > 0 ? (
            <div className="bg-white rounded-lg shadow overflow-hidden">
              <div className="px-6 py-4 bg-gray-50 border-b">
                <h2 className="text-xl font-semibold">Enhanced Price History</h2>
                <p className="text-sm text-gray-600">Latest {Math.min(20, priceHistory.length)} records with fundamental data</p>
              </div>
              <div className="overflow-x-auto">
                <table className="min-w-full divide-y divide-gray-200">
                  <thead className="bg-gray-50">
                    <tr>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Date</th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Close</th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Volume</th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">P/E</th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Dividend Yield</th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">EPS</th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Beta</th>
                    </tr>
                  </thead>
                  <tbody className="bg-white divide-y divide-gray-200">
                    {priceHistory.slice(-20).reverse().map((price, index) => (
                      <tr key={index} className="hover:bg-gray-50">
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{price.date}</td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">
                          ${price.close_price.toFixed(2)}
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                          {price.volume ? price.volume.toLocaleString() : 'N/A'}
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                          {price.pe_ratio ? price.pe_ratio.toFixed(2) : 'N/A'}
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                          {price.dividend_yield ? `${price.dividend_yield.toFixed(2)}%` : 'N/A'}
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                          {price.eps ? `$${price.eps.toFixed(2)}` : 'N/A'}
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                          {price.beta ? price.beta.toFixed(2) : 'N/A'}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          ) : (
            <div className="bg-gray-100 p-8 rounded-lg text-center">
              <p className="text-gray-600">No enhanced price history available. Fetch data to see detailed price information with fundamental metrics.</p>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default EnhancedStockDetails;