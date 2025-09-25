-- Migration: Replace Gross Margin with Net Margin in Piotroski F-Score
-- Created: 2025-09-25
-- Purpose: Improve S&P 500 coverage by using net margin instead of gross margin
-- Expected improvement: From 43.1% to 96.2% coverage (217 → 484 stocks)

-- Drop existing Piotroski views to recreate with net margin
DROP VIEW IF EXISTS piotroski_f_score_complete;
DROP VIEW IF EXISTS piotroski_screening_results;

-- Recreate the complete Piotroski F-Score view using NET MARGIN instead of GROSS MARGIN
CREATE VIEW piotroski_f_score_complete AS
SELECT
    fd.*,
    NULL as pb_ratio, -- Simplified for now

    -- ============================================================================
    -- PIOTROSKI F-SCORE WITH ANNUAL DATA (Better for Analysis)
    -- ============================================================================

    -- PROFITABILITY (4 criteria)
    -- 1. Positive Net Income
    CASE WHEN current_net_income > 0 THEN 1 ELSE 0 END as criterion_positive_net_income,

    -- 2. Positive Operating Cash Flow
    CASE WHEN current_operating_cash_flow > 0 THEN 1 ELSE 0 END as criterion_positive_operating_cash_flow,

    -- 3. Improving ROA (Return on Assets) - requires historical data
    CASE
        WHEN current_net_income / NULLIF(current_assets, 0) >
             prior_net_income / NULLIF(prior_assets, 0)
             AND current_net_income IS NOT NULL AND prior_net_income IS NOT NULL
             AND current_assets IS NOT NULL AND prior_assets IS NOT NULL THEN 1
        ELSE 0
    END as criterion_improving_roa,

    -- 4. Cash Flow Quality (Operating Cash Flow > Net Income)
    CASE
        WHEN current_operating_cash_flow > current_net_income
             AND current_operating_cash_flow IS NOT NULL
             AND current_net_income IS NOT NULL THEN 1
        ELSE 0
    END as criterion_cash_flow_quality,

    -- LEVERAGE/LIQUIDITY (3 criteria)
    -- 5. Decreasing Debt-to-Assets Ratio - requires historical data
    CASE
        WHEN current_debt / NULLIF(current_assets, 0) <
             prior_debt / NULLIF(prior_assets, 0)
             AND current_debt IS NOT NULL AND prior_debt IS NOT NULL
             AND current_assets IS NOT NULL AND prior_assets IS NOT NULL THEN 1
        ELSE 0
    END as criterion_decreasing_debt_ratio,

    -- 6. Improving Current Ratio - requires historical data
    CASE
        WHEN current_current_assets / NULLIF(current_current_liabilities, 0) >
             prior_current_assets / NULLIF(prior_current_liabilities, 0)
             AND current_current_assets IS NOT NULL AND prior_current_assets IS NOT NULL
             AND current_current_liabilities IS NOT NULL AND prior_current_liabilities IS NOT NULL THEN 1
        ELSE 0
    END as criterion_improving_current_ratio,

    -- 7. No Share Dilution - requires historical data (use balance sheet shares outstanding)
    CASE
        WHEN COALESCE(current_shares_outstanding_bs, current_shares_outstanding) <=
             COALESCE(prior_shares_outstanding_bs, prior_shares_outstanding)
             AND COALESCE(current_shares_outstanding_bs, current_shares_outstanding) IS NOT NULL
             AND COALESCE(prior_shares_outstanding_bs, prior_shares_outstanding) IS NOT NULL THEN 1
        ELSE 0
    END as criterion_no_dilution,

    -- OPERATING EFFICIENCY (2 criteria)
    -- 8. 🔄 CHANGED: Improving NET MARGIN instead of Gross Margin - requires historical data
    CASE
        WHEN current_net_income / NULLIF(current_revenue, 0) >
             prior_net_income / NULLIF(prior_revenue, 0)
             AND current_net_income IS NOT NULL AND prior_net_income IS NOT NULL
             AND current_revenue IS NOT NULL AND prior_revenue IS NOT NULL THEN 1
        ELSE 0
    END as criterion_improving_gross_margin,

    -- 9. Improving Asset Turnover - requires historical data
    CASE
        WHEN current_revenue / NULLIF(current_assets, 0) >
             prior_revenue / NULLIF(prior_assets, 0)
             AND current_revenue IS NOT NULL AND prior_revenue IS NOT NULL
             AND current_assets IS NOT NULL AND prior_assets IS NOT NULL THEN 1
        ELSE 0
    END as criterion_improving_asset_turnover,

    -- TOTAL F-SCORE CALCULATION (0-9) - Updated to use NET MARGIN
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
     -- 🔄 CHANGED: Using NET MARGIN instead of GROSS MARGIN
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
        CASE WHEN current_net_income IS NOT NULL THEN 1 ELSE 0 END + -- Changed from gross_profit to net_income
        CASE WHEN current_revenue IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_net_income IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_assets IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_debt IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_current_assets IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_current_liabilities IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN COALESCE(prior_shares_outstanding_bs, prior_shares_outstanding) IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_net_income IS NOT NULL THEN 1 ELSE 0 END + -- Changed from gross_profit to net_income
        CASE WHEN prior_revenue IS NOT NULL THEN 1 ELSE 0 END
    ) * 100 / 17 as data_completeness_score,

    -- Additional calculated metrics for display (Updated gross_margin to net_margin)
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

    -- 🔄 CHANGED: Display NET MARGIN instead of GROSS MARGIN
    CASE
        WHEN current_revenue > 0 AND current_net_income IS NOT NULL
        THEN current_net_income / current_revenue
        ELSE NULL
    END as current_gross_margin,

    CASE
        WHEN current_assets > 0
        THEN current_revenue / current_assets
        ELSE NULL
    END as current_asset_turnover

FROM piotroski_multi_year_data fd
WHERE fd.current_net_income IS NOT NULL
  OR fd.current_operating_cash_flow IS NOT NULL;

-- Create simplified screening results view (unchanged)
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
    criterion_improving_gross_margin, -- Note: Still named "gross_margin" for compatibility but uses net margin
    criterion_improving_asset_turnover,
    current_roa,
    current_debt_ratio,
    current_current_ratio,
    current_gross_margin, -- Note: Still named "gross_margin" for compatibility but displays net margin
    current_asset_turnover,
    current_operating_cash_flow,
    pb_ratio,
    CASE
        WHEN f_score_complete >= 7 THEN 1
        ELSE 0
    END as passes_screening
FROM piotroski_f_score_complete;