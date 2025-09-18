# Piotroski F-Score Screening - Architecture & Implementation Plan

## 📋 Executive Summary

Comprehensive architecture for implementing Piotroski F-Score screening strategy that integrates with our existing SolidJS frontend, Rust backend, and SQLite database. This value investing strategy uses 9 binary criteria to identify high-quality undervalued stocks with improving fundamentals.

## 🎯 Strategy Overview

### Core Concept
- **Value Filter**: P/B ≤ 1.0 (undervalued stocks)
- **F-Score Calculation**: 9 binary criteria (0-9 points)
- **Selection**: Stocks with F-Score ≥ 8 (high quality)
- **Rebalancing**: Annual portfolio updates

### 9 F-Score Criteria
**Profitability (4 points)**:
1. Positive Net Income
2. Positive Operating Cash Flow  
3. Improving ROA (vs prior year)
4. Cash Flow > Net Income

**Leverage/Liquidity (3 points)**:
5. Decreasing Long-Term Debt Ratio
6. Improving Current Ratio
7. No Share Dilution

**Operating Efficiency (2 points)**:
8. Improving Gross Margin
9. Improving Asset Turnover

## 🏗️ System Architecture

### High-Level Design
```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Piotroski F-Score Screening System                   │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────┐    ┌──────────────────┐    ┌──────────────────┐   │
│  │   SolidJS        │    │   Rust Backend   │    │   SQLite         │   │
│  │   Frontend       │───▶│   Tauri Commands │───▶│   Database       │   │
│  │   (TypeScript)   │    │   (Rust)         │    │   (Views/Tables) │   │
│  └─────────────────┘    └──────────────────┘    └──────────────────┘   │
│           │                        │                        │           │
│           ▼                        ▼                        ▼           │
│  ┌─────────────────┐    ┌──────────────────┐    ┌──────────────────┐   │
│  │   Store-based    │    │   F-Score        │    │   Financial      │   │
│  │   State Mgmt     │◀──▶│   Calculation    │◀──▶│   Data Views     │   │
│  │   (Signals)      │    │   Engine         │    │   (TTM/Annual)   │   │
│  └─────────────────┘    └──────────────────┘    └──────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

## 📊 Database Architecture

### New Database Views

#### 1. Piotroski F-Score Data View
```sql
CREATE VIEW piotroski_f_score_data AS
SELECT 
    s.id as stock_id,
    s.symbol,
    s.sector,
    s.industry,
    
    -- Market Metrics
    dp.close_price as current_price,
    dp.market_cap,
    dp.pb_ratio,
    
    -- Current Year Financials (TTM)
    current_income.net_income as current_net_income,
    current_income.operating_cash_flow as current_operating_cash_flow,
    current_income.revenue as current_revenue,
    current_income.cost_of_revenue as current_cost_of_revenue,
    current_income.shares_diluted as current_shares_outstanding,
    
    -- Prior Year Financials (TTM)
    prior_income.net_income as prior_net_income,
    prior_income.operating_cash_flow as prior_operating_cash_flow,
    prior_income.revenue as prior_revenue,
    prior_income.cost_of_revenue as prior_cost_of_revenue,
    prior_income.shares_diluted as prior_shares_outstanding,
    
    -- Current Year Balance Sheet
    current_balance.total_assets as current_total_assets,
    current_balance.long_term_debt as current_long_term_debt,
    current_balance.current_assets as current_current_assets,
    current_balance.current_liabilities as current_current_liabilities,
    
    -- Prior Year Balance Sheet
    prior_balance.total_assets as prior_total_assets,
    prior_balance.long_term_debt as prior_long_term_debt,
    prior_balance.current_assets as prior_current_assets,
    prior_balance.current_liabilities as prior_current_liabilities,
    
    -- Calculated Ratios (Current)
    CASE 
        WHEN current_total_assets > 0 THEN current_net_income / current_total_assets
        ELSE NULL 
    END as current_roa,
    
    CASE 
        WHEN current_total_assets > 0 THEN current_long_term_debt / current_total_assets
        ELSE NULL 
    END as current_debt_to_assets,
    
    CASE 
        WHEN current_current_liabilities > 0 THEN current_current_assets / current_current_liabilities
        ELSE NULL 
    END as current_current_ratio,
    
    CASE 
        WHEN current_revenue > 0 THEN (current_revenue - current_cost_of_revenue) / current_revenue
        ELSE NULL 
    END as current_gross_margin,
    
    CASE 
        WHEN current_total_assets > 0 THEN current_revenue / current_total_assets
        ELSE NULL 
    END as current_asset_turnover,
    
    -- Calculated Ratios (Prior)
    CASE 
        WHEN prior_total_assets > 0 THEN prior_net_income / prior_total_assets
        ELSE NULL 
    END as prior_roa,
    
    CASE 
        WHEN prior_total_assets > 0 THEN prior_long_term_debt / prior_total_assets
        ELSE NULL 
    END as prior_debt_to_assets,
    
    CASE 
        WHEN prior_current_liabilities > 0 THEN prior_current_assets / prior_current_liabilities
        ELSE NULL 
    END as prior_current_ratio,
    
    CASE 
        WHEN prior_revenue > 0 THEN (prior_revenue - prior_cost_of_revenue) / prior_revenue
        ELSE NULL 
    END as prior_gross_margin,
    
    CASE 
        WHEN prior_total_assets > 0 THEN prior_revenue / prior_total_assets
        ELSE NULL 
    END as prior_asset_turnover

