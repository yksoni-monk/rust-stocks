-- Working Piotroski F-Score implementation without restrictive filters
-- Migration: 20250919_create_working_piotroski_views.sql

CREATE VIEW piotroski_f_score_simple AS
WITH financial_pairs AS (
    SELECT
        s.id as stock_id,
        s.symbol,
        s.sector,

        -- Current period (most recent TTM)
        curr_i.net_income as current_net_income,
        curr_i.revenue as current_revenue,
        curr_i.gross_profit as current_gross_profit,
        curr_i.shares_diluted as current_shares,
        curr_b.total_assets as current_assets,
        curr_b.total_debt as current_debt,
        curr_b.total_equity as current_equity,

        -- Prior period (second most recent TTM)
        prev_i.net_income as prior_net_income,
        prev_i.revenue as prior_revenue,
        prev_i.gross_profit as prior_gross_profit,
        prev_i.shares_diluted as prior_shares,
        prev_b.total_assets as prior_assets,
        prev_b.total_debt as prior_debt,

        -- Current period ratios
        CASE
            WHEN curr_b.total_assets > 0 THEN curr_i.net_income / curr_b.total_assets
            ELSE NULL
        END as current_roa,
        CASE
            WHEN curr_b.total_assets > 0 THEN curr_b.total_debt / curr_b.total_assets
            ELSE NULL
        END as current_debt_ratio,
        CASE
            WHEN curr_i.revenue > 0 AND curr_i.gross_profit IS NOT NULL
            THEN curr_i.gross_profit / curr_i.revenue
            ELSE NULL
        END as current_gross_margin,
        CASE
            WHEN curr_b.total_assets > 0 THEN curr_i.revenue / curr_b.total_assets
            ELSE NULL
        END as current_asset_turnover,

        -- Prior period ratios
        CASE
            WHEN prev_b.total_assets > 0 THEN prev_i.net_income / prev_b.total_assets
            ELSE NULL
        END as prior_roa,
        CASE
            WHEN prev_b.total_assets > 0 THEN prev_b.total_debt / prev_b.total_assets
            ELSE NULL
        END as prior_debt_ratio,
        CASE
            WHEN prev_i.revenue > 0 AND prev_i.gross_profit IS NOT NULL
            THEN prev_i.gross_profit / prev_i.revenue
            ELSE NULL
        END as prior_gross_margin,
        CASE
            WHEN prev_b.total_assets > 0 THEN prev_i.revenue / prev_b.total_assets
            ELSE NULL
        END as prior_asset_turnover

    FROM stocks s

    -- Most recent TTM financials
    JOIN (SELECT stock_id, net_income, revenue, gross_profit, shares_diluted, report_date,
                 ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
          FROM income_statements WHERE period_type = 'TTM') curr_i
          ON s.id = curr_i.stock_id AND curr_i.rn = 1

    JOIN (SELECT stock_id, total_assets, total_debt, total_equity, report_date,
                 ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
          FROM balance_sheets WHERE period_type = 'TTM') curr_b
          ON s.id = curr_b.stock_id AND curr_b.rn = 1

    -- Previous TTM financials
    LEFT JOIN (SELECT stock_id, net_income, revenue, gross_profit, shares_diluted, report_date,
                      ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
               FROM income_statements WHERE period_type = 'TTM') prev_i
               ON s.id = prev_i.stock_id AND prev_i.rn = 2

    LEFT JOIN (SELECT stock_id, total_assets, total_debt, total_equity, report_date,
                      ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
               FROM balance_sheets WHERE period_type = 'TTM') prev_b
               ON s.id = prev_b.stock_id AND prev_b.rn = 2

    WHERE curr_i.net_income IS NOT NULL
)
SELECT
    *,

    -- F-Score criteria (6 realistic criteria)
    CASE WHEN current_net_income > 0 THEN 1 ELSE 0 END as criterion_positive_income,
    CASE WHEN current_roa > prior_roa AND current_roa IS NOT NULL AND prior_roa IS NOT NULL THEN 1 ELSE 0 END as criterion_improving_roa,
    CASE WHEN current_debt_ratio < prior_debt_ratio AND current_debt_ratio IS NOT NULL AND prior_debt_ratio IS NOT NULL THEN 1 ELSE 0 END as criterion_decreasing_debt,
    CASE WHEN current_shares <= prior_shares AND current_shares IS NOT NULL AND prior_shares IS NOT NULL THEN 1 ELSE 0 END as criterion_no_dilution,
    CASE WHEN current_gross_margin > prior_gross_margin AND current_gross_margin IS NOT NULL AND prior_gross_margin IS NOT NULL THEN 1 ELSE 0 END as criterion_improving_margin,
    CASE WHEN current_asset_turnover > prior_asset_turnover AND current_asset_turnover IS NOT NULL AND prior_asset_turnover IS NOT NULL THEN 1 ELSE 0 END as criterion_improving_turnover,

    -- Total F-Score (0-6)
    (CASE WHEN current_net_income > 0 THEN 1 ELSE 0 END +
     CASE WHEN current_roa > prior_roa AND current_roa IS NOT NULL AND prior_roa IS NOT NULL THEN 1 ELSE 0 END +
     CASE WHEN current_debt_ratio < prior_debt_ratio AND current_debt_ratio IS NOT NULL AND prior_debt_ratio IS NOT NULL THEN 1 ELSE 0 END +
     CASE WHEN current_shares <= prior_shares AND current_shares IS NOT NULL AND prior_shares IS NOT NULL THEN 1 ELSE 0 END +
     CASE WHEN current_gross_margin > prior_gross_margin AND current_gross_margin IS NOT NULL AND prior_gross_margin IS NOT NULL THEN 1 ELSE 0 END +
     CASE WHEN current_asset_turnover > prior_asset_turnover AND current_asset_turnover IS NOT NULL AND prior_asset_turnover IS NOT NULL THEN 1 ELSE 0 END) as f_score,

    -- Data completeness score
    CASE
        WHEN current_roa IS NOT NULL AND prior_roa IS NOT NULL AND
             current_debt_ratio IS NOT NULL AND prior_debt_ratio IS NOT NULL AND
             current_gross_margin IS NOT NULL AND prior_gross_margin IS NOT NULL AND
             current_asset_turnover IS NOT NULL AND prior_asset_turnover IS NOT NULL THEN 100
        WHEN current_roa IS NOT NULL AND current_debt_ratio IS NOT NULL THEN 75
        WHEN current_net_income IS NOT NULL THEN 50
        ELSE 25
    END as data_completeness_score

FROM financial_pairs;

-- Screening results view
CREATE VIEW piotroski_screening_results AS
SELECT *,
    CASE
        WHEN f_score >= 4 AND data_completeness_score >= 75 THEN 1
        ELSE 0
    END as passes_screening,
    CASE
        WHEN data_completeness_score >= 100 THEN 'High'
        WHEN data_completeness_score >= 75 THEN 'Medium'
        WHEN data_completeness_score >= 50 THEN 'Low'
        ELSE 'Very Low'
    END as confidence_level
FROM piotroski_f_score_simple
ORDER BY f_score DESC, data_completeness_score DESC;