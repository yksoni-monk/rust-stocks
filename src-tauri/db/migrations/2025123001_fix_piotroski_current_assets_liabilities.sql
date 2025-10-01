-- Migration: Fix Piotroski F-Score current assets/liabilities data
-- Created: 2025-12-30
-- Purpose: Fix missing current_assets and current_liabilities in Piotroski view
-- Problem: View selects most recent balance sheet which may not have current assets/liabilities
-- Solution: Select most recent balance sheet that has current assets/liabilities data

-- Drop existing views
DROP VIEW IF EXISTS piotroski_f_score_complete;
DROP VIEW IF EXISTS piotroski_screening_results;

-- Create the multi-year data view using Annual data with proper current assets/liabilities selection
CREATE VIEW piotroski_multi_year_data AS
SELECT
    s.id as stock_id,
    s.symbol,
    s.sector,
    s.industry,

    -- Current Annual data (most recent fiscal year)
    current_income.net_income as current_net_income,
    current_income.revenue as current_revenue,
    current_income.gross_profit as current_gross_profit,
    current_income.cost_of_revenue as current_cost_of_revenue,
    current_income.operating_income as current_operating_income,
    current_income.shares_diluted as current_shares,
    current_income.shares_diluted as current_shares_outstanding,

    -- Current Annual balance data (most recent with current assets/liabilities)
    current_balance.total_assets as current_assets,
    current_balance.total_debt as current_debt,
    current_balance.total_equity as current_equity,
    current_balance.current_assets as current_current_assets,
    current_balance.current_liabilities as current_current_liabilities,
    current_balance.shares_outstanding as current_shares_outstanding_bs,

    -- Prior year Annual data (for year-over-year comparisons)
    prior_income.net_income as prior_net_income,
    prior_income.revenue as prior_revenue,
    prior_income.gross_profit as prior_gross_profit,
    prior_income.cost_of_revenue as prior_cost_of_revenue,
    prior_income.operating_income as prior_operating_income,
    prior_income.shares_diluted as prior_shares,
    prior_income.shares_diluted as prior_shares_outstanding,

    -- Prior year balance data (most recent with current assets/liabilities)
    prior_balance.total_assets as prior_assets,
    prior_balance.total_debt as prior_debt,
    prior_balance.total_equity as prior_equity,
    prior_balance.current_assets as prior_current_assets,
    prior_balance.current_liabilities as prior_current_liabilities,
    prior_balance.shares_outstanding as prior_shares_outstanding_bs,

    -- Cash flow data (Annual)
    current_cashflow.operating_cash_flow as current_operating_cash_flow,
    prior_cashflow.operating_cash_flow as prior_operating_cash_flow

FROM stocks s

-- Current Annual income data
LEFT JOIN (
    SELECT stock_id, net_income, revenue, gross_profit, cost_of_revenue,
           operating_income, shares_diluted, report_date,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
    FROM income_statements
    WHERE period_type = 'Annual'
) current_income ON s.id = current_income.stock_id AND current_income.rn = 1

-- Prior Annual income data (previous fiscal year)
LEFT JOIN (
    SELECT stock_id, net_income, revenue, gross_profit, cost_of_revenue,
           operating_income, shares_diluted, report_date,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
    FROM income_statements
    WHERE period_type = 'Annual'
) prior_income ON s.id = prior_income.stock_id AND prior_income.rn = 2

-- Current Annual balance data (most recent with current assets/liabilities)
LEFT JOIN (
    SELECT stock_id, total_assets, total_debt, total_equity,
           current_assets, current_liabilities, shares_outstanding, report_date,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY 
               CASE WHEN current_assets IS NOT NULL AND current_liabilities IS NOT NULL THEN 0 ELSE 1 END,
               report_date DESC) as rn
    FROM balance_sheets
    WHERE period_type = 'Annual'
) current_balance ON s.id = current_balance.stock_id AND current_balance.rn = 1

-- Prior Annual balance data (second most recent with current assets/liabilities)
LEFT JOIN (
    SELECT stock_id, total_assets, total_debt, total_equity,
           current_assets, current_liabilities, shares_outstanding, report_date,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY 
               CASE WHEN current_assets IS NOT NULL AND current_liabilities IS NOT NULL THEN 0 ELSE 1 END,
               report_date DESC) as rn
    FROM balance_sheets
    WHERE period_type = 'Annual'
) prior_balance ON s.id = prior_balance.stock_id AND prior_balance.rn = 2

