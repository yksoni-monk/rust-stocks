-- Migration: Add GARP P/E Screening Support
-- Purpose: Add P/E-based GARP screening views and indexes
-- Safety: ADDITIVE ONLY - no data destruction, no existing table modifications
-- Date: 2025-01-16

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
    END as debt_to_equity_ratio,
    
    -- Data completeness score (0-100)
    CASE 
        WHEN ttm_income.net_income IS NOT NULL AND ttm_income.shares_diluted IS NOT NULL 
             AND prev_ttm_income.net_income IS NOT NULL AND prev_ttm_income.shares_diluted IS NOT NULL
             AND ttm_income.revenue IS NOT NULL AND prev_ttm_income.revenue IS NOT NULL
             AND ttm_balance.total_debt IS NOT NULL AND ttm_balance.total_equity IS NOT NULL THEN 100
        WHEN ttm_income.net_income IS NOT NULL AND ttm_income.shares_diluted IS NOT NULL 
             AND prev_ttm_income.net_income IS NOT NULL AND prev_ttm_income.shares_diluted IS NOT NULL
             AND ttm_income.revenue IS NOT NULL AND prev_ttm_income.revenue IS NOT NULL THEN 75
        WHEN ttm_income.net_income IS NOT NULL AND ttm_income.shares_diluted IS NOT NULL 
             AND (ttm_income.revenue IS NOT NULL OR prev_ttm_income.net_income IS NOT NULL) THEN 50
        ELSE 25
    END as data_completeness_score
    
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
