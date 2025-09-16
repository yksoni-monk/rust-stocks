# GARP Implementation Plan - Detailed Technical Specification

## Executive Summary

This document provides a comprehensive implementation plan for the Growth at Reasonable Price (GARP) stock screening system. The plan includes database migrations, API development, testing strategies, and deployment considerations.

## Current System Analysis

### ✅ **Database Technology Stack**
- **SQLx**: Version 0.8 with SQLite support
- **Migration System**: Manual SQL migrations in `db/migrations/`
- **Connection Pooling**: 20 max connections, WAL mode enabled
- **Test Infrastructure**: Test database injection system

### ✅ **Data Availability**
- **Income Statements**: 115,137 records (TTM: 54K, Annual: 15K, Quarterly: 45K)
- **Balance Sheets**: 104,833 records (TTM, Annual, Quarterly)
- **Required Fields**: Revenue, Net Income, Total Debt, Total Equity ✅

### ✅ **Current Architecture**
- **Tauri Commands**: Analysis commands in `src/commands/analysis.rs`
- **Database Helpers**: SQLx-based operations in `src/database/helpers.rs`
- **Frontend Integration**: React components with service layer

## Implementation Plan

### Phase 1: Database Schema Enhancements

#### 1.1 New Migration: `20250916000007_add_garp_screening.sql`

```sql
-- Migration: Add GARP Screening Support
-- File: 20250916000007_add_garp_screening.sql
-- Purpose: Add GARP-specific indexes and helper views
-- Safety: ADDITIVE ONLY - no data destruction

-- Step 1: Add GARP-specific indexes for performance
CREATE INDEX IF NOT EXISTS idx_income_statements_garp_screening 
ON income_statements(stock_id, period_type, report_date, revenue, net_income);

CREATE INDEX IF NOT EXISTS idx_balance_sheets_garp_screening 
ON balance_sheets(stock_id, period_type, report_date, total_debt, total_equity);

CREATE INDEX IF NOT EXISTS idx_stocks_sector_symbol 
ON stocks(sector, symbol);

-- Step 2: Create GARP screening helper view
CREATE VIEW IF NOT EXISTS garp_screening_data AS
SELECT 
    s.id as stock_id,
    s.symbol,
    s.sector,
    
    -- Latest P/S ratio
    dvr.ps_ratio_ttm as current_ps,
    dvr.market_cap,
    dvr.price,
    
    -- Revenue data (TTM)
    ttm_income.revenue as current_ttm_revenue,
    ttm_income.net_income as current_ttm_net_income,
    
    -- Previous TTM for growth calculation
    prev_ttm_income.revenue as previous_ttm_revenue,
    
    -- Annual revenue data
    annual_income.revenue as current_annual_revenue,
    annual_income.net_income as current_annual_net_income,
    
    -- Previous annual for growth calculation
    prev_annual_income.revenue as previous_annual_revenue,
    
    -- Balance sheet data
    ttm_balance.total_debt,
    ttm_balance.total_equity,
    
    -- Calculated metrics
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
    SELECT stock_id, revenue, net_income,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
    FROM income_statements 
    WHERE period_type = 'TTM'
) ttm_income ON s.id = ttm_income.stock_id AND ttm_income.rn = 1

-- Previous TTM income data
LEFT JOIN (
    SELECT stock_id, revenue,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
    FROM income_statements 
    WHERE period_type = 'TTM'
) prev_ttm_income ON s.id = prev_ttm_income.stock_id AND prev_ttm_income.rn = 2

-- Current Annual income data
LEFT JOIN (
    SELECT stock_id, revenue, net_income,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY fiscal_year DESC) as rn
    FROM income_statements 
    WHERE period_type = 'Annual'
) annual_income ON s.id = annual_income.stock_id AND annual_income.rn = 1

-- Previous Annual income data
LEFT JOIN (
    SELECT stock_id, revenue,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY fiscal_year DESC) as rn
    FROM income_statements 
    WHERE period_type = 'Annual'
) prev_annual_income ON s.id = prev_annual_income.stock_id AND prev_annual_income.rn = 2

-- Current TTM balance sheet data
LEFT JOIN (
    SELECT stock_id, total_debt, total_equity,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
    FROM balance_sheets 
    WHERE period_type = 'TTM'
) ttm_balance ON s.id = ttm_balance.stock_id AND ttm_balance.rn = 1;

-- Step 3: Create sector P/S median calculation view
CREATE VIEW IF NOT EXISTS sector_ps_medians AS
SELECT 
    s.sector,
    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY dvr.ps_ratio_ttm) as sector_median,
    COUNT(*) as stock_count
FROM stocks s
JOIN daily_valuation_ratios dvr ON s.id = dvr.stock_id
WHERE dvr.ps_ratio_ttm IS NOT NULL 
  AND dvr.ps_ratio_ttm > 0.01
  AND dvr.date = (SELECT MAX(date) FROM daily_valuation_ratios WHERE stock_id = s.id)
  AND s.sector IS NOT NULL
GROUP BY s.sector
HAVING COUNT(*) >= 5;  -- Minimum 5 stocks per sector for reliable median
```

