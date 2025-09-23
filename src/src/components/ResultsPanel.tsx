import { For, Show } from 'solid-js';
import { recommendationsStore } from '../stores/recommendationsStore';
import { stockStore } from '../stores/stockStore';
import type { Recommendation, ScreeningType } from '../stores/recommendationsStore';

// Color mapping for different screening types to match button colors
const getScreeningTypeColors = (screeningType: ScreeningType) => {
  switch (screeningType) {
    case 'garp_pe':
      return {
        primary: 'bg-blue-600',
        hover: 'hover:bg-blue-700',
        text: 'text-blue-100',
        accent: 'text-blue-600',
        gradient: 'from-blue-50 to-blue-100',
        border: 'border-blue-200'
      };
    case 'graham_value':
      return {
        primary: 'bg-gray-600',
        hover: 'hover:bg-gray-700',
        text: 'text-gray-100',
        accent: 'text-gray-600',
        gradient: 'from-gray-50 to-gray-100',
        border: 'border-gray-200'
      };
    case 'piotroski':
      return {
        primary: 'bg-green-600',
        hover: 'hover:bg-green-700',
        text: 'text-green-100',
        accent: 'text-green-600',
        gradient: 'from-green-50 to-emerald-50',
        border: 'border-green-200'
      };
    case 'oshaughnessy':
      return {
        primary: 'bg-purple-600',
        hover: 'hover:bg-purple-700',
        text: 'text-purple-100',
        accent: 'text-purple-600',
        gradient: 'from-purple-50 to-purple-100',
        border: 'border-purple-200'
      };
    default:
      return {
        primary: 'bg-blue-600',
        hover: 'hover:bg-blue-700',
        text: 'text-blue-100',
        accent: 'text-blue-600',
        gradient: 'from-blue-50 to-blue-100',
        border: 'border-blue-200'
      };
  }
};

interface ResultsPanelProps {
  onClose: () => void;
  screeningType: ScreeningType;
}

