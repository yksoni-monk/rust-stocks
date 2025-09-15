# Enhanced P/S Screening Algorithm Architecture

## Executive Summary

This document outlines the architecture for a sophisticated P/S (Price-to-Sales) screening algorithm that combines statistical undervaluation detection with revenue growth requirements. The algorithm screens S&P 500 stocks for value opportunities where P/S ratios are 1 standard deviation below the historical median AND annual/TTM revenues are still growing.

## Current Algorithm Analysis

### Existing P/S Screening (Basic)
- **Current Logic**: P/S < (Historical Mean - 0.5 × Variance) AND P/S < Historical Median
- **Data Requirements**: Minimum 20 historical data points
- **Limitations**: 
  - No revenue growth consideration
  - Simple statistical threshold (0.5 × variance)
  - No quality filters for business fundamentals

### Current Data Structure
```sql
-- Current P/S data in daily_valuation_ratios table
CREATE TABLE daily_valuation_ratios (
    stock_id INTEGER,
    date DATE,
    ps_ratio_ttm REAL,        -- Current P/S ratio
    revenue_ttm REAL,         -- TTM revenue
    market_cap REAL,
    -- ... other fields
);

-- Revenue data in income_statements table  
CREATE TABLE income_statements (
    stock_id INTEGER,
    period_type TEXT,          -- 'TTM', 'Annual', 'Quarterly'
    report_date DATE,
    revenue REAL,
    -- ... other fields
);
```

## Enhanced Algorithm Design

### Core Requirements
1. **Statistical Undervaluation**: P/S ratio 1 standard deviation below historical median
2. **Revenue Growth**: Annual/TTM revenue must be growing (positive growth rate)
3. **Data Quality**: Minimum historical data points for reliable analysis
4. **Market Cap Filter**: Configurable minimum market cap (default $500M)
5. **S&P 500 Focus**: Only analyze S&P 500 stocks for quality

### Algorithm Logic

#### Phase 1: Historical P/S Analysis
```sql
-- Calculate historical P/S statistics
WITH historical_ps_stats AS (
    SELECT 
        stock_id,
        AVG(ps_ratio_ttm) as hist_mean,
        -- Use PERCENTILE_CONT for accurate median calculation
        PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY ps_ratio_ttm) as hist_median,
        STDDEV(ps_ratio_ttm) as hist_stddev,
        MIN(ps_ratio_ttm) as hist_min,
        MAX(ps_ratio_ttm) as hist_max,
        COUNT(*) as data_points
    FROM daily_valuation_ratios dvr
    WHERE ps_ratio_ttm IS NOT NULL 
      AND ps_ratio_ttm > 0.01
      AND date < CURRENT_DATE - INTERVAL '30 days'  -- Exclude recent data
    GROUP BY stock_id
    HAVING COUNT(*) >= 50  -- Require more data points for reliability
)
```

#### Phase 2: Revenue Growth Analysis
```sql
-- Calculate revenue growth rates
WITH revenue_growth AS (
    SELECT 
        stock_id,
        -- TTM revenue growth (current vs previous TTM)
        current_ttm.revenue as current_ttm_revenue,
        previous_ttm.revenue as previous_ttm_revenue,
        CASE 
            WHEN previous_ttm.revenue > 0 THEN 
                (current_ttm.revenue - previous_ttm.revenue) / previous_ttm.revenue * 100
            ELSE NULL 
        END as ttm_growth_rate,
        
        -- Annual revenue growth (latest annual vs previous annual)
        current_annual.revenue as current_annual_revenue,
        previous_annual.revenue as previous_annual_revenue,
        CASE 
            WHEN previous_annual.revenue > 0 THEN 
                (current_annual.revenue - previous_annual.revenue) / previous_annual.revenue * 100
            ELSE NULL 
        END as annual_growth_rate
        
    FROM stocks s
    -- Current TTM revenue
    LEFT JOIN income_statements current_ttm ON s.id = current_ttm.stock_id 
        AND current_ttm.period_type = 'TTM'
        AND current_ttm.report_date = (
            SELECT MAX(report_date) FROM income_statements 
            WHERE stock_id = s.id AND period_type = 'TTM'
        )
    -- Previous TTM revenue (12 months earlier)
    LEFT JOIN income_statements previous_ttm ON s.id = previous_ttm.stock_id 
        AND previous_ttm.period_type = 'TTM'
        AND previous_ttm.report_date = (
            SELECT MAX(report_date) FROM income_statements 
            WHERE stock_id = s.id AND period_type = 'TTM'
            AND report_date < current_ttm.report_date
        )
    -- Current annual revenue
    LEFT JOIN income_statements current_annual ON s.id = current_annual.stock_id 
        AND current_annual.period_type = 'Annual'
        AND current_annual.report_date = (
            SELECT MAX(report_date) FROM income_statements 
            WHERE stock_id = s.id AND period_type = 'Annual'
        )
    -- Previous annual revenue
    LEFT JOIN income_statements previous_annual ON s.id = previous_annual.stock_id 
        AND previous_annual.period_type = 'Annual'
        AND previous_annual.report_date = (
            SELECT MAX(report_date) FROM income_statements 
            WHERE stock_id = s.id AND period_type = 'Annual'
            AND report_date < current_annual.report_date
        )
)
```

