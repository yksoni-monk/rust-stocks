import { createSignal, createEffect, For, Show } from 'solid-js';
import { recommendationsStore } from '../stores/recommendationsStore';
import { stockStore } from '../stores/stockStore';
import type { Recommendation, ScreeningType, GarpCriteria, PsCriteria } from '../stores/recommendationsStore';

interface RecommendationsPanelProps {
  onClose: () => void;
}

export default function RecommendationsPanel(props: RecommendationsPanelProps) {
  console.log('🚀 SolidJS RecommendationsPanel created');

  // Load recommendations when S&P 500 symbols are available
  createEffect(() => {
    const symbols = stockStore.sp500Symbols();
    if (symbols.length > 0) {
      console.log('📡 S&P 500 symbols loaded, loading recommendations');
      recommendationsStore.loadRecommendations(symbols);
    }
  });

  // Update GARP criteria
  const updateGarpCriteria = (updates: Partial<GarpCriteria>) => {
    recommendationsStore.updateGarpCriteria(updates);
    // Reload recommendations with new criteria
    const symbols = stockStore.sp500Symbols();
    if (symbols.length > 0) {
      recommendationsStore.loadRecommendations(symbols);
    }
  };

  // Update P/S criteria
  const updatePsCriteria = (updates: Partial<PsCriteria>) => {
    recommendationsStore.updatePsCriteria(updates);
    // Reload recommendations with new criteria
    const symbols = stockStore.sp500Symbols();
    if (symbols.length > 0) {
      recommendationsStore.loadRecommendations(symbols);
    }
  };

  // Change screening type
  const handleScreeningTypeChange = (newType: ScreeningType) => {
    recommendationsStore.setScreeningType(newType);
    recommendationsStore.clearRecommendations();
    // Reload with new type
    const symbols = stockStore.sp500Symbols();
    if (symbols.length > 0) {
      recommendationsStore.loadRecommendations(symbols);
    }
  };

  return (
    <div class="bg-white rounded-lg shadow-lg border border-gray-200 overflow-hidden">
      {/* Header */}
      <div class="bg-blue-600 text-white p-4 sm:p-6 flex justify-between items-center">
        <div class="flex-1 min-w-0">
          <h2 class="text-xl sm:text-2xl font-bold truncate">Stock Value Recommendations</h2>
          <p class="text-blue-100 mt-1 text-sm sm:text-base truncate">
            {getScreeningDescription(recommendationsStore.screeningType(), recommendationsStore.garpCriteria(), recommendationsStore.psCriteria())}
          </p>
        </div>
        <button
          onClick={props.onClose}
          class="ml-4 p-2 hover:bg-blue-700 rounded-full transition-colors"
          title="Close recommendations"
        >
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width={2} d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      {/* Stats */}
      <Show when={recommendationsStore.stats()}>
        <div class="bg-gray-50 p-4 border-b">
          <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-3 sm:gap-4 text-center">
            <div>
              <div class="text-2xl font-bold text-blue-600">{recommendationsStore.stats()!.total_sp500_stocks}</div>
              <div class="text-sm text-gray-600">Total S&P 500</div>
            </div>
            <div>
              <div class="text-2xl font-bold text-green-600">{recommendationsStore.stats()!.stocks_with_pe_data}</div>
              <div class="text-sm text-gray-600">With Data</div>
            </div>
            <div>
              <div class="text-2xl font-bold text-purple-600">{recommendationsStore.stats()!.value_stocks_found}</div>
              <div class="text-sm text-gray-600">Value Stocks</div>
            </div>
            <div>
              <div class="text-2xl font-bold text-orange-600">{recommendationsStore.stats()!.average_value_score?.toFixed(1) || 'N/A'}</div>
              <div class="text-sm text-gray-600">Avg Value Score</div>
            </div>
            <div>
              <div class="text-2xl font-bold text-red-600">{recommendationsStore.stats()!.average_risk_score?.toFixed(1) || 'N/A'}</div>
              <div class="text-sm text-gray-600">Avg Risk Score</div>
            </div>
          </div>
        </div>
      </Show>

      {/* Controls */}
      <div class="p-4 bg-gray-50 border-b">
        <div class="flex items-center justify-between mb-4">
          <div class="flex items-center gap-6">
            <div class="flex items-center gap-4">
              <label class="text-sm font-medium text-gray-700">Screening Method:</label>
              <select
                value={recommendationsStore.screeningType()}
                onChange={(e) => handleScreeningTypeChange(e.target.value as ScreeningType)}
                class="border border-gray-300 rounded px-3 py-1 text-sm"
              >
                <option value="pe">P/E Ratio (Historical)</option>
                <option value="ps">P/S Ratio (TTM)</option>
                <option value="garp_pe">GARP (P/E + PEG Based)</option>
              </select>
            </div>
            <div class="flex items-center gap-2">
              <label class="text-sm font-medium text-gray-700">Limit:</label>
              <select
                value={recommendationsStore.limit()}
                onChange={(e) => recommendationsStore.setLimit(parseInt(e.target.value))}
                class="border border-gray-300 rounded px-2 py-1 text-sm"
              >
                <option value={10}>10</option>
                <option value={20}>20</option>
                <option value={50}>50</option>
                <option value={100}>100</option>
              </select>
            </div>
          </div>
        </div>

        {/* Criteria Controls */}
        <CriteriaControls 
          screeningType={recommendationsStore.screeningType()}
          garpCriteria={recommendationsStore.garpCriteria()}
          psCriteria={recommendationsStore.psCriteria()}
          onGarpCriteriaChange={updateGarpCriteria}
          onPsCriteriaChange={updatePsCriteria}
        />
      </div>

      {/* Loading State */}
      <Show when={recommendationsStore.loading()}>
        <div class="p-8 text-center">
          <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto"></div>
          <p class="mt-4 text-gray-600">Analyzing stocks...</p>
        </div>
      </Show>

      {/* Error State */}
      <Show when={recommendationsStore.error()}>
        <div class="p-8 text-center">
          <div class="text-red-600 mb-4">
            <svg class="mx-auto h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          </div>
          <h3 class="text-lg font-medium text-gray-900 mb-2">Analysis Error</h3>
          <p class="text-gray-600 mb-4">{recommendationsStore.error()}</p>
          <button
            onClick={() => {
              const symbols = stockStore.sp500Symbols();
              if (symbols.length > 0) {
                recommendationsStore.loadRecommendations(symbols);
              }
            }}
            class="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700"
          >
            Retry
          </button>
        </div>
      </Show>

      {/* Results */}
      <Show when={!recommendationsStore.loading() && !recommendationsStore.error()}>
        <div class="max-h-96 sm:max-h-[500px] overflow-y-auto">
          <Show 
            when={recommendationsStore.recommendations().length > 0} 
            fallback={
              <div class="p-8 text-center">
                <div class="text-gray-400 mb-4">
                  <svg class="mx-auto h-16 w-16" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                  </svg>
                </div>
                <h3 class="text-lg font-medium text-gray-900 mb-2">No Results Found</h3>
                <p class="text-gray-600">
                  {getNoResultsMessage(recommendationsStore.screeningType())}
                </p>
              </div>
            }
          >
            <div class="divide-y divide-gray-200">
              <For each={recommendationsStore.recommendations()}>
                {(rec) => (
                  <div class="p-3 sm:p-4 hover:bg-gray-50">
                    <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 sm:gap-4">
                      <div class="flex-1">
                        <div class="flex items-center gap-3 mb-2">
                          <span class="text-lg font-bold text-gray-900">#{rec.rank}</span>
                          <span class="text-xl font-bold text-blue-600">{rec.symbol}</span>
                          <span class="text-gray-600">{rec.company_name}</span>
                        </div>
                        <div class="text-sm text-gray-600 mb-2">{rec.reasoning}</div>
                      </div>
                      <div class="flex flex-wrap gap-3 sm:gap-4 items-center justify-center sm:justify-end">
                        <MetricsDisplay screeningType={recommendationsStore.screeningType()} rec={rec} />
                      </div>
                    </div>
                  </div>
                )}
              </For>
            </div>
          </Show>
        </div>
      </Show>
    </div>
  );
}

