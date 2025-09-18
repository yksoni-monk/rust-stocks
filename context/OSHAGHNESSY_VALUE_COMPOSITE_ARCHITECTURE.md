# O'Shaughnessy Value Composite Screening - Critical Analysis & Implementation Plan

## 🚨 Critical Data Gaps Analysis

### **Missing Data Fields (CRITICAL BLOCKERS)**

#### 1. Operating Cash Flow Per Share
**Current Status**: `operating_cash_flow` field exists in `income_statements` but data quality is poor
**Impact**: Cannot calculate P/CF ratio (1 of 6 metrics)
**Required Action**: 
- Implement EDGAR cash flow statement extraction
- Add `cash_flow_statements` table
- Validate data completeness > 80% for S&P 500

#### 2. EBITDA Calculation
**Current Status**: Using `operating_cash_flow` as EBITDA proxy (WRONG)
**Impact**: EV/EBITDA calculations are inaccurate
**Required Action**:
- Extract EBITDA from income statements
- Add proper EBITDA calculation: `operating_income + depreciation + amortization`
- Handle cases where depreciation/amortization not available

#### 3. Share Buyback Data
**Current Status**: No share buyback tracking
**Impact**: Shareholder Yield calculation incomplete (only dividends)
**Required Action**:
- Track shares outstanding changes over time
- Calculate net buybacks: `(shares_prior - shares_current) * avg_price`
- Add buyback tracking to `daily_valuation_ratios`

#### 4. Short-term Debt
**Current Status**: `short_term_debt` field missing from `balance_sheets`
**Impact**: Enterprise Value calculation incomplete
**Required Action**:
- Add `short_term_debt` to balance sheet schema
- Extract from EDGAR balance sheet data
- Update EV calculation: `market_cap + total_debt - cash`

#### 5. Dividend Data Quality
**Current Status**: `dividend_per_share` in `daily_valuation_ratios` is unreliable
**Impact**: Shareholder Yield calculations inaccurate
**Required Action**:
- Extract annual dividend data from EDGAR
- Add `dividend_history` table
- Calculate trailing 12-month dividends

### **Data Completeness Assessment**

| **Metric** | **Current Coverage** | **Required Coverage** | **Status** |
|------------|---------------------|----------------------|------------|
| P/E Ratio | 95% | 90% | ✅ Good |
| P/B Ratio | 90% | 90% | ✅ Good |
| P/S Ratio | 95% | 90% | ✅ Good |
| P/CF Ratio | 30% | 80% | ❌ **CRITICAL** |
| EV/EBITDA | 25% | 80% | ❌ **CRITICAL** |
| Shareholder Yield | 60% | 70% | ⚠️ Needs Work |

**Overall Data Completeness**: ~65% (INSUFFICIENT for production)

## 🔧 Required Database Schema Changes

### **New Tables Required**

#### 1. Cash Flow Statements Table
```sql
CREATE TABLE cash_flow_statements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    period_type TEXT NOT NULL,
    report_date DATE NOT NULL,
    fiscal_year INTEGER,
    
    -- Cash Flow Metrics
    operating_cash_flow REAL,
    investing_cash_flow REAL,
    financing_cash_flow REAL,
    net_cash_flow REAL,
    
    -- EBITDA Components
    depreciation_expense REAL,
    amortization_expense REAL,
    
    -- Import metadata
    data_source TEXT DEFAULT 'edgar',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, period_type, report_date)
);
```

#### 2. Dividend History Table
```sql
CREATE TABLE dividend_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    ex_date DATE NOT NULL,
    payment_date DATE,
    dividend_per_share REAL,
    dividend_type TEXT DEFAULT 'regular', -- 'regular', 'special', 'stock_split'
    
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, ex_date)
);
```

#### 3. Share Buyback Tracking Table
```sql
CREATE TABLE share_buyback_tracking (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    report_date DATE NOT NULL,
    shares_outstanding REAL,
    shares_repurchased REAL,
    shares_issued REAL,
    net_shares_change REAL,
    
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, report_date)
);
```

### **Schema Extensions Required**

