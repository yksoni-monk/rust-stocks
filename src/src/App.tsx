import { createSignal, createEffect, onMount, For, Show } from 'solid-js';
import { stockStore } from './stores/stockStore';
import { recommendationsStore } from './stores/recommendationsStore';
import { uiStore } from './stores/uiStore';
import { dataRefreshStore } from './stores/dataRefreshStore';
import HeroSection from './components/HeroSection';
import StockBrowser from './components/StockBrowser';
import ResultsPanel from './components/ResultsPanel';
import DataStatusPanel from './components/DataStatusPanel';
import RefreshControls from './components/RefreshControls';
import RefreshProgress from './components/RefreshProgress';

function App() {
  console.log('🚀 SolidJS App starting...');
  
  // Initialize data on mount
  onMount(async () => {
    console.log('🔄 Loading initial data...');
    // Set GARP as default screening type
    recommendationsStore.setScreeningType('garp_pe');
    await stockStore.loadSp500Symbols();
    console.log('✅ Initial data loaded');
  });

  // Initialize data refresh status when data management tab is opened
  createEffect(() => {
    if (uiStore.activeTab() === 'data-management') {
      console.log('🔄 Data Management tab opened, loading freshness status...');
      dataRefreshStore.checkDataFreshness();
    }
  });


  return (
    <div class="min-h-screen bg-gray-50">
      <div class="container mx-auto px-4 py-8">
        {/* Header */}
        <div class="mb-6">
          <h1 class="text-2xl font-bold text-gray-900">
            📊 Stock Analysis Dashboard
          </h1>

          {/* Navigation Tabs */}
          <div class="mt-4 border-b border-gray-200">
            <nav class="-mb-px flex space-x-8">
              <button
                onClick={() => uiStore.openScreening()}
                class={`py-2 px-1 border-b-2 font-medium text-sm ${
                  uiStore.activeTab() === 'screening'
                    ? 'border-blue-500 text-blue-600'
                    : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                }`}
              >
                🔍 Stock Screening
              </button>
              <button
                onClick={() => uiStore.openDataManagement()}
                class={`py-2 px-1 border-b-2 font-medium text-sm ${
                  uiStore.activeTab() === 'data-management'
                    ? 'border-blue-500 text-blue-600'
                    : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                }`}
              >
                🔄 Data Management
                {/* Status indicator */}
                <Show when={dataRefreshStore.freshnessStatus()}>
                  <span class={`ml-2 w-2 h-2 rounded-full inline-block ${
                    dataRefreshStore.freshnessStatus()?.overall_status === 'fresh' ? 'bg-green-500' :
                    dataRefreshStore.freshnessStatus()?.overall_status === 'stale' ? 'bg-yellow-500' : 'bg-red-500'
                  }`}></span>
                </Show>
              </button>
            </nav>
          </div>
        </div>

        {/* Screening Tab Content */}
        <Show when={uiStore.activeTab() === 'screening'}>
          {/* Primary Action Area */}
          <HeroSection />

          {/* Results Area - Shows when screening is run */}
          <Show when={uiStore.showRecommendations()}>
            <div data-section="recommendations">
              <ResultsPanel onClose={uiStore.closeAllPanels} />
            </div>
          </Show>

          {/* Stock Browser - Collapsible secondary tool */}
          <StockBrowser />
        </Show>

        {/* Data Management Tab Content */}
        <Show when={uiStore.activeTab() === 'data-management'}>
          <div class="space-y-8">
            {/* Data Status Panel */}
            <DataStatusPanel />

            {/* Refresh Controls */}
            <RefreshControls />

            {/* Progress Monitoring */}
            <div>
              <h2 class="text-xl font-semibold text-gray-900 mb-4 flex items-center">
                <span class="mr-2">📊</span>
                Progress Monitoring
              </h2>
              <RefreshProgress />
            </div>
          </div>
        </Show>
      </div>
    </div>
  );
}

export default App;