#### Phase 3: Enhanced Screening Criteria
```sql
-- Enhanced screening logic
WITH enhanced_screening AS (
    SELECT 
        c.stock_id,
        c.symbol,
        c.ps_ratio_ttm as current_ps,
        h.hist_mean,
        h.hist_median,
        h.hist_stddev,
        h.hist_min,
        h.hist_max,
        h.data_points,
        
        -- Revenue growth metrics
        rg.current_ttm_revenue,
        rg.ttm_growth_rate,
        rg.current_annual_revenue,
        rg.annual_growth_rate,
        
        -- Enhanced Z-score calculation
        CASE 
            WHEN h.hist_stddev > 0 THEN 
                (c.ps_ratio_ttm - h.hist_median) / h.hist_stddev
            ELSE 0.0
        END as z_score,
        
        -- Enhanced undervaluation criteria
        CASE 
            WHEN h.hist_stddev > 0 AND h.data_points >= 50 THEN
                -- Statistical undervaluation: P/S < (Median - 1 × StdDev)
                c.ps_ratio_ttm < (h.hist_median - 1.0 * h.hist_stddev) AND
                -- Revenue growth requirement: Either TTM or Annual growth > 0%
                (rg.ttm_growth_rate > 0 OR rg.annual_growth_rate > 0)
            ELSE false
        END as is_undervalued,
        
        -- Quality score (0-100)
        CASE 
            WHEN h.data_points >= 100 AND rg.ttm_growth_rate IS NOT NULL AND rg.annual_growth_rate IS NOT NULL THEN 100
            WHEN h.data_points >= 50 AND (rg.ttm_growth_rate IS NOT NULL OR rg.annual_growth_rate IS NOT NULL) THEN 75
            WHEN h.data_points >= 50 THEN 50
            ELSE 25
        END as quality_score
        
    FROM current_data c
    JOIN historical_ps_stats h ON c.stock_id = h.stock_id
    LEFT JOIN revenue_growth rg ON c.stock_id = rg.stock_id
    WHERE c.market_cap > ?  -- Market cap filter
)
```

## Implementation Architecture

### Backend Implementation

#### New Command: `get_enhanced_undervalued_stocks_by_ps`
```rust
#[tauri::command]
pub async fn get_enhanced_undervalued_stocks_by_ps(
    stock_tickers: Vec<String>, 
    limit: Option<i32>, 
    min_market_cap: Option<f64>,
    min_growth_rate: Option<f64>  // NEW: Minimum growth rate filter
) -> Result<Vec<EnhancedUndervaluedStock>, String>
```

#### Enhanced Data Structure
```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EnhancedUndervaluedStock {
    pub stock_id: i32,
    pub symbol: String,
    pub current_ps: f64,
    
    // Historical P/S statistics
    pub historical_mean: f64,
    pub historical_median: f64,
    pub historical_stddev: f64,
    pub historical_min: f64,
    pub historical_max: f64,
    pub data_points: i32,
    
    // Revenue growth metrics
    pub current_ttm_revenue: Option<f64>,
    pub ttm_growth_rate: Option<f64>,
    pub current_annual_revenue: Option<f64>,
    pub annual_growth_rate: Option<f64>,
    
    // Enhanced metrics
    pub z_score: f64,
    pub quality_score: i32,
    pub is_undervalued: bool,
    
    // Market metrics
    pub market_cap: f64,
    pub price: f64,
    pub data_completeness_score: i32,
}
```

### Frontend Integration

#### Enhanced UI Components
```javascript
// Enhanced RecommendationsPanel with growth filters
const [minGrowthRate, setMinGrowthRate] = useState(0.0); // Default: any growth
const [growthPeriod, setGrowthPeriod] = useState('ttm'); // 'ttm' or 'annual'

// Enhanced screening options
const screeningOptions = [
    { value: 'ps_basic', label: 'P/S Ratio (Basic)' },
    { value: 'ps_enhanced', label: 'P/S Ratio (Enhanced with Growth)' }, // NEW
    { value: 'pe', label: 'P/E Ratio (Historical)' }
];
```