#### Balance Sheet Extensions
```sql
-- Add missing fields to existing balance_sheets table
ALTER TABLE balance_sheets ADD COLUMN short_term_debt REAL;
ALTER TABLE balance_sheets ADD COLUMN current_assets REAL;
ALTER TABLE balance_sheets ADD COLUMN current_liabilities REAL;
```

#### Income Statement Extensions
```sql
-- Add EBITDA components to existing income_statements table
ALTER TABLE income_statements ADD COLUMN depreciation_expense REAL;
ALTER TABLE income_statements ADD COLUMN amortization_expense REAL;
ALTER TABLE income_statements ADD COLUMN interest_expense REAL;
```

## 📊 Data Acquisition Plan

### **Phase 1: EDGAR Integration (CRITICAL - Week 1)**

#### 1.1 Cash Flow Statement Extraction
```rust
// New EDGAR extractor for cash flow statements
pub struct CashFlowExtractor {
    client: reqwest::Client,
    rate_limiter: RateLimiter,
}

impl CashFlowExtractor {
    pub async fn extract_cash_flow_data(&self, symbol: &str) -> Result<CashFlowData, Error> {
        // Extract 10-K/10-Q cash flow statements
        // Parse operating_cash_flow, depreciation, amortization
        // Handle different reporting formats
    }
}
```

#### 1.2 Enhanced Balance Sheet Extraction
```rust
// Extend existing balance sheet extractor
pub struct EnhancedBalanceSheetExtractor {
    // Add short_term_debt extraction
    // Add current_assets/liabilities extraction
    // Validate data consistency
}
```

#### 1.3 Dividend Data Extraction
```rust
// New dividend extractor
pub struct DividendExtractor {
    pub async fn extract_dividend_history(&self, symbol: &str) -> Result<Vec<DividendRecord>, Error> {
        // Extract from 10-K/10-Q filings
        // Calculate trailing 12-month dividends
        // Handle special dividends and stock splits
    }
}
```

### **Phase 2: Data Validation & Quality Control (Week 2)**

#### 2.1 Data Completeness Validation
```rust
pub struct DataCompletenessValidator {
    pub async fn validate_oshaughnessy_data(&self) -> DataCompletenessReport {
        // Check coverage for each of 6 metrics
        // Identify missing data patterns
        // Generate data quality scores
    }
}
```

#### 2.2 Cross-Validation Logic
```rust
pub struct CrossValidator {
    pub async fn validate_financial_consistency(&self, stock_id: i32) -> ValidationResult {
        // Validate: operating_cash_flow vs net_income
        // Validate: total_debt = short_term + long_term
        // Validate: shares_outstanding consistency
    }
}
```

## 🔧 Backend API Implementation

### **Enhanced Data Models**

#### 1. Data Quality Tracking
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OShaughnessyDataQuality {
    pub stock_id: i32,
    pub symbol: String,
    
    // Individual metric availability
    pub has_pe_data: bool,
    pub has_pb_data: bool,
    pub has_ps_data: bool,
    pub has_pcf_data: bool,
    pub has_ev_ebitda_data: bool,
    pub has_shareholder_yield_data: bool,
    
    // Data freshness
    pub last_financial_update: Option<chrono::NaiveDate>,
    pub last_price_update: Option<chrono::NaiveDate>,
    
    // Quality scores
    pub overall_completeness_score: i32, // 0-100
    pub data_freshness_score: i32,        // 0-100
    pub cross_validation_score: i32,     // 0-100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OShaughnessyScreeningResult {
    // ... existing fields ...
    
    // Enhanced data quality
    pub data_quality: OShaughnessyDataQuality,
    
    // Missing data indicators
    pub missing_metrics: Vec<String>,
    pub data_warnings: Vec<String>,
    
    // Confidence scoring
    pub confidence_score: f64, // 0.0-1.0
    pub reliability_rating: String, // "High", "Medium", "Low"
}
```

#### 2. Enhanced Screening Criteria
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OShaughnessyScreeningCriteria {
    // ... existing fields ...
    
    // Data quality requirements
    #[serde(rename = "minDataCompleteness")]
    pub min_data_completeness: i32, // Default: 80 (increased from 75)
    
    #[serde(rename = "minConfidenceScore")]
    pub min_confidence_score: f64, // Default: 0.7
    
    #[serde(rename = "requireAllMetrics")]
    pub require_all_metrics: bool, // Default: false (allow partial data)
    
    #[serde(rename = "maxDataAgeDays")]
    pub max_data_age_days: i32, // Default: 90
    
    // Fallback strategies
    #[serde(rename = "useEbitdaProxy")]
    pub use_ebitda_proxy: bool, // Default: true
    
    #[serde(rename = "useDividendOnlyYield")]
    pub use_dividend_only_yield: bool, // Default: true
}
```

