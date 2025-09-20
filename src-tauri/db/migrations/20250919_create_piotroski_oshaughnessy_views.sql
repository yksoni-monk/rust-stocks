-- Add Piotroski F-Score and O'Shaughnessy Value Composite Views
-- Migration: 20250919_create_piotroski_oshaughnessy_views.sql

-- ============================================================================
-- PIOTROSKI F-SCORE IMPLEMENTATION (7 CRITERIA - REALISTIC)
-- ============================================================================

CREATE VIEW piotroski_f_score_complete AS
WITH financial_data AS (
    SELECT
        s.id as stock_id,
        s.symbol,
        s.sector,

        -- Current period financials (TTM)
        current_income.net_income as current_net_income,
        current_income.revenue as current_revenue,
        current_income.gross_profit as current_gross_profit,
        current_income.shares_diluted as current_shares,
        current_balance.total_assets as current_assets,
        current_balance.total_debt as current_debt,
        current_balance.total_equity as current_equity,

        -- Prior period financials (previous TTM)
        prior_income.net_income as prior_net_income,
        prior_income.revenue as prior_revenue,
        prior_income.gross_profit as prior_gross_profit,
        prior_income.shares_diluted as prior_shares,
        prior_balance.total_assets as prior_assets,
        prior_balance.total_debt as prior_debt,

        -- Current ratios
        CASE
            WHEN current_balance.total_assets > 0
            THEN current_income.net_income / current_balance.total_assets
            ELSE NULL
        END as current_roa,

        CASE
            WHEN current_balance.total_assets > 0
            THEN current_balance.total_debt / current_balance.total_assets
            ELSE NULL
        END as current_debt_ratio,

        CASE
            WHEN current_income.revenue > 0 AND current_income.gross_profit IS NOT NULL
            THEN current_income.gross_profit / current_income.revenue
            ELSE NULL
        END as current_gross_margin,

        CASE
            WHEN current_balance.total_assets > 0
            THEN current_income.revenue / current_balance.total_assets
            ELSE NULL
        END as current_asset_turnover,

        -- Prior ratios
        CASE
            WHEN prior_balance.total_assets > 0
            THEN prior_income.net_income / prior_balance.total_assets
            ELSE NULL
        END as prior_roa,

        CASE
            WHEN prior_balance.total_assets > 0
            THEN prior_balance.total_debt / prior_balance.total_assets
            ELSE NULL
        END as prior_debt_ratio,

        CASE
            WHEN prior_income.revenue > 0 AND prior_income.gross_profit IS NOT NULL
            THEN prior_income.gross_profit / prior_income.revenue
            ELSE NULL
        END as prior_gross_margin,

        CASE
            WHEN prior_balance.total_assets > 0
            THEN prior_income.revenue / prior_balance.total_assets
            ELSE NULL
        END as prior_asset_turnover

    FROM stocks s

    -- Current TTM income data
    LEFT JOIN (
        SELECT stock_id, net_income, revenue, gross_profit, shares_diluted, report_date,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
        FROM income_statements
        WHERE period_type = 'TTM'
    ) current_income ON s.id = current_income.stock_id AND current_income.rn = 1

    -- Prior TTM income data
    LEFT JOIN (
        SELECT stock_id, net_income, revenue, gross_profit, shares_diluted, report_date,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
        FROM income_statements
        WHERE period_type = 'TTM'
    ) prior_income ON s.id = prior_income.stock_id AND prior_income.rn = 2

    -- Current TTM balance data
    LEFT JOIN (
        SELECT stock_id, total_assets, total_debt, total_equity, report_date,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
        FROM balance_sheets
        WHERE period_type = 'TTM'
    ) current_balance ON s.id = current_balance.stock_id AND current_balance.rn = 1

    -- Prior TTM balance data
    LEFT JOIN (
        SELECT stock_id, total_assets, total_debt, total_equity, report_date,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
        FROM balance_sheets
        WHERE period_type = 'TTM'
    ) prior_balance ON s.id = prior_balance.stock_id AND prior_balance.rn = 2
),
pb_ratios AS (
    SELECT DISTINCT
        stock_id,
        FIRST_VALUE(price / CASE
            WHEN b.total_equity > 0 AND i.shares_diluted > 0
            THEN (b.total_equity / i.shares_diluted)
            ELSE NULL
        END) OVER (
            PARTITION BY stock_id ORDER BY date DESC
        ) as pb_ratio
    FROM daily_valuation_ratios dvr
    JOIN (
        SELECT DISTINCT
            b.stock_id,
            b.total_equity,
            i.shares_diluted
        FROM balance_sheets b
        JOIN income_statements i ON b.stock_id = i.stock_id
            AND b.period_type = i.period_type
            AND b.report_date = i.report_date
        WHERE b.period_type = 'TTM'
          AND b.total_equity > 0
          AND i.shares_diluted > 0
    ) latest_equity ON dvr.stock_id = latest_equity.stock_id
    WHERE price IS NOT NULL
)
SELECT
    fd.*,
    pb.pb_ratio,

    -- F-Score Criteria (7 reliable criteria)
    -- Profitability (2 criteria)
    CASE WHEN current_net_income > 0 THEN 1 ELSE 0 END as profitability_positive_net_income,
    CASE
        WHEN current_roa > prior_roa
             AND current_roa IS NOT NULL
             AND prior_roa IS NOT NULL THEN 1
        ELSE 0
    END as profitability_improving_roa,

    -- Leverage/Liquidity (2 criteria)
    CASE
        WHEN current_debt_ratio < prior_debt_ratio
             AND current_debt_ratio IS NOT NULL
             AND prior_debt_ratio IS NOT NULL THEN 1
        ELSE 0
    END as leverage_decreasing_debt,
    CASE
        WHEN current_shares <= prior_shares
             AND current_shares IS NOT NULL
             AND prior_shares IS NOT NULL THEN 1
        ELSE 0
    END as leverage_no_dilution,

    -- Operating Efficiency (2 criteria)
    CASE
        WHEN current_gross_margin > prior_gross_margin
             AND current_gross_margin IS NOT NULL
             AND prior_gross_margin IS NOT NULL THEN 1
        ELSE 0
    END as efficiency_improving_margin,
    CASE
        WHEN current_asset_turnover > prior_asset_turnover
             AND current_asset_turnover IS NOT NULL
             AND prior_asset_turnover IS NOT NULL THEN 1
        ELSE 0
    END as efficiency_improving_turnover,

    -- Modified F-Score (0-6 instead of 0-9)
    (CASE WHEN current_net_income > 0 THEN 1 ELSE 0 END +
     CASE
         WHEN current_roa > prior_roa
              AND current_roa IS NOT NULL
              AND prior_roa IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN current_debt_ratio < prior_debt_ratio
              AND current_debt_ratio IS NOT NULL
              AND prior_debt_ratio IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN current_shares <= prior_shares
              AND current_shares IS NOT NULL
              AND prior_shares IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN current_gross_margin > prior_gross_margin
              AND current_gross_margin IS NOT NULL
              AND prior_gross_margin IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN current_asset_turnover > prior_asset_turnover
              AND current_asset_turnover IS NOT NULL
              AND prior_asset_turnover IS NOT NULL THEN 1
         ELSE 0
     END) as f_score_modified,

    -- Data completeness assessment
    CASE
        WHEN current_net_income IS NOT NULL
             AND current_roa IS NOT NULL AND prior_roa IS NOT NULL
             AND current_debt_ratio IS NOT NULL AND prior_debt_ratio IS NOT NULL
             AND current_shares IS NOT NULL AND prior_shares IS NOT NULL
             AND current_gross_margin IS NOT NULL AND prior_gross_margin IS NOT NULL
             AND current_asset_turnover IS NOT NULL AND prior_asset_turnover IS NOT NULL THEN 100
        WHEN current_net_income IS NOT NULL
             AND current_roa IS NOT NULL AND prior_roa IS NOT NULL
             AND current_debt_ratio IS NOT NULL AND prior_debt_ratio IS NOT NULL THEN 75
        WHEN current_net_income IS NOT NULL
             AND current_roa IS NOT NULL THEN 50
        ELSE 25
    END as data_completeness_score