#### 1.2 Migration Execution Strategy
```bash
# Run migration with safety checks
cargo run --bin db_admin -- migrate --confirm

# Verify migration success
sqlite3 db/stocks.db "SELECT COUNT(*) FROM garp_screening_data;"
sqlite3 db/stocks.db "SELECT COUNT(*) FROM sector_ps_medians;"
```

### Phase 2: Backend Implementation

#### 2.1 New Data Structures

**File**: `src-tauri/src/models/garp.rs`
```rust
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GarpScreeningResult {
    pub stock_id: i32,
    pub symbol: String,
    pub sector: Option<String>,
    
    // P/S Analysis
    pub current_ps: f64,
    pub sector_ps_threshold: f64,
    pub passes_ps_filter: bool,
    
    // Revenue Growth Analysis
    pub current_ttm_revenue: Option<f64>,
    pub ttm_growth_rate: Option<f64>,
    pub current_annual_revenue: Option<f64>,
    pub annual_growth_rate: Option<f64>,
    pub passes_growth_filter: bool,
    
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
    pub price: f64,
    pub data_completeness_score: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GarpScreeningCriteria {
    pub min_growth_rate: f64,        // Default: 15.0%
    pub min_profit_margin: f64,      // Default: 5.0%
    pub max_debt_to_equity: f64,     // Default: 2.0
    pub max_ps_ratio: f64,           // Default: 3.0
    pub use_sector_medians: bool,    // Default: true
    pub min_market_cap: f64,         // Default: $500M
    pub min_quality_score: i32,       // Default: 50
}
```

#### 2.2 New Tauri Command