### **Enhanced Tauri Commands**

#### 1. Data Quality Assessment Command
```rust
#[tauri::command]
pub async fn assess_oshaughnessy_data_quality(
    stock_tickers: Vec<String>
) -> Result<Vec<OShaughnessyDataQuality>, String> {
    // Analyze data completeness for each stock
    // Return detailed quality metrics
    // Identify data gaps and recommendations
}
```

#### 2. Enhanced Screening Command
```rust
#[tauri::command]
pub async fn get_oshaughnessy_value_composite_results_enhanced(
    stock_tickers: Vec<String>,
    criteria: Option<OShaughnessyScreeningCriteria>,
    include_partial_data: bool
) -> Result<Vec<OShaughnessyScreeningResult>, String> {
    // Enhanced screening with data quality awareness
    // Handle missing metrics gracefully
    // Provide confidence scoring
    // Include data quality warnings
}
```

#### 3. Data Refresh Command
```rust
#[tauri::command]
pub async fn refresh_oshaughnessy_data(
    stock_tickers: Vec<String>,
    force_refresh: bool
) -> Result<DataRefreshReport, String> {
    // Trigger data refresh for specific stocks
    // Update missing financial data
    // Return refresh status and new data quality scores
}
```

### **Value Composite Ranking View**
```sql
CREATE VIEW oshaughnessy_value_composite_ranking AS
WITH value_metrics AS (
    SELECT 
        ovcd.*,
        
        -- Calculate the 6 Value Composite Metrics
        CASE 
            WHEN eps > 0 THEN current_price / eps
            ELSE NULL 
        END as pe_ratio,
        
        CASE 
            WHEN book_value_per_share > 0 THEN current_price / book_value_per_share
            ELSE NULL 
        END as pb_ratio,
        
        CASE 
            WHEN sales_per_share > 0 THEN current_price / sales_per_share
            ELSE NULL 
        END as ps_ratio,
        
        CASE 
            WHEN cash_flow_per_share > 0 THEN current_price / cash_flow_per_share
            ELSE NULL 
        END as pcf_ratio,
        
        CASE 
            WHEN ebitda > 0 AND enterprise_value IS NOT NULL 
            THEN enterprise_value / ebitda
            ELSE NULL 
        END as ev_ebitda_ratio,
        
        dividend_yield_percent as shareholder_yield
        
    FROM oshaughnessy_value_composite_data ovcd
    WHERE ovcd.eps > 0 
      AND ovcd.book_value_per_share > 0
      AND ovcd.sales_per_share > 0
      AND ovcd.cash_flow_per_share > 0
      AND ovcd.ebitda > 0
),
ranked_metrics AS (
    SELECT 
        vm.*,
        
        -- Rank each metric (1 = best value)
        RANK() OVER (ORDER BY pe_ratio ASC) as pe_rank,
        RANK() OVER (ORDER BY pb_ratio ASC) as pb_rank,
        RANK() OVER (ORDER BY ps_ratio ASC) as ps_rank,
        RANK() OVER (ORDER BY pcf_ratio ASC) as pcf_rank,
        RANK() OVER (ORDER BY ev_ebitda_ratio ASC) as ev_ebitda_rank,
        RANK() OVER (ORDER BY shareholder_yield DESC) as shareholder_yield_rank
        
    FROM value_metrics vm
    WHERE vm.pe_ratio IS NOT NULL 
      AND vm.pb_ratio IS NOT NULL
      AND vm.ps_ratio IS NOT NULL
      AND vm.pcf_ratio IS NOT NULL
      AND vm.ev_ebitda_ratio IS NOT NULL
      AND vm.shareholder_yield IS NOT NULL
)
SELECT 
    rm.*,
    
    -- Calculate Composite Score (average of ranks)
    (rm.pe_rank + rm.pb_rank + rm.ps_rank + rm.pcf_rank + rm.ev_ebitda_rank + rm.shareholder_yield_rank) / 6.0 as composite_score,
    
    -- Calculate Percentile Ranks (0-100, lower = better)
    PERCENT_RANK() OVER (ORDER BY (rm.pe_rank + rm.pb_rank + rm.ps_rank + rm.pcf_rank + rm.ev_ebitda_rank + rm.shareholder_yield_rank) / 6.0) * 100 as composite_percentile,
    
    -- Overall Ranking
    RANK() OVER (ORDER BY (rm.pe_rank + rm.pb_rank + rm.ps_rank + rm.pcf_rank + rm.ev_ebitda_rank + rm.shareholder_yield_rank) / 6.0 ASC) as overall_rank

FROM ranked_metrics rm;
```

