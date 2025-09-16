# Growth at Reasonable Price (GARP) Stock Screening Architecture

## Executive Summary

This document outlines the architecture for implementing a comprehensive Growth at Reasonable Price (GARP) stock screening system based on the provided strategy writeup. The GARP approach combines value investing (low Price-to-Sales ratio) with quality checks (revenue growth and profitability) to identify undervalued stocks with strong fundamentals.

## GARP Strategy Overview

### Core Philosophy
- **Value Component**: Low P/S ratios indicate undervaluation
- **Growth Component**: Strong revenue growth ensures business momentum
- **Quality Component**: Positive profitability and manageable debt
- **Sector Awareness**: Sector-specific P/S thresholds for precision

### Investment Strategy Applications
1. **Value + Growth Hybrid**: Stocks with low P/S AND growing revenues
2. **Quality Value Investing**: High-quality companies temporarily undervalued
3. **Contrarian Growth**: Growing companies with depressed valuations

## Enhanced GARP Algorithm Design

### Core Screening Criteria (ALL FOUR Required)

1. **Low P/S Ratio**: Current P/S < 3.0 (or sector median for precision)
2. **Strong Revenue Growth**: TTM or 5-year revenue growth > 15% (or YoY > 10%)
3. **Positive Profitability**: Net profit margin > 5%
4. **Optional Quality Check**: Debt-to-Equity < 2 (if balance sheet data available)

### Algorithm Logic

#### Phase 1: P/S Ratio Screening
```sql
-- Sector-aware P/S screening
WITH sector_ps_medians AS (
    SELECT 
        s.sector,
        PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY dvr.ps_ratio_ttm) as sector_median
    FROM stocks s
    JOIN daily_valuation_ratios dvr ON s.id = dvr.stock_id
    WHERE dvr.ps_ratio_ttm IS NOT NULL 
      AND dvr.ps_ratio_ttm > 0.01
      AND dvr.date = (SELECT MAX(date) FROM daily_valuation_ratios WHERE stock_id = s.id)
    GROUP BY s.sector
),
ps_screening AS (
    SELECT 
        s.id as stock_id,
        s.symbol,
        s.sector,
        dvr.ps_ratio_ttm as current_ps,
        COALESCE(spm.sector_median, 3.0) as sector_ps_threshold,
        CASE 
            WHEN spm.sector_median IS NOT NULL THEN 
                dvr.ps_ratio_ttm < spm.sector_median
            ELSE 
                dvr.ps_ratio_ttm < 3.0
        END as passes_ps_filter
    FROM stocks s
    JOIN daily_valuation_ratios dvr ON s.id = dvr.stock_id
    LEFT JOIN sector_ps_medians spm ON s.sector = spm.sector
    WHERE dvr.date = (SELECT MAX(date) FROM daily_valuation_ratios WHERE stock_id = s.id)
      AND dvr.ps_ratio_ttm IS NOT NULL 
      AND dvr.ps_ratio_ttm > 0.01
)
```

