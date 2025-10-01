-- Migration: Simplify O'Shaughnessy to single view with all 6 metrics
-- Purpose: Remove complex simple/complete views, create one comprehensive view

DROP VIEW IF EXISTS oshaughnessy_ranking_complete;
DROP VIEW IF EXISTS oshaughnessy_ranking_simple;
DROP VIEW IF EXISTS oshaughnessy_value_composite_simple;

-- Single O'Shaughnessy Value Composite view with all 6 metrics
CREATE VIEW oshaughnessy_value_composite AS
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
  -- P/E ratio calculation
  CASE WHEN li.net_income > 0 AND li.shares_diluted > 0
       THEN lr.price / (li.net_income / li.shares_diluted)
       ELSE NULL END as pe_ratio,
  lr.pb_ratio,
  lr.ev_ebitda_ratio,
  lr.shareholder_yield,
  -- Count available metrics
  ((CASE WHEN li.net_income > 0 AND li.shares_diluted > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN lr.pb_ratio IS NOT NULL THEN 1 ELSE 0 END) +
   (CASE WHEN lr.ps_ratio IS NOT NULL THEN 1 ELSE 0 END) +
   (CASE WHEN lr.evs_ratio IS NOT NULL THEN 1 ELSE 0 END) +
   (CASE WHEN lr.ev_ebitda_ratio IS NOT NULL THEN 1 ELSE 0 END) +
   (CASE WHEN lr.shareholder_yield IS NOT NULL THEN 1 ELSE 0 END)) as metrics_available,
  -- Data completeness score (0-100)
  CASE
    WHEN ((CASE WHEN li.net_income > 0 AND li.shares_diluted > 0 THEN 1 ELSE 0 END) +
          (CASE WHEN lr.pb_ratio IS NOT NULL THEN 1 ELSE 0 END) +
          (CASE WHEN lr.ps_ratio IS NOT NULL THEN 1 ELSE 0 END) +
          (CASE WHEN lr.evs_ratio IS NOT NULL THEN 1 ELSE 0 END) +
          (CASE WHEN lr.ev_ebitda_ratio IS NOT NULL THEN 1 ELSE 0 END) +
          (CASE WHEN lr.shareholder_yield IS NOT NULL THEN 1 ELSE 0 END)) >= 5 THEN 100
    WHEN ((CASE WHEN li.net_income > 0 AND li.shares_diluted > 0 THEN 1 ELSE 0 END) +
          (CASE WHEN lr.pb_ratio IS NOT NULL THEN 1 ELSE 0 END) +
          (CASE WHEN lr.ps_ratio IS NOT NULL THEN 1 ELSE 0 END) +
          (CASE WHEN lr.evs_ratio IS NOT NULL THEN 1 ELSE 0 END) +
          (CASE WHEN lr.ev_ebitda_ratio IS NOT NULL THEN 1 ELSE 0 END) +
          (CASE WHEN lr.shareholder_yield IS NOT NULL THEN 1 ELSE 0 END)) >= 4 THEN 80
    WHEN ((CASE WHEN li.net_income > 0 AND li.shares_diluted > 0 THEN 1 ELSE 0 END) +
          (CASE WHEN lr.pb_ratio IS NOT NULL THEN 1 ELSE 0 END) +
          (CASE WHEN lr.ps_ratio IS NOT NULL THEN 1 ELSE 0 END) +
          (CASE WHEN lr.evs_ratio IS NOT NULL THEN 1 ELSE 0 END) +
          (CASE WHEN lr.ev_ebitda_ratio IS NOT NULL THEN 1 ELSE 0 END) +
          (CASE WHEN lr.shareholder_yield IS NOT NULL THEN 1 ELSE 0 END)) >= 3 THEN 60
    WHEN ((CASE WHEN li.net_income > 0 AND li.shares_diluted > 0 THEN 1 ELSE 0 END) +
          (CASE WHEN lr.pb_ratio IS NOT NULL THEN 1 ELSE 0 END) +
          (CASE WHEN lr.ps_ratio IS NOT NULL THEN 1 ELSE 0 END) +
          (CASE WHEN lr.evs_ratio IS NOT NULL THEN 1 ELSE 0 END) +
          (CASE WHEN lr.ev_ebitda_ratio IS NOT NULL THEN 1 ELSE 0 END) +
          (CASE WHEN lr.shareholder_yield IS NOT NULL THEN 1 ELSE 0 END)) >= 2 THEN 40
    ELSE 20
  END as data_completeness_score
FROM stocks s
JOIN latest_ratios lr ON s.id = lr.stock_id AND lr.rn = 1
LEFT JOIN latest_income li ON s.id = li.stock_id AND li.rn = 1
WHERE lr.market_cap > 200000000  -- $200M minimum
  AND s.is_sp500 = 1;  -- S&P 500 only

-- Single O'Shaughnessy ranking view
CREATE VIEW oshaughnessy_ranking AS
WITH ranked AS (
  SELECT *,
    RANK() OVER (ORDER BY pe_ratio ASC) as pe_rank,
    RANK() OVER (ORDER BY pb_ratio ASC) as pb_rank,
    RANK() OVER (ORDER BY ps_ratio ASC) as ps_rank,
    RANK() OVER (ORDER BY evs_ratio ASC) as evs_rank,
    RANK() OVER (ORDER BY ev_ebitda_ratio ASC) as ebitda_rank,
    RANK() OVER (ORDER BY shareholder_yield DESC) as yield_rank,
    COUNT(*) OVER () as total_stocks
  FROM oshaughnessy_value_composite
)
SELECT *,
  -- Composite score (average of available ranks)
  (COALESCE(pe_rank,0) + COALESCE(pb_rank,0) + COALESCE(ps_rank,0) + COALESCE(evs_rank,0) + COALESCE(ebitda_rank,0) + COALESCE(yield_rank,0))
    / (CASE WHEN metrics_available = 0 THEN 1 ELSE metrics_available END) as composite_score,
  -- Percentile ranking
  ROUND(((COALESCE(pe_rank,0) + COALESCE(pb_rank,0) + COALESCE(ps_rank,0) + COALESCE(evs_rank,0) + COALESCE(ebitda_rank,0) + COALESCE(yield_rank,0))
    / (CASE WHEN metrics_available = 0 THEN 1 ELSE metrics_available END) / total_stocks) * 100, 1) as composite_percentile,
  -- Overall ranking
  RANK() OVER (ORDER BY 
    (COALESCE(pe_rank,0) + COALESCE(pb_rank,0) + COALESCE(ps_rank,0) + COALESCE(evs_rank,0) + COALESCE(ebitda_rank,0) + COALESCE(yield_rank,0))
    / (CASE WHEN metrics_available = 0 THEN 1 ELSE metrics_available END) ASC) as overall_rank,
  -- Pass screening if in top 20% and has at least 3 metrics
  CASE WHEN 
    ((COALESCE(pe_rank,0) + COALESCE(pb_rank,0) + COALESCE(ps_rank,0) + COALESCE(evs_rank,0) + COALESCE(ebitda_rank,0) + COALESCE(yield_rank,0))
     / (CASE WHEN metrics_available = 0 THEN 1 ELSE metrics_available END) / total_stocks) <= 0.20
    AND metrics_available >= 3 THEN 1 ELSE 0 END as passes_screening
FROM ranked
ORDER BY composite_score ASC;
