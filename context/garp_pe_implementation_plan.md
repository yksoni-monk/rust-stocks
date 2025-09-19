# P/E-Based GARP Implementation Plan - Updated Technical Specification

## Executive Summary

This document provides a comprehensive implementation plan for the **P/E-based Growth at Reasonable Price (GARP)** stock screening system using PEG ratios instead of P/S ratios. The plan includes database migrations, API development, testing strategies, and deployment considerations.

## Updated Strategy Analysis

### ✅ **P/E-Based GARP Requirements**
1. **Positive Earnings**: Net Income > 0 (mandatory for PEG calculation)
2. **Low PEG Ratio**: PEG < 1.0 (P/E ÷ EPS Growth Rate)
3. **Strong Revenue Growth**: TTM or 5-year revenue growth > 15% (or YoY > 10%)
4. **Positive Profitability**: Net profit margin > 5%
5. **Optional Quality Check**: Debt-to-Equity < 2

### ✅ **Data Availability Confirmed**
- **P/E Ratios**: ✅ Available in `daily_valuation_ratios.pe_ratio`
- **EPS Data**: ✅ Available in `income_statements.net_income` and `shares_diluted`
- **Revenue Growth**: ✅ Available in `income_statements.revenue`
- **Profit Margins**: ✅ Calculable from `net_income / revenue`
- **Debt Data**: ✅ Available in `balance_sheets.total_debt` and `total_equity`

## Implementation Plan

### Phase 1: Database Schema Enhancements

#### 1.1 New Migration: `20250916000007_add_garp_pe_screening.sql`

```sql
-- Migration: Add P/E-Based GARP Screening Support
-- File: 20250916000007_add_garp_pe_screening.sql
-- Purpose: Add P/E-based GARP screening views and indexes
-- Safety: ADDITIVE ONLY - no data destruction

-- Step 1: Add P/E-based GARP screening indexes for performance
CREATE INDEX IF NOT EXISTS idx_income_statements_eps_growth 
ON income_statements(stock_id, period_type, report_date, net_income, shares_diluted);

CREATE INDEX IF NOT EXISTS idx_daily_ratios_pe_garp 
ON daily_valuation_ratios(stock_id, date, pe_ratio);

CREATE INDEX IF NOT EXISTS idx_balance_sheets_debt_equity 
ON balance_sheets(stock_id, period_type, report_date, total_debt, total_equity);

-- Step 2: Create P/E-based GARP screening helper view
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

-- Step 3: Create PEG ratio calculation helper view
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
# Run migration with safety checks
cargo run --bin db_admin -- migrate --confirm

# Verify migration success
sqlite3 db/stocks.db "SELECT COUNT(*) FROM garp_pe_screening_data;"
sqlite3 db/stocks.db "SELECT COUNT(*) FROM peg_ratio_analysis;"

# Test PEG calculation with sample data
sqlite3 db/stocks.db "SELECT symbol, current_pe_ratio, eps_growth_rate_ttm, peg_ratio FROM peg_ratio_analysis WHERE symbol = 'AAPL' LIMIT 1;"
```

### Phase 2: Backend Implementation

#### 2.1 New Data Structures

**File**: `src-tauri/src/models/garp_pe.rs`
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

**File**: `src-tauri/src/commands/garp_pe.rs`
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

#### 2.3 Command Registration