export default function ResultsPanel(props: ResultsPanelProps) {
  console.log('🚀 ResultsPanel created with screening type:', props.screeningType);
  
  // Get colors based on the prop - simple and direct
  const colors = getScreeningTypeColors(props.screeningType);
  console.log('🎨 Using colors for', props.screeningType, ':', colors);

  // Load recommendations when S&P 500 symbols are available
  const loadRecommendations = () => {
    const symbols = stockStore.sp500Symbols();
    if (symbols.length > 0) {
      recommendationsStore.loadRecommendations(symbols);
    }
  };

  // Trigger analysis when component mounts or screening type changes
  loadRecommendations();

  const getScreeningTitle = (type: ScreeningType) => {
    switch (type) {
      case 'garp_pe': return '🎯 GARP Analysis Results';
      case 'graham_value': return '💎 Graham Value Results';
      case 'piotroski': return '🔍 Piotroski F-Score Results';
      case 'oshaughnessy': return '💰 O\'Shaughnessy Value Results';
      case 'ps': return '📊 P/S Screening Results';
      case 'pe': return '📈 P/E Analysis Results';
      default: return '📋 Screening Results';
    }
  };

  const getScreeningSummary = (type: ScreeningType, count: number) => {
    const garpCriteria = recommendationsStore.garpCriteria();
    const grahamCriteria = recommendationsStore.grahamCriteria();
    const piotroskilCriteria = recommendationsStore.piotroskilCriteria();
    const oshaughnessyCriteria = recommendationsStore.oshaughnessyCriteria();

    switch (type) {
      case 'garp_pe':
        return `Found ${count} stocks with PEG < ${garpCriteria.maxPegRatio} and revenue growth > ${garpCriteria.minRevenueGrowth}%`;
      case 'graham_value':
        return `Found ${count} Graham value stocks with P/E < ${grahamCriteria.maxPeRatio}, P/B < ${grahamCriteria.maxPbRatio}, and strong fundamentals`;
      case 'piotroski':
        return `Found ${count} quality stocks with F-Score ≥ ${piotroskilCriteria.minFScore} and data completeness ≥ ${piotroskilCriteria.minDataCompleteness}%`;
      case 'oshaughnessy':
        return `Found ${count} value stocks with composite ranking ≤ ${oshaughnessyCriteria.maxCompositePercentile}th percentile`;
      case 'ps':
        return `Found ${count} undervalued stocks with low P/S ratios and revenue growth`;
      case 'pe': 
        return `Found ${count} historically undervalued stocks based on P/E analysis`;
      default: 
        return `Found ${count} stocks matching your criteria`;
    }
  };

  return (
    <div class={`bg-gradient-to-r ${colors.gradient} rounded-xl border-2 ${colors.border} shadow-lg mb-8`}>
      {/* Results Header */}
      <div class={`${colors.primary} text-white p-6 rounded-t-xl`}>
        <div class="flex justify-between items-start">
          <div class="flex-1">
            <h2 class="text-2xl font-bold mb-2">
              {getScreeningTitle(props.screeningType)}
            </h2>
            <Show 
              when={!recommendationsStore.loading() && !recommendationsStore.error() && recommendationsStore.recommendations().length > 0}
              fallback={
                <p class={`${colors.text} opacity-90`}>
                  Analyzing S&P 500 stocks with advanced screening algorithms
                </p>
              }
            >
              <p class={colors.text}>
                {getScreeningSummary(props.screeningType, recommendationsStore.recommendations().length)}
              </p>
            </Show>
          </div>
          
          <button
            onClick={props.onClose}
            class={`ml-4 p-2 ${colors.hover} rounded-full transition-colors`}
            title="Close results"
          >
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      </div>

      {/* Results Content */}
      <div class="p-6">
        {/* Loading State */}
        <Show when={recommendationsStore.loading()}>
          <div class="text-center py-12">
            <div class="inline-flex items-center">
              <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-green-600 mr-3"></div>
              <span class="text-gray-600 font-medium">Analyzing stocks with {recommendationsStore.screeningType().toUpperCase()} screening...</span>
            </div>
            <p class="text-gray-500 text-sm mt-2">This may take a few seconds</p>
          </div>
        </Show>

        {/* Error State */}
        <Show when={recommendationsStore.error()}>
          <div class="text-center py-8">
            <div class="bg-red-50 border border-red-200 rounded-lg p-6 max-w-md mx-auto">
              <div class="text-red-600 mb-4">
                <svg class="mx-auto h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
              </div>
              <h3 class="text-lg font-medium text-gray-900 mb-2">Analysis Error</h3>
              <p class="text-gray-600 mb-4 text-sm">{recommendationsStore.error()}</p>
              <button
                onClick={loadRecommendations}
                class="bg-red-600 text-white px-4 py-2 rounded-lg hover:bg-red-700 text-sm font-medium"
              >
                Try Again
              </button>
            </div>
          </div>
        </Show>

        {/* Results Content */}
        <Show when={!recommendationsStore.loading() && !recommendationsStore.error()}>
          <Show 
            when={recommendationsStore.recommendations().length > 0}
            fallback={
              <div class="text-center py-8">
                <div class="bg-yellow-50 border border-yellow-200 rounded-lg p-6 max-w-md mx-auto">
                  <div class="text-yellow-600 mb-4">
                    <svg class="mx-auto h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width={2} d="M9.172 16.172a4 4 0 015.656 0M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                    </svg>
                  </div>
                  <h3 class="text-lg font-medium text-gray-900 mb-2">No Stocks Found</h3>
                  <p class="text-gray-600 text-sm">
                    No S&P 500 stocks currently meet the {recommendationsStore.screeningType().toUpperCase()} screening criteria. 
                    Try adjusting the parameters or check back later.
                  </p>
                </div>
              </div>
            }
          >
            {/* Summary Stats */}
            <div class="bg-white rounded-lg p-4 mb-6 border border-green-200">
              <div class="grid grid-cols-2 sm:grid-cols-4 gap-4 text-center">
                <div>
                  <div class="text-2xl font-bold text-green-600">{recommendationsStore.recommendations().length}</div>
                  <div class="text-sm text-gray-600">Stocks Found</div>
                </div>
                <div>
                  <div class="text-2xl font-bold text-blue-600">
                    {recommendationsStore.recommendations().filter(r => r.passes_garp_screening).length}
                  </div>
                  <div class="text-sm text-gray-600">Pass All Criteria</div>
                </div>
                <Show when={recommendationsStore.screeningType() === 'graham_value'}>
                  <div>
                    <div class="text-2xl font-bold text-purple-600">
                      {(recommendationsStore.recommendations().reduce((avg, r) => avg + (r.garp_score || 0), 0) / recommendationsStore.recommendations().length).toFixed(1)}
                    </div>
                    <div class="text-sm text-gray-600">Avg Graham Score</div>
                  </div>
                  <div>
                    <div class="text-2xl font-bold text-orange-600">
                      {(recommendationsStore.recommendations().reduce((avg, r) => avg + (r.current_pe_ratio || 0), 0) / recommendationsStore.recommendations().length).toFixed(1)}
                    </div>
                    <div class="text-sm text-gray-600">Avg P/E Ratio</div>
                  </div>
                </Show>
                <Show when={recommendationsStore.screeningType() === 'garp_pe'}>
                  <div>
                    <div class="text-2xl font-bold text-purple-600">
                      {(recommendationsStore.recommendations().reduce((avg, r) => avg + (r.garp_score || 0), 0) / recommendationsStore.recommendations().length).toFixed(1)}
                    </div>
                    <div class="text-sm text-gray-600">Avg GARP Score</div>
                  </div>
                  <div>
                    <div class="text-2xl font-bold text-orange-600">
                      {Math.round(recommendationsStore.recommendations().reduce((avg, r) => avg + (r.quality_score || 0), 0) / recommendationsStore.recommendations().length)}
                    </div>
                    <div class="text-sm text-gray-600">Avg Quality</div>
                  </div>
                </Show>
                <Show when={recommendationsStore.screeningType() === 'ps' || recommendationsStore.screeningType() === 'pe'}>
                  <div>
                    <div class="text-2xl font-bold text-purple-600">
                      {(recommendationsStore.recommendations().reduce((avg, r) => avg + (r.current_pe_ratio || 0), 0) / recommendationsStore.recommendations().length).toFixed(1)}
                    </div>
                    <div class="text-sm text-gray-600">Avg P/E</div>
                  </div>
                  <div>
                    <div class="text-2xl font-bold text-orange-600">
                      {(recommendationsStore.recommendations().reduce((avg, r) => avg + (r.ps_ratio_ttm || 0), 0) / recommendationsStore.recommendations().length).toFixed(1)}
                    </div>
                    <div class="text-sm text-gray-600">Avg P/S</div>
                  </div>
                </Show>
              </div>
            </div>

            {/* Action Buttons */}
            <div class="flex justify-between items-center mb-6">
              <h3 class="text-lg font-semibold text-gray-900">Investment Opportunities</h3>
              <div class="flex gap-2">
                <button class="bg-green-600 text-white px-4 py-2 rounded-lg hover:bg-green-700 text-sm font-medium">
                  📊 Export Results
                </button>
                <button class="bg-blue-600 text-white px-4 py-2 rounded-lg hover:bg-blue-700 text-sm font-medium">
                  ⭐ Save Search
                </button>
              </div>
            </div>

            {/* Results List */}
            <div class="space-y-3">
              <For each={recommendationsStore.recommendations()}>
                {(rec) => (
                  <div class={`bg-white rounded-lg p-4 border-2 transition-all hover:shadow-md ${
                    rec.passes_garp_screening ? 'border-green-200 bg-green-50' : 'border-gray-200'
                  }`}>
                    <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
                      {/* Stock Info */}
                      <div class="flex-1">
                        <div class="flex items-center gap-3 mb-2">
                          <span class="text-lg font-bold text-gray-700">#{rec.rank}</span>
                          <span class="text-xl font-bold text-blue-600">{rec.symbol}</span>
                          <span class="text-gray-600">{rec.company_name}</span>
                          <Show when={rec.passes_garp_screening}>
                            <span class="bg-green-500 text-white text-xs px-2 py-1 rounded-full font-medium">
                              ✓ Recommended
                            </span>
                          </Show>
                        </div>
                        <div class="text-sm text-gray-600">{rec.reasoning}</div>
                      </div>

                      {/* Metrics */}
                      <div class="flex flex-wrap gap-3 sm:gap-4 items-center justify-end">
                        <MetricsDisplay screeningType={recommendationsStore.screeningType()} rec={rec} />
                      </div>
                    </div>
                  </div>
                )}
              </For>
            </div>
          </Show>
        </Show>
      </div>
    </div>
  );
}