**File**: `src-tauri/src/commands/garp.rs`
```rust
use sqlx::SqlitePool;
use crate::models::garp::{GarpScreeningResult, GarpScreeningCriteria};
use crate::database::helpers::get_database_connection;

#[tauri::command]
pub async fn get_garp_screening_results(
    stock_tickers: Vec<String>, 
    criteria: Option<GarpScreeningCriteria>,
    limit: Option<i32>
) -> Result<Vec<GarpScreeningResult>, String> {
    let pool = get_database_connection().await?;
    let criteria = criteria.unwrap_or_default();
    let limit_value = limit.unwrap_or(50);
    
    if stock_tickers.is_empty() {
        return Ok(vec![]);
    }
    
    // Create placeholders for the IN clause
    let placeholders = stock_tickers.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    
    let query = format!("
        WITH garp_data AS (
            SELECT 
                gsd.*,
                COALESCE(spm.sector_median, ?) as sector_ps_threshold,
                
                -- P/S filter logic
                CASE 
                    WHEN ? = 1 THEN  -- Use sector medians
                        gsd.current_ps < COALESCE(spm.sector_median, ?)
                    ELSE  -- Use fixed threshold
                        gsd.current_ps < ?
                END as passes_ps_filter,
                
                -- Growth filter logic
                CASE 
                    WHEN gsd.ttm_growth_rate IS NOT NULL AND gsd.ttm_growth_rate > ? THEN true
                    WHEN gsd.annual_growth_rate IS NOT NULL AND gsd.annual_growth_rate > ? THEN true
                    ELSE false
                END as passes_growth_filter,
                
                -- Profitability filter logic
                CASE 
                    WHEN gsd.net_profit_margin IS NOT NULL AND gsd.net_profit_margin > ? THEN true
                    ELSE false
                END as passes_profitability_filter,
                
                -- Debt filter logic
                CASE 
                    WHEN gsd.debt_to_equity_ratio IS NOT NULL AND gsd.debt_to_equity_ratio < ? THEN true
                    WHEN gsd.debt_to_equity_ratio IS NULL THEN true  -- Pass if no data
                    ELSE false
                END as passes_debt_filter,
                
                -- GARP Score calculation
                CASE 
                    WHEN gsd.current_ps > 0 AND gsd.ttm_growth_rate IS NOT NULL THEN 
                        gsd.ttm_growth_rate / gsd.current_ps
                    WHEN gsd.current_ps > 0 AND gsd.annual_growth_rate IS NOT NULL THEN 
                        gsd.annual_growth_rate / gsd.current_ps
                    ELSE 0
                END as garp_score,
                
                -- Quality score calculation
                CASE 
                    WHEN gsd.ttm_growth_rate IS NOT NULL 
                         AND gsd.net_profit_margin IS NOT NULL 
                         AND gsd.debt_to_equity_ratio IS NOT NULL THEN 100
                    WHEN gsd.ttm_growth_rate IS NOT NULL 
                         AND gsd.net_profit_margin IS NOT NULL THEN 75
                    WHEN gsd.ttm_growth_rate IS NOT NULL 
                         OR gsd.net_profit_margin IS NOT NULL THEN 50
                    ELSE 25
                END as quality_score
                
            FROM garp_screening_data gsd
            LEFT JOIN sector_ps_medians spm ON gsd.sector = spm.sector
            WHERE gsd.symbol IN ({})
              AND gsd.market_cap > ?
        )
        SELECT 
            stock_id,
            symbol,
            sector,
            current_ps,
            sector_ps_threshold,
            passes_ps_filter,
            current_ttm_revenue,
            ttm_growth_rate,
            current_annual_revenue,
            annual_growth_rate,
            passes_growth_filter,
            current_ttm_net_income,
            net_profit_margin,
            passes_profitability_filter,
            total_debt,
            total_equity,
            debt_to_equity_ratio,
            passes_debt_filter,
            garp_score,
            quality_score,
            
            -- Final GARP screening result
            CASE 
                WHEN passes_ps_filter 
                     AND passes_growth_filter 
                     AND passes_profitability_filter 
                     AND passes_debt_filter
                     AND quality_score >= ?
                THEN true
                ELSE false
            END as passes_garp_screening,
            
            market_cap,
            price,
            data_completeness_score
            
        FROM garp_data
        ORDER BY 
            passes_garp_screening DESC,
            garp_score DESC,
            quality_score DESC,
            current_ps ASC
        LIMIT ?
    ", placeholders);
    
    let mut query_builder = sqlx::query_as::<_, GarpScreeningResult>(&query);
    
    // Bind parameters in order
    query_builder = query_builder.bind(criteria.max_ps_ratio);  // COALESCE fallback
    query_builder = query_builder.bind(if criteria.use_sector_medians { 1 } else { 0 });
    query_builder = query_builder.bind(criteria.max_ps_ratio);  // Sector median fallback
    query_builder = query_builder.bind(criteria.max_ps_ratio);  // Fixed threshold
    
    // Growth rate thresholds (TTM and Annual)
    query_builder = query_builder.bind(criteria.min_growth_rate);
    query_builder = query_builder.bind(criteria.min_growth_rate * 0.67); // Annual threshold (10% vs 15%)
    
    query_builder = query_builder.bind(criteria.min_profit_margin);
    query_builder = query_builder.bind(criteria.max_debt_to_equity);
    
    // Bind stock tickers
    for ticker in &stock_tickers {
        query_builder = query_builder.bind(ticker);
    }
    
    // Bind remaining parameters
    query_builder = query_builder.bind(criteria.min_market_cap);
    query_builder = query_builder.bind(criteria.min_quality_score);
    query_builder = query_builder.bind(limit_value);
    
    let results = query_builder.fetch_all(&pool).await
        .map_err(|e| format!("GARP screening query failed: {}", e))?;
    
    Ok(results)
}

impl Default for GarpScreeningCriteria {
    fn default() -> Self {
        Self {
            min_growth_rate: 15.0,        // 15% TTM or 10% Annual
            min_profit_margin: 5.0,       // 5% net margin
            max_debt_to_equity: 2.0,      // D/E < 2.0
            max_ps_ratio: 3.0,            // P/S < 3.0
            use_sector_medians: true,     // Use sector-specific thresholds
            min_market_cap: 500_000_000.0, // $500M minimum
            min_quality_score: 50,        // Minimum data quality
        }
    }
}
```

#### 2.3 Command Registration