**File**: `src-tauri/src/commands/mod.rs`
```rust
pub mod garp_pe;

// Add to main.rs command registration
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            // ... existing commands ...
            crate::commands::garp_pe::get_garp_pe_screening_results,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Phase 3: Frontend Integration

#### 3.1 Enhanced API Service

**File**: `src/src/services/api.js`
```javascript
// Add to existing api.js
export const analysisAPI = {
    // ... existing methods ...
    
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

#### 3.2 Enhanced Data Service

**File**: `src/src/services/dataService.js`
```javascript
// Add to existing dataService.js
export const recommendationsDataService = {
    // ... existing methods ...
    
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

#### 3.3 Enhanced UI Component

**File**: `src/src/components/RecommendationsPanel.jsx`
```javascript
// Add P/E-based GARP screening option and filters
const screeningOptions = [
    { value: 'pe', label: 'P/E Ratio (Historical)' },
    { value: 'ps', label: 'P/S Ratio (Enhanced with Growth)' },
    { value: 'garp_pe', label: 'GARP (P/E + PEG Based)' } // NEW
];

// Add P/E-based GARP-specific state
const [garpPeCriteria, setGarpPeCriteria] = useState({
    maxPegRatio: 1.0,
    minRevenueGrowth: 15.0,
    minProfitMargin: 5.0,
    maxDebtToEquity: 2.0,
    minMarketCap: 500_000_000,
    minQualityScore: 50,
    requirePositiveEarnings: true
});

// Add P/E-based GARP-specific UI controls
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

### Phase 4: Testing Strategy

#### 4.1 Unit Tests

**File**: `src-tauri/tests/garp_pe_tests.rs`
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

#### 4.2 Integration Tests

**File**: `src-tauri/tests/backend_tests.rs`
```rust
// Add to existing backend_tests.rs
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

### Phase 5: Performance Optimization

#### 5.1 Database Indexes
```sql
-- Additional performance indexes for P/E-based GARP
CREATE INDEX IF NOT EXISTS idx_garp_pe_performance 
ON peg_ratio_analysis(stock_id, peg_ratio, eps_growth_rate_ttm);

CREATE INDEX IF NOT EXISTS idx_income_statements_eps_calculation 
ON income_statements(stock_id, period_type, report_date, net_income, shares_diluted) 
WHERE net_income > 0 AND shares_diluted > 0;
```

#### 5.2 Query Optimization
- Use prepared statements for repeated queries
- Implement query result caching for PEG calculations
- Add query execution time monitoring

### Phase 6: Deployment Strategy

#### 6.1 Migration Execution
```bash
# Step 1: Backup production database
cargo run --bin db_admin -- backup

# Step 2: Run GARP P/E migration
cargo run --bin db_admin -- migrate --confirm

# Step 3: Verify migration success
sqlite3 db/stocks.db "SELECT COUNT(*) FROM garp_pe_screening_data;"
sqlite3 db/stocks.db "SELECT COUNT(*) FROM peg_ratio_analysis;"

# Step 4: Test PEG calculation with sample data
sqlite3 db/stocks.db "SELECT symbol, current_pe_ratio, eps_growth_rate_ttm, peg_ratio FROM peg_ratio_analysis WHERE symbol = 'AAPL' LIMIT 1;"

# Step 5: Test GARP P/E screening
cargo test test_garp_pe_screening_integration --features test-utils
```

#### 6.2 Frontend Deployment
```bash
# Build frontend with GARP P/E support
cd src && npm run build

# Test GARP P/E UI components
npm test -- --testNamePattern="GARP.*PE"
```

## Implementation Timeline

### Week 1: Database & Backend
- **Day 1-2**: Create migration and test database changes
- **Day 3-4**: Implement GARP P/E command and data structures
- **Day 5**: Backend testing and optimization

### Week 2: Frontend & Integration
- **Day 1-2**: Implement API service and data service
- **Day 3-4**: Build GARP P/E UI components and filters
- **Day 5**: Integration testing and bug fixes

### Week 3: Testing & Deployment
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

## Future Improvements & Missing Features

### 🚨 **Critical Missing Features**

#### **1. Fair Value Calculation & Overvalued Detection**
**Current Status**: ⚠️ **Partial Implementation**
- **What We Have**: PEG ratio calculation and screening (PEG < 1.0)
- **What's Missing**: 
  - Absolute fair value price calculation
  - Overvalued stock detection (PEG > 2.0)
  - Percentage over/undervaluation display
  - Fair value vs current price comparison

**Required Implementation**:
```sql
-- Add to peg_ratio_analysis view:
CASE 
    WHEN peg_ratio > 0 AND eps_growth_rate_ttm > 0 THEN 
        eps_growth_rate_ttm * 1.0  -- Fair P/E (assuming PEG = 1.0 is fair)
    ELSE NULL 
END as fair_pe_ratio,

CASE 
    WHEN fair_pe_ratio > 0 AND current_eps_ttm > 0 THEN 
        fair_pe_ratio * current_eps_ttm  -- Fair Price
    ELSE NULL 
END as fair_price,

CASE 
    WHEN fair_price > 0 AND current_price > 0 THEN 
        (current_price - fair_price) / fair_price * 100  -- % Over/Under valued
    ELSE NULL 
END as valuation_premium_percent
```

#### **2. Enhanced Screening Criteria**
**Current Status**: ⚠️ **Basic Implementation**
- **What We Have**: Binary pass/fail screening
- **What's Missing**:
  - Graduated scoring system (0-100 scale)
  - Sector-relative PEG adjustments
  - Market cap-relative PEG thresholds
  - Historical PEG percentile ranking

#### **3. Risk Assessment & Quality Metrics**
**Current Status**: ⚠️ **Basic Implementation**
- **What We Have**: Simple quality score (0-100)
- **What's Missing**:
  - PEG volatility risk assessment
  - Earnings quality scoring
  - Growth sustainability analysis
  - Sector-specific risk adjustments

### 🔧 **Architecture Improvements**

#### **1. Enhanced Data Models**
```rust
pub struct GarpPeScreeningResult {
    // ... existing fields ...
    
    // NEW: Fair value fields
    pub fair_pe_ratio: Option<f64>,
    pub fair_price: Option<f64>,
    pub valuation_premium_percent: Option<f64>,
    pub valuation_status: Option<String>, // "Overvalued", "Undervalued", "Fair Value"
    
    // NEW: Enhanced screening
    pub is_overvalued: bool,
    pub is_undervalued: bool,
    pub is_fair_value: bool,
    
    // NEW: Risk metrics
    pub peg_volatility_risk: Option<f64>,
    pub earnings_quality_score: Option<f64>,
    pub growth_sustainability_score: Option<f64>,
    
    // NEW: Sector-relative metrics
    pub sector_relative_peg: Option<f64>,
    pub sector_percentile_rank: Option<f64>,
}
```

#### **2. Enhanced Screening Criteria**
```rust
pub struct GarpPeScreeningCriteria {
    // ... existing fields ...
    
    // NEW: Fair value thresholds
    pub fair_peg_threshold: f64,        // Default: 1.0
    pub overvalued_peg_threshold: f64,   // Default: 2.0
    pub max_valuation_premium: f64,      // Default: 20% (reject if >20% overvalued)
    
    // NEW: Risk management
    pub max_peg_volatility: f64,         // Default: 0.5
    pub min_earnings_quality: f64,        // Default: 70
    pub min_growth_sustainability: f64,   // Default: 60
    
    // NEW: Sector adjustments
    pub enable_sector_adjustments: bool,  // Default: true
    pub sector_percentile_threshold: f64, // Default: 25th percentile
}
```

#### **3. New Tauri Commands**
```rust
// Enhanced GARP screening with fair value
#[tauri::command]
pub async fn get_garp_pe_screening_results_enhanced(
    stock_tickers: Vec<String>,
    criteria: Option<GarpPeScreeningCriteria>,
    include_overvalued: bool,  // NEW: Include overvalued stocks
    limit: Option<i32>
) -> Result<Vec<GarpPeScreeningResult>, String>

// Fair value calculation for individual stocks
#[tauri::command]
pub async fn calculate_fair_value(
    symbol: String,
    method: String  // "peg", "dcf", "graham", "consensus"
) -> Result<FairValueResult, String>

// Overvalued stock screening
#[tauri::command]
pub async fn get_overvalued_stocks(
    stock_tickers: Vec<String>,
    criteria: Option<OvervaluedScreeningCriteria>
) -> Result<Vec<OvervaluedStock>, String>
```

### 📊 **Frontend Enhancements**

#### **1. Valuation Dashboard**
```jsx
// Enhanced GARP results display
<div className="valuation-dashboard">
    <div className="valuation-summary">
        <div className="valuation-status">
            {stock.valuation_status}: {stock.valuation_premium_percent?.toFixed(1)}%
        </div>
        <div className="fair-value-info">
            Fair Price: ${stock.fair_price?.toFixed(2)}
            Current Price: ${stock.current_price?.toFixed(2)}
        </div>
    </div>
    
    <div className="risk-metrics">
        <div className="risk-score">
            PEG Volatility Risk: {stock.peg_volatility_risk?.toFixed(1)}
        </div>
        <div className="quality-score">
            Earnings Quality: {stock.earnings_quality_score?.toFixed(0)}/100
        </div>
    </div>
</div>
```

#### **2. Enhanced Filtering Options**
```jsx
// New filtering options
<div className="enhanced-filters">
    <div className="valuation-filters">
        <label>Include Overvalued Stocks:</label>
        <input type="checkbox" checked={includeOvervalued} />
        
        <label>Max Valuation Premium:</label>
        <select value={maxValuationPremium}>
            <option value={10}>10%</option>
            <option value={20}>20%</option>
            <option value={50}>50%</option>
        </select>
    </div>
    
    <div className="risk-filters">
        <label>Max PEG Volatility:</label>
        <select value={maxPegVolatility}>
            <option value={0.3}>0.3</option>
            <option value={0.5}>0.5</option>
            <option value={1.0}>1.0</option>
        </select>
    </div>
</div>
```

### 🎯 **Implementation Priority**

#### **Phase 1: Fair Value Calculation (High Priority)**
- Add fair value price calculation to database views
- Update Rust models to include fair value fields
- Enhance frontend to display valuation percentages
- **Timeline**: 1-2 weeks

#### **Phase 2: Overvalued Detection (High Priority)**
- Implement overvalued stock screening
- Add overvalued stock display in frontend
- Create risk management dashboard
- **Timeline**: 1-2 weeks

#### **Phase 3: Enhanced Risk Metrics (Medium Priority)**
- Implement PEG volatility risk assessment
- Add earnings quality scoring
- Create sector-relative adjustments
- **Timeline**: 2-3 weeks

#### **Phase 4: Advanced Features (Low Priority)**
- DCF-based fair value calculation
- Consensus fair value from multiple methods
- Historical fair value tracking
- **Timeline**: 3-4 weeks

### 📈 **Expected Benefits**

#### **1. Complete Valuation Picture**
- Users see both undervalued AND overvalued stocks
- Precise percentage over/undervaluation
- Fair value vs current price comparison

#### **2. Risk Management**
- Identify overvalued stocks to avoid or short
- PEG volatility risk assessment
- Earnings quality scoring

#### **3. Enhanced Decision Making**
- Portfolio balance between undervalued and fairly valued stocks
- Confidence scoring for valuation accuracy
- Sector-relative valuation adjustments

---

**This P/E-based GARP implementation plan provides a comprehensive roadmap for implementing the enhanced GARP screening system using PEG ratios and earnings growth for superior investment decision-making.**
