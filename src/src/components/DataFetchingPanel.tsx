import { createSignal, onMount, Show } from 'solid-js';
import { systemAPI } from '../services/api';
import type { DatabaseStats, InitializationStatus } from '../utils/types';

interface DataFetchingPanelProps {
  onClose: () => void;
}

export default function DataFetchingPanel(props: DataFetchingPanelProps) {
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [dbStats, setDbStats] = createSignal<DatabaseStats | null>(null);
  const [initStatus, setInitStatus] = createSignal<InitializationStatus | null>(null);

  onMount(async () => {
    await loadSystemInfo();
  });

  const loadSystemInfo = async () => {
    setLoading(true);
    setError(null);

    try {
      const [stats, status] = await Promise.all([
        systemAPI.getDatabaseStats(),
        systemAPI.getInitializationStatus()
      ]);

      setDbStats(stats);
      setInitStatus(status);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load system information');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div class="bg-white rounded-lg shadow-lg border border-gray-200 overflow-hidden">
      {/* Header */}
      <div class="bg-purple-600 text-white p-4 sm:p-6 flex justify-between items-center">
        <div class="flex-1 min-w-0">
          <h2 class="text-xl sm:text-2xl font-bold truncate">Data & System Status</h2>
          <p class="text-purple-100 mt-1 text-sm sm:text-base truncate">
            Database statistics and system initialization status
          </p>
        </div>
        <button
          onClick={props.onClose}
          class="ml-4 p-2 hover:bg-purple-700 rounded-full transition-colors"
          title="Close data panel"
        >
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width={2} d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <div class="p-6">
        <Show when={loading()}>
          <div class="flex items-center justify-center py-8">
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-purple-600 mr-3"></div>
            <span class="text-gray-600">Loading system information...</span>
          </div>
        </Show>

        <Show when={error()}>
          <div class="bg-red-50 border border-red-200 rounded-lg p-4 mb-6">
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
            {/* Database Statistics */}
            <div class="bg-gray-50 rounded-lg p-4">
              <h3 class="text-lg font-semibold text-gray-900 mb-4">Database Statistics</h3>
              <Show when={dbStats()}>
                <div class="space-y-3">
                  <div class="flex justify-between items-center">
                    <span class="text-gray-600">Total Stocks:</span>
                    <span class="text-2xl font-bold text-blue-600">
                      {dbStats()!.total_stocks.toLocaleString()}
                    </span>
                  </div>
                  <div class="flex justify-between items-center">
                    <span class="text-gray-600">Stocks with Data:</span>
                    <span class="text-2xl font-bold text-green-600">
                      {dbStats()!.stocks_with_data.toLocaleString()}
                    </span>
                  </div>
                  <div class="flex justify-between items-center">
                    <span class="text-gray-600">Price Records:</span>
                    <span class="text-2xl font-bold text-purple-600">
                      {dbStats()!.total_price_records.toLocaleString()}
                    </span>
                  </div>
                  <Show when={dbStats()!.latest_data_date}>
                    <div class="flex justify-between items-center">
                      <span class="text-gray-600">Latest Data:</span>
                      <span class="text-sm font-medium text-gray-900">
                        {dbStats()!.latest_data_date}
                      </span>
                    </div>
                  </Show>
                  
                  {/* Data Coverage Percentage */}
                  <div class="mt-4">
                    <div class="flex justify-between text-sm text-gray-600 mb-1">
                      <span>Data Coverage</span>
                      <span>{((dbStats()!.stocks_with_data / dbStats()!.total_stocks) * 100).toFixed(1)}%</span>
                    </div>
                    <div class="w-full bg-gray-200 rounded-full h-2">
                      <div 
                        class="bg-green-500 h-2 rounded-full transition-all duration-300"
                        style={`width: ${(dbStats()!.stocks_with_data / dbStats()!.total_stocks) * 100}%`}
                      ></div>
                    </div>
                  </div>
                </div>
              </Show>
            </div>

            {/* System Status */}
            <div class="bg-gray-50 rounded-lg p-4">
              <h3 class="text-lg font-semibold text-gray-900 mb-4">System Status</h3>
              <Show when={initStatus()}>
                <div class="space-y-3">
                  <div class="flex items-center justify-between">
                    <span class="text-gray-600">Database Ready:</span>
                    <span class={`inline-flex items-center px-2 py-1 rounded-full text-xs font-medium ${
                      initStatus()!.database_ready 
                        ? 'bg-green-100 text-green-800' 
                        : 'bg-red-100 text-red-800'
                    }`}>
                      {initStatus()!.database_ready ? '✓ Ready' : '✗ Not Ready'}
                    </span>
                  </div>
                  
                  <div class="flex items-center justify-between">
                    <span class="text-gray-600">Stocks Loaded:</span>
                    <span class={`inline-flex items-center px-2 py-1 rounded-full text-xs font-medium ${
                      initStatus()!.stocks_loaded 
                        ? 'bg-green-100 text-green-800' 
                        : 'bg-red-100 text-red-800'
                    }`}>
                      {initStatus()!.stocks_loaded ? '✓ Loaded' : '✗ Not Loaded'}
                    </span>
                  </div>
                  
                  <div class="flex items-center justify-between">
                    <span class="text-gray-600">Price Data Available:</span>
                    <span class={`inline-flex items-center px-2 py-1 rounded-full text-xs font-medium ${
                      initStatus()!.price_data_available 
                        ? 'bg-green-100 text-green-800' 
                        : 'bg-red-100 text-red-800'
                    }`}>
                      {initStatus()!.price_data_available ? '✓ Available' : '✗ Not Available'}
                    </span>
                  </div>
                  
                  <Show when={initStatus()!.message}>
                    <div class="mt-4 p-3 bg-blue-50 rounded-lg">
                      <p class="text-sm text-blue-800">{initStatus()!.message}</p>
                    </div>
                  </Show>
                </div>
              </Show>
            </div>
          </div>

          {/* Refresh Button */}
          <div class="mt-6 text-center">
            <button
              onClick={loadSystemInfo}
              class="bg-purple-600 text-white px-6 py-2 rounded-lg hover:bg-purple-700 transition-colors"
            >
              Refresh System Info
            </button>
          </div>
        </Show>
      </div>
    </div>
  );
}