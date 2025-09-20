import { Show } from 'solid-js';
import { dataRefreshStore } from '../stores/dataRefreshStore';

interface RefreshButtonProps {
  mode: 'market' | 'financials' | 'ratios';
  title: string;
  duration: string;
  icon: string;
  description: string;
}

function RefreshButton(props: RefreshButtonProps) {
  const isDisabled = () => dataRefreshStore.isRefreshing();

  const handleRefresh = async () => {
    await dataRefreshStore.startRefresh(props.mode);
  };

  return (
    <div class="bg-white border rounded-lg p-4 hover:shadow-md transition-shadow">
      <div class="flex items-center mb-3">
        <span class="text-2xl mr-3">{props.icon}</span>
        <div>
          <h4 class="font-medium text-gray-900">{props.title}</h4>
          <p class="text-xs text-gray-500">{props.description}</p>
        </div>
      </div>

      <div class="mb-3">
        <div class="flex items-center justify-between text-sm">
          <span class="text-gray-600">Duration:</span>
          <span class="font-medium text-gray-900">{props.duration}</span>
        </div>
      </div>

      <button
        onClick={handleRefresh}
        disabled={isDisabled()}
        class={`w-full px-4 py-2 text-sm font-medium rounded-lg transition-colors ${
          isDisabled()
            ? 'bg-gray-100 text-gray-400 cursor-not-allowed'
            : 'bg-blue-600 hover:bg-blue-700 text-white'
        }`}
      >
        <Show
          when={!isDisabled()}
          fallback={
            <span class="flex items-center justify-center">
              <div class="animate-spin rounded-full h-4 w-4 border-2 border-gray-400 border-t-transparent mr-2"></div>
              Refreshing...
            </span>
          }
        >
          🔄 Refresh
        </Show>
      </button>
    </div>
  );
}

interface BulkRefreshButtonProps {
  type: 'quick' | 'full';
  title: string;
  duration: string;
  description: string;
  icon: string;
  bgColor: string;
  hoverColor: string;
}

function BulkRefreshButton(props: BulkRefreshButtonProps) {
  const isDisabled = () => dataRefreshStore.isRefreshing();

  const handleBulkRefresh = async () => {
    await dataRefreshStore.startBulkRefresh(props.type);
  };

  return (
    <button
      onClick={handleBulkRefresh}
      disabled={isDisabled()}
      class={`flex-1 px-6 py-4 rounded-lg font-medium transition-colors ${
        isDisabled()
          ? 'bg-gray-100 text-gray-400 cursor-not-allowed'
          : `${props.bgColor} ${props.hoverColor} text-white`
      }`}
    >
      <Show
        when={!isDisabled()}
        fallback={
          <span class="flex items-center justify-center">
            <div class="animate-spin rounded-full h-4 w-4 border-2 border-white border-t-transparent mr-2"></div>
            Refreshing...
          </span>
        }
      >
        <div class="text-center">
          <div class="text-lg mb-1">{props.icon} {props.title}</div>
          <div class="text-sm opacity-90">{props.description}</div>
          <div class="text-xs opacity-75 mt-1">{props.duration}</div>
        </div>
      </Show>
    </button>
  );
}