## 🔧 Backend Implementation

### **Data Models (`src-tauri/src/models/oshaughnessy.rs`)**
```rust
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OShaughnessyValueCompositeResult {
    pub stock_id: i32,
    pub symbol: String,
    pub sector: Option<String>,
    pub industry: Option<String>,
    
    // Market Metrics
    pub current_price: f64,
    pub market_cap: f64,
    pub shares_outstanding: f64,
    
    // Value Composite Metrics
    pub pe_ratio: Option<f64>,
    pub pb_ratio: Option<f64>,
    pub ps_ratio: Option<f64>,
    pub pcf_ratio: Option<f64>,
    pub ev_ebitda_ratio: Option<f64>,
    pub shareholder_yield: f64,
    
    // Ranking Data
    pub pe_rank: i32,
    pub pb_rank: i32,
    pub ps_rank: i32,
    pub pcf_rank: i32,
    pub ev_ebitda_rank: i32,
    pub shareholder_yield_rank: i32,
    
    // Composite Scoring
    pub composite_score: f64,
    pub composite_percentile: f64,
    pub overall_rank: i32,
    
    // Data Quality
    pub data_completeness_score: i32,
    pub passes_value_composite_screening: bool,
    
    // Raw Financial Data
    pub eps: Option<f64>,
    pub book_value_per_share: Option<f64>,
    pub sales_per_share: Option<f64>,
    pub cash_flow_per_share: Option<f64>,
    pub ebitda: Option<f64>,
    pub enterprise_value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OShaughnessyScreeningCriteria {
    #[serde(rename = "minMarketCap")]
    pub min_market_cap: f64,        // Default: $200M
    #[serde(rename = "maxCompositePercentile")]
    pub max_composite_percentile: f64, // Default: 20.0 (top 20%)
    #[serde(rename = "minDataCompleteness")]
    pub min_data_completeness: i32, // Default: 75
    #[serde(rename = "maxResults")]
    pub max_results: i32,           // Default: 50
    #[serde(rename = "requirePositiveEarnings")]
    pub require_positive_earnings: bool, // Default: true
    #[serde(rename = "requirePositiveCashFlow")]
    pub require_positive_cash_flow: bool, // Default: true
}

impl Default for OShaughnessyScreeningCriteria {
    fn default() -> Self {
        Self {
            min_market_cap: 200_000_000.0,   // $200M
            max_composite_percentile: 20.0,  // Top 20%
            min_data_completeness: 75,
            max_results: 50,
            require_positive_earnings: true,
            require_positive_cash_flow: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OShaughnessyScreeningSummary {
    pub total_stocks_analyzed: i32,
    pub stocks_passing_screening: i32,
    pub average_composite_score: f64,
    pub average_composite_percentile: f64,
    pub average_data_completeness: f64,
    pub screening_criteria: OShaughnessyScreeningCriteria,
    pub execution_time_ms: u64,
}
```