#### Phase 2: Revenue Growth Analysis
```sql
-- Multi-period revenue growth analysis
WITH revenue_growth_analysis AS (
    SELECT 
        s.id as stock_id,
        s.symbol,
        
        -- Current TTM revenue
        current_ttm.revenue as current_ttm_revenue,
        
        -- TTM revenue growth (current vs previous TTM)
        CASE 
            WHEN prev_ttm.revenue > 0 THEN 
                ((current_ttm.revenue - prev_ttm.revenue) / prev_ttm.revenue) * 100
            ELSE NULL 
        END as ttm_growth_rate,
        
        -- Current Annual revenue
        current_annual.revenue as current_annual_revenue,
        
        -- Annual revenue growth (latest vs previous annual)
        CASE 
            WHEN prev_annual.revenue > 0 THEN 
                ((current_annual.revenue - prev_annual.revenue) / prev_annual.revenue) * 100
            ELSE NULL 
        END as annual_growth_rate,
        
        -- 5-year revenue growth (if available)
        CASE 
            WHEN revenue_5y_ago.revenue > 0 THEN 
                ((current_annual.revenue - revenue_5y_ago.revenue) / revenue_5y_ago.revenue) * 100
            ELSE NULL 
        END as five_year_growth_rate,
        
        -- Growth requirement check
        CASE 
            WHEN prev_ttm.revenue > 0 AND ((current_ttm.revenue - prev_ttm.revenue) / prev_ttm.revenue) * 100 > 15 THEN true
            WHEN prev_annual.revenue > 0 AND ((current_annual.revenue - prev_annual.revenue) / prev_annual.revenue) * 100 > 10 THEN true
            WHEN revenue_5y_ago.revenue > 0 AND ((current_annual.revenue - revenue_5y_ago.revenue) / revenue_5y_ago.revenue) * 100 > 15 THEN true
            ELSE false
        END as passes_growth_filter
        
    FROM stocks s
    
    -- Current TTM revenue
    LEFT JOIN (
        SELECT stock_id, revenue, report_date,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
        FROM income_statements 
        WHERE period_type = 'TTM'
    ) current_ttm ON s.id = current_ttm.stock_id AND current_ttm.rn = 1
    
    -- Previous TTM revenue (12 months earlier)
    LEFT JOIN (
        SELECT stock_id, revenue, report_date,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
        FROM income_statements 
        WHERE period_type = 'TTM'
    ) prev_ttm ON s.id = prev_ttm.stock_id AND prev_ttm.rn = 2
    
    -- Current Annual revenue
    LEFT JOIN (
        SELECT stock_id, revenue, fiscal_year,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY fiscal_year DESC) as rn
        FROM income_statements 
        WHERE period_type = 'Annual'
    ) current_annual ON s.id = current_annual.stock_id AND current_annual.rn = 1
    
    -- Previous Annual revenue
    LEFT JOIN (
        SELECT stock_id, revenue, fiscal_year,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY fiscal_year DESC) as rn
        FROM income_statements 
        WHERE period_type = 'Annual'
    ) prev_annual ON s.id = prev_annual.stock_id AND prev_annual.rn = 2
    
    -- 5-year ago revenue
    LEFT JOIN (
        SELECT stock_id, revenue, fiscal_year,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY fiscal_year DESC) as rn
        FROM income_statements 
        WHERE period_type = 'Annual'
    ) revenue_5y_ago ON s.id = revenue_5y_ago.stock_id AND revenue_5y_ago.rn = 6
)
```

#### Phase 3: Profitability Analysis
```sql
-- Profitability screening
WITH profitability_analysis AS (
    SELECT 
        s.id as stock_id,
        s.symbol,
        
        -- Current net income and revenue for margin calculation
        current_income.net_income as current_net_income,
        current_income.revenue as current_revenue,
        
        -- Net profit margin calculation
        CASE 
            WHEN current_income.revenue > 0 THEN 
                (current_income.net_income / current_income.revenue) * 100
            ELSE NULL 
        END as net_profit_margin,
        
        -- Profitability requirement check
        CASE 
            WHEN current_income.revenue > 0 AND (current_income.net_income / current_income.revenue) * 100 > 5 THEN true
            ELSE false
        END as passes_profitability_filter
        
    FROM stocks s
    
    -- Current income statement data (TTM preferred, Annual fallback)
    LEFT JOIN (
        SELECT stock_id, net_income, revenue, period_type,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY 
                   CASE WHEN period_type = 'TTM' THEN 1 ELSE 2 END,
                   report_date DESC) as rn
        FROM income_statements 
        WHERE period_type IN ('TTM', 'Annual')
    ) current_income ON s.id = current_income.stock_id AND current_income.rn = 1
)
```

#### Phase 4: Debt-to-Equity Analysis (Optional)
```sql
-- Debt-to-equity analysis
WITH debt_equity_analysis AS (
    SELECT 
        s.id as stock_id,
        s.symbol,
        
        -- Current debt and equity data
        current_balance.total_debt,
        current_balance.total_equity,
        
        -- Debt-to-equity ratio calculation
        CASE 
            WHEN current_balance.total_equity > 0 THEN 
                current_balance.total_debt / current_balance.total_equity
            ELSE NULL 
        END as debt_to_equity_ratio,
        
        -- Debt quality check (optional filter)
        CASE 
            WHEN current_balance.total_equity > 0 AND (current_balance.total_debt / current_balance.total_equity) < 2 THEN true
            WHEN current_balance.total_debt IS NULL OR current_balance.total_equity IS NULL THEN true  -- Pass if no data
            ELSE false
        END as passes_debt_filter
        
    FROM stocks s
    
    -- Current balance sheet data
    LEFT JOIN (
        SELECT stock_id, total_debt, total_equity, period_type,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY 
                   CASE WHEN period_type = 'TTM' THEN 1 ELSE 2 END,
                   report_date DESC) as rn
        FROM balance_sheets 
        WHERE period_type IN ('TTM', 'Annual')
    ) current_balance ON s.id = current_balance.stock_id AND current_balance.rn = 1
)
```

