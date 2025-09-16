# GARP P/E Screening - Net New Feature Architecture

## Executive Summary

This document outlines the architecture for implementing **GARP P/E Screening** as a **net new feature** alongside existing P/S screening capabilities. The new feature uses P/E ratios and PEG (Price/Earnings to Growth) ratios to identify growth stocks at reasonable prices, without affecting any existing functionality.

## Feature Coexistence Architecture

### Current Screening Features (PRESERVED)
```
┌─────────────────────────────────────────────────────────────┐
│                    EXISTING FEATURES                        │
├─────────────────────────────────────────────────────────────┤
│ 1. P/E Ratio Screening (Historical)                        │
│    - Command: get_value_recommendations_with_stats         │
│    - UI: "P/E Ratio (Historical)"                          │
│    - Logic: P/E ≤ Historical Minimum × 1.20              │
├─────────────────────────────────────────────────────────────┤
│ 2. P/S Ratio Screening (Basic)                            │
│    - Command: get_undervalued_stocks_by_ps                 │
│    - UI: "P/S Ratio (TTM)"                                │
│    - Logic: P/S < (Historical Mean - 0.5 × Variance)      │
├─────────────────────────────────────────────────────────────┤
│ 3. P/S Ratio Screening (Enhanced with Growth)             │
│    - Command: get_ps_screening_with_revenue_growth        │
│    - UI: "P/S Ratio (Enhanced with Growth)"               │
│    - Logic: P/S < (Historical Median - 1.0 × Std Dev) +    │
│             Revenue Growth > 0% + Quality Score ≥ 50      │
└─────────────────────────────────────────────────────────────┘
```

### New GARP P/E Feature (ADDITION)
```
┌─────────────────────────────────────────────────────────────┐
│                    NEW FEATURE                              │
├─────────────────────────────────────────────────────────────┤
│ 4. GARP P/E Screening (PEG-Based)                         │
│    - Command: get_garp_pe_screening_results               │
│    - UI: "GARP (P/E + PEG Based)"                         │
│    - Logic: PEG < 1.0 + Revenue Growth > 15% +            │
│             Profit Margin > 5% + Net Income > 0          │
└─────────────────────────────────────────────────────────────┘
```

## Technical Architecture

### Database Layer (ADDITIVE ONLY)

#### New Database Views
```sql
-- NEW: GARP P/E screening data view
CREATE VIEW garp_pe_screening_data AS ...

-- NEW: PEG ratio analysis view  
CREATE VIEW peg_ratio_analysis AS ...

-- EXISTING: All current tables and views remain unchanged
-- - daily_valuation_ratios (existing)
-- - income_statements (existing)
-- - balance_sheets (existing)
-- - stocks (existing)
```

#### New Database Indexes
```sql
-- NEW: Performance indexes for GARP P/E
CREATE INDEX idx_income_statements_eps_growth ...
CREATE INDEX idx_daily_ratios_pe_garp ...
CREATE INDEX idx_garp_pe_performance ...

-- EXISTING: All current indexes remain unchanged
```

### Backend Layer (ADDITIVE ONLY)

#### New Data Structures
```rust
// NEW: GARP P/E specific data structures
pub struct GarpPeScreeningResult { ... }
pub struct GarpPeScreeningCriteria { ... }

// EXISTING: All current data structures remain unchanged
// - SmartUndervaluedStock (existing)
// - PsRevenueGrowthStock (existing)
// - StockInfo (existing)
```

#### New Tauri Commands
```rust
// NEW: GARP P/E screening command
#[tauri::command]
pub async fn get_garp_pe_screening_results(...) -> Result<Vec<GarpPeScreeningResult>, String>

// EXISTING: All current commands remain unchanged
// - get_value_recommendations_with_stats (existing)
// - get_undervalued_stocks_by_ps (existing)
// - get_ps_screening_with_revenue_growth (existing)
```