### **Tauri Command (`src-tauri/src/commands/oshaughnessy.rs`)**
```rust
use crate::models::oshaughnessy::{OShaughnessyValueCompositeResult, OShaughnessyScreeningCriteria, OShaughnessyScreeningSummary};
use crate::database::helpers::get_database_connection;
use std::time::Instant;

#[tauri::command]
pub async fn get_oshaughnessy_value_composite_results(
    stock_tickers: Vec<String>, 
    criteria: Option<OShaughnessyScreeningCriteria>,
    limit: Option<i32>
) -> Result<Vec<OShaughnessyValueCompositeResult>, String> {
    let start_time = Instant::now();
    let pool = get_database_connection().await?;
    let criteria = criteria.unwrap_or_default();
    let limit_value = limit.unwrap_or(criteria.max_results);
    
    if stock_tickers.is_empty() {
        return Ok(vec![]);
    }
    
    let placeholders = stock_tickers.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    
    let query = format!("
        SELECT 
            ovcr.stock_id,
            ovcr.symbol,
            ovcr.sector,
            ovcr.industry,
            ovcr.current_price,
            ovcr.market_cap,
            ovcr.shares_outstanding,
            
            -- Value Composite Metrics
            ovcr.pe_ratio,
            ovcr.pb_ratio,
            ovcr.ps_ratio,
            ovcr.pcf_ratio,
            ovcr.ev_ebitda_ratio,
            ovcr.shareholder_yield,
            
            -- Ranking Data
            ovcr.pe_rank,
            ovcr.pb_rank,
            ovcr.ps_rank,
            ovcr.pcf_rank,
            ovcr.ev_ebitda_rank,
            ovcr.shareholder_yield_rank,
            
            -- Composite Scoring
            ovcr.composite_score,
            ovcr.composite_percentile,
            ovcr.overall_rank,
            
            -- Data Quality
            ovcr.data_completeness_score,
            
            -- Final Screening Result
            CASE 
                WHEN ovcr.composite_percentile <= ?
                     AND ovcr.data_completeness_score >= ?
                     AND ovcr.market_cap >= ?
                THEN true
                ELSE false
            END as passes_value_composite_screening,
            
            -- Raw Financial Data
            ovcr.eps,
            ovcr.book_value_per_share,
            ovcr.sales_per_share,
            ovcr.cash_flow_per_share,
            ovcr.ebitda,
            ovcr.enterprise_value
            
        FROM oshaughnessy_value_composite_ranking ovcr
        WHERE ovcr.symbol IN ({})
        ORDER BY 
            passes_value_composite_screening DESC,
            ovcr.overall_rank ASC,
            ovcr.composite_score ASC
        LIMIT ?
    ", placeholders);
    
    let mut query_builder = sqlx::query_as::<_, OShaughnessyValueCompositeResult>(&query);
    
    // Bind parameters
    query_builder = query_builder.bind(criteria.max_composite_percentile);
    query_builder = query_builder.bind(criteria.min_data_completeness);
    query_builder = query_builder.bind(criteria.min_market_cap);
    
    // Bind stock tickers
    for ticker in &stock_tickers {
        query_builder = query_builder.bind(ticker);
    }
    
    query_builder = query_builder.bind(limit_value);
    
    let results = query_builder.fetch_all(&pool).await
        .map_err(|e| format!("Database error: {}", e))?;
    
    let execution_time = start_time.elapsed().as_millis() as u64;
    
    log::info!("O'Shaughnessy Value Composite screening completed in {}ms for {} stocks", 
               execution_time, stock_tickers.len());
    
    Ok(results)
}

#[tauri::command]
pub async fn get_oshaughnessy_screening_summary(
    stock_tickers: Vec<String>,
    criteria: Option<OShaughnessyScreeningCriteria>
) -> Result<OShaughnessyScreeningSummary, String> {
    let start_time = Instant::now();
    let results = get_oshaughnessy_value_composite_results(stock_tickers.clone(), criteria.clone(), None).await?;
    let execution_time = start_time.elapsed().as_millis() as u64;
    
    let criteria = criteria.unwrap_or_default();
    let total_stocks = results.len() as i32;
    let passing_stocks = results.iter().filter(|r| r.passes_value_composite_screening).count() as i32;
    
    let avg_composite_score = if total_stocks > 0 {
        results.iter().map(|r| r.composite_score).sum::<f64>() / total_stocks as f64
    } else { 0.0 };
    
    let avg_composite_percentile = if total_stocks > 0 {
        results.iter().map(|r| r.composite_percentile).sum::<f64>() / total_stocks as f64
    } else { 0.0 };
    
    let avg_data_completeness = if total_stocks > 0 {
        results.iter().map(|r| r.data_completeness_score as f64).sum::<f64>() / total_stocks as f64
    } else { 0.0 };
    
    Ok(OShaughnessyScreeningSummary {
        total_stocks_analyzed: total_stocks,
        stocks_passing_screening: passing_stocks,
        average_composite_score: avg_composite_score,
        average_composite_percentile: avg_composite_percentile,
        average_data_completeness: avg_data_completeness,
        screening_criteria: criteria,
        execution_time_ms: execution_time,
    })
}
```

