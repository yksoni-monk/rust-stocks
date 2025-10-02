-- Migration: Create pure O'Shaughnessy SQL views without daily_valuation_ratios dependency
-- Purpose: Implement pure SQL view architecture for O'Shaughnessy Value screening
-- Architecture: Calculate all 6 metrics on-demand from raw EDGAR data + daily_prices

-- Drop existing O'Shaughnessy views that depend on daily_valuation_ratios
DROP VIEW IF EXISTS oshaughnessy_ranking;
DROP VIEW IF EXISTS oshaughnessy_value_composite;

-- Create pure O'Shaughnessy Value Composite view
-- Calculates all 6 metrics on-demand from raw EDGAR data + daily_prices
CREATE VIEW oshaughnessy_value_composite AS
WITH latest_data AS (
  SELECT 
    s.id as stock_id,
    s.symbol,
    s.sector,
    s.industry,
    
    -- Latest price data
    dp.price as current_price,
    dp.market_cap,
    
    -- Latest income statement data (Annual)
    i.net_income,
    i.revenue,
    i.shares_diluted,
    i.operating_income,
    
    -- Latest balance sheet data (Annual)
    b.total_equity,
    b.shares_outstanding,
    b.total_debt,
    b.cash_and_equivalents,
    
    -- Latest cash flow data (Annual)
    cf.dividends_paid,
    cf.share_repurchases,
    cf.depreciation_expense,
    cf.amortization_expense,
    
    -- Calculate enterprise value: Market Cap + Total Debt - Cash
    (dp.market_cap + COALESCE(b.total_debt, 0) - COALESCE(b.cash_and_equivalents, 0)) as enterprise_value,
    
    -- Calculate EBITDA: Operating Income + Depreciation + Amortization
    (COALESCE(i.operating_income, 0) + COALESCE(cf.depreciation_expense, 0) + COALESCE(cf.amortization_expense, 0)) as ebitda,
    
    -- Row numbers for latest data selection
    ROW_NUMBER() OVER (PARTITION BY s.id ORDER BY dp.date DESC) as price_rn,
    ROW_NUMBER() OVER (PARTITION BY s.id ORDER BY i.report_date DESC) as income_rn,
    ROW_NUMBER() OVER (PARTITION BY s.id ORDER BY b.report_date DESC) as balance_rn,
    ROW_NUMBER() OVER (PARTITION BY s.id ORDER BY cf.report_date DESC) as cashflow_rn
    
  FROM stocks s
  LEFT JOIN daily_prices dp ON s.id = dp.stock_id
  LEFT JOIN income_statements i ON s.id = i.stock_id AND i.period_type = 'Annual'
  LEFT JOIN balance_sheets b ON s.id = b.stock_id AND b.period_type = 'Annual'
  LEFT JOIN cash_flow_statements cf ON s.id = cf.stock_id AND cf.period_type = 'Annual'
  WHERE s.is_sp500 = 1
)
SELECT 
  stock_id,
  symbol,
  sector,
  industry,
  current_price,
  market_cap,
  enterprise_value,
  
  -- Calculate all 6 O'Shaughnessy metrics on-demand
  
  -- 1. P/E Ratio: Price / (Net Income / Shares Diluted)
  CASE WHEN net_income > 0 AND shares_diluted > 0 
       THEN current_price / (net_income / shares_diluted) 
       ELSE NULL END as pe_ratio,
  
  -- 2. P/B Ratio: Price / (Total Equity / Shares Outstanding)
  CASE WHEN total_equity > 0 AND shares_outstanding > 0 
       THEN current_price / (total_equity / shares_outstanding) 
       ELSE NULL END as pb_ratio,
  
  -- 3. P/S Ratio: Market Cap / Revenue
  CASE WHEN revenue > 0 
       THEN market_cap / revenue 
       ELSE NULL END as ps_ratio,
  
  -- 4. EV/S Ratio: Enterprise Value / Revenue
  CASE WHEN revenue > 0 
       THEN enterprise_value / revenue 
       ELSE NULL END as evs_ratio,
  
  -- 5. EV/EBITDA Ratio: Enterprise Value / EBITDA
  CASE WHEN ebitda > 0 
       THEN enterprise_value / ebitda 
       ELSE NULL END as ev_ebitda_ratio,
  
  -- 6. Shareholder Yield: (Dividends + Share Repurchases) / Market Cap
  CASE WHEN market_cap > 0 
       THEN (COALESCE(dividends_paid, 0) + COALESCE(share_repurchases, 0)) / market_cap 
       ELSE NULL END as shareholder_yield,
  
  -- Data completeness score (0-100 based on available metrics)
  ((CASE WHEN net_income > 0 AND shares_diluted > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN total_equity > 0 AND shares_outstanding > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN revenue > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN revenue > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN ebitda > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN market_cap > 0 THEN 1 ELSE 0 END)) * 16.67 as data_completeness_score

FROM latest_data
WHERE price_rn = 1 AND income_rn = 1 AND balance_rn = 1 AND cashflow_rn = 1
  AND market_cap > 200000000;  -- $200M minimum market cap

-- Create O'Shaughnessy ranking view with composite scoring
CREATE VIEW oshaughnessy_ranking AS
WITH ranked AS (
  SELECT *,
    -- Rank each metric (lower rank = better value)
    RANK() OVER (ORDER BY pe_ratio ASC) as pe_rank,
    RANK() OVER (ORDER BY pb_ratio ASC) as pb_rank,
    RANK() OVER (ORDER BY ps_ratio ASC) as ps_rank,
    RANK() OVER (ORDER BY evs_ratio ASC) as evs_rank,
    RANK() OVER (ORDER BY ev_ebitda_ratio ASC) as ebitda_rank,
    RANK() OVER (ORDER BY shareholder_yield DESC) as yield_rank,
    COUNT(*) OVER () as total_stocks
  FROM oshaughnessy_value_composite
  WHERE pe_ratio IS NOT NULL AND pb_ratio IS NOT NULL 
    AND ps_ratio IS NOT NULL AND evs_ratio IS NOT NULL
    AND ev_ebitda_ratio IS NOT NULL AND shareholder_yield IS NOT NULL
)
SELECT *,
  -- Composite score (average of all 6 ranks)
  CAST((pe_rank + pb_rank + ps_rank + evs_rank + ebitda_rank + yield_rank) / 6.0 AS REAL) as composite_score,
  
  -- Percentile ranking (0-100)
  CAST(ROUND(((pe_rank + pb_rank + ps_rank + evs_rank + ebitda_rank + yield_rank) / 6.0 / total_stocks) * 100, 1) AS REAL) as composite_percentile,
  
  -- Overall ranking
  RANK() OVER (ORDER BY (pe_rank + pb_rank + ps_rank + evs_rank + ebitda_rank + yield_rank) / 6.0 ASC) as overall_rank,
  
  -- Pass screening if in top 10 stocks
  CASE WHEN 
    RANK() OVER (ORDER BY (pe_rank + pb_rank + ps_rank + evs_rank + ebitda_rank + yield_rank) / 6.0 ASC) <= 10
    THEN 1 ELSE 0 END as passes_screening
FROM ranked
ORDER BY composite_score ASC;
