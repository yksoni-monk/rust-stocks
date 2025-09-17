import { createSignal, createEffect, onMount, For, Show } from 'solid-js';
import { stockStore } from './stores/stockStore';
import { recommendationsStore } from './stores/recommendationsStore';
import { uiStore } from './stores/uiStore';
import HeroSection from './components/HeroSection';
import StockBrowser from './components/StockBrowser';
import ResultsPanel from './components/ResultsPanel';

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


  return (
    <div class="min-h-screen bg-gray-50">
      <div class="container mx-auto px-4 py-8">
        {/* Header */}
        <div class="mb-6">
          <h1 class="text-2xl font-bold text-gray-900">
            📊 Stock Analysis Dashboard
          </h1>
        </div>

        {/* Primary Action Area */}
        <HeroSection />

        {/* Results Area - Shows when screening is run */}
        <Show when={uiStore.showRecommendations()}>
          <ResultsPanel onClose={uiStore.closeAllPanels} />
        </Show>

        {/* Stock Browser - Collapsible secondary tool */}
        <StockBrowser />
      </div>
    </div>
  );
}

export default App;