#### Command Registration (ADDITIVE)
```rust
// ADD to existing command list (no removal)
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            // EXISTING COMMANDS (preserved)
            crate::commands::recommendations::get_value_recommendations_with_stats,
            crate::commands::analysis::get_undervalued_stocks_by_ps,
            crate::commands::analysis::get_ps_screening_with_revenue_growth,
            
            // NEW COMMAND (added)
            crate::commands::garp_pe::get_garp_pe_screening_results,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Frontend Layer (ADDITIVE ONLY)

#### New API Service Methods
```javascript
// NEW: GARP P/E API methods
export const analysisAPI = {
    // EXISTING METHODS (preserved)
    async getValueRecommendationsWithStats(...) { ... },
    async getUndervaluedStocksByPs(...) { ... },
    async getPsScreeningWithRevenueGrowth(...) { ... },
    
    // NEW METHOD (added)
    async getGarpPeScreeningResults(...) { ... }
};
```

#### New Data Service Methods
```javascript
// NEW: GARP P/E data service methods
export const recommendationsDataService = {
    // EXISTING METHODS (preserved)
    async loadValueRecommendationsWithStats(...) { ... },
    async loadUndervaluedStocksByPs(...) { ... },
    async loadPsScreeningWithRevenueGrowth(...) { ... },
    
    // NEW METHOD (added)
    async loadGarpPeScreeningResults(...) { ... }
};
```

#### Enhanced UI Components (ADDITIVE)
```javascript
// ADD to existing screening options (no removal)
const screeningOptions = [
    { value: 'pe', label: 'P/E Ratio (Historical)' },           // EXISTING
    { value: 'ps', label: 'P/S Ratio (TTM)' },                 // EXISTING
    { value: 'ps_enhanced', label: 'P/S Ratio (Enhanced with Growth)' }, // EXISTING
    { value: 'garp_pe', label: 'GARP (P/E + PEG Based)' }      // NEW
];

// NEW: GARP P/E specific state (additive)
const [garpPeCriteria, setGarpPeCriteria] = useState({ ... });

// NEW: GARP P/E specific UI controls (additive)
{screeningType === 'garp_pe' && (
    <div className="garp-pe-controls space-y-4">
        {/* NEW: GARP P/E specific filters */}
    </div>
)}
```

## Implementation Plan

### Phase 1: Database Schema (ADDITIVE ONLY)

#### 1.1 New Migration File
**File**: `src-tauri/db/migrations/20250916000007_add_garp_pe_screening.sql`

```sql
-- Migration: Add GARP P/E Screening Support
-- Purpose: Add P/E-based GARP screening views and indexes
-- Safety: ADDITIVE ONLY - no data destruction, no existing table modifications

-- Step 1: Add GARP P/E screening indexes (NEW)
CREATE INDEX IF NOT EXISTS idx_income_statements_eps_growth 
ON income_statements(stock_id, period_type, report_date, net_income, shares_diluted);

CREATE INDEX IF NOT EXISTS idx_daily_ratios_pe_garp 
ON daily_valuation_ratios(stock_id, date, pe_ratio);

CREATE INDEX IF NOT EXISTS idx_balance_sheets_debt_equity 
ON balance_sheets(stock_id, period_type, report_date, total_debt, total_equity);

-- Step 2: Create GARP P/E screening helper view (NEW)
CREATE VIEW IF NOT EXISTS garp_pe_screening_data AS
SELECT 
    s.id as stock_id,
    s.symbol,
    s.sector,
    
    -- Current P/E ratio and price
    dvr.pe_ratio as current_pe_ratio,
    dvr.price as current_price,
    dvr.market_cap,
    
    -- Current EPS (TTM)
    ttm_income.net_income / ttm_income.shares_diluted as current_eps_ttm,
    ttm_income.net_income as current_ttm_net_income,
    ttm_income.shares_diluted as current_ttm_shares,
    
    -- Previous TTM EPS for growth calculation
    prev_ttm_income.net_income / prev_ttm_income.shares_diluted as previous_eps_ttm,
    
    -- Current Annual EPS
    annual_income.net_income / annual_income.shares_diluted as current_eps_annual,
    annual_income.net_income as current_annual_net_income,
    annual_income.shares_diluted as current_annual_shares,
    
    -- Previous Annual EPS for growth calculation
    prev_annual_income.net_income / prev_annual_income.shares_diluted as previous_eps_annual,
    
    -- Revenue data (TTM)
    ttm_income.revenue as current_ttm_revenue,
    prev_ttm_income.revenue as previous_ttm_revenue,
    
    -- Revenue data (Annual)
    annual_income.revenue as current_annual_revenue,
    prev_annual_income.revenue as previous_annual_revenue,
    
    -- Balance sheet data
    ttm_balance.total_debt,
    ttm_balance.total_equity,
    
    -- Calculated metrics
    CASE 
        WHEN prev_ttm_income.net_income > 0 AND prev_ttm_income.shares_diluted > 0 THEN 
            ((ttm_income.net_income / ttm_income.shares_diluted) - 
             (prev_ttm_income.net_income / prev_ttm_income.shares_diluted)) / 
            (prev_ttm_income.net_income / prev_ttm_income.shares_diluted) * 100
        ELSE NULL 
    END as eps_growth_rate_ttm,
    
    CASE 
        WHEN prev_annual_income.net_income > 0 AND prev_annual_income.shares_diluted > 0 THEN 
            ((annual_income.net_income / annual_income.shares_diluted) - 
             (prev_annual_income.net_income / prev_annual_income.shares_diluted)) / 
            (prev_annual_income.net_income / prev_annual_income.shares_diluted) * 100
        ELSE NULL 
    END as eps_growth_rate_annual,
    
    CASE 
        WHEN prev_ttm_income.revenue > 0 THEN 
            ((ttm_income.revenue - prev_ttm_income.revenue) / prev_ttm_income.revenue) * 100
        ELSE NULL 
    END as ttm_growth_rate,
    
    CASE 
        WHEN prev_annual_income.revenue > 0 THEN 
            ((annual_income.revenue - prev_annual_income.revenue) / prev_annual_income.revenue) * 100
        ELSE NULL 
    END as annual_growth_rate,
    
    CASE 
        WHEN ttm_income.revenue > 0 THEN 
            (ttm_income.net_income / ttm_income.revenue) * 100
        ELSE NULL 
    END as net_profit_margin,
    
    CASE 
        WHEN ttm_balance.total_equity > 0 THEN 
            ttm_balance.total_debt / ttm_balance.total_equity
        ELSE NULL 
    END as debt_to_equity_ratio
    
