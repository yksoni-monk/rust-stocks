-- Working O'Shaughnessy Value Composite implementation using available data
-- Migration: 20250919_create_working_oshaughnessy_views.sql

-- First, create a view using P/S ratios (which we have current data for)
CREATE VIEW oshaughnessy_value_composite_simple AS
WITH base_data AS (
    SELECT
        s.id as stock_id,
        s.symbol,
        s.sector,
        s.industry,

        -- Latest market data from daily_valuation_ratios
        dvr.price as current_price,
        dvr.market_cap,
        dvr.ps_ratio_ttm as ps_ratio,
        dvr.evs_ratio_ttm as evs_ratio,

        -- Financial data from income statements and balance sheets
        i.net_income,
        i.revenue,
        i.shares_diluted,
        b.total_equity,

        -- Calculate P/B ratio manually
        CASE
            WHEN b.total_equity > 0 AND i.shares_diluted > 0
            THEN dvr.price / (b.total_equity / i.shares_diluted)
            ELSE NULL
        END as pb_ratio,

        -- Calculate P/E ratio manually (fallback since P/E data is stale)
        CASE
            WHEN i.net_income > 0 AND i.shares_diluted > 0
            THEN dvr.price / (i.net_income / i.shares_diluted)
            ELSE NULL
        END as pe_ratio_calculated,

        -- Dividend yield (simplified)
        COALESCE((SELECT SUM(dividend_per_share)
                  FROM dividend_history dh
                  WHERE dh.stock_id = s.id
                    AND dh.ex_date >= date('now', '-1 year')), 0) / dvr.price as dividend_yield,

        -- Data quality assessment
        CASE
            WHEN dvr.ps_ratio_ttm IS NOT NULL AND b.total_equity IS NOT NULL
                 AND i.net_income IS NOT NULL AND dvr.evs_ratio_ttm IS NOT NULL THEN 100
            WHEN dvr.ps_ratio_ttm IS NOT NULL AND b.total_equity IS NOT NULL THEN 75
            WHEN dvr.ps_ratio_ttm IS NOT NULL THEN 50
            ELSE 25
        END as data_completeness_score

    FROM stocks s

    -- Latest valuation ratios (use P/S data which is current)
    JOIN (SELECT stock_id, price, market_cap, ps_ratio_ttm, evs_ratio_ttm,
                 ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY date DESC) as rn
          FROM daily_valuation_ratios
          WHERE ps_ratio_ttm IS NOT NULL AND ps_ratio_ttm > 0) dvr
          ON s.id = dvr.stock_id AND dvr.rn = 1

    -- Latest financial data
    JOIN (SELECT stock_id, net_income, revenue, shares_diluted,
                 ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
          FROM income_statements WHERE period_type = 'TTM') i
          ON s.id = i.stock_id AND i.rn = 1

    JOIN (SELECT stock_id, total_equity,
                 ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
          FROM balance_sheets WHERE period_type = 'TTM') b
          ON s.id = b.stock_id AND b.rn = 1

    WHERE dvr.market_cap > 200000000  -- $200M minimum
      AND dvr.ps_ratio_ttm > 0
      AND i.net_income IS NOT NULL
      AND b.total_equity > 0
)
SELECT *
FROM base_data;

-- O'Shaughnessy ranking view using available metrics
CREATE VIEW oshaughnessy_ranking_simple AS
WITH ranked_data AS (
    SELECT *,
        -- Rank each metric (lower rank = better value)
        RANK() OVER (ORDER BY pe_ratio_calculated ASC) as pe_rank,
        RANK() OVER (ORDER BY pb_ratio ASC) as pb_rank,
        RANK() OVER (ORDER BY ps_ratio ASC) as ps_rank,
        RANK() OVER (ORDER BY evs_ratio ASC) as evs_rank,
        RANK() OVER (ORDER BY dividend_yield DESC) as dividend_rank,
        COUNT(*) OVER () as total_stocks
    FROM oshaughnessy_value_composite_simple
    WHERE pe_ratio_calculated IS NOT NULL
      AND pb_ratio IS NOT NULL
      AND ps_ratio IS NOT NULL
      AND pe_ratio_calculated > 0
      AND pb_ratio > 0
)
SELECT *,
    -- Composite score (average of available ranks - 5 metrics instead of 6)
    (pe_rank + pb_rank + ps_ratio + evs_rank + dividend_rank) / 5.0 as composite_score,

    -- Percentile ranking
    ROUND(((pe_rank + pb_rank + ps_ratio + evs_rank + dividend_rank) / 5.0 / total_stocks) * 100, 1) as composite_percentile,

    -- Overall ranking
    RANK() OVER (ORDER BY (pe_rank + pb_rank + ps_ratio + evs_rank + dividend_rank) / 5.0 ASC) as overall_rank,

    -- Screening result (top 20% by default)
    CASE
        WHEN (pe_rank + pb_rank + ps_ratio + evs_rank + dividend_rank) / 5.0 <= (total_stocks * 0.20)
             AND data_completeness_score >= 75 THEN 1
        ELSE 0
    END as passes_screening

FROM ranked_data
ORDER BY (pe_rank + pb_rank + ps_ratio + evs_rank + dividend_rank) / 5.0 ASC;