-- Migration: Update O'Shaughnessy views to use Annual-only columns and data
-- Purpose: Remove all TTM dependencies after Annual-only conversion

DROP VIEW IF EXISTS oshaughnessy_ranking_complete;
DROP VIEW IF EXISTS oshaughnessy_ranking_simple;
DROP VIEW IF EXISTS oshaughnessy_value_composite_simple;

-- Base value composite using Annual ratios and financials
CREATE VIEW oshaughnessy_value_composite_simple AS
WITH latest_ratios AS (
  SELECT 
    stock_id, date, price, market_cap, enterprise_value,
    ps_ratio_annual as ps_ratio,
    evs_ratio_annual as evs_ratio,
    pb_ratio_annual as pb_ratio,
    ev_ebitda_ratio_annual as ev_ebitda_ratio,
    shareholder_yield_annual as shareholder_yield,
    ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY date DESC) rn
  FROM daily_valuation_ratios
  WHERE ps_ratio_annual IS NOT NULL AND ps_ratio_annual > 0
), latest_income AS (
  SELECT stock_id, net_income, revenue, shares_diluted, report_date,
         ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) rn
  FROM income_statements
  WHERE period_type = 'Annual'
), latest_balance AS (
  SELECT stock_id, total_equity, report_date,
         ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) rn
  FROM balance_sheets
  WHERE period_type = 'Annual'
)
SELECT
  s.id as stock_id,
  s.symbol,
  s.sector,
  s.industry,
  lr.price as current_price,
  lr.market_cap,
  lr.enterprise_value,
  lr.ps_ratio,
  lr.evs_ratio,
  -- Optional: compute P/E if not stored elsewhere
  CASE WHEN li.net_income > 0 AND li.shares_diluted > 0
       THEN lr.price / (li.net_income / li.shares_diluted)
       ELSE NULL END as pe_ratio,
  lr.pb_ratio,
  lr.ev_ebitda_ratio,
  lr.shareholder_yield,
  -- Data completeness (simple heuristic)
  CASE
    WHEN lr.ps_ratio IS NOT NULL AND lr.evs_ratio IS NOT NULL AND lr.pb_ratio IS NOT NULL THEN 100
    WHEN lr.ps_ratio IS NOT NULL AND lr.evs_ratio IS NOT NULL THEN 75
    WHEN lr.ps_ratio IS NOT NULL THEN 50
    ELSE 25
  END as data_completeness_score
FROM stocks s
JOIN latest_ratios lr ON s.id = lr.stock_id AND lr.rn = 1
LEFT JOIN latest_income li ON s.id = li.stock_id AND li.rn = 1
LEFT JOIN latest_balance lb ON s.id = lb.stock_id AND lb.rn = 1
WHERE lr.market_cap > 200000000; -- $200M minimum

-- Ranking view (simple)
CREATE VIEW oshaughnessy_ranking_simple AS
WITH ranked AS (
  SELECT *,
    RANK() OVER (ORDER BY pe_ratio ASC) as pe_rank,
    RANK() OVER (ORDER BY pb_ratio ASC) as pb_rank,
    RANK() OVER (ORDER BY ps_ratio ASC) as ps_rank,
    RANK() OVER (ORDER BY evs_ratio ASC) as evs_rank,
    RANK() OVER (ORDER BY shareholder_yield DESC) as yield_rank,
    COUNT(*) OVER () as total
  FROM oshaughnessy_value_composite_simple
)
SELECT *,
  (COALESCE(pe_rank,0) + COALESCE(pb_rank,0) + COALESCE(ps_rank,0) + COALESCE(evs_rank,0) + COALESCE(yield_rank,0))
    / (CASE WHEN (pe_ratio IS NOT NULL) + (pb_ratio IS NOT NULL) + (ps_ratio IS NOT NULL) + (evs_ratio IS NOT NULL) + (shareholder_yield IS NOT NULL) = 0 
            THEN 1 ELSE ((pe_ratio IS NOT NULL) + (pb_ratio IS NOT NULL) + (ps_ratio IS NOT NULL) + (evs_ratio IS NOT NULL) + (shareholder_yield IS NOT NULL)) END) 
    as composite_score,
  ROUND(((COALESCE(pe_rank,0) + COALESCE(pb_rank,0) + COALESCE(ps_rank,0) + COALESCE(evs_rank,0) + COALESCE(yield_rank,0))
    / (CASE WHEN (pe_ratio IS NOT NULL) + (pb_ratio IS NOT NULL) + (ps_ratio IS NOT NULL) + (evs_ratio IS NOT NULL) + (shareholder_yield IS NOT NULL) = 0 
            THEN 1 ELSE ((pe_ratio IS NOT NULL) + (pb_ratio IS NOT NULL) + (ps_ratio IS NOT NULL) + (evs_ratio IS NOT NULL) + (shareholder_yield IS NOT NULL)) END) / total) * 100, 1) as composite_percentile,
  RANK() OVER (ORDER BY 
    (COALESCE(pe_rank,0) + COALESCE(pb_rank,0) + COALESCE(ps_rank,0) + COALESCE(evs_rank,0) + COALESCE(yield_rank,0))
    / (CASE WHEN (pe_ratio IS NOT NULL) + (pb_ratio IS NOT NULL) + (ps_ratio IS NOT NULL) + (evs_ratio IS NOT NULL) + (shareholder_yield IS NOT NULL) = 0 
            THEN 1 ELSE ((pe_ratio IS NOT NULL) + (pb_ratio IS NOT NULL) + (ps_ratio IS NOT NULL) + (evs_ratio IS NOT NULL) + (shareholder_yield IS NOT NULL)) END)
    ASC) as overall_rank,
  CASE WHEN data_completeness_score >= 75 THEN 1 ELSE 0 END as passes_screening
