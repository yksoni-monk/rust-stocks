import { createSignal, Show } from 'solid-js';
import { recommendationsStore } from '../stores/recommendationsStore';
import { uiStore } from '../stores/uiStore';

export default function HeroSection() {
  const [showAdvanced, setShowAdvanced] = createSignal(false);

  const handleRunGarpAnalysis = () => {
    console.log('🎯 Running GARP Analysis (primary action)');
    recommendationsStore.setScreeningType('garp_pe');
    uiStore.openRecommendations();
  };

  const handleAlternativeScreening = (type: 'ps' | 'pe') => {
    console.log('📊 Running alternative screening:', type);
    recommendationsStore.setScreeningType(type);
    uiStore.openRecommendations();
  };

  return (
    <div class="bg-gradient-to-br from-blue-50 to-indigo-100 rounded-xl p-8 mb-8 border border-blue-200">
      <div class="max-w-4xl mx-auto text-center">
        {/* Main Value Proposition */}
        <div class="mb-6">
          <h2 class="text-3xl font-bold text-gray-900 mb-3">
            🎯 Find Value Stocks with GARP Screening
          </h2>
          <p class="text-lg text-gray-700 max-w-2xl mx-auto">
            Growth at Reasonable Price - Discover quality stocks with strong fundamentals 
            and fair valuations using our advanced screening algorithm
          </p>
        </div>

        {/* Primary Call-to-Action */}
        <div class="mb-6">
          <button
            onClick={handleRunGarpAnalysis}
            class="bg-blue-600 hover:bg-blue-700 text-white text-xl font-semibold px-12 py-4 rounded-xl shadow-lg hover:shadow-xl transition-all duration-200 transform hover:scale-105"
          >
            🔍 Run GARP Analysis
          </button>
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