## 🎨 Frontend Implementation

### **Store Integration (`src/stores/oshaughnessyStore.ts`)**
```typescript
import { createSignal, createEffect } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import type { OShaughnessyValueCompositeResult, OShaughnessyScreeningCriteria } from '../utils/types';

export function createOShaughnessyStore() {
  const [results, setResults] = createSignal<OShaughnessyValueCompositeResult[]>([]);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [criteria, setCriteria] = createSignal<OShaughnessyScreeningCriteria>({
    minMarketCap: 200_000_000,
    maxCompositePercentile: 20.0,
    minDataCompleteness: 75,
    maxResults: 50,
    requirePositiveEarnings: true,
    requirePositiveCashFlow: true
  });

  const loadOShaughnessyResults = async (stockTickers: string[]) => {
    if (stockTickers.length === 0) {
      setResults([]);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const results = await invoke<OShaughnessyValueCompositeResult[]>('get_oshaughnessy_value_composite_results', {
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

  const updateCriteria = (updates: Partial<OShaughnessyScreeningCriteria>) => {
    setCriteria(prev => ({ ...prev, ...updates }));
  };

  return {
    results,
    loading,
    error,
    criteria,
    loadOShaughnessyResults,
    updateCriteria
  };
}

export const oshaughnessyStore = createOShaughnessyStore();
```

