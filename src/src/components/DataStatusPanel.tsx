import { Show, For } from 'solid-js';
import { dataRefreshStore } from '../stores/dataRefreshStore';
import type { DataTypeStatus } from '../utils/types';

interface DataTypeCardProps {
  title: string;
  status: DataTypeStatus | undefined;
  icon: string;
  dataType: 'market' | 'financials' | 'ratios';
}

function DataTypeCard(props: DataTypeCardProps) {
  const getStatusBadge = (status: DataTypeStatus | undefined) => {
    if (!status) {
      return <span class="px-2 py-1 text-xs font-medium bg-gray-100 text-gray-600 rounded-full">Unknown</span>;
    }

    if (status.is_fresh) {
      return <span class="px-2 py-1 text-xs font-medium bg-green-100 text-green-800 rounded-full">Fresh</span>;
    } else if (status.hours_since_update < status.freshness_threshold_hours * 2) {
      return <span class="px-2 py-1 text-xs font-medium bg-yellow-100 text-yellow-800 rounded-full">Stale</span>;
    } else {
      return <span class="px-2 py-1 text-xs font-medium bg-red-100 text-red-800 rounded-full">Critical</span>;
    }
  };

  const formatTimeAgo = (hoursAgo: number): string => {
    if (hoursAgo < 1) {
      return 'Less than 1 hour ago';
    } else if (hoursAgo < 24) {
      return `${Math.floor(hoursAgo)} hours ago`;
    } else {
      const days = Math.floor(hoursAgo / 24);
      return `${days} day${days > 1 ? 's' : ''} ago`;
    }
  };

  return (
    <div class="bg-white rounded-lg p-6 shadow-sm border hover:shadow-md transition-shadow">
      <div class="flex items-center justify-between mb-4">
        <div class="flex items-center">
          <span class="text-2xl mr-3">{props.icon}</span>
          <h3 class="text-lg font-semibold text-gray-900">{props.title}</h3>
        </div>
        {getStatusBadge(props.status)}
      </div>

      <Show when={props.status} fallback={
        <div class="text-sm text-gray-500">Status information unavailable</div>
      }>
        <div class="space-y-3">
          <div class="text-sm text-gray-600">
            <div class="flex justify-between">
              <span>Last Updated:</span>
              <span class="font-medium">
                {formatTimeAgo(props.status?.hours_since_update || 0)}
              </span>
            </div>
          </div>

          <div class="text-sm text-gray-600">
            <div class="flex justify-between">
              <span>Next Refresh:</span>
              <span class="font-medium">
                {props.status?.next_recommended_refresh ?
                  new Date(props.status.next_recommended_refresh).toLocaleDateString() :
                  'Not scheduled'
                }
              </span>
            </div>
          </div>

          <div class="pt-3 border-t">
            <button
              onClick={() => dataRefreshStore.startRefresh(props.dataType)}
              disabled={dataRefreshStore.isRefreshing()}
              class={`w-full px-4 py-2 text-sm font-medium rounded-lg transition-colors ${
                dataRefreshStore.isRefreshing()
                  ? 'bg-gray-100 text-gray-400 cursor-not-allowed'
                  : 'bg-blue-600 hover:bg-blue-700 text-white'
              }`}
            >
              <Show
                when={!dataRefreshStore.isRefreshing()}
                fallback={
                  <span class="flex items-center justify-center">
                    <div class="animate-spin rounded-full h-4 w-4 border-2 border-gray-400 border-t-transparent mr-2"></div>
                    Refreshing...
                  </span>
                }
              >
                🔄 Refresh {props.title}
              </Show>
            </button>
          </div>
        </div>
      </Show>
    </div>
  );
}

interface ScreeningReadinessProps {
  readiness: {
    garp_screening: boolean;
    graham_screening: boolean;
    piotroski_screening: boolean;
    oshaughnessy_screening: boolean;
    blocking_issues: string[];
  } | undefined;
}

