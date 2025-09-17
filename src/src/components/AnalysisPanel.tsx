import { createSignal, onMount, Show } from 'solid-js';
import type { Stock, PriceData, ValuationRatios, DateRange } from '../utils/types';
import { analysisAPI } from '../services/api';

interface AnalysisPanelProps {
  stock: Stock;
}

export default function AnalysisPanel(props: AnalysisPanelProps) {
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [dateRange, setDateRange] = createSignal<DateRange | null>(null);
  const [priceHistory, setPriceHistory] = createSignal<PriceData[]>([]);
  const [valuationRatios, setValuationRatios] = createSignal<ValuationRatios | null>(null);

  onMount(async () => {
    await loadAnalysisData();
  });

  const loadAnalysisData = async () => {
    setLoading(true);
    setError(null);

    try {
      // Load date range first
      const range = await analysisAPI.getStockDateRange(props.stock.symbol);
      setDateRange(range);

      // Load recent price history (last 30 days)
      const endDate = new Date().toISOString().split('T')[0];
      const startDate = new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString().split('T')[0];
      
      const [history, ratios] = await Promise.all([
        analysisAPI.getPriceHistory(props.stock.symbol, startDate, endDate),
        analysisAPI.getValuationRatios(props.stock.symbol)
      ]);

      setPriceHistory(history);
      setValuationRatios(ratios);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load analysis data');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div class="p-6 bg-gray-50">
      <div class="mb-4">
        <h3 class="text-lg font-semibold text-gray-900">
          Analysis for {props.stock.symbol}
        </h3>
        <Show when={props.stock.company_name}>
          <p class="text-sm text-gray-600">{props.stock.company_name}</p>
        </Show>
      </div>

      <Show when={loading()}>
        <div class="flex items-center justify-center py-8">
          <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 mr-3"></div>
          <span class="text-gray-600">Loading analysis data...</span>
        </div>
      </Show>

      <Show when={error()}>
        <div class="bg-red-50 border border-red-200 rounded-lg p-4">
          <div class="flex items-center">
            <svg class="w-5 h-5 text-red-400 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <span class="text-red-800">{error()}</span>
          </div>
        </div>
      </Show>

      <Show when={!loading() && !error()}>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          {/* Basic Info */}
          <div class="bg-white rounded-lg p-4 shadow-sm">
            <h4 class="font-medium text-gray-900 mb-3">Stock Information</h4>
            <div class="space-y-2 text-sm">
              <div class="flex justify-between">
                <span class="text-gray-600">Symbol:</span>
                <span class="font-medium">{props.stock.symbol}</span>
              </div>
              <Show when={props.stock.sector}>
                <div class="flex justify-between">
                  <span class="text-gray-600">Sector:</span>
                  <span class="font-medium">{props.stock.sector}</span>
                </div>
              </Show>
              <Show when={props.stock.market_cap}>
                <div class="flex justify-between">
                  <span class="text-gray-600">Market Cap:</span>
                  <span class="font-medium">${(props.stock.market_cap! / 1_000_000_000).toFixed(2)}B</span>
                </div>
              </Show>
              <Show when={dateRange()}>
                <div class="flex justify-between">
                  <span class="text-gray-600">Data Range:</span>
                  <span class="font-medium text-xs">
                    {dateRange()!.min_date} to {dateRange()!.max_date}
                  </span>
                </div>
              </Show>
            </div>
          </div>

          {/* Valuation Ratios */}
          <div class="bg-white rounded-lg p-4 shadow-sm">
            <h4 class="font-medium text-gray-900 mb-3">Valuation Ratios</h4>
            <Show 
              when={valuationRatios()}
              fallback={<p class="text-sm text-gray-500">No valuation data available</p>}
            >
              <div class="space-y-2 text-sm">
                <Show when={valuationRatios()!.pe_ratio}>
                  <div class="flex justify-between">
                    <span class="text-gray-600">P/E Ratio:</span>
                    <span class="font-medium">{valuationRatios()!.pe_ratio!.toFixed(2)}</span>
                  </div>
                </Show>
                <Show when={valuationRatios()!.ps_ratio_ttm}>
                  <div class="flex justify-between">
                    <span class="text-gray-600">P/S Ratio (TTM):</span>
                    <span class="font-medium">{valuationRatios()!.ps_ratio_ttm!.toFixed(2)}</span>
                  </div>
                </Show>
                <Show when={valuationRatios()!.evs_ratio_ttm}>
                  <div class="flex justify-between">
                    <span class="text-gray-600">EV/S Ratio (TTM):</span>
                    <span class="font-medium">{valuationRatios()!.evs_ratio_ttm!.toFixed(2)}</span>
                  </div>
                </Show>
                <Show when={valuationRatios()!.market_cap}>
                  <div class="flex justify-between">
                    <span class="text-gray-600">Market Cap:</span>
                    <span class="font-medium">${(valuationRatios()!.market_cap! / 1_000_000_000).toFixed(2)}B</span>
                  </div>
                </Show>
              </div>
            </Show>
          </div>

          {/* Recent Price Data */}
          <div class="bg-white rounded-lg p-4 shadow-sm md:col-span-2">
            <h4 class="font-medium text-gray-900 mb-3">Recent Price History (Last 30 Days)</h4>
            <Show 
              when={priceHistory().length > 0}
              fallback={<p class="text-sm text-gray-500">No recent price data available</p>}
            >
              <div class="overflow-x-auto">
                <table class="min-w-full text-sm">
                  <thead class="bg-gray-50">
                    <tr>
                      <th class="px-3 py-2 text-left text-gray-600">Date</th>
                      <th class="px-3 py-2 text-right text-gray-600">Open</th>
                      <th class="px-3 py-2 text-right text-gray-600">High</th>
                      <th class="px-3 py-2 text-right text-gray-600">Low</th>
                      <th class="px-3 py-2 text-right text-gray-600">Close</th>
                      <th class="px-3 py-2 text-right text-gray-600">Volume</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-gray-200">
                    {priceHistory().slice(-10).map((price) => (
                      <tr class="hover:bg-gray-50">
                        <td class="px-3 py-2 text-gray-900">{price.date}</td>
                        <td class="px-3 py-2 text-right text-gray-900">${price.open_price.toFixed(2)}</td>
                        <td class="px-3 py-2 text-right text-gray-900">${price.high_price.toFixed(2)}</td>
                        <td class="px-3 py-2 text-right text-gray-900">${price.low_price.toFixed(2)}</td>
                        <td class="px-3 py-2 text-right font-medium text-gray-900">${price.close_price.toFixed(2)}</td>
                        <td class="px-3 py-2 text-right text-gray-600">
                          {price.volume ? price.volume.toLocaleString() : 'N/A'}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </Show>
          </div>
        </div>
      </Show>
    </div>
  );
}