export default function RefreshControls() {
  const durationEstimates = dataRefreshStore.durationEstimates();

  // Format duration estimates from the backend
  const getDuration = (type: 'market' | 'financials' | 'ratios'): string => {
    const minutes = durationEstimates?.[type];
    if (!minutes) return '~estimate loading';
    return dataRefreshStore.formatDuration(minutes);
  };

  return (
    <div class="space-y-6">
      {/* Individual Data Type Refreshes */}
      <div class="bg-white rounded-lg p-6 shadow-sm border">
        <h3 class="text-lg font-semibold text-gray-900 mb-4 flex items-center">
          <span class="mr-2">🎛️</span>
          Individual Data Refreshes
        </h3>
        <p class="text-sm text-gray-600 mb-6">
          Refresh specific data types independently. Each operation focuses on a single data source.
        </p>

        <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
          <RefreshButton
            mode="market"
            title="Market Data"
            duration={getDuration('market')}
            icon="📈"
            description="Daily prices, shares, market cap"
          />
          <RefreshButton
            mode="financials"
            title="Financial Data"
            duration={getDuration('financials')}
            icon="📋"
            description="Income, balance, cash flow statements"
          />
          <RefreshButton
            mode="ratios"
            title="Calculated Ratios"
            duration={getDuration('ratios')}
            icon="🧮"
            description="P/E, P/S, screening ratios"
          />
        </div>
      </div>

      {/* Bulk Operations */}
      <div class="bg-white rounded-lg p-6 shadow-sm border">
        <h3 class="text-lg font-semibold text-gray-900 mb-4 flex items-center">
          <span class="mr-2">⚡</span>
          Bulk Operations
        </h3>
        <p class="text-sm text-gray-600 mb-6">
          Comprehensive refresh operations that update multiple data types in sequence.
        </p>

        <div class="flex flex-col sm:flex-row gap-4">
          <BulkRefreshButton
            type="quick"
            title="Quick Refresh"
            description="Market data + calculated ratios"
            duration={`~${((durationEstimates?.market || 15) + (durationEstimates?.ratios || 10))} min`}
            icon="⚡"
            bgColor="bg-blue-600"
            hoverColor="hover:bg-blue-700"
          />
          <BulkRefreshButton
            type="full"
            title="Full Refresh"
            description="All data types (Market + Financials + Ratios)"
            duration={`~${((durationEstimates?.market || 15) + (durationEstimates?.financials || 90) + (durationEstimates?.ratios || 10))} min`}
            icon="🔋"
            bgColor="bg-purple-600"
            hoverColor="hover:bg-purple-700"
          />
        </div>

        <div class="mt-4 p-4 bg-yellow-50 border border-yellow-200 rounded-lg">
          <h4 class="font-medium text-yellow-800 mb-2 flex items-center">
            <span class="mr-2">⚠️</span>
            Important Notes
          </h4>
          <ul class="text-sm text-yellow-700 space-y-1">
            <li>• Refresh operations cannot be paused, only cancelled</li>
            <li>• Financial data refresh takes the longest (~90 minutes)</li>
            <li>• Market data should be refreshed daily for current screening</li>
            <li>• Ratios require both market and financial data to be current</li>
          </ul>
        </div>
      </div>

      {/* Advanced Options */}
      <div class="bg-white rounded-lg p-6 shadow-sm border">
        <h3 class="text-lg font-semibold text-gray-900 mb-4 flex items-center">
          <span class="mr-2">🔧</span>
          Advanced Options
        </h3>

        <div class="space-y-4">
          {/* Force Refresh Toggle */}
          <div class="flex items-center justify-between p-4 bg-gray-50 rounded-lg">
            <div>
              <h4 class="font-medium text-gray-900">Force Refresh</h4>
              <p class="text-sm text-gray-600">
                Skip freshness checks and force complete data reload
              </p>
            </div>
            <label class="flex items-center">
              <input
                type="checkbox"
                class="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                disabled={dataRefreshStore.isRefreshing()}
              />
              <span class="ml-2 text-sm text-gray-700">Force</span>
            </label>
          </div>

          {/* Auto-refresh Settings */}
          <div class="flex items-center justify-between p-4 bg-gray-50 rounded-lg">
            <div>
              <h4 class="font-medium text-gray-900">Auto-refresh Market Data</h4>
              <p class="text-sm text-gray-600">
                Automatically refresh market data when it becomes stale
              </p>
            </div>
            <label class="flex items-center">
              <input
                type="checkbox"
                class="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                checked
                disabled
              />
              <span class="ml-2 text-sm text-gray-700">Enabled</span>
            </label>
          </div>
        </div>
      </div>

      {/* Current Operation Status */}
      <Show when={dataRefreshStore.isRefreshing()}>
        <div class="bg-blue-50 border border-blue-200 rounded-lg p-6">
          <h3 class="font-medium text-blue-900 mb-3 flex items-center">
            <span class="mr-2">🔄</span>
            Operation in Progress
          </h3>
          <p class="text-sm text-blue-700 mb-4">
            A refresh operation is currently running. You can monitor progress below or cancel if needed.
          </p>
          <div class="flex justify-between items-center">
            <span class="text-sm text-blue-600">
              Session: {dataRefreshStore.currentSessionId()?.slice(0, 8)}...
            </span>
            <button
              onClick={() => dataRefreshStore.cancelRefresh()}
              class="px-4 py-2 text-sm font-medium text-red-600 hover:text-red-700 border border-red-300 hover:border-red-400 rounded-lg transition-colors"
            >
              🚫 Cancel Operation
            </button>
          </div>
        </div>
      </Show>
    </div>
  );
}