// Enhanced Metrics Display
interface MetricsDisplayProps {
  screeningType: ScreeningType;
  rec: Recommendation;
}

function MetricsDisplay(props: MetricsDisplayProps) {
  return (
    <>
      <Show when={props.screeningType === 'garp_pe'}>
        <div class="text-center bg-gray-50 rounded-lg p-2 min-w-[60px]">
          <div class="text-lg font-bold text-gray-900">
            {props.rec.current_pe_ratio?.toFixed(1) || 'N/A'}
          </div>
          <div class="text-xs text-gray-500">P/E</div>
        </div>
        <div class="text-center bg-gray-50 rounded-lg p-2 min-w-[60px]">
          <div class={`text-sm font-bold ${props.rec.peg_ratio && props.rec.peg_ratio < 1.0 ? 'text-green-600' : 'text-gray-700'}`}>
            {props.rec.peg_ratio?.toFixed(2) || 'N/A'}
          </div>
          <div class="text-xs text-gray-500">PEG</div>
        </div>
        <div class="text-center bg-gray-50 rounded-lg p-2 min-w-[60px]">
          <div class={`text-sm font-bold ${(props.rec.ttm_growth_rate || 0) > 15 ? 'text-green-600' : 'text-gray-700'}`}>
            {props.rec.ttm_growth_rate?.toFixed(0) || props.rec.annual_growth_rate?.toFixed(0) || 'N/A'}%
          </div>
          <div class="text-xs text-gray-500">Growth</div>
        </div>
        <div class="text-center bg-blue-50 rounded-lg p-2 min-w-[60px]">
          <div class="text-sm font-bold text-blue-600">
            {props.rec.garp_score?.toFixed(1) || 'N/A'}
          </div>
          <div class="text-xs text-gray-500">GARP</div>
        </div>
        <div class="text-center bg-gray-50 rounded-lg p-2 min-w-[50px]">
          <div class={`text-lg font-bold ${props.rec.passes_garp_screening ? 'text-green-600' : 'text-red-500'}`}>
            {props.rec.passes_garp_screening ? '✓' : '✗'}
          </div>
          <div class="text-xs text-gray-500">Pass</div>
        </div>
      </Show>
      
      <Show when={props.screeningType === 'graham_value'}>
        <div class="text-center bg-gray-50 rounded-lg p-2 min-w-[60px]">
          <div class="text-lg font-bold text-gray-900">
            {props.rec.current_pe_ratio?.toFixed(1) || 'N/A'}
          </div>
          <div class="text-xs text-gray-500">P/E</div>
        </div>
        <div class="text-center bg-gray-50 rounded-lg p-2 min-w-[60px]">
          <div class="text-sm font-bold text-gray-700">
            N/A
          </div>
          <div class="text-xs text-gray-500">P/B</div>
        </div>
        <div class="text-center bg-gray-50 rounded-lg p-2 min-w-[60px]">
          <div class="text-sm font-bold text-gray-700">
            {props.rec.debt_to_equity_ratio?.toFixed(2) || 'N/A'}
          </div>
          <div class="text-xs text-gray-500">Debt/Eq</div>
        </div>
        <div class="text-center bg-green-50 rounded-lg p-2 min-w-[60px]">
          <div class="text-sm font-bold text-green-600">
            {props.rec.garp_score?.toFixed(1) || 'N/A'}
          </div>
          <div class="text-xs text-gray-500">Graham</div>
        </div>
        <div class="text-center bg-gray-50 rounded-lg p-2 min-w-[50px]">
          <div class={`text-lg font-bold ${props.rec.passes_garp_screening ? 'text-green-600' : 'text-red-500'}`}>
            {props.rec.passes_garp_screening ? '✓' : '✗'}
          </div>
          <div class="text-xs text-gray-500">Pass</div>
        </div>
      </Show>
      
      <Show when={props.screeningType === 'ps'}>
        <div class="text-center bg-gray-50 rounded-lg p-2 min-w-[60px]">
          <div class="text-lg font-bold text-gray-900">
            {props.rec.ps_ratio_ttm?.toFixed(2) || 'N/A'}
          </div>
          <div class="text-xs text-gray-500">P/S</div>
        </div>
        <div class="text-center bg-gray-50 rounded-lg p-2 min-w-[60px]">
          <div class="text-sm font-bold text-gray-700">
            {props.rec.z_score?.toFixed(2) || 'N/A'}
          </div>
          <div class="text-xs text-gray-500">Z-Score</div>
        </div>
      </Show>
      
      <Show when={props.screeningType === 'piotroski'}>
        <div class="text-center bg-gray-50 rounded-lg p-2 min-w-[60px]">
          <div class="text-lg font-bold text-green-600">
            {(props.rec as any).f_score_complete || 'N/A'}/9
          </div>
          <div class="text-xs text-gray-500">F-Score</div>
        </div>
        <div class="text-center bg-gray-50 rounded-lg p-2 min-w-[60px]">
          <div class="text-sm font-bold text-blue-600">
            {(props.rec as any).data_completeness_score || 'N/A'}%
          </div>
          <div class="text-xs text-gray-500">Data Quality</div>
        </div>
        <div class="text-center bg-gray-50 rounded-lg p-2 min-w-[40px]">
          <div class={`text-lg font-bold ${(props.rec as any).criterion_positive_net_income ? 'text-green-600' : 'text-red-500'}`}>
            {(props.rec as any).criterion_positive_net_income ? '✓' : '✗'}
          </div>
          <div class="text-xs text-gray-500">Income</div>
        </div>
        <div class="text-center bg-gray-50 rounded-lg p-2 min-w-[40px]">
          <div class={`text-lg font-bold ${(props.rec as any).criterion_improving_roa ? 'text-green-600' : 'text-red-500'}`}>
            {(props.rec as any).criterion_improving_roa ? '✓' : '✗'}
          </div>
          <div class="text-xs text-gray-500">ROA</div>
        </div>
        <div class="text-center bg-gray-50 rounded-lg p-2 min-w-[40px]">
          <div class={`text-lg font-bold ${(props.rec as any).criterion_decreasing_debt_ratio ? 'text-green-600' : 'text-red-500'}`}>
            {(props.rec as any).criterion_decreasing_debt_ratio ? '✓' : '✗'}
          </div>
          <div class="text-xs text-gray-500">Debt</div>
        </div>
        <div class="text-center bg-gray-50 rounded-lg p-2 min-w-[40px]">
          <div class={`text-lg font-bold ${(props.rec as any).criterion_positive_operating_cash_flow ? 'text-green-600' : 'text-red-500'}`}>
            {(props.rec as any).criterion_positive_operating_cash_flow ? '✓' : '✗'}
          </div>
          <div class="text-xs text-gray-500">Cash Flow</div>
        </div>
        <div class="text-center bg-gray-50 rounded-lg p-2 min-w-[40px]">
          <div class={`text-lg font-bold ${(props.rec as any).criterion_cash_flow_quality ? 'text-green-600' : 'text-red-500'}`}>
            {(props.rec as any).criterion_cash_flow_quality ? '✓' : '✗'}
          </div>
          <div class="text-xs text-gray-500">CF Quality</div>
        </div>
        <div class="text-center bg-gray-50 rounded-lg p-2 min-w-[40px]">
          <div class={`text-lg font-bold ${(props.rec as any).criterion_improving_current_ratio ? 'text-green-600' : 'text-red-500'}`}>
            {(props.rec as any).criterion_improving_current_ratio ? '✓' : '✗'}
          </div>
          <div class="text-xs text-gray-500">Current Ratio</div>
        </div>
        <div class="text-center bg-gray-50 rounded-lg p-2 min-w-[40px]">
          <div class={`text-lg font-bold ${(props.rec as any).criterion_no_dilution ? 'text-green-600' : 'text-red-500'}`}>
            {(props.rec as any).criterion_no_dilution ? '✓' : '✗'}
          </div>
          <div class="text-xs text-gray-500">Shares</div>
        </div>
        <div class="text-center bg-gray-50 rounded-lg p-2 min-w-[40px]">
          <div class={`text-lg font-bold ${(props.rec as any).criterion_improving_gross_margin ? 'text-green-600' : 'text-red-500'}`}>
            {(props.rec as any).criterion_improving_gross_margin ? '✓' : '✗'}
          </div>
          <div class="text-xs text-gray-500">Gross Margin</div>
        </div>
        <div class="text-center bg-gray-50 rounded-lg p-2 min-w-[40px]">
          <div class={`text-lg font-bold ${(props.rec as any).criterion_improving_asset_turnover ? 'text-green-600' : 'text-red-500'}`}>
            {(props.rec as any).criterion_improving_asset_turnover ? '✓' : '✗'}
          </div>
          <div class="text-xs text-gray-500">Asset Turn</div>
        </div>
      </Show>

      <Show when={props.screeningType === 'pe'}>
        <div class="text-center bg-gray-50 rounded-lg p-2 min-w-[60px]">
          <div class="text-lg font-bold text-gray-900">
            {props.rec.current_pe_ratio?.toFixed(2) || 'N/A'}
          </div>
          <div class="text-xs text-gray-500">P/E</div>
        </div>
      </Show>
    </>
  );
}