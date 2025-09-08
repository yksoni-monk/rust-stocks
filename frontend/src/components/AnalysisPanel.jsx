import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';

function AnalysisPanel({ stock }) {
  const [selectedMetric, setSelectedMetric] = useState('price');
  const [selectedPeriod, setSelectedPeriod] = useState('all_time');
  const [customStartDate, setCustomStartDate] = useState('2024-01-01');
  const [customEndDate, setCustomEndDate] = useState('2024-12-31');
  const [priceHistory, setPriceHistory] = useState([]);
  const [quickMetrics, setQuickMetrics] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);
  const [stockDateRange, setStockDateRange] = useState(null);

  const metricOptions = [
    { value: 'price', label: 'Price History' },
    { value: 'pe_ratio', label: 'P/E Ratio Trend' },
    { value: 'eps', label: 'Earnings Per Share' },
    { value: 'dividend_yield', label: 'Dividend Yield' },
    { value: 'volume', label: 'Trading Volume' },
    { value: 'market_cap', label: 'Market Cap Changes' },
    { value: 'beta', label: 'Beta (Risk)' },
  ];

  const periodOptions = [
    { value: '1M', label: '1 Month', days: 30 },
    { value: '3M', label: '3 Months', days: 90 },
    { value: '6M', label: '6 Months', days: 180 },
    { value: '1Y', label: '1 Year', days: 365 },
    { value: '2Y', label: '2 Years', days: 730 },
    { value: 'all_time', label: 'All Time', days: null },
    { value: 'custom', label: 'Custom Range' },
  ];

  const getDateRange = () => {
    if (selectedPeriod === 'custom') {
      return { startDate: customStartDate, endDate: customEndDate };
    }
    
    if (selectedPeriod === 'all_time' && stockDateRange) {
      return { 
        startDate: stockDateRange.earliest_date, 
        endDate: stockDateRange.latest_date 
      };
    }
    
    const period = periodOptions.find(p => p.value === selectedPeriod);
    const endDate = new Date();
    const startDate = new Date(endDate.getTime() - (period.days * 24 * 60 * 60 * 1000));
    
    return {
      startDate: startDate.toISOString().split('T')[0],
      endDate: endDate.toISOString().split('T')[0]
    };
  };

  const loadStockDateRange = async () => {
    if (!stock?.symbol) return;
    
    try {
      const dateRange = await invoke('get_stock_date_range', {
        symbol: stock.symbol
      });
      setStockDateRange(dateRange);
    } catch (err) {
      console.error('Failed to load stock date range:', err);
      setStockDateRange(null);
    }
  };

  const loadData = async () => {
    if (!stock?.symbol) return;

    setLoading(true);
    setError(null);

    try {
      const { startDate, endDate } = getDateRange();
      
      // Load price history using the working API call
      const history = await invoke('get_price_history', {
        symbol: stock.symbol,
        startDate,
        endDate
      });

      setPriceHistory(history || []);

    } catch (err) {
      setError(err.toString());
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (stock?.symbol) {
      loadStockDateRange();
    }
  }, [stock.symbol]);

  useEffect(() => {
    loadData();
  }, [stock.symbol, selectedPeriod, customStartDate, customEndDate, stockDateRange]);

  const getMetricValue = (record, metric) => {
    switch (metric) {
      case 'price': return record.close || record.close_price;
      case 'pe_ratio': return record.pe_ratio;
      case 'eps': return record.eps;
      case 'dividend_yield': return record.dividend_yield;
      case 'volume': return record.volume;
      case 'market_cap': return record.market_cap;
      case 'beta': return record.beta;
      default: return record.close || record.close_price;
    }
  };

  const formatMetricValue = (value, metric) => {
    if (value === null || value === undefined) return 'N/A';
    
    switch (metric) {
      case 'price':
      case 'eps':
        return `$${value.toFixed(2)}`;
      case 'dividend_yield':
        return `${value.toFixed(2)}%`;
      case 'volume':
        return value.toLocaleString();
      case 'market_cap':
        if (value > 1e12) return `$${(value / 1e12).toFixed(2)}T`;
        if (value > 1e9) return `$${(value / 1e9).toFixed(2)}B`;
        if (value > 1e6) return `$${(value / 1e6).toFixed(2)}M`;
        return `$${value.toFixed(0)}`;
      default:
        return value.toFixed(2);
    }
  };

  const exportData = async (format) => {
    try {
      const result = await invoke('export_data', {
        symbol: stock.symbol,
        format
      });
      alert(`Export successful: ${result}`);
    } catch (err) {
      alert(`Export failed: ${err}`);
    }
  };

  const getChartData = () => {
    const chartData = priceHistory
      .filter(record => getMetricValue(record, selectedMetric) !== null)
      .map(record => ({
        date: record.date,
        value: getMetricValue(record, selectedMetric),
        formattedValue: formatMetricValue(getMetricValue(record, selectedMetric), selectedMetric)
      }));
    
    
    return chartData;
  };

  const chartData = getChartData();

  return (
    <div className="space-y-6">
      {/* Quick Metrics Row */}
      {priceHistory.length > 0 && (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="bg-white p-3 rounded-lg border text-center">
            <div className="text-sm text-gray-500">Latest Close</div>
            <div className="text-lg font-bold text-blue-600">
              ${priceHistory[priceHistory.length - 1]?.close?.toFixed(2) || 'N/A'}
            </div>
          </div>
          <div className="bg-white p-3 rounded-lg border text-center">
            <div className="text-sm text-gray-500">Highest</div>
            <div className="text-lg font-bold text-green-600">
              ${Math.max(...priceHistory.map(p => p.high || 0)).toFixed(2)}
            </div>
          </div>
          <div className="bg-white p-3 rounded-lg border text-center">
            <div className="text-sm text-gray-500">Lowest</div>
            <div className="text-lg font-bold text-red-600">
              ${Math.min(...priceHistory.filter(p => p.low > 0).map(p => p.low)).toFixed(2)}
            </div>
          </div>
          <div className="bg-white p-3 rounded-lg border text-center">
            <div className="text-sm text-gray-500">Avg Volume</div>
            <div className="text-lg font-bold text-purple-600">
              {Math.round(priceHistory.reduce((sum, p) => sum + (p.volume || 0), 0) / priceHistory.length).toLocaleString()}
            </div>
          </div>
        </div>
      )}

      {/* Controls */}
      <div className="flex flex-wrap items-center gap-4 p-4 bg-white rounded-lg border">
        <div className="flex items-center gap-2">
          <label className="text-sm font-medium">Metric:</label>
          <select
            value={selectedMetric}
            onChange={(e) => setSelectedMetric(e.target.value)}
            className="px-3 py-1 border border-gray-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            {metricOptions.map(option => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </div>

        <div className="flex items-center gap-2">
          <label className="text-sm font-medium">Period:</label>
          <select
            value={selectedPeriod}
            onChange={(e) => setSelectedPeriod(e.target.value)}
            className="px-3 py-1 border border-gray-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            {periodOptions.map(option => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </div>

        {selectedPeriod === 'custom' && (
          <div className="flex items-center gap-2">
            <input
              type="date"
              value={customStartDate}
              onChange={(e) => setCustomStartDate(e.target.value)}
              className="px-2 py-1 border border-gray-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
            <span className="text-sm text-gray-500">to</span>
            <input
              type="date"
              value={customEndDate}
              onChange={(e) => setCustomEndDate(e.target.value)}
              className="px-2 py-1 border border-gray-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>
        )}

        {selectedPeriod === 'all_time' && stockDateRange && (
          <div className="flex items-center gap-2 text-sm text-gray-600">
            <span className="bg-blue-100 text-blue-800 px-2 py-1 rounded">
              📅 {stockDateRange.earliest_date} to {stockDateRange.latest_date}
            </span>
            <span className="bg-green-100 text-green-800 px-2 py-1 rounded">
              📊 {stockDateRange.total_records.toLocaleString()} records
            </span>
          </div>
        )}

        <div className="flex gap-2 ml-auto">
          <button
            onClick={() => exportData('csv')}
            disabled={loading || chartData.length === 0}
            className="px-3 py-1 bg-green-600 text-white text-sm rounded hover:bg-green-700 disabled:bg-gray-400"
          >
            Export CSV
          </button>
          <button
            onClick={() => exportData('json')}
            disabled={loading || chartData.length === 0}
            className="px-3 py-1 bg-purple-600 text-white text-sm rounded hover:bg-purple-700 disabled:bg-gray-400"
          >
            Export JSON
          </button>
        </div>
      </div>

      {/* Chart/Data Display */}
      <div className="bg-white rounded-lg border p-4">
        {loading ? (
          <div className="text-center py-8">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 mx-auto mb-2"></div>
            <p className="text-sm text-gray-600">Loading {metricOptions.find(m => m.value === selectedMetric)?.label}...</p>
          </div>
        ) : error ? (
          <div className="text-center py-8 text-red-600">
            <p>Error: {error}</p>
          </div>
        ) : chartData.length > 0 ? (
          <div>
            <h3 className="text-lg font-semibold mb-4">
              {metricOptions.find(m => m.value === selectedMetric)?.label} - {stock.symbol}
            </h3>
            
            {/* Professional Chart with Recharts */}
            <div className="mb-6">
              <h4 className="text-lg font-semibold mb-4">
                {metricOptions.find(m => m.value === selectedMetric)?.label} over Time
              </h4>
              <div className="bg-white p-6 rounded-lg border shadow-sm">
                <ResponsiveContainer width="100%" height={400}>
                  <LineChart
                    data={chartData.map(point => ({
                      date: point.date,
                      value: point.value,
                      formattedValue: point.formattedValue
                    }))}
                    margin={{
                      top: 20,
                      right: 30,
                      left: 20,
                      bottom: 60,
                    }}
                  >
                    <CartesianGrid strokeDasharray="3 3" stroke="#f0f0f0" />
                    <XAxis 
                      dataKey="date" 
                      angle={-45}
                      textAnchor="end"
                      height={80}
                      tick={{ fontSize: 12 }}
                      tickFormatter={(value) => value.slice(5)} // Show MM-DD
                    />
                    <YAxis 
                      tick={{ fontSize: 12 }}
                      tickFormatter={(value) => formatMetricValue(value, selectedMetric)}
                    />
                    <Tooltip 
                      formatter={(value) => [formatMetricValue(value, selectedMetric), metricOptions.find(m => m.value === selectedMetric)?.label]}
                      labelFormatter={(label) => `Date: ${label}`}
                      contentStyle={{
                        backgroundColor: '#f8fafc',
                        border: '1px solid #e2e8f0',
                        borderRadius: '6px'
                      }}
                    />
                    <Legend />
                    <Line 
                      type="monotone" 
                      dataKey="value" 
                      stroke="#3b82f6" 
                      strokeWidth={3}
                      dot={{ fill: '#3b82f6', strokeWidth: 2, r: 4 }}
                      activeDot={{ r: 6, stroke: '#3b82f6', strokeWidth: 2 }}
                      name={metricOptions.find(m => m.value === selectedMetric)?.label}
                    />
                  </LineChart>
                </ResponsiveContainer>
              </div>
            </div>

            {/* Data Table */}
            <div className="overflow-x-auto">
              <table className="min-w-full text-sm">
                <thead className="bg-gray-50">
                  <tr>
                    <th className="px-3 py-2 text-left font-medium text-gray-500">Date</th>
                    <th className="px-3 py-2 text-left font-medium text-gray-500">
                      {metricOptions.find(m => m.value === selectedMetric)?.label}
                    </th>
                    {selectedMetric === 'price' && (
                      <>
                        <th className="px-3 py-2 text-left font-medium text-gray-500">Volume</th>
                        <th className="px-3 py-2 text-left font-medium text-gray-500">P/E Ratio</th>
                      </>
                    )}
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-200">
                  {chartData.slice(-10).reverse().map((point, index) => {
                    const record = priceHistory.find(r => r.date === point.date);
                    return (
                      <tr key={index} className="hover:bg-gray-50">
                        <td className="px-3 py-2">{point.date}</td>
                        <td className="px-3 py-2 font-medium">{point.formattedValue}</td>
                        {selectedMetric === 'price' && record && (
                          <>
                            <td className="px-3 py-2">{record.volume?.toLocaleString() || 'N/A'}</td>
                            <td className="px-3 py-2">{record.pe_ratio ? record.pe_ratio.toFixed(2) : 'N/A'}</td>
                          </>
                        )}
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          </div>
        ) : (
          <div className="text-center py-8 text-gray-600">
            <p>No data available for the selected period.</p>
            <p className="text-sm mt-2">Try fetching data first or selecting a different time range.</p>
          </div>
        )}
      </div>
    </div>
  );
}

export default AnalysisPanel;