// Helper components
interface CriteriaControlsProps {
  screeningType: ScreeningType;
  garpCriteria: GarpCriteria;
  psCriteria: PsCriteria;
  onGarpCriteriaChange: (updates: Partial<GarpCriteria>) => void;
  onPsCriteriaChange: (updates: Partial<PsCriteria>) => void;
}

function CriteriaControls(props: CriteriaControlsProps) {
  return (
    <Show when={props.screeningType === 'garp_pe' || props.screeningType === 'ps'}>
      <div class="space-y-4">
        <Show when={props.screeningType === 'garp_pe'}>
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 sm:gap-4">
            <div>
              <label class="text-sm font-medium text-gray-700">Max PEG Ratio:</label>
              <select
                value={props.garpCriteria.maxPegRatio}
                onChange={(e) => props.onGarpCriteriaChange({ maxPegRatio: Number(e.target.value) })}
                class="border border-gray-300 rounded px-2 py-1 text-sm w-full"
              >
                <option value={0.5}>0.5</option>
                <option value={0.8}>0.8</option>
                <option value={1.0}>1.0</option>
                <option value={1.2}>1.2</option>
                <option value={1.5}>1.5</option>
                <option value={2.0}>2.0</option>
              </select>
            </div>
            <div>
              <label class="text-sm font-medium text-gray-700">Min Revenue Growth (%):</label>
              <select
                value={props.garpCriteria.minRevenueGrowth}
                onChange={(e) => props.onGarpCriteriaChange({ minRevenueGrowth: Number(e.target.value) })}
                class="border border-gray-300 rounded px-2 py-1 text-sm w-full"
              >
                <option value={5}>5%</option>
                <option value={10}>10%</option>
                <option value={15}>15%</option>
                <option value={20}>20%</option>
                <option value={25}>25%</option>
              </select>
            </div>
          </div>
        </Show>
        
        <Show when={props.screeningType === 'ps'}>
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 sm:gap-4">
            <div>
              <label class="text-sm font-medium text-gray-700">P/S Ratio Threshold:</label>
              <select
                value={props.psCriteria.psRatio}
                onChange={(e) => props.onPsCriteriaChange({ psRatio: Number(e.target.value) })}
                class="border border-gray-300 rounded px-2 py-1 text-sm w-full"
              >
                <option value={1.0}>1.0</option>
                <option value={1.5}>1.5</option>
                <option value={2.0}>2.0</option>
                <option value={2.5}>2.5</option>
                <option value={3.0}>3.0</option>
              </select>
            </div>
          </div>
        </Show>
      </div>
    </Show>
  );
}

