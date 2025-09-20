-- Simple but working Piotroski and O'Shaughnessy implementations
-- Migration: 20250919_create_simple_screening_views.sql

-- Drop existing views if they exist
DROP VIEW IF EXISTS piotroski_f_score_complete;
DROP VIEW IF EXISTS piotroski_screening_results;
DROP VIEW IF EXISTS oshaughnessy_value_composite_complete;
DROP VIEW IF EXISTS oshaughnessy_ranking_complete;

-- ============================================================================
-- SIMPLIFIED PIOTROSKI F-SCORE (WORKING VERSION)
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
        END as prior_asset_turnover,

        -- P/B ratio calculation
        CASE
            WHEN curr_b.total_equity > 0 AND curr_i.shares_diluted > 0
            THEN (SELECT price FROM daily_valuation_ratios dvr
                  WHERE dvr.stock_id = s.id AND dvr.price IS NOT NULL
                  ORDER BY dvr.date DESC LIMIT 1) / (curr_b.total_equity / curr_i.shares_diluted)
            ELSE NULL
        END as pb_ratio

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

FROM financial_pairs
WHERE pb_ratio IS NOT NULL AND pb_ratio <= 1.0;

-- Screening results view
CREATE VIEW piotroski_screening_results AS
SELECT *,
    CASE
        WHEN f_score >= 5 AND data_completeness_score >= 75 THEN 1
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
-- SIMPLIFIED O'SHAUGHNESSY VALUE COMPOSITE (WORKING VERSION)
-- ============================================================================

CREATE VIEW oshaughnessy_value_composite_simple AS
WITH base_data AS (
    SELECT
        s.id as stock_id,
        s.symbol,
        s.sector,
        s.industry,

        -- Latest market data
        dvr.price as current_price,
        dvr.market_cap,
        dvr.pe_ratio,
        dvr.ps_ratio_ttm as ps_ratio,
        dvr.evs_ratio_ttm as evs_ratio,

        -- Financial data
        i.net_income,
        i.revenue,
        i.shares_diluted,
        b.total_equity,

        -- P/B ratio calculation
        CASE
            WHEN b.total_equity > 0 AND i.shares_diluted > 0
            THEN dvr.price / (b.total_equity / i.shares_diluted)
            ELSE NULL
        END as pb_ratio,

        -- P/CF proxy (using net income)
        CASE
            WHEN i.net_income > 0 AND i.shares_diluted > 0
            THEN dvr.price / (i.net_income / i.shares_diluted)
            ELSE NULL
        END as pcf_ratio_proxy,

        -- Dividend yield (simplified)
        COALESCE((SELECT SUM(dividend_per_share)
                  FROM dividend_history dh
                  WHERE dh.stock_id = s.id
                    AND dh.ex_date >= date('now', '-1 year')), 0) / dvr.price as dividend_yield,

        -- Data quality
        CASE
            WHEN dvr.pe_ratio IS NOT NULL AND dvr.ps_ratio_ttm IS NOT NULL AND
                 b.total_equity IS NOT NULL AND i.net_income IS NOT NULL THEN 100
            WHEN dvr.pe_ratio IS NOT NULL AND dvr.ps_ratio_ttm IS NOT NULL THEN 75
            WHEN dvr.pe_ratio IS NOT NULL THEN 50
            ELSE 25
        END as data_completeness_score

    FROM stocks s

    -- Latest valuation ratios
    JOIN (SELECT stock_id, price, market_cap, pe_ratio, ps_ratio_ttm, evs_ratio_ttm,
                 ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY date DESC) as rn
          FROM daily_valuation_ratios
          WHERE pe_ratio IS NOT NULL) dvr ON s.id = dvr.stock_id AND dvr.rn = 1

    -- Latest financial data
    JOIN (SELECT stock_id, net_income, revenue, shares_diluted,
                 ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
          FROM income_statements WHERE period_type = 'TTM') i ON s.id = i.stock_id AND i.rn = 1

    JOIN (SELECT stock_id, total_equity,
                 ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
          FROM balance_sheets WHERE period_type = 'TTM') b ON s.id = b.stock_id AND b.rn = 1

    WHERE dvr.market_cap > 200000000  -- $200M minimum
      AND dvr.pe_ratio > 0
      AND dvr.ps_ratio_ttm > 0
)
SELECT *
FROM base_data;

-- O'Shaughnessy ranking view
CREATE VIEW oshaughnessy_ranking_simple AS
WITH ranked_data AS (
    SELECT *,
        -- Rank each metric (lower rank = better value)
        RANK() OVER (ORDER BY pe_ratio ASC) as pe_rank,
        RANK() OVER (ORDER BY pb_ratio ASC) as pb_rank,
        RANK() OVER (ORDER BY ps_ratio ASC) as ps_rank,
        RANK() OVER (ORDER BY pcf_ratio_proxy ASC) as pcf_rank,
        RANK() OVER (ORDER BY evs_ratio ASC) as evs_rank,
        RANK() OVER (ORDER BY dividend_yield DESC) as dividend_rank,
        COUNT(*) OVER () as total_stocks
    FROM oshaughnessy_value_composite_simple
    WHERE pe_ratio IS NOT NULL AND pb_ratio IS NOT NULL AND ps_ratio IS NOT NULL
)
SELECT *,
    -- Composite score (average of ranks)
    (pe_rank + pb_rank + ps_rank + pcf_rank + evs_rank + dividend_rank) / 6.0 as composite_score,

    -- Percentile ranking
    ROUND((composite_score / total_stocks) * 100, 1) as composite_percentile,

    -- Overall ranking
    RANK() OVER (ORDER BY composite_score ASC) as overall_rank,

    -- Screening result
    CASE
        WHEN composite_score <= (total_stocks * 0.20) AND data_completeness_score >= 75 THEN 1
        ELSE 0
    END as passes_screening

FROM ranked_data
ORDER BY composite_score ASC;