**File**: `src-tauri/src/commands/mod.rs`
```rust
pub mod garp;

// Add to main.rs command registration
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            // ... existing commands ...
            crate::commands::garp::get_garp_screening_results,
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
    
    async getGarpScreeningResults(stockTickers, criteria, limit) {
        try {
            const result = await invoke('get_garp_screening_results', {
                stockTickers,
                criteria: criteria || {
                    minGrowthRate: 15.0,
                    minProfitMargin: 5.0,
                    maxDebtToEquity: 2.0,
                    maxPsRatio: 3.0,
                    useSectorMedians: true,
                    minMarketCap: 500_000_000,
                    minQualityScore: 50
                },
                limit: limit || 50
            });
            return result;
        } catch (error) {
            console.error('GARP screening API error:', error);
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
    
    async loadGarpScreeningResults(stockTickers, criteria, limit) {
        try {
            const result = await analysisAPI.getGarpScreeningResults(stockTickers, criteria, limit);
            
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
// Add GARP screening option and filters
const screeningOptions = [
    { value: 'pe', label: 'P/E Ratio (Historical)' },
    { value: 'ps', label: 'P/S Ratio (Enhanced with Growth)' },
    { value: 'garp', label: 'GARP (Growth at Reasonable Price)' } // NEW
];

// Add GARP-specific state
const [garpCriteria, setGarpCriteria] = useState({
    minGrowthRate: 15.0,
    minProfitMargin: 5.0,
    maxDebtToEquity: 2.0,
    maxPsRatio: 3.0,
    useSectorMedians: true,
    minMarketCap: 500_000_000,
    minQualityScore: 50
});

// Add GARP-specific UI controls
{screeningType === 'garp' && (
    <div className="garp-controls space-y-4">
        <div className="grid grid-cols-2 gap-4">
            <div>
                <label className="text-sm font-medium text-gray-700">
                    Min Growth Rate (%):
                </label>
                <select
                    value={garpCriteria.minGrowthRate}
                    onChange={(e) => setGarpCriteria(prev => ({
                        ...prev,
                        minGrowthRate: Number(e.target.value)
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
                    value={garpCriteria.minProfitMargin}
                    onChange={(e) => setGarpCriteria(prev => ({
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
                    value={garpCriteria.maxDebtToEquity}
                    onChange={(e) => setGarpCriteria(prev => ({
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
            
            <div>
                <label className="text-sm font-medium text-gray-700">
                    Max P/S Ratio:
                </label>
                <select
                    value={garpCriteria.maxPsRatio}
                    onChange={(e) => setGarpCriteria(prev => ({
                        ...prev,
                        maxPsRatio: Number(e.target.value)
                    }))}
                    className="border border-gray-300 rounded px-2 py-1 text-sm w-full"
                >
                    <option value={2}>2.0</option>
                    <option value={3}>3.0</option>
                    <option value={5}>5.0</option>
                    <option value={10}>10.0</option>
                </select>
            </div>
        </div>
        
        <div className="flex items-center gap-4">
            <label className="flex items-center gap-2">
                <input
                    type="checkbox"
                    checked={garpCriteria.useSectorMedians}
                    onChange={(e) => setGarpCriteria(prev => ({
                        ...prev,
                        useSectorMedians: e.target.checked
                    }))}
                    className="rounded"
                />
                <span className="text-sm text-gray-700">Use Sector-Specific P/S Thresholds</span>
            </label>
        </div>
    </div>
)}
```

### Phase 4: Testing Strategy

#### 4.1 Unit Tests

