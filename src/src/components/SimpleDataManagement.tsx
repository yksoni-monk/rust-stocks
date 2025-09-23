import { createSignal, createEffect, Show, For } from 'solid-js';
import { dataRefreshStore } from '../stores/dataRefreshStore';

interface DataCard {
  id: 'market' | 'financial' | 'ratios';
  title: string;
  icon: string;
  description: string;
}

const DATA_CARDS: DataCard[] = [
  {
    id: 'market',
    title: 'Market Data',
    icon: '📈',
    description: 'Daily prices, shares, market cap'
  },
  {
    id: 'financial',
    title: 'Financial Data',
    icon: '📋',
    description: 'Income, balance, cash flow statements'
  },
  {
    id: 'ratios',
    title: 'Ratios',
    icon: '🧮',
    description: 'P/E, P/S, screening ratios'
  }
];

interface CardStatusProps {
  cardId: 'market' | 'financial' | 'ratios';
  onRefresh: () => void;
}

function DataStatusCard(props: CardStatusProps) {
  const card = DATA_CARDS.find(c => c.id === props.cardId)!;

  // Map our card IDs to the data source keys from the backend
  const getDataSourceKey = (cardId: string) => {
    switch(cardId) {
      case 'market': return 'daily_prices';
      case 'financial': return 'financial_statements';
      case 'ratios': return 'ps_evs_ratios';
      default: return 'daily_prices';
    }
  };

  // Get reactive data - this will update when the store changes
  const freshnessData = () => dataRefreshStore.freshnessStatus();
  const isCardRefreshingForThis = () => {
    // Map our card ID to backend refresh mode for checking refresh state
    const refreshMode = props.cardId === 'financial' ? 'financials' : props.cardId;
    return dataRefreshStore.isCardRefreshing(refreshMode);
  };
  const dataSource = () => {
    const freshness = freshnessData();
    switch(props.cardId) {
      case 'market': return freshness?.market_data;
      case 'financial': return freshness?.financial_data;
      case 'ratios': return freshness?.calculated_ratios;
      default: return null;
    }
  };

  // Determine status and styling - made reactive
  const statusInfo = () => {
    const freshness = freshnessData();
    const currentDataSource = dataSource();

    // If freshness data exists but this specific data source is missing
    if (freshness && !currentDataSource) {
      return {
        status: '❌ No Data',
        statusColor: 'text-red-800',
        bgColor: 'bg-red-50',
        borderColor: 'border-red-200',
        actionText: '⚡ Import Data',
        actionDisabled: false,
        actionColor: 'bg-red-600 hover:bg-red-700 text-white'
      };
    }

    // If no freshness data at all - show disabled state
    if (!currentDataSource) {
      return {
        status: '❓ Unknown',
        statusColor: 'text-gray-700',
        bgColor: 'bg-gray-50',
        borderColor: 'border-gray-300',
        actionText: 'Use Check Status above',
        actionDisabled: true,
        actionColor: 'bg-gray-100 text-gray-400'
      };
    }

    const staleness = currentDataSource.staleness_days ? Number(currentDataSource.staleness_days) : 0;

    if (currentDataSource.status === 'Current' && staleness <= 2) {
      return {
        status: '✅ Fresh',
        statusColor: 'text-green-800',
        bgColor: 'bg-green-50',
        borderColor: 'border-green-200',
        actionText: '✓ Ready',
        actionDisabled: true,
        actionColor: 'bg-green-100 text-green-600'
      };
    } else if (staleness <= 7) {
      return {
        status: '⚠️ Stale',
        statusColor: 'text-yellow-800',
        bgColor: 'bg-yellow-50',
        borderColor: 'border-yellow-200',
        actionText: '🔄 Refresh',
        actionDisabled: false,
        actionColor: 'bg-yellow-600 hover:bg-yellow-700 text-white'
      };
    } else {
      return {
        status: '❌ Critical',
        statusColor: 'text-red-800',
        bgColor: 'bg-red-50',
        borderColor: 'border-red-200',
        actionText: '⚡ Update',
        actionDisabled: false,
        actionColor: 'bg-red-600 hover:bg-red-700 text-white'
      };
    }
  };


  const formatMetrics = () => {
    const freshness = freshnessData();
    const currentDataSource = dataSource();

    // If freshness exists but this data source is missing
    if (freshness && !currentDataSource) {
      return {
        primary: 'No data imported yet',
        secondary: 'Click refresh to import data',
        details: []
      };
    }

    // If no status check done yet
    if (!currentDataSource) {
      return {
        primary: 'Click button to check status',
        secondary: 'Data status unknown',
        details: []
      };
    }

    const dataSummary = currentDataSource.data_summary;
    const staleness = currentDataSource.staleness_days ? Number(currentDataSource.staleness_days) : 0;
    const timeStr = staleness === 0 ? 'Today' :
                   staleness === 1 ? '1 day ago' :
                   staleness < 7 ? `${staleness} days ago` :
                   staleness < 30 ? `${Math.floor(staleness/7)} weeks ago` :
                   `${Math.floor(staleness/30)} months ago`;

    // Enhanced display using data summary
    const primary = dataSummary?.stock_count
      ? `${dataSummary.stock_count} stocks`
      : `${currentDataSource.records_count} records`;

    const secondary = dataSummary?.date_range
      ? `${dataSummary.date_range} • ${timeStr}`
      : timeStr;

    const details = [
      ...(dataSummary?.data_types || []),
      ...(dataSummary?.key_metrics || [])
    ];

    return { primary, secondary, details };
  };

  return (
    <div class={`rounded-lg p-6 border-2 transition-all duration-200 hover:shadow-md ${statusInfo().bgColor} ${statusInfo().borderColor}`}>
      {/* Header */}
      <div class="flex items-center mb-4">
        <span class="text-3xl mr-3">{card.icon}</span>
        <div>
          <h3 class="text-lg font-semibold text-gray-900">{card.title}</h3>
          <p class="text-sm text-gray-600">{card.description}</p>
        </div>
      </div>

      {/* Status */}
      <div class="mb-4">
        <div class={`text-lg font-medium mb-1 ${statusInfo().statusColor}`}>
          {statusInfo().status}
        </div>
        <div class="text-sm text-gray-600 space-y-1">
          <div class="font-medium">{formatMetrics().primary}</div>
          <div class="text-xs text-gray-500">{formatMetrics().secondary}</div>
          {formatMetrics().details.length > 0 && (
            <div class="text-xs text-gray-500">
              <div class="flex flex-wrap gap-1 mt-1">
                {formatMetrics().details.slice(0, 4).map((detail, index) => (
                  <span key={index} class="bg-gray-100 px-2 py-0.5 rounded text-xs">
                    {detail}
                  </span>
                ))}
                {formatMetrics().details.length > 4 && (
                  <span class="text-gray-400">+{formatMetrics().details.length - 4} more</span>
                )}
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Action Button */}
      <button
        onClick={props.onRefresh}
        disabled={statusInfo().actionDisabled || isCardRefreshingForThis()}
        class={`w-full px-4 py-2 text-sm font-medium rounded-lg transition-colors ${statusInfo().actionColor} ${statusInfo().actionDisabled ? 'cursor-not-allowed' : 'cursor-pointer'}`}
      >
        {isCardRefreshingForThis() ? '🔄 Refreshing...' : statusInfo().actionText}
      </button>
    </div>
  );
}

export default function SimpleDataManagement() {
  const [selectedCards, setSelectedCards] = createSignal<Set<string>>(new Set());
  const [showMultiSelect, setShowMultiSelect] = createSignal(false);
  const [isChecking, setIsChecking] = createSignal(false);

  // Manual check function - only called when user clicks button
  const handleCheckStatus = async () => {
    console.log('🔄 User clicked Check Status');
    setIsChecking(true);
    try {
      await dataRefreshStore.checkDataFreshness();
    } finally {
      setIsChecking(false);
    }
  };

  const handleCardRefresh = (cardId: 'market' | 'financial' | 'ratios') => {
    // Map card IDs to backend refresh modes
    const refreshMode = cardId === 'financial' ? 'financials' : cardId;
    dataRefreshStore.startRefresh(refreshMode as 'market' | 'financials' | 'ratios');
  };

  const handleCardSelection = (cardId: string, selected: boolean) => {
    const newSelected = new Set(selectedCards());
    if (selected) {
      newSelected.add(cardId);
    } else {
      newSelected.delete(cardId);
    }
    setSelectedCards(newSelected);
  };

  const handleBatchRefresh = () => {
    selectedCards().forEach(cardId => {
      handleCardRefresh(cardId as 'market' | 'financial' | 'ratios');
    });
    setSelectedCards(new Set());
    setShowMultiSelect(false);
  };

  const freshnessData = () => dataRefreshStore.freshnessStatus();

  return (
    <div class="space-y-8">
      {/* Header */}
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-2xl font-bold text-gray-900">📊 Data Management</h1>
          <p class="text-gray-600 mt-1">Monitor and refresh your data sources</p>
        </div>
        <div class="flex items-center space-x-3">
          <div class="text-sm text-gray-500">
            Last updated: {freshnessData()?.last_check ?
              new Date(freshnessData()!.last_check).toLocaleTimeString() :
              'Never'
            }
          </div>
          <button
            onClick={handleCheckStatus}
            disabled={isChecking()}
            class={`px-3 py-1 text-sm rounded-lg transition-colors ${
              isChecking()
                ? 'text-gray-400 border-gray-200 cursor-not-allowed'
                : 'text-blue-600 hover:text-blue-700 border border-blue-300 hover:border-blue-400'
            }`}
          >
            {isChecking() ? '🔄 Checking...' : '🔄 Check Status'}
          </button>
        </div>
      </div>

      {/* Data Cards */}
      <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
        <For each={DATA_CARDS}>
          {(card) => (
            <div class="relative">
              <Show when={showMultiSelect()}>
                <div class="absolute top-4 right-4 z-10">
                  <input
                    type="checkbox"
                    checked={selectedCards().has(card.id)}
                    onChange={(e) => handleCardSelection(card.id, e.target.checked)}
                    class="w-5 h-5 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
                  />
                </div>
              </Show>
              <DataStatusCard
                cardId={card.id}
                onRefresh={() => handleCardRefresh(card.id)}
              />
            </div>
          )}
        </For>
      </div>

      {/* Multi-Select Controls */}
      <div class="bg-white rounded-lg p-6 border">
        <div class="flex items-center justify-between">
          <label class="flex items-center space-x-3">
            <input
              type="checkbox"
              checked={showMultiSelect()}
              onChange={(e) => setShowMultiSelect(e.target.checked)}
              class="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
            />
            <span class="text-sm font-medium text-gray-700">
              Select multiple cards for batch refresh
            </span>
          </label>

          <Show when={showMultiSelect() && selectedCards().size > 0}>
            <div class="flex items-center space-x-3">
              <span class="text-sm text-gray-600">
                {selectedCards().size} selected
              </span>
              <button
                onClick={handleBatchRefresh}
                disabled={selectedCards().size === 0}
                class="px-4 py-2 text-sm font-medium bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors disabled:bg-gray-300"
              >
                🔄 Refresh Selected
              </button>
            </div>
          </Show>
        </div>
      </div>

    </div>
  );
}