FROM stocks s
JOIN daily_valuation_ratios dp ON s.id = dp.stock_id 
    AND dp.date = (SELECT MAX(date) FROM daily_valuation_ratios WHERE stock_id = s.id)

-- Current TTM Income Statement
LEFT JOIN (
    SELECT stock_id, net_income, operating_cash_flow, revenue, cost_of_revenue, shares_diluted,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
    FROM income_statements 
    WHERE period_type = 'TTM' AND report_date >= date('now', '-1 year')
) current_income ON s.id = current_income.stock_id AND current_income.rn = 1

-- Prior TTM Income Statement
LEFT JOIN (
    SELECT stock_id, net_income, operating_cash_flow, revenue, cost_of_revenue, shares_diluted,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
    FROM income_statements 
    WHERE period_type = 'TTM' AND report_date < (
        SELECT MAX(report_date) FROM income_statements 
        WHERE stock_id = s.id AND period_type = 'TTM'
    )
) prior_income ON s.id = prior_income.stock_id AND prior_income.rn = 1

-- Current TTM Balance Sheet
LEFT JOIN (
    SELECT stock_id, total_assets, long_term_debt, current_assets, current_liabilities,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
    FROM balance_sheets 
    WHERE period_type = 'TTM' AND report_date >= date('now', '-1 year')
) current_balance ON s.id = current_balance.stock_id AND current_balance.rn = 1

-- Prior TTM Balance Sheet
LEFT JOIN (
    SELECT stock_id, total_assets, long_term_debt, current_assets, current_liabilities,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
    FROM balance_sheets 
    WHERE period_type = 'TTM' AND report_date < (
        SELECT MAX(report_date) FROM balance_sheets 
        WHERE stock_id = s.id AND period_type = 'TTM'
    )
) prior_balance ON s.id = prior_balance.stock_id AND prior_balance.rn = 1

WHERE dp.pb_ratio IS NOT NULL 
  AND dp.pb_ratio > 0
  AND dp.market_cap > 100000000; -- $100M minimum