FROM financial_data fd
LEFT JOIN pb_ratios pb ON fd.stock_id = pb.stock_id
WHERE fd.current_net_income IS NOT NULL
  AND pb.pb_ratio IS NOT NULL
  AND pb.pb_ratio <= 1.0; -- Piotroski value filter

-- Screening View with Adjusted Criteria
CREATE VIEW piotroski_screening_results AS
SELECT
    *,
    -- Adjusted screening criteria (account for modified scoring)
    CASE
        WHEN f_score_modified >= 5 -- Equivalent to 7/9 in original
             AND data_completeness_score >= 75
             AND pb_ratio <= 1.0
        THEN 1
        ELSE 0
    END as passes_piotroski_screening,

    -- Confidence scoring
    CASE
        WHEN data_completeness_score >= 100 THEN 'High'
        WHEN data_completeness_score >= 75 THEN 'Medium'
        WHEN data_completeness_score >= 50 THEN 'Low'
        ELSE 'Very Low'
    END as confidence_rating

FROM piotroski_f_score_complete
ORDER BY f_score_modified DESC, data_completeness_score DESC;

-- ============================================================================
-- O'SHAUGHNESSY VALUE COMPOSITE IMPLEMENTATION (REALISTIC)
-- ============================================================================

CREATE VIEW oshaughnessy_value_composite_complete AS
WITH latest_ratios AS (
    SELECT DISTINCT
        stock_id,
        FIRST_VALUE(pe_ratio) OVER (PARTITION BY stock_id ORDER BY date DESC) as pe_ratio,
        FIRST_VALUE(ps_ratio_ttm) OVER (PARTITION BY stock_id ORDER BY date DESC) as ps_ratio,
        FIRST_VALUE(evs_ratio_ttm) OVER (PARTITION BY stock_id ORDER BY date DESC) as evs_ratio,
        FIRST_VALUE(price) OVER (PARTITION BY stock_id ORDER BY date DESC) as current_price,
        FIRST_VALUE(market_cap) OVER (PARTITION BY stock_id ORDER BY date DESC) as market_cap,
        FIRST_VALUE(enterprise_value) OVER (PARTITION BY stock_id ORDER BY date DESC) as enterprise_value
    FROM daily_valuation_ratios
    WHERE pe_ratio IS NOT NULL OR ps_ratio_ttm IS NOT NULL
),
latest_financials AS (
    SELECT
        i.stock_id,
        i.net_income,
        i.revenue,
        i.shares_diluted,
        b.total_assets,
        b.total_equity,
        -- Calculate P/B ratio from balance sheet
        CASE
            WHEN b.total_equity > 0 AND i.shares_diluted > 0
            THEN (b.total_equity / i.shares_diluted)
            ELSE NULL
        END as book_value_per_share,
        -- Cash flow per share proxy (using net income if no cash flow data)
        CASE
            WHEN i.net_income IS NOT NULL AND i.shares_diluted > 0
            THEN (i.net_income / i.shares_diluted)
            ELSE NULL
        END as cash_flow_per_share_proxy,
        -- EBITDA estimation using gross profit
        CASE
            WHEN i.gross_profit IS NOT NULL
            THEN i.gross_profit * 0.8  -- Conservative EBITDA estimate
            ELSE NULL
        END as ebitda_estimate,
        ROW_NUMBER() OVER (PARTITION BY i.stock_id ORDER BY i.report_date DESC) as rn
    FROM income_statements i
    JOIN balance_sheets b ON i.stock_id = b.stock_id
        AND i.period_type = b.period_type
        AND i.report_date = b.report_date
    WHERE i.period_type = 'TTM'
      AND i.net_income IS NOT NULL
      AND b.total_equity IS NOT NULL
)
SELECT
    s.id as stock_id,
    s.symbol,
    s.sector,
    s.industry,

    -- Market Metrics
    lr.current_price,
    lr.market_cap,

    -- The 6 O'Shaughnessy Metrics (Realistic Implementation)
    lr.pe_ratio,
    CASE
        WHEN lf.book_value_per_share > 0
        THEN lr.current_price / lf.book_value_per_share
        ELSE NULL
    END as pb_ratio,
    lr.ps_ratio,
    CASE
        WHEN lf.cash_flow_per_share_proxy > 0
        THEN lr.current_price / lf.cash_flow_per_share_proxy
        ELSE NULL
    END as pcf_ratio_proxy,
    CASE
        WHEN lf.ebitda_estimate > 0 AND lr.enterprise_value IS NOT NULL
        THEN lr.enterprise_value / lf.ebitda_estimate
        ELSE lr.evs_ratio  -- Fallback to EV/Sales
    END as ev_ebitda_ratio_proxy,

    -- Shareholder yield (simplified - dividends only for now)
    COALESCE(
        (SELECT SUM(dividend_per_share) FROM dividend_history dh
         WHERE dh.stock_id = s.id
           AND dh.ex_date >= date('now', '-1 year')), 0
    ) as shareholder_yield,

    -- Raw Financial Data
    lf.net_income,
    lf.revenue,
    lf.total_equity,
    lf.book_value_per_share,
    lf.cash_flow_per_share_proxy,

    -- Data Quality Assessment (Realistic)
    CASE
        WHEN lr.pe_ratio IS NOT NULL AND lr.ps_ratio IS NOT NULL
             AND lf.book_value_per_share IS NOT NULL
             AND lf.cash_flow_per_share_proxy IS NOT NULL
             AND lr.evs_ratio IS NOT NULL THEN 100
        WHEN lr.pe_ratio IS NOT NULL AND lr.ps_ratio IS NOT NULL
             AND lf.book_value_per_share IS NOT NULL THEN 75
        WHEN lr.pe_ratio IS NOT NULL AND lr.ps_ratio IS NOT NULL THEN 50
        ELSE 25
    END as data_completeness_score