#### Phase 5: GARP Score Calculation and Final Screening
```sql
-- Final GARP screening with scoring
SELECT 
    ps.stock_id,
    ps.symbol,
    ps.sector,
    ps.current_ps,
    ps.sector_ps_threshold,
    
    -- Revenue growth metrics
    rga.current_ttm_revenue,
    rga.ttm_growth_rate,
    rga.current_annual_revenue,
    rga.annual_growth_rate,
    rga.five_year_growth_rate,
    
    -- Profitability metrics
    pa.current_net_income,
    pa.current_revenue,
    pa.net_profit_margin,
    
    -- Debt metrics (optional)
    dea.total_debt,
    dea.total_equity,
    dea.debt_to_equity_ratio,
    
    -- GARP Score: Revenue Growth % / P/S Ratio
    CASE 
        WHEN ps.current_ps > 0 AND rga.ttm_growth_rate IS NOT NULL THEN 
            rga.ttm_growth_rate / ps.current_ps
        WHEN ps.current_ps > 0 AND rga.annual_growth_rate IS NOT NULL THEN 
            rga.annual_growth_rate / ps.current_ps
        ELSE 0
    END as garp_score,
    
    -- Quality score (0-100)
    CASE 
        WHEN rga.ttm_growth_rate IS NOT NULL AND pa.net_profit_margin IS NOT NULL AND dea.debt_to_equity_ratio IS NOT NULL THEN 100
        WHEN rga.ttm_growth_rate IS NOT NULL AND pa.net_profit_margin IS NOT NULL THEN 75
        WHEN rga.ttm_growth_rate IS NOT NULL OR pa.net_profit_margin IS NOT NULL THEN 50
        ELSE 25
    END as quality_score,
    
    -- GARP screening result
    CASE 
        WHEN ps.passes_ps_filter 
             AND rga.passes_growth_filter 
             AND pa.passes_profitability_filter 
             AND dea.passes_debt_filter
        THEN true
        ELSE false
    END as passes_garp_screening,
    
    -- Market metrics
    dvr.market_cap,
    dvr.price,
    dvr.data_completeness_score
    
FROM ps_screening ps
JOIN revenue_growth_analysis rga ON ps.stock_id = rga.stock_id
JOIN profitability_analysis pa ON ps.stock_id = pa.stock_id
LEFT JOIN debt_equity_analysis dea ON ps.stock_id = dea.stock_id
JOIN daily_valuation_ratios dvr ON ps.stock_id = dvr.stock_id 
    AND dvr.date = (SELECT MAX(date) FROM daily_valuation_ratios WHERE stock_id = ps.stock_id)
WHERE dvr.market_cap > ?  -- Market cap filter
ORDER BY 
    passes_garp_screening DESC,
    garp_score DESC,
    quality_score DESC,
    ps.current_ps ASC
LIMIT ?;
```

## Backend Implementation

### New Command: `get_garp_screening_results`
```rust
#[tauri::command]
pub async fn get_garp_screening_results(
    stock_tickers: Vec<String>, 
    limit: Option<i32>, 
    min_market_cap: Option<f64>,
    min_growth_rate: Option<f64>,        // NEW: Minimum growth rate filter
    min_profit_margin: Option<f64>,       // NEW: Minimum profit margin filter
    max_debt_to_equity: Option<f64>,     // NEW: Maximum debt-to-equity filter
    use_sector_medians: Option<bool>     // NEW: Use sector-specific P/S thresholds
) -> Result<Vec<GarpScreeningResult>, String>
```

### Enhanced Data Structure
```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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
    pub five_year_growth_rate: Option<f64>,
    pub passes_growth_filter: bool,
    
    // Profitability Analysis
    pub current_net_income: Option<f64>,
    pub current_revenue: Option<f64>,
    pub net_profit_margin: Option<f64>,
    pub passes_profitability_filter: bool,
    
    // Debt Analysis (Optional)
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
```

## Frontend Integration

### Enhanced UI Components
```javascript
// Enhanced RecommendationsPanel with GARP filters
const [minGrowthRate, setMinGrowthRate] = useState(15.0); // Default: 15% growth
const [minProfitMargin, setMinProfitMargin] = useState(5.0); // Default: 5% margin
const [maxDebtToEquity, setMaxDebtToEquity] = useState(2.0); // Default: 2.0 max
const [useSectorMedians, setUseSectorMedians] = useState(true); // Default: sector-aware

// Enhanced screening options
const screeningOptions = [
    { value: 'pe', label: 'P/E Ratio (Historical)' },
    { value: 'ps_basic', label: 'P/S Ratio (Basic)' },
    { value: 'ps_enhanced', label: 'P/S Ratio (Enhanced with Growth)' },
    { value: 'garp', label: 'GARP (Growth at Reasonable Price)' } // NEW
];
```

