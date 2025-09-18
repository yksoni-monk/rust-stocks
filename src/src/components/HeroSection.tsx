import { createSignal, Show } from 'solid-js';
import { recommendationsStore } from '../stores/recommendationsStore';
import { uiStore } from '../stores/uiStore';

export default function HeroSection() {
  const [showAdvanced, setShowAdvanced] = createSignal(false);

  const handleRunGarpAnalysis = () => {
    console.log('🎯 Running GARP Analysis (primary action)');
    recommendationsStore.setScreeningType('garp_pe');
    uiStore.openRecommendations();
    
    // Smooth scroll to recommendations panel after a brief delay
    setTimeout(() => {
      const recommendationsElement = document.querySelector('[data-section="recommendations"]');
      if (recommendationsElement) {
        recommendationsElement.scrollIntoView({ 
          behavior: 'smooth', 
          block: 'start' 
        });
      }
    }, 100);
  };

  const handleGrahamAnalysis = () => {
    console.log('💎 Running Graham Value Analysis');
    recommendationsStore.setScreeningType('graham_value');
    uiStore.openRecommendations();
    
    // Smooth scroll to recommendations panel after a brief delay
    setTimeout(() => {
      const recommendationsElement = document.querySelector('[data-section="recommendations"]');
      if (recommendationsElement) {
        recommendationsElement.scrollIntoView({ 
          behavior: 'smooth', 
          block: 'start' 
        });
      }
    }, 100);
  };

  const handleAlternativeScreening = (type: 'ps' | 'pe') => {
    console.log('📊 Running alternative screening:', type);
    recommendationsStore.setScreeningType(type);
    uiStore.openRecommendations();
    
    // Smooth scroll to recommendations panel after a brief delay
    setTimeout(() => {
      const recommendationsElement = document.querySelector('[data-section="recommendations"]');
      if (recommendationsElement) {
        recommendationsElement.scrollIntoView({ 
          behavior: 'smooth', 
          block: 'start' 
        });
      }
    }, 100);
  };

  return (
    <div class="bg-gradient-to-br from-blue-50 to-indigo-100 rounded-xl p-8 mb-8 border border-blue-200">
      <div class="max-w-4xl mx-auto text-center">
        {/* Main Value Proposition */}
        <div class="mb-6">
          <h2 class="text-3xl font-bold text-gray-900 mb-3">
            🎯 Discover Value Stocks with Advanced Screening
          </h2>
          <p class="text-lg text-gray-700 max-w-2xl mx-auto">
            Find undervalued companies using proven investment methodologies - 
            from Benjamin Graham's classic value principles to modern GARP strategies
          </p>
        </div>

        {/* Primary Screening Options */}
        <div class="mb-6">
          <div class="flex flex-col gap-4 justify-center items-center">
            {/* Main GARP Button */}
            <button
              onClick={handleRunGarpAnalysis}
              disabled={recommendationsStore.loading()}
              class={`text-white text-xl font-semibold px-8 py-4 rounded-xl shadow-lg transition-all duration-200 transform ${
                recommendationsStore.loading() 
                  ? 'bg-blue-400 cursor-not-allowed' 
                  : 'bg-blue-600 hover:bg-blue-700 hover:shadow-xl hover:scale-105'
              }`}
            >
              <Show 
                when={!recommendationsStore.loading()} 
                fallback={
                  <div class="flex items-center gap-2">
                    <div class="animate-spin rounded-full h-5 w-5 border-2 border-white border-t-transparent"></div>
                    Analyzing...
                  </div>
                }
              >
                🔍 GARP Analysis
              </Show>
            </button>
            
            {/* Secondary Options */}
            <div class="flex gap-3 text-sm">
              <button
                onClick={handleGrahamAnalysis}
                class="bg-gray-200 hover:bg-gray-300 text-gray-700 font-medium px-4 py-2 rounded-lg transition-colors"
              >
                💎 Graham Value
              </button>
              <button
                onClick={() => handleAlternativeScreening('ps')}
                class="bg-gray-200 hover:bg-gray-300 text-gray-700 font-medium px-4 py-2 rounded-lg transition-colors"
              >
                📊 P/S Screening
              </button>
              <button
                onClick={() => handleAlternativeScreening('pe')}
                class="bg-gray-200 hover:bg-gray-300 text-gray-700 font-medium px-4 py-2 rounded-lg transition-colors"
              >
                📈 P/E Screening
              </button>
            </div>
          </div>
          
          {/* Loading Progress Indicator */}
          <Show when={recommendationsStore.loading()}>
            <div class="mt-4 text-center">
              <div class="inline-flex items-center gap-3 bg-blue-50 border border-blue-200 rounded-lg px-4 py-2">
                <div class="animate-spin rounded-full h-4 w-4 border-2 border-blue-600 border-t-transparent"></div>
                <span class="text-blue-700 font-medium">Analyzing S&P 500 stocks...</span>
              </div>
              <div class="mt-2 text-xs text-blue-600">
                This may take a few seconds. Results will appear below.
              </div>
            </div>
          </Show>
          
          <div class="mt-3 text-sm text-gray-600">
            <span class="font-medium">GARP:</span> Growth at reasonable price with PEG ratios (recommended) │ 
            <span class="font-medium">Graham:</span> Classic value (limited results in current market)
          </div>
        </div>

        {/* Quick Settings */}
        <div class="mb-6">
          <button
            onClick={() => setShowAdvanced(!showAdvanced())}
            class="text-blue-600 hover:text-blue-700 text-sm font-medium"
          >
            {showAdvanced() ? '▲ Hide' : '▼ Show'} Quick Settings
          </button>
          
          <Show when={showAdvanced()}>
            <div class="mt-4 bg-white rounded-lg p-4 shadow-sm border border-gray-200">
              {/* GARP Settings */}
              <div class="mb-4">
                <h4 class="text-sm font-semibold text-gray-700 mb-2">🔍 GARP Criteria</h4>
                <div class="grid grid-cols-1 sm:grid-cols-3 gap-4 text-sm">
                  <div>
                    <label class="block text-gray-600 mb-1">Max PEG Ratio:</label>
                    <select
                      value={recommendationsStore.garpCriteria().maxPegRatio}
                      onChange={(e) => recommendationsStore.updateGarpCriteria({ maxPegRatio: Number(e.target.value) })}
                      class="w-full border border-gray-300 rounded px-2 py-1"
                    >
                      <option value={0.5}>0.5 (Very Strict)</option>
                      <option value={0.8}>0.8 (Strict)</option>
                      <option value={1.0}>1.0 (Balanced)</option>
                      <option value={1.2}>1.2 (Relaxed)</option>
                      <option value={1.5}>1.5 (Very Relaxed)</option>
                    </select>
                  </div>
                  
                  <div>
                    <label class="block text-gray-600 mb-1">Min Growth Rate:</label>
                    <select
                      value={recommendationsStore.garpCriteria().minRevenueGrowth}
                      onChange={(e) => recommendationsStore.updateGarpCriteria({ minRevenueGrowth: Number(e.target.value) })}
                      class="w-full border border-gray-300 rounded px-2 py-1"
                    >
                      <option value={10}>10% (Low Growth)</option>
                      <option value={15}>15% (Moderate Growth)</option>
                      <option value={20}>20% (High Growth)</option>
                      <option value={25}>25% (Very High Growth)</option>
                    </select>
                  </div>
                  
                  <div>
                    <label class="block text-gray-600 mb-1">Quality Level:</label>
                    <select
                      value={recommendationsStore.garpCriteria().minQualityScore}
                      onChange={(e) => recommendationsStore.updateGarpCriteria({ minQualityScore: Number(e.target.value) })}
                      class="w-full border border-gray-300 rounded px-2 py-1"
                    >
                      <option value={25}>Basic (25+ score)</option>
                      <option value={50}>Good (50+ score)</option>
                      <option value={75}>High (75+ score)</option>
                      <option value={100}>Excellent (100 score)</option>
                    </select>
                  </div>
                </div>
              </div>
              
              {/* Graham Settings */}
              <div class="border-t pt-4">
                <h4 class="text-sm font-semibold text-gray-700 mb-2">💎 Graham Value Criteria</h4>
                <div class="grid grid-cols-1 sm:grid-cols-3 gap-4 text-sm">
                  <div>
                    <label class="block text-gray-600 mb-1">Max P/E Ratio:</label>
                    <select
                      value={recommendationsStore.grahamCriteria().maxPeRatio}
                      onChange={(e) => recommendationsStore.updateGrahamCriteria({ maxPeRatio: Number(e.target.value) })}
                      class="w-full border border-gray-300 rounded px-2 py-1"
                    >
                      <option value={10}>10 (Very Strict)</option>
                      <option value={12}>12 (Strict)</option>
                      <option value={15}>15 (Graham Classic)</option>
                      <option value={18}>18 (Moderate)</option>
                      <option value={20}>20 (Relaxed)</option>
                    </select>
                  </div>
                  
                  <div>
                    <label class="block text-gray-600 mb-1">Max P/B Ratio:</label>
                    <select
                      value={recommendationsStore.grahamCriteria().maxPbRatio}
                      onChange={(e) => recommendationsStore.updateGrahamCriteria({ maxPbRatio: Number(e.target.value) })}
                      class="w-full border border-gray-300 rounded px-2 py-1"
                    >
                      <option value={1.0}>1.0 (Very Strict)</option>
                      <option value={1.2}>1.2 (Strict)</option>
                      <option value={1.5}>1.5 (Graham Classic)</option>
                      <option value={2.0}>2.0 (Moderate)</option>
                      <option value={2.5}>2.5 (Relaxed)</option>
                    </select>
                  </div>
                  
                  <div>
                    <label class="block text-gray-600 mb-1">Max Debt/Equity:</label>
                    <select
                      value={recommendationsStore.grahamCriteria().maxDebtToEquity}
                      onChange={(e) => recommendationsStore.updateGrahamCriteria({ maxDebtToEquity: Number(e.target.value) })}
                      class="w-full border border-gray-300 rounded px-2 py-1"
                    >
                      <option value={0.5}>0.5 (Conservative)</option>
                      <option value={1.0}>1.0 (Balanced)</option>
                      <option value={1.5}>1.5 (Moderate)</option>
                      <option value={2.0}>2.0 (Aggressive)</option>
                    </select>
                  </div>
                </div>
              </div>
            </div>
          </Show>
        </div>

        {/* Alternative Methods */}
        <div class="border-t border-blue-200 pt-6">
          <p class="text-sm text-gray-600 mb-3">Or try alternative screening methods:</p>
          <div class="flex justify-center gap-4">
            <button
              onClick={() => handleAlternativeScreening('ps')}
              class="text-blue-600 hover:text-blue-800 text-sm font-medium px-4 py-2 rounded-lg hover:bg-blue-50 transition-colors"
            >
              📊 P/S Screening
            </button>
            <span class="text-gray-300">|</span>
            <button
              onClick={() => handleAlternativeScreening('pe')}
              class="text-blue-600 hover:text-blue-800 text-sm font-medium px-4 py-2 rounded-lg hover:bg-blue-50 transition-colors"
            >
              📈 P/E Analysis
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}