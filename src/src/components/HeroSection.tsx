import { createSignal, Show } from 'solid-js';
import { recommendationsStore } from '../stores/recommendationsStore';
import { uiStore } from '../stores/uiStore';

export default function HeroSection() {
  const [showAdvanced, setShowAdvanced] = createSignal(false);


  const handlePiotroskilAnalysis = () => {
    console.log('🔍 Running Piotroski F-Score Analysis');
    recommendationsStore.setScreeningType('piotroski');
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

  const handleOShaughnessyAnalysis = () => {
    console.log('📈 Running O\'Shaughnessy Value Composite Analysis');
    recommendationsStore.setScreeningType('oshaughnessy');
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
            from Piotroski F-Score financial strength analysis to O'Shaughnessy value composite
          </p>
        </div>

        {/* Primary Screening Options */}
        <div class="mb-6">
          <div class="flex flex-col gap-4 justify-center items-center">
            {/* Remaining Screening Methods */}
            <div class="grid grid-cols-2 lg:grid-cols-2 gap-4 w-full max-w-2xl">

              <button
                onClick={handlePiotroskilAnalysis}
                disabled={recommendationsStore.loading()}
                class={`text-white text-lg font-semibold px-6 py-4 rounded-xl shadow-lg transition-all duration-200 transform ${
                  recommendationsStore.loading()
                    ? 'bg-green-400 cursor-not-allowed'
                    : 'bg-green-600 hover:bg-green-700 hover:shadow-xl hover:scale-105'
                }`}
              >
                📊 Piotroski F-Score
              </button>

              <button
                onClick={handleOShaughnessyAnalysis}
                disabled={recommendationsStore.loading()}
                class={`text-white text-lg font-semibold px-6 py-4 rounded-xl shadow-lg transition-all duration-200 transform ${
                  recommendationsStore.loading()
                    ? 'bg-purple-400 cursor-not-allowed'
                    : 'bg-purple-600 hover:bg-purple-700 hover:shadow-xl hover:scale-105'
                }`}
              >
                📈 O'Shaughnessy Value
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
            <span class="font-medium">Piotroski:</span> Financial strength scoring │
            <span class="font-medium">O'Shaughnessy:</span> Multi-metric value composite
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
            </div>
          </Show>
        </div>

      </div>
    </div>
  );
}