### **UI Component (`src/components/OShaughnessyPanel.tsx`)**
```typescript
import { createSignal, createEffect, Show, For } from 'solid-js';
import { oshaughnessyStore } from '../stores/oshaughnessyStore';
import { stockStore } from '../stores/stockStore';

export default function OShaughnessyPanel() {
  const [showDetails, setShowDetails] = createSignal(false);

  createEffect(() => {
    const sp500Symbols = stockStore.sp500Symbols();
    if (sp500Symbols.length > 0) {
      oshaughnessyStore.loadOShaughnessyResults(sp500Symbols);
    }
  });

  const getCompositeScoreColor = (percentile: number) => {
    if (percentile <= 10) return 'text-green-600 bg-green-100';
    if (percentile <= 20) return 'text-blue-600 bg-blue-100';
    if (percentile <= 40) return 'text-yellow-600 bg-yellow-100';
    return 'text-red-600 bg-red-100';
  };

  const getMetricRankColor = (rank: number, totalStocks: number) => {
    const percentile = (rank / totalStocks) * 100;
    if (percentile <= 20) return 'text-green-600';
    if (percentile <= 40) return 'text-blue-600';
    if (percentile <= 60) return 'text-yellow-600';
    return 'text-red-600';
  };

  return (
    <div class="bg-white rounded-lg shadow-lg p-6">
      <div class="flex items-center justify-between mb-6">
        <h2 class="text-2xl font-bold text-gray-800">
          📊 O'Shaughnessy Value Composite Screening
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
          Multi-metric value screening using 6 valuation ratios ranked across the universe.
          Lower composite scores indicate better value opportunities.
        </p>
      </div>

      <Show when={oshaughnessyStore.loading()}>
        <div class="flex items-center justify-center py-8">
          <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
          <span class="ml-2 text-gray-600">Calculating Value Composite Scores...</span>
        </div>
      </Show>

      <Show when={oshaughnessyStore.error()}>
        <div class="bg-red-50 border border-red-200 rounded-lg p-4 mb-6">
          <p class="text-red-800">{oshaughnessyStore.error()}</p>
        </div>
      </Show>

      <Show when={!oshaughnessyStore.loading() && !oshaughnessyStore.error()}>
        <div class="space-y-4">
          <For each={oshaughnessyStore.results()}>
            {(result) => (
              <div class="border border-gray-200 rounded-lg p-4 hover:shadow-md transition-shadow">
                <div class="flex items-center justify-between mb-3">
                  <div class="flex items-center space-x-4">
                    <h4 class="font-semibold text-lg">{result.symbol}</h4>
                    <span class={`px-3 py-1 rounded-full text-sm font-medium ${getCompositeScoreColor(result.composite_percentile)}`}>
                      Composite: {result.composite_score.toFixed(2)} ({result.composite_percentile.toFixed(1)}%)
                    </span>
                    <span class="text-sm text-gray-600">Rank: #{result.overall_rank}</span>
                  </div>
                  <div class="text-right">
                    <p class="text-sm text-gray-600">Market Cap: ${(result.market_cap / 1_000_000).toFixed(0)}M</p>
                    <p class="text-sm text-gray-600">Data Quality: {result.data_completeness_score}%</p>
                  </div>
                </div>

                <Show when={showDetails()}>
                  <div class="mt-4 pt-4 border-t border-gray-200">
                    <h5 class="font-medium text-gray-700 mb-3">Value Composite Breakdown</h5>
                    <div class="grid grid-cols-2 md:grid-cols-3 gap-4">
                      <div class="p-2 rounded text-sm">
                        <span class="font-medium">P/E:</span> 
                        <span class={getMetricRankColor(result.pe_rank, 500)}>
                          {result.pe_ratio?.toFixed(2) || 'N/A'} (Rank #{result.pe_rank})
                        </span>
                      </div>
                      <div class="p-2 rounded text-sm">
                        <span class="font-medium">P/B:</span> 
                        <span class={getMetricRankColor(result.pb_rank, 500)}>
                          {result.pb_ratio?.toFixed(2) || 'N/A'} (Rank #{result.pb_rank})
                        </span>
                      </div>
                      <div class="p-2 rounded text-sm">
                        <span class="font-medium">P/S:</span> 
                        <span class={getMetricRankColor(result.ps_rank, 500)}>
                          {result.ps_ratio?.toFixed(2) || 'N/A'} (Rank #{result.ps_rank})
                        </span>
                      </div>
                      <div class="p-2 rounded text-sm">
                        <span class="font-medium">P/CF:</span> 
                        <span class={getMetricRankColor(result.pcf_rank, 500)}>
                          {result.pcf_ratio?.toFixed(2) || 'N/A'} (Rank #{result.pcf_rank})
                        </span>
                      </div>
                      <div class="p-2 rounded text-sm">
                        <span class="font-medium">EV/EBITDA:</span> 
                        <span class={getMetricRankColor(result.ev_ebitda_rank, 500)}>
                          {result.ev_ebitda_ratio?.toFixed(2) || 'N/A'} (Rank #{result.ev_ebitda_rank})
                        </span>
                      </div>
                      <div class="p-2 rounded text-sm">
                        <span class="font-medium">Shareholder Yield:</span> 
                        <span class={getMetricRankColor(result.shareholder_yield_rank, 500)}>
                          {result.shareholder_yield.toFixed(2)}% (Rank #{result.shareholder_yield_rank})
                        </span>
                      </div>
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

### **Backend Tests (`src-tauri/tests/backend_tests.rs`)**
```rust
#[tokio::test]
async fn test_oshaughnessy_value_composite_screening() {
    let test_db = SimpleTestDatabase::new().await.unwrap();
    set_test_database_pool(test_db.pool().clone()).await;
    
    let sp500_symbols = vec!["AAPL".to_string(), "MSFT".to_string(), "GOOGL".to_string()];
    
    let result = get_oshaughnessy_value_composite_results(sp500_symbols, None, Some(10)).await
        .expect("O'Shaughnessy screening should work");
    
    assert!(!result.is_empty(), "Should return some results");
    
    for stock in &result {
        assert!(stock.composite_score > 0.0, "Composite score should be positive");
        assert!(stock.composite_percentile >= 0.0 && stock.composite_percentile <= 100.0, 
                "Composite percentile should be 0-100");
        assert!(stock.overall_rank > 0, "Overall rank should be positive");
        assert!(stock.market_cap > 0.0, "Market cap should be positive");
    }
    
    clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}
