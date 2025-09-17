import { Show } from 'solid-js';
import type { Stock } from '../utils/types';
import AnalysisPanel from './AnalysisPanel';

interface StockRowProps {
  stock: Stock;
  isExpanded: boolean;
  expandedPanel?: string;
  onToggleExpansion: (panelType?: string) => void;
}

export default function StockRow(props: StockRowProps) {
  const handleExpansionToggle = (panelType: string) => {
    props.onToggleExpansion(panelType);
  };

  return (
    <div class="bg-white rounded-lg shadow-sm border border-gray-200 overflow-hidden">
      {/* Stock Header */}
      <div class="p-4 hover:bg-gray-50 transition-colors">
        <div class="flex items-center justify-between">
          {/* Stock Info */}
          <div class="flex-1">
            <div class="flex items-center gap-3">
              <span class="text-lg font-bold text-blue-600">{props.stock.symbol}</span>
              <Show when={props.stock.company_name && props.stock.company_name !== props.stock.symbol}>
                <span class="text-gray-600">{props.stock.company_name}</span>
              </Show>
              <Show when={props.stock.has_data}>
                <span title="Has data available">📊</span>
              </Show>
              <Show when={!props.stock.has_data}>
                <span title="No data available">📋</span>
              </Show>
            </div>
            <Show when={props.stock.sector || props.stock.industry}>
              <div class="text-sm text-gray-500 mt-1">
                {props.stock.sector && (
                  <span class="bg-gray-100 px-2 py-1 rounded mr-2">{props.stock.sector}</span>
                )}
                {props.stock.industry && (
                  <span class="bg-gray-100 px-2 py-1 rounded">{props.stock.industry}</span>
                )}
              </div>
            </Show>
          </div>

          {/* Market Cap */}
          <Show when={props.stock.market_cap}>
            <div class="text-right mr-4">
              <div class="text-sm text-gray-500">Market Cap</div>
              <div class="text-lg font-semibold text-gray-900">
                ${(props.stock.market_cap! / 1_000_000_000).toFixed(1)}B
              </div>
            </div>
          </Show>

          {/* Action Buttons */}
          <div class="flex items-center gap-2">
            <Show when={props.stock.has_data}>
              <button
                onClick={() => handleExpansionToggle('analysis')}
                class={`px-3 py-1 rounded text-sm font-medium transition-colors ${
                  props.isExpanded && props.expandedPanel === 'analysis'
                    ? 'bg-blue-600 text-white'
                    : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                }`}
              >
                {props.isExpanded && props.expandedPanel === 'analysis' ? 'Hide Analysis' : 'Analyze'}
              </button>
            </Show>
            
            <Show when={!props.stock.has_data}>
              <span class="px-3 py-1 rounded text-sm bg-gray-100 text-gray-500">
                No Data
              </span>
            </Show>
          </div>
        </div>
      </div>

      {/* Expanded Panel */}
      <Show when={props.isExpanded && props.expandedPanel === 'analysis'}>
        <div class="border-t border-gray-200">
          <AnalysisPanel stock={props.stock} />
        </div>
      </Show>
    </div>
  );
}