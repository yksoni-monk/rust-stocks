-- Migration: Fix O'Shaughnessy views to use correct Annual data sources
-- Problem: View used shares_diluted from income_statements (only 12 records)
-- Solution: Use shares_outstanding from balance_sheets (64,355 records)
-- Impact: O'Shaughnessy screening will work with 85%+ coverage

-- Drop existing views
DROP VIEW IF EXISTS oshaughnessy_ranking;
DROP VIEW IF EXISTS oshaughnessy_value_composite;

-- =============================================================================
-- O'Shaughnessy Value Composite View - Annual Data Only
-- =============================================================================

CREATE VIEW oshaughnessy_value_composite AS
SELECT
  s.id as stock_id,
  s.symbol,
  s.sector,

  -- Latest price data
  (SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) as current_price,
  (SELECT market_cap FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) as market_cap,

  -- Latest Annual income statement data
  i.net_income,
  i.revenue,
  i.operating_income,

  -- Latest Annual balance sheet data
  b.total_equity,
  b.shares_outstanding,  -- FIXED: Use balance_sheets, not income_statements
  b.total_debt,
  b.cash_and_equivalents,

  -- Latest Annual cash flow data
  cf.dividends_paid,
  cf.share_repurchases,
  cf.depreciation_expense,
  cf.amortization_expense,

  -- Enterprise Value: Market Cap + Total Debt - Cash
  ((SELECT market_cap FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) +
   COALESCE(b.total_debt, 0) - COALESCE(b.cash_and_equivalents, 0)) as enterprise_value,

  -- EBITDA: Operating Income + Depreciation + Amortization
  (COALESCE(i.operating_income, 0) + COALESCE(cf.depreciation_expense, 0) +
   COALESCE(cf.amortization_expense, 0)) as ebitda,

  -- 1. P/E Ratio: Market Cap / Net Income
  CASE WHEN i.net_income > 0
       THEN (SELECT market_cap FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) / i.net_income
       ELSE NULL END as pe_ratio,

  -- 2. P/B Ratio: Market Cap / Total Equity
  CASE WHEN b.total_equity > 0
       THEN (SELECT market_cap FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) / b.total_equity
       ELSE NULL END as pb_ratio,

  -- 3. P/S Ratio: Market Cap / Revenue
  CASE WHEN i.revenue > 0
       THEN (SELECT market_cap FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) / i.revenue
       ELSE NULL END as ps_ratio,

  -- 4. EV/S Ratio: Enterprise Value / Revenue
  CASE WHEN i.revenue > 0
       THEN ((SELECT market_cap FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) +
             COALESCE(b.total_debt, 0) - COALESCE(b.cash_and_equivalents, 0)) / i.revenue
       ELSE NULL END as evs_ratio,

  -- 5. EV/EBITDA Ratio: Enterprise Value / EBITDA
  CASE WHEN (COALESCE(i.operating_income, 0) + COALESCE(cf.depreciation_expense, 0) +
             COALESCE(cf.amortization_expense, 0)) > 0
       THEN ((SELECT market_cap FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) +
             COALESCE(b.total_debt, 0) - COALESCE(b.cash_and_equivalents, 0)) /
            (COALESCE(i.operating_income, 0) + COALESCE(cf.depreciation_expense, 0) +
             COALESCE(cf.amortization_expense, 0))
       ELSE NULL END as ev_ebitda_ratio,

  -- 6. Shareholder Yield: (Dividends + Buybacks) / Market Cap
  CASE WHEN (SELECT market_cap FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) > 0
       THEN (COALESCE(cf.dividends_paid, 0) + COALESCE(cf.share_repurchases, 0)) /
            (SELECT market_cap FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1)
       ELSE NULL END as shareholder_yield,

  -- Data completeness score (0-100)
  ((CASE WHEN i.net_income > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN b.total_equity > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN i.revenue > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN i.revenue > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN (COALESCE(i.operating_income, 0) + COALESCE(cf.depreciation_expense, 0) +
               COALESCE(cf.amortization_expense, 0)) > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN (SELECT market_cap FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) > 0 THEN 1 ELSE 0 END)
  ) * 16.67 as data_completeness_score

FROM stocks s
LEFT JOIN (
  SELECT stock_id, net_income, revenue, operating_income, report_date,
         ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
  FROM income_statements
  WHERE period_type = 'Annual' AND revenue IS NOT NULL
) i ON s.id = i.stock_id AND i.rn = 1
LEFT JOIN (
  SELECT stock_id, total_equity, shares_outstanding, total_debt, cash_and_equivalents, report_date,
         ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
  FROM balance_sheets
  WHERE period_type = 'Annual' AND total_equity IS NOT NULL
) b ON s.id = b.stock_id AND b.rn = 1
LEFT JOIN (
  SELECT stock_id, dividends_paid, share_repurchases, depreciation_expense, amortization_expense, report_date,
         ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
  FROM cash_flow_statements
  WHERE period_type = 'Annual' AND operating_cash_flow IS NOT NULL
) cf ON s.id = cf.stock_id AND cf.rn = 1
WHERE s.is_sp500 = 1;

-- =============================================================================
-- O'Shaughnessy Ranking View - Composite Scoring
-- =============================================================================

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
    THEN 1 ELSE 0 END as passes_screening,

  -- Metrics available count (6 metrics)
  6 as metrics_available
FROM ranked
ORDER BY composite_score ASC;