```

## 📈 Performance Considerations

### **Database Indexing**
```sql
-- Performance indexes for O'Shaughnessy screening
CREATE INDEX idx_oshaughnessy_composite_score 
ON oshaughnessy_value_composite_ranking(composite_score ASC);

CREATE INDEX idx_oshaughnessy_percentile 
ON oshaughnessy_value_composite_ranking(composite_percentile ASC);

CREATE INDEX idx_oshaughnessy_market_cap 
ON oshaughnessy_value_composite_ranking(market_cap DESC);

CREATE INDEX idx_oshaughnessy_screening 
ON oshaughnessy_value_composite_ranking(composite_percentile, data_completeness_score, market_cap);
```

## 🚀 Realistic Implementation Timeline

### **Week 1: Data Infrastructure (CRITICAL)**
- **Day 1-2**: Implement EDGAR cash flow extraction
- **Day 3-4**: Add missing balance sheet fields
- **Day 5**: Data quality validation and testing

### **Week 2: Backend Implementation**
- **Day 1-2**: Enhanced data models and API commands
- **Day 3-4**: Data quality assessment and confidence scoring
- **Day 5**: Comprehensive testing

### **Week 3: Frontend Integration**
- **Day 1-2**: Store integration with data quality awareness
- **Day 3-4**: UI components with quality indicators
- **Day 5**: User testing and feedback

### **Week 4: Production Readiness**
- **Day 1-2**: Performance optimization
- **Day 3-4**: Data refresh automation
- **Day 5**: Production deployment and monitoring

## ⚠️ Critical Risks & Mitigation

### **Risk 1: Data Availability**
**Probability**: High
**Impact**: High
**Mitigation**: 
- Implement fallback strategies for missing metrics
- Use proxy calculations where possible
- Clear data quality indicators in UI

### **Risk 2: Performance Degradation**
**Probability**: Medium
**Impact**: High
**Mitigation**:
- Materialized views for complex calculations
- Strategic database indexing
- Query optimization and caching

### **Risk 3: Data Quality Issues**
**Probability**: High
**Impact**: Medium
**Mitigation**:
- Cross-validation logic
- Data freshness monitoring
- Automated data quality alerts

## 📊 Success Metrics (Realistic)**

### **Data Quality Targets**
- **P/CF Data Coverage**: > 80% (currently ~30%)
- **EV/EBITDA Data Coverage**: > 80% (currently ~25%)
- **Shareholder Yield Accuracy**: > 90% (currently ~60%)
- **Overall Data Completeness**: > 85% (currently ~65%)

### **Performance Targets**
- **Query Execution Time**: < 5 seconds for S&P 500 (realistic)
- **Data Refresh Time**: < 30 minutes for full universe
- **Memory Usage**: < 200MB during screening operations

### **User Experience Targets**
- **Data Quality Transparency**: 100% of results show quality indicators
- **Missing Data Handling**: Graceful degradation with clear warnings
- **Confidence Scoring**: Accurate reliability assessment

---

## 🔍 **CRITICAL IMPLEMENTATION REQUIREMENTS**

**DO NOT PROCEED** with implementation until:

1. **Data Infrastructure Complete**: All 6 metrics have > 80% data coverage
2. **EDGAR Integration Working**: Cash flow and enhanced balance sheet extraction operational
3. **Data Quality Validation**: Cross-validation logic implemented and tested
4. **Fallback Strategies**: Graceful handling of missing data implemented

**Current Status**: Architecture is sound but data infrastructure is insufficient for production deployment.