FROM stocks s
JOIN latest_ratios lr ON s.id = lr.stock_id
LEFT JOIN latest_financials lf ON s.id = lf.stock_id AND lf.rn = 1
WHERE lr.market_cap > 200000000 -- $200M minimum (O'Shaughnessy requirement)
  AND lr.pe_ratio > 0 -- Exclude negative P/E
  AND lr.ps_ratio > 0; -- Exclude negative P/S

-- Ranking View (O'Shaughnessy Method)
CREATE VIEW oshaughnessy_ranking_complete AS
WITH ranked_metrics AS (
    SELECT
        *,
        -- Rank each metric (1 = best value, lower is better)
        RANK() OVER (ORDER BY pe_ratio ASC) as pe_rank,
        RANK() OVER (ORDER BY pb_ratio ASC) as pb_rank,
        RANK() OVER (ORDER BY ps_ratio ASC) as ps_rank,
        RANK() OVER (ORDER BY pcf_ratio_proxy ASC) as pcf_rank,
        RANK() OVER (ORDER BY ev_ebitda_ratio_proxy ASC) as ev_ebitda_rank,
        RANK() OVER (ORDER BY shareholder_yield DESC) as yield_rank,
        COUNT(*) OVER () as total_stocks
    FROM oshaughnessy_value_composite_complete
    WHERE pe_ratio IS NOT NULL
      AND pb_ratio IS NOT NULL
      AND ps_ratio IS NOT NULL
      AND pcf_ratio_proxy IS NOT NULL
)
SELECT
    *,
    -- Calculate Value Composite Score (average of ranks)
    (pe_rank + pb_rank + ps_rank + pcf_rank + ev_ebitda_rank + yield_rank) / 6.0 as composite_score,

    -- Calculate percentile (lower percentile = better value)
    ROUND((composite_score / total_stocks) * 100, 1) as composite_percentile,

    -- Overall ranking
    RANK() OVER (ORDER BY composite_score ASC) as overall_rank,

    -- Pass/Fail screening (top 20% by default)
    CASE
        WHEN composite_score <= (total_stocks * 0.20)
             AND data_completeness_score >= 75
        THEN 1
        ELSE 0
    END as passes_oshaughnessy_screening

FROM ranked_metrics
ORDER BY composite_score ASC;

-- Note: Views cannot be indexed directly in SQLite
-- Indexes are applied to the underlying tables