**File**: `src-tauri/tests/garp_tests.rs`
```rust
#[cfg(test)]
mod garp_tests {
    use super::*;
    use crate::tests::helpers::database::SimpleTestDatabase;
    use crate::commands::garp::get_garp_screening_results;
    use crate::models::garp::GarpScreeningCriteria;

    #[tokio::test]
    async fn test_garp_screening_basic() {
        let test_db = SimpleTestDatabase::new().await.unwrap();
        set_test_database_pool(test_db.pool().clone()).await;
        
        let criteria = GarpScreeningCriteria {
            min_growth_rate: 15.0,
            min_profit_margin: 5.0,
            max_debt_to_equity: 2.0,
            max_ps_ratio: 3.0,
            use_sector_medians: true,
            min_market_cap: 500_000_000.0,
            min_quality_score: 50,
        };
        
        let result = get_garp_screening_results(
            vec!["AAPL".to_string(), "MSFT".to_string()],
            Some(criteria),
            Some(10)
        ).await;
        
        assert!(result.is_ok());
        let stocks = result.unwrap();
        assert!(!stocks.is_empty());
        
        // Verify GARP criteria are met
        for stock in &stocks {
            if stock.passes_garp_screening {
                assert!(stock.passes_ps_filter);
                assert!(stock.passes_growth_filter);
                assert!(stock.passes_profitability_filter);
                assert!(stock.passes_debt_filter);
                assert!(stock.quality_score >= 50);
            }
        }
        
        clear_test_database_pool().await;
        test_db.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_garp_screening_performance() {
        let test_db = SimpleTestDatabase::new().await.unwrap();
        set_test_database_pool(test_db.pool().clone()).await;
        
        let start_time = std::time::Instant::now();
        
        let result = get_garp_screening_results(
            vec!["AAPL".to_string(), "MSFT".to_string(), "GOOGL".to_string()],
            None, // Use default criteria
            Some(50)
        ).await;
        
        let duration = start_time.elapsed();
        
        assert!(result.is_ok());
        assert!(duration.as_secs() < 5, "GARP screening should complete in <5 seconds");
        
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
async fn test_garp_screening_integration() {
    let test_db = SimpleTestDatabase::new().await.unwrap();
    set_test_database_pool(test_db.pool().clone()).await;
    
    // Test with S&P 500 symbols
    let sp500_symbols = vec![
        "AAPL".to_string(), "MSFT".to_string(), "GOOGL".to_string(),
        "AMZN".to_string(), "TSLA".to_string(), "META".to_string()
    ];
    
    let result = get_garp_screening_results(sp500_symbols, None, Some(20)).await;
    
    assert!(result.is_ok());
    let stocks = result.unwrap();
    
    // Verify we get some results
    assert!(!stocks.is_empty());
    
    // Verify data quality
    for stock in &stocks {
        assert!(!stock.symbol.is_empty());
        assert!(stock.current_ps > 0.0);
        assert!(stock.market_cap > 0.0);
    }
    
    clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}
```

### Phase 5: Performance Optimization

#### 5.1 Database Indexes
```sql
-- Additional performance indexes
CREATE INDEX IF NOT EXISTS idx_garp_screening_performance 
ON garp_screening_data(stock_id, current_ps, ttm_growth_rate, net_profit_margin);

CREATE INDEX IF NOT EXISTS idx_sector_ps_performance 
ON sector_ps_medians(sector, sector_median);
```

#### 5.2 Query Optimization
- Use prepared statements for repeated queries
- Implement query result caching for sector medians
- Add query execution time monitoring

### Phase 6: Deployment Strategy

#### 6.1 Migration Execution
```bash
# Step 1: Backup production database
cargo run --bin db_admin -- backup

# Step 2: Run GARP migration
cargo run --bin db_admin -- migrate --confirm

# Step 3: Verify migration success
sqlite3 db/stocks.db "SELECT COUNT(*) FROM garp_screening_data;"
sqlite3 db/stocks.db "SELECT COUNT(*) FROM sector_ps_medians;"

# Step 4: Test GARP screening
cargo test test_garp_screening_integration --features test-utils
```

#### 6.2 Frontend Deployment
```bash
# Build frontend with GARP support
cd src && npm run build

# Test GARP UI components
npm test -- --testNamePattern="GARP"
```

## Implementation Timeline

### Week 1: Database & Backend
- **Day 1-2**: Create migration and test database changes
- **Day 3-4**: Implement GARP command and data structures
- **Day 5**: Backend testing and optimization

### Week 2: Frontend & Integration
- **Day 1-2**: Implement API service and data service
- **Day 3-4**: Build GARP UI components and filters
- **Day 5**: Integration testing and bug fixes

### Week 3: Testing & Deployment
- **Day 1-2**: Comprehensive testing (unit, integration, performance)
- **Day 3-4**: Performance optimization and monitoring
- **Day 5**: Production deployment and validation

## Success Metrics

### Performance Targets
- **Query Time**: < 3 seconds for S&P 500 GARP screening
- **Memory Usage**: < 100MB additional memory overhead
- **Data Coverage**: > 80% of S&P 500 stocks with GARP data

### Quality Targets
- **Test Coverage**: > 90% for GARP-specific code
- **Data Accuracy**: > 95% accuracy for GARP calculations
- **User Experience**: < 2 second UI response time

## Risk Mitigation

### Database Safety
- **Automatic Backups**: Before any migration
- **Rollback Plan**: Timestamped backups for restoration
- **Production Detection**: Safety checks for large databases

### Performance Risks
- **Query Optimization**: Comprehensive indexing strategy
- **Caching Strategy**: Sector median caching
- **Monitoring**: Query execution time tracking

### Data Quality Risks
- **Validation**: Comprehensive data validation
- **Error Handling**: Graceful degradation for missing data
- **Quality Scoring**: Data completeness assessment

---

**This implementation plan provides a comprehensive roadmap for implementing the GARP screening system with proper database migrations, API development, testing strategies, and deployment considerations.**