#### Enhanced Data Service
```javascript
// Enhanced data service method
export const recommendationsDataService = {
    async loadEnhancedUndervaluedStocksByPs(stockTickers, limit, minMarketCap, minGrowthRate) {
        try {
            const result = await analysisAPI.getEnhancedUndervaluedStocksByPs(
                stockTickers, 
                limit, 
                minMarketCap, 
                minGrowthRate
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

#### 1. Value + Growth Hybrid Strategy
- **Target**: Stocks with low P/S ratios AND growing revenues
- **Rationale**: Combines value investing (low P/S) with growth investing (revenue growth)
- **Risk Management**: Revenue growth reduces risk of value traps

#### 2. Quality Value Investing
- **Target**: High-quality companies temporarily undervalued
- **Rationale**: Growing revenues indicate business strength despite low valuation
- **Screening**: Statistical undervaluation + fundamental quality

#### 3. Contrarian Growth Strategy
- **Target**: Growing companies with temporarily depressed valuations
- **Rationale**: Market may be undervaluing growing businesses
- **Opportunity**: Potential for multiple expansion as growth continues

### Risk Management Features

#### 1. Data Quality Scoring
- **Quality Score**: 0-100 based on data completeness
- **Requirements**: Minimum historical data points
- **Reliability**: Higher scores indicate more reliable analysis

#### 2. Growth Validation
- **Dual Growth Metrics**: Both TTM and Annual growth rates
- **Flexibility**: Accepts either TTM or Annual growth > 0%
- **Robustness**: Reduces false positives from single-period anomalies

#### 3. Statistical Rigor
- **Enhanced Z-Score**: Based on median instead of mean (more robust)
- **Standard Deviation**: Uses 1× std dev threshold (more conservative)
- **Data Requirements**: Minimum 50 data points (vs 20 in basic algorithm)

## Performance Characteristics

### Expected Performance Metrics
- **Query Time**: ~2-3 seconds for S&P 500 analysis (vs ~1 second for basic)
- **Data Requirements**: Minimum 50 historical data points per stock
- **Coverage**: ~80-90% of S&P 500 stocks (vs ~95% for basic)
- **Precision**: Higher precision, lower recall (fewer but higher quality results)

### Database Optimization
```sql
-- Enhanced indexes for performance
CREATE INDEX idx_daily_ratios_ps_enhanced ON daily_valuation_ratios(stock_id, date, ps_ratio_ttm);
CREATE INDEX idx_income_statements_growth ON income_statements(stock_id, period_type, report_date, revenue);
CREATE INDEX idx_income_statements_ttm_lookup ON income_statements(stock_id, period_type, report_date) 
    WHERE period_type = 'TTM';
CREATE INDEX idx_income_statements_annual_lookup ON income_statements(stock_id, period_type, report_date) 
    WHERE period_type = 'Annual';
```

## Migration Strategy

### Phase 1: Backend Implementation
1. **Create Enhanced Command**: Implement `get_enhanced_undervalued_stocks_by_ps`
2. **Database Optimization**: Add performance indexes
3. **Testing**: Comprehensive testing with production data

### Phase 2: Frontend Integration
1. **UI Enhancement**: Add growth rate filters to RecommendationsPanel
2. **Service Layer**: Update data service with enhanced methods
3. **User Experience**: Add tooltips and explanations for new features

### Phase 3: Algorithm Refinement
1. **Performance Tuning**: Optimize SQL queries for speed
2. **Parameter Tuning**: Fine-tune growth rate thresholds
3. **Quality Metrics**: Enhance quality scoring algorithm

## Expected Outcomes

### Investment Quality Improvement
- **Reduced False Positives**: Revenue growth requirement eliminates value traps
- **Higher Quality Results**: Enhanced screening criteria focus on quality companies
- **Better Risk-Adjusted Returns**: Growth requirement reduces downside risk

### User Experience Enhancement
- **More Sophisticated Analysis**: Users get deeper insights into stock quality
- **Flexible Filtering**: Configurable growth rate requirements
- **Educational Value**: Algorithm teaches users about value + growth investing

### System Performance
- **Maintained Speed**: Optimized queries keep response times reasonable
- **Scalable Architecture**: Algorithm scales with additional data
- **Future-Proof Design**: Architecture supports additional enhancements

---

*This enhanced P/S screening algorithm represents a significant improvement over the basic statistical approach, combining quantitative screening with fundamental quality requirements for superior investment decision-making.*
