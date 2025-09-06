import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

function DataFetchingPanel({ stock }) {
  const [fetchMode, setFetchMode] = useState('compact');
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState('');
  const [progress, setProgress] = useState(null);

  const handleFetchData = async () => {
    if (!stock?.symbol) return;

    setLoading(true);
    setMessage('');
    setProgress(null);

    try {
      const result = await invoke('fetch_stock_data_comprehensive', {
        symbol: stock.symbol,
        fetchMode: fetchMode
      });

      setMessage(`✅ ${result}`);
    } catch (error) {
      console.error('Fetch failed:', error);
      setMessage(`❌ Failed to fetch data: ${error}`);
    } finally {
      setLoading(false);
      setProgress(null);
    }
  };

  // Poll for progress updates while loading
  useEffect(() => {
    if (!loading || !stock?.id) return;

    const progressInterval = setInterval(async () => {
      try {
        const progressData = await invoke('get_processing_status_for_stock', { 
          stockId: stock.id 
        });
        setProgress(progressData);
      } catch (error) {
        console.error('Failed to get progress:', error);
      }
    }, 1000);

    return () => clearInterval(progressInterval);
  }, [loading, stock?.id]);

  const getFetchModeDescription = () => {
    switch (fetchMode) {
      case 'compact':
        return 'Latest 100 trading days (~4 months)';
      case 'full':
        return '20+ years of historical data';
      default:
        return '';
    }
  };

  const getEstimatedTime = () => {
    switch (fetchMode) {
      case 'compact':
        return '~10-15 seconds';
      case 'full':
        return '~20-30 seconds';
      default:
        return '';
    }
  };

  return (
    <div className="space-y-6">
      <div className="bg-white rounded-lg border p-6">
        <h3 className="text-lg font-semibold mb-4">Comprehensive Data Fetching</h3>
        
        <div className="space-y-4">
          {/* Fetch Mode Selection */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-3">
              Data Range
            </label>
            <div className="space-y-3">
              <label className="flex items-start space-x-3 cursor-pointer">
                <input 
                  type="radio" 
                  value="compact" 
                  checked={fetchMode === 'compact'}
                  onChange={(e) => setFetchMode(e.target.value)}
                  className="mt-1 text-blue-600 focus:ring-blue-500"
                />
                <div className="flex-1">
                  <div className="font-medium text-gray-900">Compact (Recommended)</div>
                  <div className="text-sm text-gray-500">{getFetchModeDescription()}</div>
                  <div className="text-xs text-gray-400">Estimated time: {getEstimatedTime()}</div>
                </div>
              </label>
              
              <label className="flex items-start space-x-3 cursor-pointer">
                <input 
                  type="radio" 
                  value="full" 
                  checked={fetchMode === 'full'}
                  onChange={(e) => setFetchMode(e.target.value)}
                  className="mt-1 text-blue-600 focus:ring-blue-500"
                />
                <div className="flex-1">
                  <div className="font-medium text-gray-900">Full Historical Data</div>
                  <div className="text-sm text-gray-500">20+ years of historical data</div>
                  <div className="text-xs text-gray-400">Estimated time: ~20-30 seconds</div>
                </div>
              </label>
            </div>
          </div>

          {/* Fetch Button */}
          <div className="pt-4">
            <button
              onClick={handleFetchData}
              disabled={loading}
              className="w-full bg-blue-600 text-white py-3 px-4 rounded-lg hover:bg-blue-700 disabled:bg-gray-400 font-medium transition-colors"
            >
              {loading ? (
                <div className="flex items-center justify-center space-x-2">
                  <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-white"></div>
                  <span>Fetching {fetchMode} data...</span>
                </div>
              ) : (
                `Fetch ${fetchMode === 'compact' ? 'Compact' : 'Full'} Data`
              )}
            </button>
          </div>

          {/* Progress Display */}
          {progress && (
            <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
              <div className="text-sm font-medium text-blue-900">Processing Status</div>
              <div className="text-sm text-blue-700 mt-1">
                Status: {progress.status}
              </div>
              {progress.records_processed > 0 && progress.total_records > 0 && (
                <div className="text-sm text-blue-600">
                  Progress: {progress.records_processed} / {progress.total_records}
                </div>
              )}
              {progress.error_message && (
                <div className="text-sm text-red-600 mt-1">
                  Error: {progress.error_message}
                </div>
              )}
            </div>
          )}

          {/* Status Message */}
          {message && (
            <div className={`p-4 rounded-lg text-sm whitespace-pre-line ${
              message.startsWith('✅') 
                ? 'bg-green-50 text-green-800 border border-green-200' 
                : 'bg-red-50 text-red-800 border border-red-200'
            }`}>
              {message}
            </div>
          )}

          {/* What will be fetched info */}
          <div className="bg-gray-50 rounded-lg p-4 border border-gray-200">
            <h4 className="font-medium text-gray-900 mb-2">What will be fetched:</h4>
            <ul className="text-sm text-gray-600 space-y-1 list-disc list-inside">
              <li><strong>Daily OHLCV data:</strong> Open, High, Low, Close, Volume for each trading day</li>
              <li><strong>Quarterly earnings:</strong> EPS data for P/E ratio calculations</li>
              <li><strong>Daily P/E ratios:</strong> Calculated for every trading day using latest available EPS</li>
              <li><strong>Data quality metrics:</strong> Coverage statistics and validation</li>
            </ul>
            
            <div className="mt-3 p-3 bg-yellow-50 border border-yellow-200 rounded-md">
              <div className="text-xs text-yellow-800">
                <strong>🌟 Alpha Vantage API:</strong> Real market data with comprehensive historical coverage.
                <br />
                <strong>📊 Smart P/E Calculation:</strong> Uses quarterly EPS data to calculate accurate daily P/E ratios.
              </div>
            </div>
          </div>

          {/* Rate limiting info */}
          <div className="text-xs text-gray-500">
            <div className="flex items-center space-x-2">
              <span>⚠️</span>
              <span>API rate limits apply. Data fetching respects 5 calls/minute limit.</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export default DataFetchingPanel;