FROM stocks s
JOIN daily_valuation_ratios dvr ON s.id = dvr.stock_id 
    AND dvr.date = (SELECT MAX(date) FROM daily_valuation_ratios WHERE stock_id = s.id)

-- Current TTM income data
LEFT JOIN (
    SELECT stock_id, revenue, net_income, shares_diluted, report_date,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
    FROM income_statements 
    WHERE period_type = 'TTM' AND net_income > 0 AND shares_diluted > 0
) ttm_income ON s.id = ttm_income.stock_id AND ttm_income.rn = 1

-- Previous TTM income data
LEFT JOIN (
    SELECT stock_id, revenue, net_income, shares_diluted, report_date,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
    FROM income_statements 
    WHERE period_type = 'TTM' AND net_income > 0 AND shares_diluted > 0
) prev_ttm_income ON s.id = prev_ttm_income.stock_id AND prev_ttm_income.rn = 2

-- Current Annual income data
LEFT JOIN (
    SELECT stock_id, revenue, net_income, shares_diluted, fiscal_year,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY fiscal_year DESC) as rn
    FROM income_statements 
    WHERE period_type = 'Annual' AND net_income > 0 AND shares_diluted > 0
) annual_income ON s.id = annual_income.stock_id AND annual_income.rn = 1

-- Previous Annual income data
LEFT JOIN (
    SELECT stock_id, revenue, net_income, shares_diluted, fiscal_year,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY fiscal_year DESC) as rn
    FROM income_statements 
    WHERE period_type = 'Annual' AND net_income > 0 AND shares_diluted > 0
) prev_annual_income ON s.id = prev_annual_income.stock_id AND prev_annual_income.rn = 2

-- Current TTM balance sheet data
LEFT JOIN (
    SELECT stock_id, total_debt, total_equity, report_date,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
    FROM balance_sheets 
    WHERE period_type = 'TTM'
) ttm_balance ON s.id = ttm_balance.stock_id AND ttm_balance.rn = 1;

-- Step 3: Create PEG ratio analysis helper view (NEW)
CREATE VIEW IF NOT EXISTS peg_ratio_analysis AS
SELECT 
    gpsd.*,
    
    -- PEG ratio calculations
    CASE 
        WHEN gpsd.eps_growth_rate_ttm > 0 AND gpsd.current_pe_ratio > 0 THEN 
            gpsd.current_pe_ratio / gpsd.eps_growth_rate_ttm
        WHEN gpsd.eps_growth_rate_annual > 0 AND gpsd.current_pe_ratio > 0 THEN 
            gpsd.current_pe_ratio / gpsd.eps_growth_rate_annual
        ELSE NULL 
    END as peg_ratio,
    
    -- Screening criteria
    CASE 
        WHEN gpsd.current_ttm_net_income > 0 THEN true
        ELSE false
    END as passes_positive_earnings,
    
    CASE 
        WHEN gpsd.eps_growth_rate_ttm > 0 AND gpsd.current_pe_ratio > 0 AND 
             (gpsd.current_pe_ratio / gpsd.eps_growth_rate_ttm) < 1.0 THEN true
        WHEN gpsd.eps_growth_rate_annual > 0 AND gpsd.current_pe_ratio > 0 AND 
             (gpsd.current_pe_ratio / gpsd.eps_growth_rate_annual) < 1.0 THEN true
        ELSE false
    END as passes_peg_filter,
    
    CASE 
        WHEN gpsd.ttm_growth_rate > 15 OR gpsd.annual_growth_rate > 10 THEN true
        ELSE false
    END as passes_revenue_growth_filter,
    
    CASE 
        WHEN gpsd.net_profit_margin > 5 THEN true
        ELSE false
    END as passes_profitability_filter,
    
    CASE 
        WHEN gpsd.debt_to_equity_ratio < 2 OR gpsd.debt_to_equity_ratio IS NULL THEN true
        ELSE false
    END as passes_debt_filter
    
