-- S&P 500 Only - Piotroski F-Score and O'Shaughnessy Value Composite
-- Migration: 20250919_create_sp500_only_screening_views.sql

-- ============================================================================
-- PIOTROSKI F-SCORE - S&P 500 ONLY
-- ============================================================================

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
      AND s.is_sp500 = 1  -- S&P 500 ONLY
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

-- ============================================================================
-- O'SHAUGHNESSY VALUE COMPOSITE - S&P 500 ONLY
-- ============================================================================

CREATE VIEW oshaughnessy_value_composite_simple AS
SELECT
    dvr.stock_id,
    s.symbol,
    s.sector,
    s.industry,
    dvr.price as current_price,
    dvr.market_cap,
    dvr.ps_ratio_ttm as ps_ratio,
    dvr.evs_ratio_ttm as evs_ratio,

    -- Data quality score (simplified)
    CASE
        WHEN dvr.ps_ratio_ttm IS NOT NULL AND dvr.evs_ratio_ttm IS NOT NULL THEN 100
        WHEN dvr.ps_ratio_ttm IS NOT NULL THEN 75
        ELSE 50
    END as data_completeness_score

FROM (
    SELECT stock_id, price, market_cap, ps_ratio_ttm, evs_ratio_ttm,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY date DESC) as rn
    FROM daily_valuation_ratios
    WHERE ps_ratio_ttm IS NOT NULL AND ps_ratio_ttm > 0
) dvr
JOIN stocks s ON dvr.stock_id = s.id
WHERE dvr.rn = 1
  AND s.is_sp500 = 1  -- S&P 500 ONLY
  AND dvr.ps_ratio_ttm > 0;

-- O'Shaughnessy ranking view
CREATE VIEW oshaughnessy_ranking_simple AS
WITH ranked_data AS (
    SELECT *,
        -- Rank each metric (lower rank = better value)
        RANK() OVER (ORDER BY ps_ratio ASC) as ps_rank,
        RANK() OVER (ORDER BY evs_ratio ASC) as evs_rank,
        COUNT(*) OVER () as total_stocks
    FROM oshaughnessy_value_composite_simple
    WHERE ps_ratio IS NOT NULL AND evs_ratio IS NOT NULL
)
SELECT *,
    -- Simplified composite score (average of 2 available ranks)
    (ps_rank + evs_rank) / 2.0 as composite_score,

    -- Percentile ranking
    ROUND(((ps_rank + evs_rank) / 2.0 / total_stocks) * 100, 1) as composite_percentile,

    -- Overall ranking
    RANK() OVER (ORDER BY (ps_rank + evs_rank) / 2.0 ASC) as overall_rank,

    -- Screening result (top 20% by default)
    CASE
        WHEN (ps_rank + evs_rank) / 2.0 <= (total_stocks * 0.20)
             AND data_completeness_score >= 75 THEN 1
        ELSE 0
    END as passes_screening

FROM ranked_data
ORDER BY (ps_rank + evs_rank) / 2.0 ASC;