import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

function DataFetchingPanel({ stock }) {
  const [importStats, setImportStats] = useState(null);
  const [loading, setLoading] = useState(false);

  // Load SimFin data statistics on component mount
  useEffect(() => {
    const loadImportStats = async () => {
      setLoading(true);
      try {
        // Get database statistics to show SimFin import status
        const stats = await invoke('get_database_stats');
        setImportStats(stats);
      } catch (error) {
        console.error('Failed to get database stats:', error);
      } finally {
        setLoading(false);
      }
    };

    loadImportStats();
  }, []);

  const formatNumber = (num) => {
    if (!num) return '0';
    return new Intl.NumberFormat().format(num);
  };

  return (
    <div className="space-y-6">
      <div className="bg-white rounded-lg border p-6">
        <h3 className="text-lg font-semibold mb-4">SimFin Historical Data Status</h3>
        
        {loading ? (
          <div className="flex items-center justify-center py-8">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
            <span className="ml-3 text-gray-600">Loading data status...</span>
          </div>
        ) : (
          <div className="space-y-4">
            {/* Data Import Status */}
            <div className="bg-gradient-to-r from-green-50 to-blue-50 rounded-lg p-4 border border-green-200">
              <h4 className="font-medium text-gray-900 mb-3 flex items-center">
                <span className="w-3 h-3 bg-green-500 rounded-full mr-2"></span>
                SimFin Historical Data (2019-2024)
              </h4>
              
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
                <div className="bg-white rounded-lg p-3 border">
                  <div className="font-medium text-gray-900">📊 Daily Prices</div>
                  <div className="text-2xl font-bold text-blue-600 mt-1">
                    {formatNumber(importStats?.daily_prices_count || 0)}
                  </div>
                  <div className="text-gray-500 text-xs">price records</div>
                </div>
                
                <div className="bg-white rounded-lg p-3 border">
                  <div className="font-medium text-gray-900">🏢 Stocks</div>
                  <div className="text-2xl font-bold text-green-600 mt-1">
                    {formatNumber(importStats?.stocks_count || 0)}
                  </div>
                  <div className="text-gray-500 text-xs">companies</div>
                </div>
                
                <div className="bg-white rounded-lg p-3 border">
                  <div className="font-medium text-gray-900">📈 P/E Ratios</div>
                  <div className="text-2xl font-bold text-purple-600 mt-1">
                    {formatNumber(importStats?.pe_ratios_count || 0)}
                  </div>
                  <div className="text-gray-500 text-xs">calculated ratios</div>
                </div>
              </div>
            </div>

            {/* Data Coverage Information */}
            <div className="bg-gray-50 rounded-lg p-4 border border-gray-200">
              <h4 className="font-medium text-gray-900 mb-2">📋 Data Coverage</h4>
              <ul className="text-sm text-gray-600 space-y-1 list-disc list-inside">
                <li><strong>Time Period:</strong> October 2019 to September 2024 (5 years)</li>
                <li><strong>Market Coverage:</strong> 5,876+ US stocks from SimFin database</li>
                <li><strong>Daily Metrics:</strong> OHLCV prices, calculated P/E ratios, shares outstanding</li>
                <li><strong>Quarterly Financials:</strong> Net income, diluted shares, calculated EPS</li>
                <li><strong>Data Quality:</strong> Comprehensive historical dataset with no API rate limits</li>
              </ul>
            </div>

            {/* Import Tool Information */}
            <div className="bg-blue-50 rounded-lg p-4 border border-blue-200">
              <h4 className="font-medium text-blue-900 mb-2">🔧 Import Tool Available</h4>
              <div className="text-sm text-blue-700">
                <p className="mb-2">
                  To import fresh SimFin data, use the command-line import tool:
                </p>
                <div className="bg-blue-900 text-blue-100 p-3 rounded font-mono text-xs overflow-x-auto">
                  cargo run --bin import_simfin -- <br/>
                  &nbsp;&nbsp;--prices ~/simfin_data/us-shareprices-daily.csv <br/>
                  &nbsp;&nbsp;--income ~/simfin_data/us-income-quarterly.csv <br/>
                  &nbsp;&nbsp;--db ./stocks.db
                </div>
              </div>
            </div>

            {/* Data Source Attribution */}
            <div className="bg-yellow-50 rounded-lg p-4 border border-yellow-200">
              <div className="text-xs text-yellow-800">
                <strong>📊 Data Source:</strong> SimFin - High-quality financial data for 5,000+ companies.
                <br />
                <strong>🧮 EPS Calculations:</strong> Net Income ÷ Diluted Shares Outstanding from quarterly reports.
                <br />
                <strong>📈 P/E Ratios:</strong> Daily closing price ÷ latest available EPS for each trading day.
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default DataFetchingPanel;