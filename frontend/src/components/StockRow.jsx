import { useState } from 'react';
import ExpandablePanel from './ExpandablePanel';
import AnalysisPanel from './AnalysisPanel';
import DataFetchingPanel from './DataFetchingPanel';

function StockRow({ stock, isExpanded, expandedPanel, onToggleExpansion }) {
  const getDataStatusIcon = (stock) => {
    if (stock.has_data === true) return '📊';
    if (stock.has_data === false) return '📋';
    return '🔍';
  };

  const getDataStatusText = (stock) => {
    if (stock.has_data === true) return `${stock.data_count || 0} records`;
    if (stock.has_data === false) return 'No data';
    return 'Checking...';
  };

  const handleAnalyzeClick = () => {
    onToggleExpansion(stock.id || stock.symbol, expandedPanel === 'analysis' ? null : 'analysis');
  };

  const handleFetchClick = () => {
    onToggleExpansion(stock.id || stock.symbol, expandedPanel === 'fetch' ? null : 'fetch');
  };

  return (
    <div className="bg-white border border-gray-200 rounded-lg shadow-sm mb-2 overflow-hidden">
      {/* Stock Summary Row */}
      <div className="flex items-center justify-between p-4 hover:bg-gray-50">
        <div className="flex items-center space-x-4 flex-1">
          {/* Stock Info */}
          <div className="flex items-center space-x-3">
            <div className="text-lg font-bold text-gray-900">
              {stock.symbol}
            </div>
            <div className="text-sm text-gray-600">
              {stock.company_name}
            </div>
            {stock.sector && (
              <div className="px-2 py-1 bg-blue-100 text-blue-800 text-xs rounded-full">
                {stock.sector}
              </div>
            )}
          </div>

          {/* Data Status */}
          <div className="flex items-center space-x-2 text-sm text-gray-600">
            <span className="text-base">{getDataStatusIcon(stock)}</span>
            <span>{getDataStatusText(stock)}</span>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex items-center space-x-2">
          <button
            onClick={handleAnalyzeClick}
            className={`flex items-center space-x-1 px-3 py-2 text-sm rounded-lg transition-colors ${
              expandedPanel === 'analysis'
                ? 'bg-blue-100 text-blue-700 border border-blue-300'
                : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
            }`}
          >
            <span>📊</span>
            <span>{expandedPanel === 'analysis' ? 'Close' : 'Analyze'}</span>
          </button>
          
          <button
            onClick={handleFetchClick}
            className={`flex items-center space-x-1 px-3 py-2 text-sm rounded-lg transition-colors ${
              expandedPanel === 'fetch'
                ? 'bg-green-100 text-green-700 border border-green-300'
                : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
            }`}
          >
            <span>📥</span>
            <span>{expandedPanel === 'fetch' ? 'Close' : 'Fetch'}</span>
          </button>
        </div>
      </div>

      {/* Expandable Panel */}
      <ExpandablePanel isExpanded={isExpanded}>
        <div className="border-t border-gray-200 bg-gray-50 p-4">
          {expandedPanel === 'analysis' && (
            <AnalysisPanel stock={stock} />
          )}
          {expandedPanel === 'fetch' && (
            <DataFetchingPanel stock={stock} />
          )}
        </div>
      </ExpandablePanel>
    </div>
  );
}

export default StockRow;