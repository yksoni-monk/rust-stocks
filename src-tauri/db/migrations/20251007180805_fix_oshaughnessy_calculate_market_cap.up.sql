DROP VIEW IF EXISTS oshaughnessy_ranking;
DROP VIEW IF EXISTS oshaughnessy_value_composite;

CREATE VIEW oshaughnessy_value_composite AS
SELECT
  s.id as stock_id,
  s.symbol,
  s.sector,
  (SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) as current_price,
  (SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) * b.shares_outstanding as market_cap,
  i.net_income,
  i.revenue,
  i.operating_income,
  b.total_equity,
  b.shares_outstanding,
  b.total_debt,
  b.cash_and_equivalents,
  cf.dividends_paid,
  cf.share_repurchases,
  cf.depreciation_expense,
  cf.amortization_expense,
  (((SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) * b.shares_outstanding) + COALESCE(b.total_debt, 0) - COALESCE(b.cash_and_equivalents, 0)) as enterprise_value,
  (COALESCE(i.operating_income, 0) + COALESCE(cf.depreciation_expense, 0) + COALESCE(cf.amortization_expense, 0)) as ebitda,
  CASE WHEN i.net_income > 0 AND b.shares_outstanding > 0 THEN ((SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) * b.shares_outstanding) / i.net_income ELSE NULL END as pe_ratio,
  CASE WHEN b.total_equity > 0 AND b.shares_outstanding > 0 THEN ((SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) * b.shares_outstanding) / b.total_equity ELSE NULL END as pb_ratio,
  CASE WHEN i.revenue > 0 AND b.shares_outstanding > 0 THEN ((SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) * b.shares_outstanding) / i.revenue ELSE NULL END as ps_ratio,
  CASE WHEN i.revenue > 0 AND b.shares_outstanding > 0 THEN (((SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) * b.shares_outstanding) + COALESCE(b.total_debt, 0) - COALESCE(b.cash_and_equivalents, 0)) / i.revenue ELSE NULL END as evs_ratio,
  CASE WHEN (COALESCE(i.operating_income, 0) + COALESCE(cf.depreciation_expense, 0) + COALESCE(cf.amortization_expense, 0)) > 0 AND b.shares_outstanding > 0 THEN (((SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) * b.shares_outstanding) + COALESCE(b.total_debt, 0) - COALESCE(b.cash_and_equivalents, 0)) / (COALESCE(i.operating_income, 0) + COALESCE(cf.depreciation_expense, 0) + COALESCE(cf.amortization_expense, 0)) ELSE NULL END as ev_ebitda_ratio,
  CASE WHEN b.shares_outstanding > 0 AND ((SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) * b.shares_outstanding) > 0 THEN (COALESCE(cf.dividends_paid, 0) + COALESCE(cf.share_repurchases, 0)) / ((SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) * b.shares_outstanding) ELSE NULL END as shareholder_yield,
  ((CASE WHEN i.net_income > 0 AND b.shares_outstanding > 0 THEN 1 ELSE 0 END) + (CASE WHEN b.total_equity > 0 AND b.shares_outstanding > 0 THEN 1 ELSE 0 END) + (CASE WHEN i.revenue > 0 AND b.shares_outstanding > 0 THEN 1 ELSE 0 END) + (CASE WHEN i.revenue > 0 AND b.shares_outstanding > 0 THEN 1 ELSE 0 END) + (CASE WHEN (COALESCE(i.operating_income, 0) + COALESCE(cf.depreciation_expense, 0) + COALESCE(cf.amortization_expense, 0)) > 0 AND b.shares_outstanding > 0 THEN 1 ELSE 0 END) + (CASE WHEN b.shares_outstanding > 0 AND (SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) > 0 THEN 1 ELSE 0 END)) * 16.67 as data_completeness_score
FROM stocks s
LEFT JOIN (SELECT stock_id, net_income, revenue, operating_income, report_date, ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn FROM income_statements WHERE period_type = 'Annual' AND revenue IS NOT NULL) i ON s.id = i.stock_id AND i.rn = 1
LEFT JOIN (SELECT stock_id, total_equity, shares_outstanding, total_debt, cash_and_equivalents, report_date, ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn FROM balance_sheets WHERE period_type = 'Annual' AND total_equity IS NOT NULL) b ON s.id = b.stock_id AND b.rn = 1
LEFT JOIN (SELECT stock_id, dividends_paid, share_repurchases, depreciation_expense, amortization_expense, report_date, ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn FROM cash_flow_statements WHERE period_type = 'Annual' AND operating_cash_flow IS NOT NULL) cf ON s.id = cf.stock_id AND cf.rn = 1
WHERE s.is_sp500 = 1;

CREATE VIEW oshaughnessy_ranking AS
WITH ranked AS (
  SELECT *, RANK() OVER (ORDER BY pe_ratio ASC) as pe_rank, RANK() OVER (ORDER BY pb_ratio ASC) as pb_rank, RANK() OVER (ORDER BY ps_ratio ASC) as ps_rank, RANK() OVER (ORDER BY evs_ratio ASC) as evs_rank, RANK() OVER (ORDER BY ev_ebitda_ratio ASC) as ebitda_rank, RANK() OVER (ORDER BY shareholder_yield DESC) as yield_rank, COUNT(*) OVER () as total_stocks
  FROM oshaughnessy_value_composite
  WHERE pe_ratio IS NOT NULL AND pb_ratio IS NOT NULL AND ps_ratio IS NOT NULL AND evs_ratio IS NOT NULL AND ev_ebitda_ratio IS NOT NULL AND shareholder_yield IS NOT NULL
)
SELECT *, CAST((pe_rank + pb_rank + ps_rank + evs_rank + ebitda_rank + yield_rank) / 6.0 AS REAL) as composite_score, CAST(ROUND(((pe_rank + pb_rank + ps_rank + evs_rank + ebitda_rank + yield_rank) / 6.0 / total_stocks) * 100, 1) AS REAL) as composite_percentile, RANK() OVER (ORDER BY (pe_rank + pb_rank + ps_rank + evs_rank + ebitda_rank + yield_rank) / 6.0 ASC) as overall_rank, CASE WHEN RANK() OVER (ORDER BY (pe_rank + pb_rank + ps_rank + evs_rank + ebitda_rank + yield_rank) / 6.0 ASC) <= 10 THEN 1 ELSE 0 END as passes_screening, 6 as metrics_available
FROM ranked
ORDER BY composite_score ASC;