FROM garp_pe_screening_data gpsd
WHERE gpsd.current_pe_ratio IS NOT NULL 
  AND gpsd.current_pe_ratio > 0;
```

#### 1.2 Migration Execution Strategy
```bash
# Step 1: Backup production database
cargo run --bin db_admin -- backup

# Step 2: Run GARP P/E migration (ADDITIVE ONLY)
cargo run --bin db_admin -- migrate --confirm

# Step 3: Verify migration success (NEW views created)
sqlite3 db/stocks.db "SELECT COUNT(*) FROM garp_pe_screening_data;"
sqlite3 db/stocks.db "SELECT COUNT(*) FROM peg_ratio_analysis;"

# Step 4: Verify existing functionality (UNCHANGED)
sqlite3 db/stocks.db "SELECT COUNT(*) FROM daily_valuation_ratios;"
sqlite3 db/stocks.db "SELECT COUNT(*) FROM income_statements;"

# Step 5: Test PEG calculation with sample data
sqlite3 db/stocks.db "SELECT symbol, current_pe_ratio, eps_growth_rate_ttm, peg_ratio FROM peg_ratio_analysis WHERE symbol = 'AAPL' LIMIT 1;"
```

### Phase 2: Backend Implementation (ADDITIVE ONLY)

#### 2.1 New Data Structures
**File**: `src-tauri/src/models/garp_pe.rs` (NEW FILE)

```rust
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GarpPeScreeningResult {
    pub stock_id: i32,
    pub symbol: String,
    pub sector: Option<String>,
    
    // P/E and PEG Analysis
    pub current_pe_ratio: f64,
    pub peg_ratio: Option<f64>,
    pub current_price: f64,
    pub passes_positive_earnings: bool,
    pub passes_peg_filter: bool,
    
    // EPS Analysis
    pub current_eps_ttm: Option<f64>,
    pub current_eps_annual: Option<f64>,
    pub eps_growth_rate_ttm: Option<f64>,
    pub eps_growth_rate_annual: Option<f64>,
    
    // Revenue Growth Analysis
    pub current_ttm_revenue: Option<f64>,
    pub ttm_growth_rate: Option<f64>,
    pub current_annual_revenue: Option<f64>,
    pub annual_growth_rate: Option<f64>,
    pub passes_revenue_growth_filter: bool,
    
    // Profitability Analysis
    pub current_ttm_net_income: Option<f64>,
    pub net_profit_margin: Option<f64>,
    pub passes_profitability_filter: bool,
    
    // Debt Analysis
    pub total_debt: Option<f64>,
    pub total_equity: Option<f64>,
    pub debt_to_equity_ratio: Option<f64>,
    pub passes_debt_filter: bool,
    
    // GARP Scoring
    pub garp_score: f64,
    pub quality_score: i32,
    pub passes_garp_screening: bool,
    
    // Market Metrics
    pub market_cap: f64,
    pub data_completeness_score: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GarpPeScreeningCriteria {
    pub max_peg_ratio: f64,           // Default: 1.0
    pub min_revenue_growth: f64,      // Default: 15.0% (TTM) or 10.0% (Annual)
    pub min_profit_margin: f64,       // Default: 5.0%
    pub max_debt_to_equity: f64,      // Default: 2.0
    pub min_market_cap: f64,          // Default: $500M
    pub min_quality_score: i32,       // Default: 50
    pub require_positive_earnings: bool, // Default: true
}

impl Default for GarpPeScreeningCriteria {
    fn default() -> Self {
        Self {
            max_peg_ratio: 1.0,              // PEG < 1.0
            min_revenue_growth: 15.0,        // 15% TTM or 10% Annual
            min_profit_margin: 5.0,          // 5% net margin
            max_debt_to_equity: 2.0,         // D/E < 2.0
            min_market_cap: 500_000_000.0,   // $500M minimum
            min_quality_score: 50,           // Minimum data quality
            require_positive_earnings: true, // Net Income > 0
        }
    }
}
```

#### 2.2 New Tauri Command
**File**: `src-tauri/src/commands/garp_pe.rs` (NEW FILE)

```rust
use sqlx::SqlitePool;
use crate::models::garp_pe::{GarpPeScreeningResult, GarpPeScreeningCriteria};
use crate::database::helpers::get_database_connection;

#[tauri::command]
pub async fn get_garp_pe_screening_results(
    stock_tickers: Vec<String>, 
    criteria: Option<GarpPeScreeningCriteria>,
    limit: Option<i32>
) -> Result<Vec<GarpPeScreeningResult>, String> {
    let pool = get_database_connection().await?;
    let criteria = criteria.unwrap_or_default();
    let limit_value = limit.unwrap_or(50);
    
    if stock_tickers.is_empty() {
        return Ok(vec![]);
    }
    
    // Create placeholders for the IN clause
    let placeholders = stock_tickers.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    
    let query = format!("
        SELECT 
            pra.stock_id,
            pra.symbol,
            pra.sector,
            pra.current_pe_ratio,
            pra.peg_ratio,
            pra.current_price,
            pra.passes_positive_earnings,
            pra.passes_peg_filter,
            pra.current_eps_ttm,
            pra.current_eps_annual,
            pra.eps_growth_rate_ttm,
            pra.eps_growth_rate_annual,
            pra.current_ttm_revenue,
            pra.ttm_growth_rate,
            pra.current_annual_revenue,
            pra.annual_growth_rate,
            pra.passes_revenue_growth_filter,
            pra.current_ttm_net_income,
            pra.net_profit_margin,
            pra.passes_profitability_filter,
            pra.total_debt,
            pra.total_equity,
            pra.debt_to_equity_ratio,
            pra.passes_debt_filter,
            
            -- GARP Score: Revenue Growth % / PEG Ratio
            CASE 
                WHEN pra.peg_ratio > 0 AND pra.ttm_growth_rate IS NOT NULL THEN 
                    pra.ttm_growth_rate / pra.peg_ratio
                WHEN pra.peg_ratio > 0 AND pra.annual_growth_rate IS NOT NULL THEN 
                    pra.annual_growth_rate / pra.peg_ratio
                ELSE 0
            END as garp_score,
            
            -- Quality score (0-100)
            CASE 
                WHEN pra.eps_growth_rate_ttm IS NOT NULL 
                     AND pra.ttm_growth_rate IS NOT NULL 
                     AND pra.net_profit_margin IS NOT NULL 
                     AND pra.debt_to_equity_ratio IS NOT NULL THEN 100
                WHEN pra.eps_growth_rate_ttm IS NOT NULL 
                     AND pra.ttm_growth_rate IS NOT NULL 
                     AND pra.net_profit_margin IS NOT NULL THEN 75
                WHEN pra.eps_growth_rate_ttm IS NOT NULL 
                     AND (pra.ttm_growth_rate IS NOT NULL OR pra.net_profit_margin IS NOT NULL) THEN 50
                ELSE 25
            END as quality_score,
            
            -- Final GARP screening result
            CASE 
                WHEN pra.passes_positive_earnings 
                     AND pra.passes_peg_filter 
                     AND pra.passes_revenue_growth_filter 
                     AND pra.passes_profitability_filter 
                     AND pra.passes_debt_filter
                     AND (CASE 
                        WHEN pra.eps_growth_rate_ttm IS NOT NULL 
                             AND pra.ttm_growth_rate IS NOT NULL 
                             AND pra.net_profit_margin IS NOT NULL 
                             AND pra.debt_to_equity_ratio IS NOT NULL THEN 100
                        WHEN pra.eps_growth_rate_ttm IS NOT NULL 
                             AND pra.ttm_growth_rate IS NOT NULL 
                             AND pra.net_profit_margin IS NOT NULL THEN 75
                        WHEN pra.eps_growth_rate_ttm IS NOT NULL 
                             AND (pra.ttm_growth_rate IS NOT NULL OR pra.net_profit_margin IS NOT NULL) THEN 50
                        ELSE 25
                     END) >= ?
                THEN true
                ELSE false
            END as passes_garp_screening,
            
            pra.market_cap,
            pra.data_completeness_score
            
        FROM peg_ratio_analysis pra
        WHERE pra.symbol IN ({})
          AND pra.market_cap > ?
        ORDER BY 
            passes_garp_screening DESC,
            garp_score DESC,
            quality_score DESC,
            pra.peg_ratio ASC
        LIMIT ?
    ", placeholders);
    
    let mut query_builder = sqlx::query_as::<_, GarpPeScreeningResult>(&query);
    
    // Bind parameters
    query_builder = query_builder.bind(criteria.min_quality_score);
    
    // Bind stock tickers
    for ticker in &stock_tickers {
        query_builder = query_builder.bind(ticker);
    }
    
    // Bind remaining parameters
    query_builder = query_builder.bind(criteria.min_market_cap);
    query_builder = query_builder.bind(limit_value);
    
    let results = query_builder.fetch_all(&pool).await
        .map_err(|e| format!("GARP P/E screening query failed: {}", e))?;
    
    Ok(results)
}
```

#### 2.3 Command Registration (ADDITIVE)
**File**: `src-tauri/src/commands/mod.rs` (MODIFY - ADD ONLY)

```rust
// ADD new module (no removal)
pub mod garp_pe;

// EXISTING modules remain unchanged
pub mod stocks;
pub mod analysis;
pub mod recommendations;
pub mod fetching;
pub mod earnings;
```

**File**: `src-tauri/src/main.rs` (MODIFY - ADD ONLY)

```rust
// ADD to existing command list (no removal)
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            // EXISTING COMMANDS (preserved)
            crate::commands::stocks::get_stocks_paginated,
            crate::commands::stocks::search_stocks,
            crate::commands::stocks::get_stocks_with_data_status,
            crate::commands::stocks::get_sp500_symbols,
            crate::commands::analysis::get_stock_date_range,
            crate::commands::analysis::get_price_history,
            crate::commands::analysis::get_valuation_ratios,
            crate::commands::analysis::get_ps_evs_history,
            crate::commands::analysis::get_undervalued_stocks_by_ps,
            crate::commands::analysis::get_ps_screening_with_revenue_growth,
            crate::commands::analysis::export_data,
            crate::commands::recommendations::get_value_recommendations_with_stats,
            crate::commands::fetching::get_initialization_status,
            crate::commands::fetching::get_database_stats,
            
            // NEW COMMAND (added)
            crate::commands::garp_pe::get_garp_pe_screening_results,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Phase 3: Frontend Implementation (ADDITIVE ONLY)

#### 3.1 New API Service Methods
**File**: `src/src/services/api.js` (MODIFY - ADD ONLY)

```javascript
// ADD to existing analysisAPI (no removal)
export const analysisAPI = {
    // EXISTING METHODS (preserved)
    async getValueRecommendationsWithStats(limit, minMarketCap) { ... },
    async getUndervaluedStocksByPs(stockTickers, limit, minMarketCap) { ... },
    async getPsScreeningWithRevenueGrowth(stockTickers, limit, minMarketCap) { ... },
    
    // NEW METHOD (added)
    async getGarpPeScreeningResults(stockTickers, criteria, limit) {
        try {
            const result = await invoke('get_garp_pe_screening_results', {
                stockTickers,
                criteria: criteria || {
                    maxPegRatio: 1.0,
                    minRevenueGrowth: 15.0,
                    minProfitMargin: 5.0,
                    maxDebtToEquity: 2.0,
                    minMarketCap: 500_000_000,
                    minQualityScore: 50,
                    requirePositiveEarnings: true
                },
                limit: limit || 50
            });
            return result;
        } catch (error) {
            console.error('GARP P/E screening API error:', error);
            throw error;
        }
    }
};
```

#### 3.2 New Data Service Methods
**File**: `src/src/services/dataService.js` (MODIFY - ADD ONLY)

```javascript
// ADD to existing recommendationsDataService (no removal)
export const recommendationsDataService = {
    // EXISTING METHODS (preserved)
    async loadValueRecommendationsWithStats(limit, minMarketCap) { ... },
    async loadUndervaluedStocksByPs(stockTickers, limit, minMarketCap) { ... },
    async loadPsScreeningWithRevenueGrowth(stockTickers, limit, minMarketCap) { ... },
    
    // NEW METHOD (added)
    async loadGarpPeScreeningResults(stockTickers, criteria, limit) {
        try {
            const result = await analysisAPI.getGarpPeScreeningResults(stockTickers, criteria, limit);
            
            return {
                success: true,
                stocks: result,
                error: null
            };
        } catch (error) {
            return {
                success: false,
                stocks: [],
                error: error.message
            };
        }
    }
};
```

#### 3.3 Enhanced UI Components (ADDITIVE)
**File**: `src/src/components/RecommendationsPanel.jsx` (MODIFY - ADD ONLY)

```javascript
// ADD to existing screening options (no removal)
const screeningOptions = [
    { value: 'pe', label: 'P/E Ratio (Historical)' },           // EXISTING
    { value: 'ps', label: 'P/S Ratio (TTM)' },                 // EXISTING
    { value: 'ps_enhanced', label: 'P/S Ratio (Enhanced with Growth)' }, // EXISTING
    { value: 'garp_pe', label: 'GARP (P/E + PEG Based)' }      // NEW
];

// ADD new state for GARP P/E criteria (additive)
const [garpPeCriteria, setGarpPeCriteria] = useState({
    maxPegRatio: 1.0,
    minRevenueGrowth: 15.0,
    minProfitMargin: 5.0,
    maxDebtToEquity: 2.0,
    minMarketCap: 500_000_000,
    minQualityScore: 50,
    requirePositiveEarnings: true
});

// ADD new UI controls for GARP P/E (additive)
{screeningType === 'garp_pe' && (
    <div className="garp-pe-controls space-y-4">
        <div className="grid grid-cols-2 gap-4">
            <div>
                <label className="text-sm font-medium text-gray-700">
                    Max PEG Ratio:
                </label>
                <select
                    value={garpPeCriteria.maxPegRatio}
                    onChange={(e) => setGarpPeCriteria(prev => ({
                        ...prev,
                        maxPegRatio: Number(e.target.value)
                    }))}
                    className="border border-gray-300 rounded px-2 py-1 text-sm w-full"
                >
                    <option value={0.5}>0.5</option>
                    <option value={0.8}>0.8</option>
                    <option value={1.0}>1.0</option>
                    <option value={1.2}>1.2</option>
                </select>
            </div>
            
            <div>
                <label className="text-sm font-medium text-gray-700">
                    Min Revenue Growth (%):
                </label>
                <select
                    value={garpPeCriteria.minRevenueGrowth}
                    onChange={(e) => setGarpPeCriteria(prev => ({
                        ...prev,
                        minRevenueGrowth: Number(e.target.value)
                    }))}
                    className="border border-gray-300 rounded px-2 py-1 text-sm w-full"
                >
                    <option value={10}>10%</option>
                    <option value={15}>15%</option>
                    <option value={20}>20%</option>
                    <option value={25}>25%</option>
                </select>
            </div>
            
            <div>
                <label className="text-sm font-medium text-gray-700">
                    Min Profit Margin (%):
                </label>
                <select
                    value={garpPeCriteria.minProfitMargin}
                    onChange={(e) => setGarpPeCriteria(prev => ({
                        ...prev,
                        minProfitMargin: Number(e.target.value)
                    }))}
                    className="border border-gray-300 rounded px-2 py-1 text-sm w-full"
                >
                    <option value={3}>3%</option>
                    <option value={5}>5%</option>
                    <option value={7}>7%</option>
                    <option value={10}>10%</option>
                </select>
            </div>
            
            <div>
                <label className="text-sm font-medium text-gray-700">
                    Max Debt-to-Equity:
                </label>
                <select
                    value={garpPeCriteria.maxDebtToEquity}
                    onChange={(e) => setGarpPeCriteria(prev => ({
                        ...prev,
                        maxDebtToEquity: Number(e.target.value)
                    }))}
                    className="border border-gray-300 rounded px-2 py-1 text-sm w-full"
                >
                    <option value={1}>1.0</option>
                    <option value={2}>2.0</option>
                    <option value={3}>3.0</option>
                    <option value={5}>5.0</option>
                </select>
            </div>
        </div>
        
        <div className="flex items-center gap-4">
            <label className="flex items-center gap-2">
                <input
                    type="checkbox"
                    checked={garpPeCriteria.requirePositiveEarnings}
                    onChange={(e) => setGarpPeCriteria(prev => ({
                        ...prev,
                        requirePositiveEarnings: e.target.checked
                    }))}
                    className="rounded"
                />
                <span className="text-sm text-gray-700">Require Positive Earnings (Net Income > 0)</span>
            </label>
        </div>
    </div>
)}
```

### Phase 4: Testing Strategy (ADDITIVE ONLY)

#### 4.1 New Unit Tests
**File**: `src-tauri/tests/garp_pe_tests.rs` (NEW FILE)

```rust
#[cfg(test)]
mod garp_pe_tests {
    use super::*;
    use crate::tests::helpers::database::SimpleTestDatabase;
    use crate::commands::garp_pe::get_garp_pe_screening_results;
    use crate::models::garp_pe::GarpPeScreeningCriteria;

    #[tokio::test]
    async fn test_garp_pe_screening_basic() {
        let test_db = SimpleTestDatabase::new().await.unwrap();
        set_test_database_pool(test_db.pool().clone()).await;
        
        let criteria = GarpPeScreeningCriteria {
            max_peg_ratio: 1.0,
            min_revenue_growth: 15.0,
            min_profit_margin: 5.0,
            max_debt_to_equity: 2.0,
            min_market_cap: 500_000_000.0,
            min_quality_score: 50,
            require_positive_earnings: true,
        };
        
        let result = get_garp_pe_screening_results(
            vec!["AAPL".to_string(), "MSFT".to_string()],
            Some(criteria),
            Some(10)
        ).await;
        
        assert!(result.is_ok());
        let stocks = result.unwrap();
        assert!(!stocks.is_empty());
        
        // Verify GARP P/E criteria are met
        for stock in &stocks {
            if stock.passes_garp_screening {
                assert!(stock.passes_positive_earnings);
                assert!(stock.passes_peg_filter);
                assert!(stock.passes_revenue_growth_filter);
                assert!(stock.passes_profitability_filter);
                assert!(stock.passes_debt_filter);
                assert!(stock.quality_score >= 50);
                
                // Verify PEG ratio is calculated correctly
                if let Some(peg_ratio) = stock.peg_ratio {
                    assert!(peg_ratio < 1.0);
                }
            }
        }
        
        clear_test_database_pool().await;
        test_db.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_garp_pe_screening_performance() {
        let test_db = SimpleTestDatabase::new().await.unwrap();
        set_test_database_pool(test_db.pool().clone()).await;
        
        let start_time = std::time::Instant::now();
        
        let result = get_garp_pe_screening_results(
            vec!["AAPL".to_string(), "MSFT".to_string(), "GOOGL".to_string()],
            None, // Use default criteria
            Some(50)
        ).await;
        
        let duration = start_time.elapsed();
        
        assert!(result.is_ok());
        assert!(duration.as_secs() < 5, "GARP P/E screening should complete in <5 seconds");
        
        clear_test_database_pool().await;
        test_db.cleanup().await.unwrap();
    }
}
```

#### 4.2 Integration Tests (ADDITIVE)
**File**: `src-tauri/tests/backend_tests.rs` (MODIFY - ADD ONLY)

```rust
// ADD to existing backend_tests.rs (no removal)
#[tokio::test]
async fn test_garp_pe_screening_integration() {
    let test_db = SimpleTestDatabase::new().await.unwrap();
    set_test_database_pool(test_db.pool().clone()).await;
    
    // Test with S&P 500 symbols
    let sp500_symbols = vec![
        "AAPL".to_string(), "MSFT".to_string(), "GOOGL".to_string(),
        "AMZN".to_string(), "TSLA".to_string(), "META".to_string()
    ];
    
    let result = get_garp_pe_screening_results(sp500_symbols, None, Some(20)).await;
    
    assert!(result.is_ok());
    let stocks = result.unwrap();
    
    // Verify we get some results
    assert!(!stocks.is_empty());
    
    // Verify data quality
    for stock in &stocks {
        assert!(!stock.symbol.is_empty());
        assert!(stock.current_pe_ratio > 0.0);
        assert!(stock.market_cap > 0.0);
        
        // Verify PEG ratio calculation
        if let Some(peg_ratio) = stock.peg_ratio {
            assert!(peg_ratio > 0.0);
        }
    }
    
    clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}
```

## Implementation Timeline

### Week 1: Database & Backend (ADDITIVE ONLY)
- **Day 1-2**: Create migration and test database changes
- **Day 3-4**: Implement GARP P/E command and data structures
- **Day 5**: Backend testing and optimization

### Week 2: Frontend & Integration (ADDITIVE ONLY)
- **Day 1-2**: Implement API service and data service
- **Day 3-4**: Build GARP P/E UI components and filters
- **Day 5**: Integration testing and bug fixes

### Week 3: Testing & Deployment (ADDITIVE ONLY)
- **Day 1-2**: Comprehensive testing (unit, integration, performance)
- **Day 3-4**: Performance optimization and monitoring
- **Day 5**: Production deployment and validation

## Success Metrics

### Performance Targets
- **Query Time**: < 3 seconds for S&P 500 GARP P/E screening
- **Memory Usage**: < 100MB additional memory overhead
- **Data Coverage**: > 60% of S&P 500 stocks with GARP P/E data

### Quality Targets
- **Test Coverage**: > 90% for GARP P/E-specific code
- **Data Accuracy**: > 95% accuracy for PEG calculations
- **User Experience**: < 2 second UI response time

## Risk Mitigation

### Database Safety
- **Automatic Backups**: Before any migration
- **Rollback Plan**: Timestamped backups for restoration
- **Production Detection**: Safety checks for large databases

### Performance Risks
- **Query Optimization**: Comprehensive indexing strategy
- **Caching Strategy**: PEG ratio caching
- **Monitoring**: Query execution time tracking

### Data Quality Risks
- **Validation**: Comprehensive data validation
- **Error Handling**: Graceful degradation for missing data
- **Quality Scoring**: Data completeness assessment

---

**This architecture ensures GARP P/E Screening is implemented as a net new feature without affecting any existing functionality. All existing P/S screening capabilities remain unchanged and fully functional.**