### Enhanced Data Service
```javascript
// Enhanced data service method
export const recommendationsDataService = {
    async loadGarpScreeningResults(stockTickers, limit, minMarketCap, minGrowthRate, minProfitMargin, maxDebtToEquity, useSectorMedians) {
        try {
            const result = await analysisAPI.getGarpScreeningResults(
                stockTickers, 
                limit, 
                minMarketCap, 
                minGrowthRate,
                minProfitMargin,
                maxDebtToEquity,
                useSectorMedians
            );
            
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

## Algorithm Benefits

### Investment Strategy Applications

#### 1. GARP Strategy Implementation
- **Target**: Stocks with low P/S ratios AND growing revenues AND positive profitability
- **Rationale**: Combines value investing (low P/S) with growth investing (revenue growth) and quality investing (profitability)
- **Risk Management**: Multiple quality filters reduce risk of value traps

#### 2. Sector-Aware Screening
- **Sector-Specific Thresholds**: Uses sector medians for P/S ratios when available
- **Precision**: More accurate screening within industry contexts
- **Flexibility**: Falls back to fixed thresholds when sector data unavailable

#### 3. Multi-Dimensional Quality Scoring
- **GARP Score**: Revenue Growth % / P/S Ratio for ranking
- **Quality Score**: Data completeness and reliability assessment
- **Comprehensive Filtering**: Revenue growth, profitability, and debt quality

### Risk Management Features

#### 1. Multi-Layer Quality Filters
- **Revenue Growth**: Ensures business momentum (15% TTM or 10% Annual)
- **Profitability**: Confirms operational efficiency (5% net margin)
- **Debt Quality**: Optional debt-to-equity screening (< 2.0)
- **Data Quality**: Quality scoring based on data completeness

#### 2. Flexible Growth Requirements
- **TTM Growth**: Primary metric for recent momentum
- **Annual Growth**: Secondary metric for yearly trends
- **5-Year Growth**: Long-term growth validation
- **Flexible Thresholds**: Configurable growth rate requirements

#### 3. Sector-Aware Valuation
- **Sector Medians**: Industry-specific P/S thresholds
- **Fallback Logic**: Fixed thresholds when sector data unavailable
- **Precision**: More accurate undervaluation detection

## Performance Characteristics

### Expected Performance Metrics
- **Query Time**: ~3-5 seconds for S&P 500 analysis (complex multi-table joins)
- **Data Requirements**: Comprehensive financial data (income statements, balance sheets)
- **Coverage**: ~70-80% of S&P 500 stocks (due to strict quality requirements)
- **Precision**: Very high precision, moderate recall (fewer but very high quality results)

### Database Optimization
```sql
-- Enhanced indexes for GARP performance
CREATE INDEX idx_income_statements_garp ON income_statements(stock_id, period_type, report_date, revenue, net_income);
CREATE INDEX idx_balance_sheets_garp ON balance_sheets(stock_id, period_type, report_date, total_debt, total_equity);
CREATE INDEX idx_daily_ratios_sector ON daily_valuation_ratios(stock_id, date, ps_ratio_ttm);
CREATE INDEX idx_stocks_sector ON stocks(sector, symbol);
```

## Migration Strategy

### Phase 1: Backend Implementation
1. **Create GARP Command**: Implement `get_garp_screening_results`
2. **Database Optimization**: Add performance indexes for multi-table joins
3. **Testing**: Comprehensive testing with production data

### Phase 2: Frontend Integration
1. **UI Enhancement**: Add GARP filters to RecommendationsPanel
2. **Service Layer**: Update data service with GARP methods
3. **User Experience**: Add tooltips and explanations for GARP concepts

### Phase 3: Algorithm Refinement
1. **Performance Tuning**: Optimize SQL queries for speed
2. **Parameter Tuning**: Fine-tune growth rate and profitability thresholds
3. **Quality Metrics**: Enhance quality scoring algorithm

## Expected Outcomes

### Investment Quality Improvement
- **Reduced False Positives**: Multiple quality filters eliminate low-quality stocks
- **Higher Quality Results**: GARP screening focuses on fundamentally sound companies
- **Better Risk-Adjusted Returns**: Growth and profitability requirements reduce downside risk

### User Experience Enhancement
- **More Sophisticated Analysis**: Users get comprehensive GARP insights
- **Flexible Filtering**: Configurable growth, profitability, and debt requirements
- **Educational Value**: Algorithm teaches users about GARP investing principles

### System Performance
- **Maintained Speed**: Optimized queries keep response times reasonable
- **Scalable Architecture**: Algorithm scales with additional financial data
- **Future-Proof Design**: Architecture supports additional GARP enhancements

---

*This GARP screening architecture represents a significant upgrade from basic P/S screening, implementing a comprehensive Growth at Reasonable Price strategy that combines value, growth, and quality investing principles for superior investment decision-making.*
