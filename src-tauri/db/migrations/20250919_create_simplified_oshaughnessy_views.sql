-- Simplified O'Shaughnessy Value Composite using only reliable data
-- Migration: 20250919_create_simplified_oshaughnessy_views.sql

-- Create simplified view using only P/S ratios and market cap (data we know works)
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
  AND dvr.market_cap > 200000000  -- $200M minimum
  AND dvr.ps_ratio_ttm > 0;

-- Create ranking view
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