-- Current Annual cash flow data
LEFT JOIN (
    SELECT stock_id, operating_cash_flow, report_date,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
    FROM cash_flow_statements
    WHERE period_type = 'Annual'
) current_cashflow ON s.id = current_cashflow.stock_id AND current_cashflow.rn = 1

-- Prior Annual cash flow data
LEFT JOIN (
    SELECT stock_id, operating_cash_flow, report_date,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
    FROM cash_flow_statements
    WHERE period_type = 'Annual'
) prior_cashflow ON s.id = prior_cashflow.stock_id AND prior_cashflow.rn = 2;

-- Create the complete Piotroski F-Score view using NET MARGIN
CREATE VIEW piotroski_f_score_complete AS
SELECT
    fd.*,
    NULL as pb_ratio,

    -- PROFITABILITY (4 criteria)
    CASE WHEN current_net_income > 0 THEN 1 ELSE 0 END as criterion_positive_net_income,
    CASE WHEN current_operating_cash_flow > 0 THEN 1 ELSE 0 END as criterion_positive_operating_cash_flow,
    CASE
        WHEN current_net_income / NULLIF(current_assets, 0) >
             prior_net_income / NULLIF(prior_assets, 0)
             AND current_net_income IS NOT NULL AND prior_net_income IS NOT NULL
             AND current_assets IS NOT NULL AND prior_assets IS NOT NULL THEN 1
        ELSE 0
    END as criterion_improving_roa,
    CASE
        WHEN current_operating_cash_flow > current_net_income
             AND current_operating_cash_flow IS NOT NULL
             AND current_net_income IS NOT NULL THEN 1
        ELSE 0
    END as criterion_cash_flow_quality,

    -- LEVERAGE/LIQUIDITY (3 criteria)
    CASE
        WHEN current_debt / NULLIF(current_assets, 0) <
             prior_debt / NULLIF(prior_assets, 0)
             AND current_debt IS NOT NULL AND prior_debt IS NOT NULL
             AND current_assets IS NOT NULL AND prior_assets IS NOT NULL THEN 1
        ELSE 0
    END as criterion_decreasing_debt_ratio,
    CASE
        WHEN current_current_assets / NULLIF(current_current_liabilities, 0) >
             prior_current_assets / NULLIF(prior_current_liabilities, 0)
             AND current_current_assets IS NOT NULL AND prior_current_assets IS NOT NULL
             AND current_current_liabilities IS NOT NULL AND prior_current_liabilities IS NOT NULL THEN 1
        ELSE 0
    END as criterion_improving_current_ratio,
    CASE
        WHEN COALESCE(current_shares_outstanding_bs, current_shares_outstanding) <=
             COALESCE(prior_shares_outstanding_bs, prior_shares_outstanding)
             AND COALESCE(current_shares_outstanding_bs, current_shares_outstanding) IS NOT NULL
             AND COALESCE(prior_shares_outstanding_bs, prior_shares_outstanding) IS NOT NULL THEN 1
        ELSE 0
    END as criterion_no_dilution,

    -- OPERATING EFFICIENCY (2 criteria) - USING NET MARGIN
    CASE
        WHEN current_net_income / NULLIF(current_revenue, 0) >
             prior_net_income / NULLIF(prior_revenue, 0)
             AND current_net_income IS NOT NULL AND prior_net_income IS NOT NULL
             AND current_revenue IS NOT NULL AND prior_revenue IS NOT NULL THEN 1
        ELSE 0
    END as criterion_improving_net_margin,
    CASE
        WHEN current_revenue / NULLIF(current_assets, 0) >
             prior_revenue / NULLIF(prior_assets, 0)
             AND current_revenue IS NOT NULL AND prior_revenue IS NOT NULL
             AND current_assets IS NOT NULL AND prior_assets IS NOT NULL THEN 1
        ELSE 0
    END as criterion_improving_asset_turnover,

    -- TOTAL F-SCORE CALCULATION (0-9) - USING NET MARGIN
    (CASE WHEN current_net_income > 0 THEN 1 ELSE 0 END +
     CASE WHEN current_operating_cash_flow > 0 THEN 1 ELSE 0 END +
     CASE
         WHEN current_net_income / NULLIF(current_assets, 0) >
              prior_net_income / NULLIF(prior_assets, 0)
              AND current_net_income IS NOT NULL AND prior_net_income IS NOT NULL
              AND current_assets IS NOT NULL AND prior_assets IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN current_operating_cash_flow > current_net_income
              AND current_operating_cash_flow IS NOT NULL
              AND current_net_income IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN current_debt / NULLIF(current_assets, 0) <
              prior_debt / NULLIF(prior_assets, 0)
              AND current_debt IS NOT NULL AND prior_debt IS NOT NULL
              AND current_assets IS NOT NULL AND prior_assets IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN current_current_assets / NULLIF(current_current_liabilities, 0) >
              prior_current_assets / NULLIF(prior_current_liabilities, 0)
              AND current_current_assets IS NOT NULL AND prior_current_assets IS NOT NULL
              AND current_current_liabilities IS NOT NULL AND prior_current_liabilities IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN COALESCE(current_shares_outstanding_bs, current_shares_outstanding) <=
              COALESCE(prior_shares_outstanding_bs, prior_shares_outstanding)
              AND COALESCE(current_shares_outstanding_bs, current_shares_outstanding) IS NOT NULL
              AND COALESCE(prior_shares_outstanding_bs, prior_shares_outstanding) IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN current_net_income / NULLIF(current_revenue, 0) >
              prior_net_income / NULLIF(prior_revenue, 0)
              AND current_net_income IS NOT NULL AND prior_net_income IS NOT NULL
              AND current_revenue IS NOT NULL AND prior_revenue IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN current_revenue / NULLIF(current_assets, 0) >
              prior_revenue / NULLIF(prior_assets, 0)
              AND current_revenue IS NOT NULL AND prior_revenue IS NOT NULL
              AND current_assets IS NOT NULL AND prior_assets IS NOT NULL THEN 1
         ELSE 0
     END) as f_score_complete,

    -- DATA COMPLETENESS CALCULATION (Updated for net margin)
    (
        CASE WHEN current_net_income IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_operating_cash_flow IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_assets IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_debt IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_current_assets IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_current_liabilities IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN COALESCE(current_shares_outstanding_bs, current_shares_outstanding) IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_net_income IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_revenue IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_net_income IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_assets IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_debt IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_current_assets IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_current_liabilities IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN COALESCE(prior_shares_outstanding_bs, prior_shares_outstanding) IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_net_income IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_revenue IS NOT NULL THEN 1 ELSE 0 END
    ) * 100 / 17 as data_completeness_score,

    -- Additional calculated metrics for display
    CASE
        WHEN current_assets > 0
        THEN current_net_income / current_assets
        ELSE NULL
    END as current_roa,

    CASE
        WHEN current_assets > 0
        THEN current_debt / current_assets
        ELSE NULL
    END as current_debt_ratio,

    CASE
        WHEN current_current_liabilities > 0
        THEN current_current_assets / current_current_liabilities
        ELSE NULL
    END as current_current_ratio,

    -- NET MARGIN (instead of gross margin)
    CASE
        WHEN current_revenue > 0 AND current_net_income IS NOT NULL
        THEN current_net_income / current_revenue
        ELSE NULL
    END as current_net_margin,

    CASE
        WHEN current_assets > 0
        THEN current_revenue / current_assets
        ELSE NULL
    END as current_asset_turnover

FROM piotroski_multi_year_data fd
WHERE fd.current_net_income IS NOT NULL
  OR fd.current_operating_cash_flow IS NOT NULL;

-- Create screening results view
CREATE VIEW piotroski_screening_results AS
SELECT
    stock_id,
    symbol,
    sector,
    industry,
    current_net_income,
    f_score_complete,
    data_completeness_score,
    criterion_positive_net_income,
    criterion_positive_operating_cash_flow,
    criterion_improving_roa,
    criterion_cash_flow_quality,
    criterion_decreasing_debt_ratio,
    criterion_improving_current_ratio,
    criterion_no_dilution,
    criterion_improving_net_margin,
    criterion_improving_asset_turnover,
    current_roa,
    current_debt_ratio,
    current_current_ratio,
    current_net_margin,
    current_asset_turnover,
    current_operating_cash_flow,
    pb_ratio,
    CASE
        WHEN f_score_complete >= 7 THEN 1
        ELSE 0
    END as passes_screening
FROM piotroski_f_score_complete;
