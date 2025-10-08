-- Fix Piotroski views to use 'sector' instead of non-existent 'industry' column

-- Drop existing views
DROP VIEW IF EXISTS piotroski_screening_results;
DROP VIEW IF EXISTS piotroski_multi_year_data;

-- Recreate piotroski_multi_year_data view with 'sector' instead of 'industry'
CREATE VIEW piotroski_multi_year_data AS
WITH financial_data AS (
    SELECT
        s.id as stock_id,
        s.symbol,
        s.sector,
        s.sector as industry, -- Use sector as industry for compatibility

        -- Current TTM data (most recent)
        current_income.net_income as current_net_income,
        current_income.revenue as current_revenue,
        current_income.gross_profit as current_gross_profit,
        current_income.cost_of_revenue as current_cost_of_revenue,
        current_income.interest_expense as current_interest_expense,
        current_income.shares_diluted as current_shares,
        current_income.shares_diluted as current_shares_outstanding,

        -- Current TTM balance data
        current_balance.total_assets as current_assets,
        current_balance.total_debt as current_debt,
        current_balance.total_equity as current_equity,
        current_balance.current_assets as current_current_assets,
        current_balance.current_liabilities as current_current_liabilities,
        current_balance.short_term_debt as current_short_term_debt,
        current_balance.long_term_debt as current_long_term_debt,

        -- Prior year TTM data (for year-over-year comparisons)
        prior_income.net_income as prior_net_income,
        prior_income.revenue as prior_revenue,
        prior_income.gross_profit as prior_gross_profit,
        prior_income.cost_of_revenue as prior_cost_of_revenue,
        prior_income.interest_expense as prior_interest_expense,
        prior_income.shares_diluted as prior_shares,
        prior_income.shares_diluted as prior_shares_outstanding,

        -- Prior year balance data
        prior_balance.total_assets as prior_assets,
        prior_balance.total_debt as prior_debt,
        prior_balance.total_equity as prior_equity,
        prior_balance.current_assets as prior_current_assets,
        prior_balance.current_liabilities as prior_current_liabilities,
        prior_balance.short_term_debt as prior_short_term_debt,
        prior_balance.long_term_debt as prior_long_term_debt,

        -- Current TTM cash flow data
        current_cashflow.operating_cash_flow as current_operating_cash_flow,
        current_cashflow.investing_cash_flow as current_investing_cash_flow,
        current_cashflow.financing_cash_flow as current_financing_cash_flow,
        current_cashflow.net_cash_flow as current_net_cash_flow,

        -- Prior year cash flow data
        prior_cashflow.operating_cash_flow as prior_operating_cash_flow,
        prior_cashflow.investing_cash_flow as prior_investing_cash_flow,
        prior_cashflow.financing_cash_flow as prior_financing_cash_flow,
        prior_cashflow.net_cash_flow as prior_net_cash_flow

    FROM stocks s

    -- Current TTM income data (most recent)
    LEFT JOIN (
        SELECT stock_id, net_income, revenue, gross_profit, cost_of_revenue,
               interest_expense, shares_diluted, report_date, fiscal_year,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY fiscal_year DESC, report_date DESC) as rn
        FROM income_statements
        WHERE period_type = 'TTM'
    ) current_income ON s.id = current_income.stock_id AND current_income.rn = 1

    -- Prior year TTM income data (previous year)
    LEFT JOIN (
        SELECT stock_id, net_income, revenue, gross_profit, cost_of_revenue,
               interest_expense, shares_diluted, report_date, fiscal_year,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY fiscal_year DESC, report_date DESC) as rn
        FROM income_statements
        WHERE period_type = 'TTM'
    ) prior_income ON s.id = prior_income.stock_id AND prior_income.rn = 2

    -- Current TTM balance data (most recent)
    LEFT JOIN (
        SELECT stock_id, total_assets, total_debt, total_equity,
               current_assets, current_liabilities, short_term_debt, long_term_debt, report_date, fiscal_year,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY fiscal_year DESC, report_date DESC) as rn
        FROM balance_sheets
        WHERE period_type = 'TTM'
    ) current_balance ON s.id = current_balance.stock_id AND current_balance.rn = 1

    -- Prior year TTM balance data (previous year)
    LEFT JOIN (
        SELECT stock_id, total_assets, total_debt, total_equity,
               current_assets, current_liabilities, short_term_debt, long_term_debt, report_date, fiscal_year,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY fiscal_year DESC, report_date DESC) as rn
        FROM balance_sheets
        WHERE period_type = 'TTM'
    ) prior_balance ON s.id = prior_balance.stock_id AND prior_balance.rn = 2

    -- Current TTM cash flow data (most recent)
    LEFT JOIN (
        SELECT stock_id, operating_cash_flow, investing_cash_flow,
               financing_cash_flow, net_cash_flow, report_date, fiscal_year,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY fiscal_year DESC, report_date DESC) as rn
        FROM cash_flow_statements
        WHERE period_type = 'TTM'
    ) current_cashflow ON s.id = current_cashflow.stock_id AND current_cashflow.rn = 1

    -- Prior year TTM cash flow data (previous year)
    LEFT JOIN (
        SELECT stock_id, operating_cash_flow, investing_cash_flow,
               financing_cash_flow, net_cash_flow, report_date, fiscal_year,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY fiscal_year DESC, report_date DESC) as rn
        FROM cash_flow_statements
        WHERE period_type = 'TTM'
    ) prior_cashflow ON s.id = prior_cashflow.stock_id AND prior_cashflow.rn = 2
)
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
        WHEN current_shares_outstanding <= prior_shares_outstanding
             AND current_shares_outstanding IS NOT NULL
             AND prior_shares_outstanding IS NOT NULL THEN 1
        ELSE 0
    END as criterion_no_dilution,

    -- OPERATING EFFICIENCY (2 criteria)
    CASE
        WHEN current_gross_profit / NULLIF(current_revenue, 0) >
             prior_gross_profit / NULLIF(prior_revenue, 0)
             AND current_gross_profit IS NOT NULL AND prior_gross_profit IS NOT NULL
             AND current_revenue IS NOT NULL AND prior_revenue IS NOT NULL THEN 1
        ELSE 0
    END as criterion_improving_gross_margin,
    CASE
        WHEN current_revenue / NULLIF(current_assets, 0) >
             prior_revenue / NULLIF(prior_assets, 0)
             AND current_revenue IS NOT NULL AND prior_revenue IS NOT NULL
             AND current_assets IS NOT NULL AND prior_assets IS NOT NULL THEN 1
        ELSE 0
    END as criterion_improving_asset_turnover,

    -- F-SCORE CALCULATION
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
         WHEN current_shares_outstanding <= prior_shares_outstanding
              AND current_shares_outstanding IS NOT NULL
              AND prior_shares_outstanding IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN current_gross_profit / NULLIF(current_revenue, 0) >
              prior_gross_profit / NULLIF(prior_revenue, 0)
              AND current_gross_profit IS NOT NULL AND prior_gross_profit IS NOT NULL
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

    -- DATA COMPLETENESS CALCULATION
    (
        CASE WHEN current_net_income IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_operating_cash_flow IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_assets IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_debt IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_current_assets IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_current_liabilities IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_shares_outstanding IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_gross_profit IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_revenue IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_net_income IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_assets IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_debt IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_current_assets IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_current_liabilities IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_shares_outstanding IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_gross_profit IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_revenue IS NOT NULL THEN 1 ELSE 0 END
    ) * 100 / 17 as data_completeness_score,

    -- Additional calculated metrics
    CASE WHEN current_assets > 0 THEN current_net_income / current_assets ELSE NULL END as current_roa,
    CASE WHEN current_assets > 0 THEN current_debt / current_assets ELSE NULL END as current_debt_ratio,
    CASE WHEN current_current_liabilities > 0 THEN current_current_assets / current_current_liabilities ELSE NULL END as current_current_ratio,
    CASE WHEN current_revenue > 0 AND current_gross_profit IS NOT NULL THEN current_gross_profit / current_revenue ELSE NULL END as current_gross_margin,
    CASE WHEN current_assets > 0 THEN current_revenue / current_assets ELSE NULL END as current_asset_turnover

FROM financial_data fd
WHERE fd.current_net_income IS NOT NULL
  AND fd.current_operating_cash_flow IS NOT NULL;

-- Recreate piotroski_screening_results view
CREATE VIEW piotroski_screening_results AS
SELECT
    *,
    CASE
        WHEN f_score_complete >= 6
             AND data_completeness_score >= 60
        THEN 1
        ELSE 0
    END as passes_screening,
    CASE
        WHEN data_completeness_score >= 90 THEN 'High'
        WHEN data_completeness_score >= 70 THEN 'Medium'
        WHEN data_completeness_score >= 50 THEN 'Low'
        ELSE 'Very Low'
    END as confidence_level,
    CASE
        WHEN f_score_complete >= 7 THEN 'Excellent'
        WHEN f_score_complete >= 5 THEN 'Good'
        WHEN f_score_complete >= 3 THEN 'Average'
        WHEN f_score_complete >= 1 THEN 'Poor'
        ELSE 'Very Poor'
    END as f_score_interpretation
FROM piotroski_multi_year_data
ORDER BY f_score_complete DESC, data_completeness_score DESC;