```

#### 2. F-Score Calculation View
```sql
CREATE VIEW piotroski_f_score_calculation AS
SELECT 
    pfsd.*,
    
    -- F-Score Criteria (Binary: 1 if met, 0 if not)
    
    -- Profitability (4 points)
    CASE WHEN current_net_income > 0 THEN 1 ELSE 0 END as profitability_positive_net_income,
    CASE WHEN current_operating_cash_flow > 0 THEN 1 ELSE 0 END as profitability_positive_cash_flow,
    CASE WHEN current_roa > prior_roa AND current_roa IS NOT NULL AND prior_roa IS NOT NULL THEN 1 ELSE 0 END as profitability_improving_roa,
    CASE WHEN current_operating_cash_flow > current_net_income AND current_operating_cash_flow IS NOT NULL AND current_net_income IS NOT NULL THEN 1 ELSE 0 END as profitability_cash_flow_quality,
    
    -- Leverage/Liquidity (3 points)
    CASE WHEN current_debt_to_assets < prior_debt_to_assets AND current_debt_to_assets IS NOT NULL AND prior_debt_to_assets IS NOT NULL THEN 1 ELSE 0 END as leverage_decreasing_debt,
    CASE WHEN current_current_ratio > prior_current_ratio AND current_current_ratio IS NOT NULL AND prior_current_ratio IS NOT NULL THEN 1 ELSE 0 END as leverage_improving_liquidity,
    CASE WHEN current_shares_outstanding <= prior_shares_outstanding AND current_shares_outstanding IS NOT NULL AND prior_shares_outstanding IS NOT NULL THEN 1 ELSE 0 END as leverage_no_dilution,
    
    -- Operating Efficiency (2 points)
    CASE WHEN current_gross_margin > prior_gross_margin AND current_gross_margin IS NOT NULL AND prior_gross_margin IS NOT NULL THEN 1 ELSE 0 END as efficiency_improving_margin,
    CASE WHEN current_asset_turnover > prior_asset_turnover AND current_asset_turnover IS NOT NULL AND prior_asset_turnover IS NOT NULL THEN 1 ELSE 0 END as efficiency_improving_turnover,
    
    -- Total F-Score (0-9)
    (CASE WHEN current_net_income > 0 THEN 1 ELSE 0 END +
     CASE WHEN current_operating_cash_flow > 0 THEN 1 ELSE 0 END +
     CASE WHEN current_roa > prior_roa AND current_roa IS NOT NULL AND prior_roa IS NOT NULL THEN 1 ELSE 0 END +
     CASE WHEN current_operating_cash_flow > current_net_income AND current_operating_cash_flow IS NOT NULL AND current_net_income IS NOT NULL THEN 1 ELSE 0 END +
     CASE WHEN current_debt_to_assets < prior_debt_to_assets AND current_debt_to_assets IS NOT NULL AND prior_debt_to_assets IS NOT NULL THEN 1 ELSE 0 END +
     CASE WHEN current_current_ratio > prior_current_ratio AND current_current_ratio IS NOT NULL AND prior_current_ratio IS NOT NULL THEN 1 ELSE 0 END +
     CASE WHEN current_shares_outstanding <= prior_shares_outstanding AND current_shares_outstanding IS NOT NULL AND prior_shares_outstanding IS NOT NULL THEN 1 ELSE 0 END +
     CASE WHEN current_gross_margin > prior_gross_margin AND current_gross_margin IS NOT NULL AND prior_gross_margin IS NOT NULL THEN 1 ELSE 0 END +
     CASE WHEN current_asset_turnover > prior_asset_turnover AND current_asset_turnover IS NOT NULL AND prior_asset_turnover IS NOT NULL THEN 1 ELSE 0 END) as f_score,
    
    -- Data Completeness Score (0-100)
    CASE 
        WHEN current_net_income IS NOT NULL AND current_operating_cash_flow IS NOT NULL 
             AND current_roa IS NOT NULL AND prior_roa IS NOT NULL
             AND current_debt_to_assets IS NOT NULL AND prior_debt_to_assets IS NOT NULL
             AND current_current_ratio IS NOT NULL AND prior_current_ratio IS NOT NULL
             AND current_shares_outstanding IS NOT NULL AND prior_shares_outstanding IS NOT NULL
             AND current_gross_margin IS NOT NULL AND prior_gross_margin IS NOT NULL
             AND current_asset_turnover IS NOT NULL AND prior_asset_turnover IS NOT NULL THEN 100
        WHEN current_net_income IS NOT NULL AND current_operating_cash_flow IS NOT NULL 
             AND current_roa IS NOT NULL AND prior_roa IS NOT NULL
             AND current_debt_to_assets IS NOT NULL AND prior_debt_to_assets IS NOT NULL
             AND current_current_ratio IS NOT NULL AND prior_current_ratio IS NOT NULL THEN 75
        WHEN current_net_income IS NOT NULL AND current_operating_cash_flow IS NOT NULL 
             AND (current_roa IS NOT NULL OR current_debt_to_assets IS NOT NULL) THEN 50
        ELSE 25
    END as data_completeness_score

