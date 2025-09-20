import { Show } from 'solid-js';
import { dataRefreshStore } from '../stores/dataRefreshStore';

export default function RefreshProgress() {
  const progress = dataRefreshStore.refreshProgress();
  const lastResult = dataRefreshStore.lastRefreshResult();

  const formatElapsedTime = (minutes: number): string => {
    if (minutes < 1) {
      return 'Just started';
    } else if (minutes < 60) {
      return `${Math.floor(minutes)} min`;
    } else {
      const hours = Math.floor(minutes / 60);
      const remainingMinutes = Math.floor(minutes % 60);
      return `${hours}h ${remainingMinutes}m`;
    }
  };

  const getProgressColor = (percent: number): string => {
    if (percent < 25) return 'bg-red-500';
    if (percent < 50) return 'bg-yellow-500';
    if (percent < 75) return 'bg-blue-500';
    return 'bg-green-500';
  };

  const getStatusIcon = (status: string): string => {
    switch (status) {
      case 'running': return '🔄';
      case 'completed': return '✅';
      case 'failed': return '❌';
      case 'cancelled': return '🚫';
      default: return '⏳';
    }
  };

  return (
    <div class="space-y-6">
      {/* Current Progress */}
      <Show when={dataRefreshStore.isRefreshing() && progress()}>
        <div class="bg-blue-50 border border-blue-200 rounded-lg p-6">
          <div class="flex justify-between items-center mb-4">
            <h3 class="font-semibold text-blue-900 flex items-center">
              <span class="mr-2">{getStatusIcon(progress()?.status || '')}</span>
              {progress()?.operation_type} in Progress
            </h3>
            <button
              onClick={() => dataRefreshStore.cancelRefresh()}
              class="text-red-600 hover:text-red-700 text-sm font-medium px-3 py-1 border border-red-300 hover:border-red-400 rounded-lg transition-colors"
            >
              🚫 Cancel
            </button>
          </div>

          <div class="space-y-4">
            {/* Current Step */}
            <div class="flex justify-between text-sm">
              <span class="text-blue-700 font-medium">
                {progress()?.current_step_name || 'Processing...'}
              </span>
              <span class="text-blue-600">
                Step {progress()?.completed_steps} of {progress()?.total_steps}
              </span>
            </div>

            {/* Progress Bar */}
            <div class="space-y-2">
              <div class="flex justify-between text-sm text-blue-700">
                <span>Overall Progress</span>
                <span class="font-medium">{progress()?.overall_progress_percent.toFixed(1)}%</span>
              </div>
              <div class="w-full bg-blue-200 rounded-full h-3">
                <div
                  class={`h-3 rounded-full transition-all duration-300 ${getProgressColor(progress()?.overall_progress_percent || 0)}`}
                  style={`width: ${progress()?.overall_progress_percent}%`}
                />
              </div>
            </div>

            {/* Time Information */}
            <div class="grid grid-cols-1 sm:grid-cols-3 gap-4 pt-3 border-t border-blue-200">
              <div class="text-center">
                <div class="text-xs text-blue-600 uppercase tracking-wide">Elapsed</div>
                <div class="text-sm font-medium text-blue-900">
                  {formatElapsedTime(progress()?.elapsed_minutes || 0)}
                </div>
              </div>
              <div class="text-center">
                <div class="text-xs text-blue-600 uppercase tracking-wide">Started</div>
                <div class="text-sm font-medium text-blue-900">
                  {progress()?.start_time ?
                    new Date(progress()!.start_time).toLocaleTimeString() :
                    'Unknown'
                  }
                </div>
              </div>
              <div class="text-center">
                <div class="text-xs text-blue-600 uppercase tracking-wide">ETA</div>
                <div class="text-sm font-medium text-blue-900">
                  {progress()?.estimated_completion ?
                    new Date(progress()!.estimated_completion).toLocaleTimeString() :
                    'Calculating...'
                  }
                </div>
              </div>
            </div>

            {/* Session Information */}
            <div class="text-xs text-blue-600 bg-blue-100 rounded px-3 py-2">
              Session ID: {progress()?.session_id}
            </div>
          </div>
        </div>
      </Show>

      {/* Last Refresh Result */}
      <Show when={!dataRefreshStore.isRefreshing() && lastResult()}>
        <div class={`rounded-lg p-6 border ${
          lastResult()?.success
            ? 'bg-green-50 border-green-200'
            : 'bg-red-50 border-red-200'
        }`}>
          <h3 class={`font-semibold mb-3 flex items-center ${
            lastResult()?.success ? 'text-green-900' : 'text-red-900'
          }`}>
            <span class="mr-2">{lastResult()?.success ? '✅' : '❌'}</span>
            Last Refresh Result
          </h3>

          <div class="space-y-3">
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div>
                <div class={`text-xs uppercase tracking-wide ${
                  lastResult()?.success ? 'text-green-600' : 'text-red-600'
                }`}>
                  Operation
                </div>
                <div class={`text-sm font-medium ${
                  lastResult()?.success ? 'text-green-900' : 'text-red-900'
                }`}>
                  {lastResult()?.operation_type}
                </div>
              </div>
              <div>
                <div class={`text-xs uppercase tracking-wide ${
                  lastResult()?.success ? 'text-green-600' : 'text-red-600'
                }`}>
                  Duration
                </div>
                <div class={`text-sm font-medium ${
                  lastResult()?.success ? 'text-green-900' : 'text-red-900'
                }`}>
                  {formatElapsedTime(lastResult()?.duration_minutes || 0)}
                </div>
              </div>
              <div>
                <div class={`text-xs uppercase tracking-wide ${
                  lastResult()?.success ? 'text-green-600' : 'text-red-600'
                }`}>
                  Records Processed
                </div>
                <div class={`text-sm font-medium ${
                  lastResult()?.success ? 'text-green-900' : 'text-red-900'
                }`}>
                  {lastResult()?.total_records_processed.toLocaleString()}
                </div>
              </div>
              <div>
                <div class={`text-xs uppercase tracking-wide ${
                  lastResult()?.success ? 'text-green-600' : 'text-red-600'
                }`}>
                  Completed
                </div>
                <div class={`text-sm font-medium ${
                  lastResult()?.success ? 'text-green-900' : 'text-red-900'
                }`}>
                  {lastResult()?.end_time ?
                    new Date(lastResult()!.end_time).toLocaleString() :
                    'Unknown'
                  }
                </div>
              </div>
            </div>

            {/* Error Message */}
            <Show when={!lastResult()?.success && lastResult()?.error_message}>
              <div class="mt-3 p-3 bg-red-100 border border-red-200 rounded">
                <div class="text-xs text-red-600 uppercase tracking-wide mb-1">Error Details</div>
                <div class="text-sm text-red-800">{lastResult()?.error_message}</div>
              </div>
            </Show>

            {/* Success Summary */}
            <Show when={lastResult()?.success}>
              <div class="mt-3 p-3 bg-green-100 border border-green-200 rounded">
                <div class="text-xs text-green-600 uppercase tracking-wide mb-1">Summary</div>
                <div class="text-sm text-green-800">
                  Successfully processed {lastResult()?.total_records_processed.toLocaleString()} records
                  in {formatElapsedTime(lastResult()?.duration_minutes || 0)}
                </div>
              </div>
            </Show>

            {/* Session Information */}
            <div class={`text-xs rounded px-3 py-2 ${
              lastResult()?.success
                ? 'text-green-600 bg-green-100'
                : 'text-red-600 bg-red-100'
            }`}>
              Session ID: {lastResult()?.session_id}
            </div>
          </div>
        </div>
      </Show>

      {/* No Activity State */}
      <Show when={!dataRefreshStore.isRefreshing() && !lastResult()}>
        <div class="bg-gray-50 border border-gray-200 rounded-lg p-8 text-center">
          <div class="text-4xl mb-3">⏸️</div>
          <h3 class="font-medium text-gray-900 mb-2">No Active Operations</h3>
          <p class="text-sm text-gray-600">
            No data refresh operations are currently running.
            Use the refresh controls to start a new operation.
          </p>
        </div>
      </Show>

      {/* Quick Actions */}
      <Show when={!dataRefreshStore.isRefreshing()}>
        <div class="bg-white rounded-lg p-6 shadow-sm border">
          <h3 class="font-medium text-gray-900 mb-4 flex items-center">
            <span class="mr-2">⚡</span>
            Quick Actions
          </h3>
          <div class="flex flex-wrap gap-3">
            <button
              onClick={() => dataRefreshStore.startRefresh('market')}
              class="px-4 py-2 text-sm font-medium bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors"
            >
              📈 Refresh Market Data
            </button>
            <button
              onClick={() => dataRefreshStore.startBulkRefresh('quick')}
              class="px-4 py-2 text-sm font-medium bg-purple-600 hover:bg-purple-700 text-white rounded-lg transition-colors"
            >
              ⚡ Quick Refresh
            </button>
            <button
              onClick={() => dataRefreshStore.checkDataFreshness()}
              class="px-4 py-2 text-sm font-medium bg-gray-600 hover:bg-gray-700 text-white rounded-lg transition-colors"
            >
              🔍 Check Status
            </button>
          </div>
        </div>
      </Show>
    </div>
  );
}