FROM ranked;

-- Complete ranking view (all 6 O'Shaughnessy metrics)
CREATE VIEW oshaughnessy_ranking_complete AS
WITH ranked AS (
  SELECT *,
    RANK() OVER (ORDER BY pe_ratio ASC) as pe_rank,
    RANK() OVER (ORDER BY pb_ratio ASC) as pb_rank,
    RANK() OVER (ORDER BY ps_ratio ASC) as ps_rank,
    RANK() OVER (ORDER BY evs_ratio ASC) as evs_rank,
    RANK() OVER (ORDER BY ev_ebitda_ratio ASC) as ebitda_rank,
    RANK() OVER (ORDER BY shareholder_yield DESC) as yield_rank,
    COUNT(*) OVER () as total
  FROM oshaughnessy_value_composite_simple
)
SELECT *,
  (COALESCE(pe_rank,0) + COALESCE(pb_rank,0) + COALESCE(ps_rank,0) + COALESCE(evs_rank,0) + COALESCE(ebitda_rank,0) + COALESCE(yield_rank,0))
    / (CASE WHEN (pe_ratio IS NOT NULL) + (pb_ratio IS NOT NULL) + (ps_ratio IS NOT NULL) + (evs_ratio IS NOT NULL) + (ev_ebitda_ratio IS NOT NULL) + (shareholder_yield IS NOT NULL) = 0 
            THEN 1 ELSE ((pe_ratio IS NOT NULL) + (pb_ratio IS NOT NULL) + (ps_ratio IS NOT NULL) + (evs_ratio IS NOT NULL) + (ev_ebitda_ratio IS NOT NULL) + (shareholder_yield IS NOT NULL)) END) 
    as composite_score,
  ROUND(((COALESCE(pe_rank,0) + COALESCE(pb_rank,0) + COALESCE(ps_rank,0) + COALESCE(evs_rank,0) + COALESCE(ebitda_rank,0) + COALESCE(yield_rank,0))
    / (CASE WHEN (pe_ratio IS NOT NULL) + (pb_ratio IS NOT NULL) + (ps_ratio IS NOT NULL) + (evs_ratio IS NOT NULL) + (ev_ebitda_ratio IS NOT NULL) + (shareholder_yield IS NOT NULL) = 0 
            THEN 1 ELSE ((pe_ratio IS NOT NULL) + (pb_ratio IS NOT NULL) + (ps_ratio IS NOT NULL) + (evs_ratio IS NOT NULL) + (ev_ebitda_ratio IS NOT NULL) + (shareholder_yield IS NOT NULL)) END) / total) * 100, 1) as composite_percentile,
  RANK() OVER (ORDER BY 
    (COALESCE(pe_rank,0) + COALESCE(pb_rank,0) + COALESCE(ps_rank,0) + COALESCE(evs_rank,0) + COALESCE(ebitda_rank,0) + COALESCE(yield_rank,0))
    / (CASE WHEN (pe_ratio IS NOT NULL) + (pb_ratio IS NOT NULL) + (ps_ratio IS NOT NULL) + (evs_ratio IS NOT NULL) + (ev_ebitda_ratio IS NOT NULL) + (shareholder_yield IS NOT NULL) = 0 
            THEN 1 ELSE ((pe_ratio IS NOT NULL) + (pb_ratio IS NOT NULL) + (ps_ratio IS NOT NULL) + (evs_ratio IS NOT NULL) + (ev_ebitda_ratio IS NOT NULL) + (shareholder_yield IS NOT NULL)) END)
    ASC) as overall_rank,
  CASE WHEN data_completeness_score >= 75 THEN 1 ELSE 0 END as passes_screening,
  -- Count available metrics for debugging
  ((pe_ratio IS NOT NULL) + (pb_ratio IS NOT NULL) + (ps_ratio IS NOT NULL) + (evs_ratio IS NOT NULL) + (ev_ebitda_ratio IS NOT NULL) + (shareholder_yield IS NOT NULL)) as metrics_available
FROM ranked;