FROM piotroski_f_score_data pfsd
WHERE pfsd.current_net_income IS NOT NULL 
  AND pfsd.current_operating_cash_flow IS NOT NULL;
```

## 🔧 Backend Implementation

### 1. Data Models (`src-tauri/src/models/piotroski.rs`)
```rust
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PiotroskiFScoreResult {
    pub stock_id: i32,
    pub symbol: String,
    pub sector: Option<String>,
    pub industry: Option<String>,
    
    // Market Metrics
    pub current_price: f64,
    pub market_cap: f64,
    pub pb_ratio: f64,
    
    // F-Score Components
    pub profitability_positive_net_income: i32,
    pub profitability_positive_cash_flow: i32,
    pub profitability_improving_roa: i32,
    pub profitability_cash_flow_quality: i32,
    pub leverage_decreasing_debt: i32,
    pub leverage_improving_liquidity: i32,
    pub leverage_no_dilution: i32,
    pub efficiency_improving_margin: i32,
    pub efficiency_improving_turnover: i32,
    
    // Calculated Metrics
    pub f_score: i32,
    pub data_completeness_score: i32,
    pub passes_piotroski_screening: bool,
    
    // Financial Ratios (Current)
    pub current_roa: Option<f64>,
    pub current_debt_to_assets: Option<f64>,
    pub current_current_ratio: Option<f64>,
    pub current_gross_margin: Option<f64>,
    pub current_asset_turnover: Option<f64>,
    
    // Financial Ratios (Prior)
    pub prior_roa: Option<f64>,
    pub prior_debt_to_assets: Option<f64>,
    pub prior_current_ratio: Option<f64>,
    pub prior_gross_margin: Option<f64>,
    pub prior_asset_turnover: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiotroskiScreeningCriteria {
    #[serde(rename = "maxPbRatio")]
    pub max_pb_ratio: f64,           // Default: 1.0
    #[serde(rename = "minMarketCap")]
    pub min_market_cap: f64,        // Default: $100M
    #[serde(rename = "minFScore")]
    pub min_f_score: i32,           // Default: 8
    #[serde(rename = "minDataCompleteness")]
    pub min_data_completeness: i32, // Default: 75
    #[serde(rename = "maxResults")]
    pub max_results: i32,           // Default: 20
}

impl Default for PiotroskiScreeningCriteria {
    fn default() -> Self {
        Self {
            max_pb_ratio: 1.0,
            min_market_cap: 100_000_000.0,  // $100M
            min_f_score: 8,
            min_data_completeness: 75,
            max_results: 20,
        }
    }
}
```

### 2. Tauri Command (`src-tauri/src/commands/piotroski.rs`)
```rust
use crate::models::piotroski::{PiotroskiFScoreResult, PiotroskiScreeningCriteria};
use crate::database::helpers::get_database_connection;

#[tauri::command]
pub async fn get_piotroski_f_score_results(
    stock_tickers: Vec<String>, 
    criteria: Option<PiotroskiScreeningCriteria>,
    limit: Option<i32>
) -> Result<Vec<PiotroskiFScoreResult>, String> {
    let pool = get_database_connection().await?;
    let criteria = criteria.unwrap_or_default();
    let limit_value = limit.unwrap_or(criteria.max_results);
    
    if stock_tickers.is_empty() {
        return Ok(vec![]);
    }
    
    let placeholders = stock_tickers.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    
    let query = format!("
        SELECT 
            pfsc.stock_id,
            pfsc.symbol,
            pfsc.sector,
            pfsc.industry,
            pfsc.current_price,
            pfsc.market_cap,
            pfsc.pb_ratio,
            
            -- F-Score Components
            pfsc.profitability_positive_net_income,
            pfsc.profitability_positive_cash_flow,
            pfsc.profitability_improving_roa,
            pfsc.profitability_cash_flow_quality,
            pfsc.leverage_decreasing_debt,
            pfsc.leverage_improving_liquidity,
            pfsc.leverage_no_dilution,
            pfsc.efficiency_improving_margin,
            pfsc.efficiency_improving_turnover,
            
            -- Calculated Metrics
            pfsc.f_score,
            pfsc.data_completeness_score,
            
            -- Final Screening Result
            CASE 
                WHEN pfsc.f_score >= ? 
                     AND pfsc.data_completeness_score >= ?
                     AND pfsc.pb_ratio <= ?
                     AND pfsc.market_cap >= ?
                THEN true
                ELSE false
            END as passes_piotroski_screening,
            
            -- Financial Ratios (Current)
            pfsc.current_roa,
            pfsc.current_debt_to_assets,
            pfsc.current_current_ratio,
            pfsc.current_gross_margin,
            pfsc.current_asset_turnover,
            
            -- Financial Ratios (Prior)
            pfsc.prior_roa,
            pfsc.prior_debt_to_assets,
            pfsc.prior_current_ratio,
            pfsc.prior_gross_margin,
            pfsc.prior_asset_turnover
            
        FROM piotroski_f_score_calculation pfsc
        WHERE pfsc.symbol IN ({})
        ORDER BY 
            passes_piotroski_screening DESC,
            pfsc.f_score DESC,
            pfsc.data_completeness_score DESC,
            pfsc.pb_ratio ASC
        LIMIT ?
    ", placeholders);
    
    let mut query_builder = sqlx::query_as::<_, PiotroskiFScoreResult>(&query);
    
    // Bind parameters
    query_builder = query_builder.bind(criteria.min_f_score);
    query_builder = query_builder.bind(criteria.min_data_completeness);
    query_builder = query_builder.bind(criteria.max_pb_ratio);
    query_builder = query_builder.bind(criteria.min_market_cap);
    
    // Bind stock tickers
    for ticker in &stock_tickers {
        query_builder = query_builder.bind(ticker);
    }
    
    query_builder = query_builder.bind(limit_value);
    
    let results = query_builder.fetch_all(&pool).await
        .map_err(|e| format!("Database error: {}", e))?;
    
    Ok(results)
}
```

## 🎨 Frontend Implementation

### 1. Store Integration (`src/stores/piotroskiStore.ts`)
```typescript
import { createSignal, createEffect } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import type { PiotroskiFScoreResult, PiotroskiScreeningCriteria } from '../utils/types';

export function createPiotroskiStore() {
  const [results, setResults] = createSignal<PiotroskiFScoreResult[]>([]);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [criteria, setCriteria] = createSignal<PiotroskiScreeningCriteria>({
    maxPbRatio: 1.0,
    minMarketCap: 100_000_000,
    minFScore: 8,
    minDataCompleteness: 75,
    maxResults: 20
  });

  const loadPiotroskiResults = async (stockTickers: string[]) => {
    if (stockTickers.length === 0) {
      setResults([]);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const results = await invoke<PiotroskiFScoreResult[]>('get_piotroski_f_score_results', {
        stockTickers,
        criteria: criteria(),
        limit: criteria().maxResults
      });
      
      setResults(results);
    } catch (err) {
      setError(err as string);
      setResults([]);
    } finally {
      setLoading(false);
    }
  };

  const updateCriteria = (updates: Partial<PiotroskiScreeningCriteria>) => {
    setCriteria(prev => ({ ...prev, ...updates }));
  };

  return {
    results,
    loading,
    error,
    criteria,
    loadPiotroskiResults,
    updateCriteria
  };
}

export const piotroskiStore = createPiotroskiStore();
```

### 2. UI Component (`src/components/PiotroskiPanel.tsx`)
```typescript
import { createSignal, createEffect, Show } from 'solid-js';
import { piotroskiStore } from '../stores/piotroskiStore';
import { stockStore } from '../stores/stockStore';

export default function PiotroskiPanel() {
  const [showDetails, setShowDetails] = createSignal(false);

  createEffect(() => {
    const sp500Symbols = stockStore.sp500Symbols();
    if (sp500Symbols.length > 0) {
      piotroskiStore.loadPiotroskiResults(sp500Symbols);
    }
  });

  const getFScoreColor = (score: number) => {
    if (score >= 8) return 'text-green-600 bg-green-100';
    if (score >= 6) return 'text-yellow-600 bg-yellow-100';
    return 'text-red-600 bg-red-100';
  };

  const getCriteriaStatus = (result: PiotroskiFScoreResult) => {
    const criteria = [
      { name: 'Positive Net Income', passed: result.profitability_positive_net_income === 1 },
      { name: 'Positive Cash Flow', passed: result.profitability_positive_cash_flow === 1 },
      { name: 'Improving ROA', passed: result.profitability_improving_roa === 1 },
      { name: 'Cash Flow Quality', passed: result.profitability_cash_flow_quality === 1 },
      { name: 'Decreasing Debt', passed: result.leverage_decreasing_debt === 1 },
      { name: 'Improving Liquidity', passed: result.leverage_improving_liquidity === 1 },
      { name: 'No Dilution', passed: result.leverage_no_dilution === 1 },
      { name: 'Improving Margin', passed: result.efficiency_improving_margin === 1 },
      { name: 'Improving Turnover', passed: result.efficiency_improving_turnover === 1 }
    ];
    return criteria;
  };

  return (
    <div class="bg-white rounded-lg shadow-lg p-6">
      <div class="flex items-center justify-between mb-6">
        <h2 class="text-2xl font-bold text-gray-800">
          📊 Piotroski F-Score Screening
        </h2>
        <button
          onClick={() => setShowDetails(!showDetails())}
          class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
        >
          {showDetails() ? 'Hide Details' : 'Show Details'}
        </button>
      </div>

      <div class="mb-6 p-4 bg-gray-50 rounded-lg">
        <h3 class="font-semibold text-gray-700 mb-2">Strategy Overview</h3>
        <p class="text-sm text-gray-600">
          Identifies undervalued stocks (P/B ≤ 1.0) with improving fundamentals using 9 binary criteria.
          F-Score ≥ 8 indicates high-quality value opportunities.
        </p>
      </div>

      <Show when={piotroskiStore.loading()}>
        <div class="flex items-center justify-center py-8">
          <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
          <span class="ml-2 text-gray-600">Calculating F-Scores...</span>
        </div>
      </Show>

      <Show when={piotroskiStore.error()}>
        <div class="bg-red-50 border border-red-200 rounded-lg p-4 mb-6">
          <p class="text-red-800">{piotroskiStore.error()}</p>
        </div>
      </Show>

      <Show when={!piotroskiStore.loading() && !piotroskiStore.error()}>
        <div class="space-y-4">
          <For each={piotroskiStore.results()}>
            {(result) => (
              <div class="border border-gray-200 rounded-lg p-4 hover:shadow-md transition-shadow">
                <div class="flex items-center justify-between mb-3">
                  <div class="flex items-center space-x-4">
                    <h4 class="font-semibold text-lg">{result.symbol}</h4>
                    <span class={`px-3 py-1 rounded-full text-sm font-medium ${getFScoreColor(result.f_score)}`}>
                      F-Score: {result.f_score}/9
                    </span>
                    <span class="text-sm text-gray-600">P/B: {result.pb_ratio.toFixed(2)}</span>
                  </div>
                  <div class="text-right">
                    <p class="text-sm text-gray-600">Market Cap: ${(result.market_cap / 1_000_000).toFixed(0)}M</p>
                    <p class="text-sm text-gray-600">Data Quality: {result.data_completeness_score}%</p>
                  </div>
                </div>

                <Show when={showDetails()}>
                  <div class="mt-4 pt-4 border-t border-gray-200">
                    <h5 class="font-medium text-gray-700 mb-3">F-Score Breakdown</h5>
                    <div class="grid grid-cols-3 gap-4">
                      <For each={getCriteriaStatus(result)}>
                        {(criterion) => (
                          <div class={`p-2 rounded text-sm ${criterion.passed ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}`}>
                            {criterion.passed ? '✅' : '❌'} {criterion.name}
                          </div>
                        )}
                      </For>
                    </div>
                  </div>
                </Show>
              </div>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}
```

## 🧪 Testing Strategy

### 1. Backend Tests (`src-tauri/tests/backend_tests.rs`)
```rust
#[tokio::test]
async fn test_piotroski_f_score_screening() {
    let test_db = SimpleTestDatabase::new().await.unwrap();
    set_test_database_pool(test_db.pool().clone()).await;
    
    // Test with S&P 500 symbols
    let sp500_symbols = vec!["AAPL".to_string(), "MSFT".to_string(), "GOOGL".to_string()];
    
    let result = get_piotroski_f_score_results(sp500_symbols, None, Some(10)).await
        .expect("Piotroski screening should work");
    
    // Verify results structure
    assert!(!result.is_empty(), "Should return some results");
    
    for stock in &result {
        assert!(stock.f_score >= 0 && stock.f_score <= 9, "F-Score should be 0-9");
        assert!(stock.pb_ratio > 0.0, "P/B ratio should be positive");
        assert!(stock.market_cap > 0.0, "Market cap should be positive");
    }
    
    clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}
```

### 2. Integration Tests
```rust
#[tokio::test]
async fn test_piotroski_data_completeness() {
    let test_db = SimpleTestDatabase::new().await.unwrap();
    set_test_database_pool(test_db.pool().clone()).await;
    
    // Test data completeness requirements
    let result = get_piotroski_f_score_results(vec!["AAPL".to_string()], None, Some(1)).await
        .expect("Should work for Apple");
    
    if !result.is_empty() {
        let apple = &result[0];
        assert!(apple.data_completeness_score >= 50, "Should have reasonable data completeness");
    }
    
    clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}
```

## 📈 Performance Considerations

### 1. Database Indexing
```sql
-- Performance indexes for Piotroski screening
CREATE INDEX idx_income_statements_piotroski 
ON income_statements(stock_id, period_type, report_date, net_income, operating_cash_flow);

CREATE INDEX idx_balance_sheets_piotroski 
ON balance_sheets(stock_id, period_type, report_date, total_assets, long_term_debt, current_assets, current_liabilities);

CREATE INDEX idx_daily_ratios_pb_piotroski 
ON daily_valuation_ratios(stock_id, date, pb_ratio) WHERE pb_ratio <= 1.0;
```

### 2. Query Optimization
- **View Materialization**: Consider materialized views for large datasets
- **Batch Processing**: Process stocks in batches for memory efficiency
- **Caching**: Cache F-Score calculations for frequently accessed stocks

## 🚀 Implementation Phases

### Phase 1: Database Foundation (Day 1)
1. Create migration file with views and indexes
2. Test database views with sample data
3. Validate F-Score calculations against known examples

### Phase 2: Backend Implementation (Day 1-2)
1. Implement data models and Tauri command
2. Add comprehensive error handling
3. Create backend tests

### Phase 3: Frontend Integration (Day 2-3)
1. Create store and UI components
2. Integrate with existing screening panel
3. Add responsive design and user interactions

### Phase 4: Testing & Validation (Day 3-4)
1. End-to-end testing
2. Performance optimization
3. Data quality validation

### Phase 5: Production Deployment (Day 4)
1. Database migration execution
2. Frontend deployment
3. User acceptance testing

## 🎯 Success Criteria

### Technical Requirements
- ✅ F-Score calculation accuracy (validated against academic paper)
- ✅ Database performance (< 2 seconds for S&P 500 screening)
- ✅ Frontend responsiveness (smooth UI interactions)
- ✅ Data completeness (> 80% for S&P 500 stocks)

### Business Requirements
- ✅ Identifies high-quality value stocks (F-Score ≥ 8)
- ✅ Filters out value traps (poor fundamentals)
- ✅ Provides actionable investment insights
- ✅ Integrates seamlessly with existing screening tools

This architecture provides a robust, scalable foundation for Piotroski F-Score screening while maintaining consistency with our existing codebase patterns and ensuring high performance with large datasets.