interface MetricsDisplayProps {
  screeningType: ScreeningType;
  rec: Recommendation;
}

function MetricsDisplay(props: MetricsDisplayProps) {
  return (
    <>
      <Show when={props.screeningType === 'garp_pe'}>
        <div class="text-center">
          <div class="text-lg font-bold text-gray-900">
            {props.rec.current_pe_ratio?.toFixed(2) || 'N/A'}
          </div>
          <div class="text-xs text-gray-500">Current P/E</div>
        </div>
        <div class="text-center">
          <div class="text-sm font-bold text-gray-700">
            {props.rec.peg_ratio?.toFixed(2) || 'N/A'}
          </div>
          <div class="text-xs text-gray-500">PEG Ratio</div>
        </div>
        <div class="text-center">
          <div class="text-sm font-bold text-gray-700">
            {props.rec.ttm_growth_rate?.toFixed(1) + '%' || props.rec.annual_growth_rate?.toFixed(1) + '%' || 'N/A'}
          </div>
          <div class="text-xs text-gray-500">Revenue Growth</div>
        </div>
        <div class="text-center">
          <div class="text-sm font-bold text-blue-600">
            {props.rec.garp_score?.toFixed(2) || 'N/A'}
          </div>
          <div class="text-xs text-gray-500">GARP Score</div>
        </div>
        <div class="text-center">
          <div class={`text-sm font-bold ${props.rec.passes_garp_screening ? 'text-green-600' : 'text-red-600'}`}>
            {props.rec.passes_garp_screening ? '✓' : '✗'}
          </div>
          <div class="text-xs text-gray-500">Passes Screen</div>
        </div>
      </Show>
      
      <Show when={props.screeningType === 'ps'}>
        <div class="text-center">
          <div class="text-lg font-bold text-gray-900">
            {props.rec.ps_ratio_ttm?.toFixed(2) || 'N/A'}
          </div>
          <div class="text-xs text-gray-500">Current P/S</div>
        </div>
        <div class="text-center">
          <div class="text-sm font-bold text-gray-700">
            {props.rec.z_score?.toFixed(2) || 'N/A'}
          </div>
          <div class="text-xs text-gray-500">Z-Score</div>
        </div>
      </Show>
      
      <Show when={props.screeningType === 'pe'}>
        <div class="text-center">
          <div class="text-lg font-bold text-gray-900">
            {props.rec.current_pe_ratio?.toFixed(2) || 'N/A'}
          </div>
          <div class="text-xs text-gray-500">Current P/E</div>
        </div>
      </Show>
    </>
  );
}

// Helper functions
function getScreeningDescription(type: ScreeningType, garpCriteria: GarpCriteria, psCriteria: PsCriteria): string {
  switch (type) {
    case 'ps':
      return `P/S Screening with Revenue Growth: Statistical undervaluation + growth requirements (S&P 500 only, Market Cap > $${(psCriteria.minMarketCap / 1_000_000).toFixed(0)}M)`;
    case 'garp_pe':
      return `GARP P/E Screening: Growth at Reasonable Price using PEG ratios (S&P 500 only, PEG < ${garpCriteria.maxPegRatio}, Revenue Growth > ${garpCriteria.minRevenueGrowth}%)`;
    case 'pe':
      return 'P/E Screening: Historical undervaluation analysis (Current P/E ≤ Historical Minimum × 1.20)';
    default:
      return 'Stock value screening analysis';
  }
}

function getNoResultsMessage(type: ScreeningType): string {
  switch (type) {
    case 'ps':
      return 'No S&P 500 stocks currently meet our P/S screening criteria. Try adjusting the criteria or check back later.';
    case 'garp_pe':
      return 'No S&P 500 stocks currently meet our GARP P/E screening criteria. Try relaxing the PEG ratio or revenue growth requirements.';
    case 'pe':
      return 'No S&P 500 stocks currently meet our P/E screening criteria. The market may be fairly valued or overvalued.';
    default:
      return 'No stocks found matching the current criteria.';
  }
}