function ScreeningReadinessIndicator(props: ScreeningReadinessProps) {
  const features = [
    { key: 'garp_screening', name: 'GARP Screening', icon: '🎯' },
    { key: 'graham_screening', name: 'Graham Screening', icon: '💎' },
    { key: 'piotroski_screening', name: 'Piotroski Screening', icon: '📊' },
    { key: 'oshaughnessy_screening', name: "O'Shaughnessy Screening", icon: '📈' },
  ];

  return (
    <div class="bg-gray-50 rounded-lg p-6">
      <h4 class="font-medium text-gray-900 mb-4 flex items-center">
        <span class="mr-2">🛠️</span>
        Screening Feature Status
      </h4>

      <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 mb-4">
        <For each={features}>
          {(feature) => {
            const isReady = props.readiness?.[feature.key as keyof typeof props.readiness] as boolean;
            return (
              <div class="flex items-center justify-between p-3 bg-white rounded-lg border">
                <div class="flex items-center">
                  <span class="text-lg mr-2">{feature.icon}</span>
                  <span class="text-sm font-medium text-gray-700">{feature.name}</span>
                </div>
                <span class={`px-2 py-1 text-xs font-medium rounded-full ${
                  isReady
                    ? 'bg-green-100 text-green-800'
                    : 'bg-red-100 text-red-800'
                }`}>
                  {isReady ? 'Ready' : 'Not Ready'}
                </span>
              </div>
            );
          }}
        </For>
      </div>

      <Show when={props.readiness?.blocking_issues?.length}>
        <div class="bg-yellow-50 border border-yellow-200 rounded-lg p-4">
          <h5 class="font-medium text-yellow-800 mb-2">⚠️ Blocking Issues:</h5>
          <ul class="text-sm text-yellow-700 space-y-1">
            <For each={props.readiness?.blocking_issues}>
              {(issue) => <li>• {issue}</li>}
            </For>
          </ul>
        </div>
      </Show>
    </div>
  );
}

export default function DataStatusPanel() {
  const status = dataRefreshStore.freshnessStatus();

  return (
    <div class="space-y-6">
      {/* Overall System Status */}
      <div class="bg-white rounded-lg p-6 shadow-sm border">
        <div class="flex items-center justify-between mb-4">
          <h2 class="text-xl font-semibold text-gray-900">📊 Data Freshness Status</h2>
          <button
            onClick={() => dataRefreshStore.checkDataFreshness()}
            class="text-sm text-blue-600 hover:text-blue-700 font-medium"
          >
            🔄 Refresh Status
          </button>
        </div>

        <Show when={status} fallback={
          <div class="text-center py-8">
            <div class="animate-spin rounded-full h-8 w-8 border-2 border-blue-600 border-t-transparent mx-auto mb-4"></div>
            <p class="text-gray-600">Loading data status...</p>
          </div>
        }>
          <div class="mb-6">
            <div class="flex items-center">
              <span class="text-sm font-medium text-gray-700 mr-3">Overall System Status:</span>
              <span class={`px-3 py-1 text-sm font-medium rounded-full ${
                status?.overall_status === 'fresh' ? 'bg-green-100 text-green-800' :
                status?.overall_status === 'stale' ? 'bg-yellow-100 text-yellow-800' :
                'bg-red-100 text-red-800'
              }`}>
                {status?.overall_status === 'fresh' ? '🟢 Fresh' :
                 status?.overall_status === 'stale' ? '🟡 Stale' :
                 '🔴 Critical'}
              </span>
            </div>
          </div>
        </Show>
      </div>

      {/* Individual Data Type Status */}
      <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
        <DataTypeCard
          title="Market Data"
          status={status?.market_data}
          icon="📈"
          dataType="market"
        />
        <DataTypeCard
          title="Financial Data"
          status={status?.financial_data}
          icon="📋"
          dataType="financials"
        />
        <DataTypeCard
          title="Calculated Ratios"
          status={status?.calculated_ratios}
          icon="🧮"
          dataType="ratios"
        />
      </div>

      {/* Screening Feature Readiness */}
      <ScreeningReadinessIndicator readiness={status?.screening_readiness} />

      {/* Error Display */}
      <Show when={dataRefreshStore.error()}>
        <div class="bg-red-50 border border-red-200 rounded-lg p-4">
          <h4 class="font-medium text-red-800 mb-2">❌ Error</h4>
          <p class="text-sm text-red-700">{dataRefreshStore.error()}</p>
        </div>
      </Show>
    </div>
  );
}