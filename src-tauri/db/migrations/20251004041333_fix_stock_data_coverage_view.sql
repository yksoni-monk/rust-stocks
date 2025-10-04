-- Migration: Fix v_stock_data_coverage view to use v_price_data_coverage
-- Risk: LOW RISK (fixing broken view)
-- Impact: Fixes view that references removed company_metadata table

-- Drop the broken view
DROP VIEW IF EXISTS v_stock_data_coverage;

-- Recreate the view using v_price_data_coverage instead of company_metadata
CREATE VIEW v_stock_data_coverage AS
SELECT
    s.symbol,
    s.company_name,
    NULL as ipo_date,                    -- No longer available after removing company_metadata
    NULL as listing_date,                 -- No longer available after removing company_metadata
    vpdc.earliest_data_date,
    vpdc.latest_data_date,
    vpdc.total_trading_days as metadata_trading_days,

    -- Coverage summary from price_data_coverage (if it exists)
    COALESCE(pdc.total_periods, 0) as coverage_periods,
    COALESCE(pdc.total_data_points, 0) as total_data_points,
    COALESCE(pdc.avg_coverage, 0) as average_coverage_percentage,
    COALESCE(pdc.periods_with_gaps, 0) as periods_with_gaps,

    -- Data quality flags based on price coverage
    CASE
        WHEN vpdc.earliest_data_date IS NULL THEN 'No Data'
        WHEN vpdc.coverage_status = 'Current' THEN 'Excellent'
        WHEN vpdc.coverage_status = 'Recent' THEN 'Good'
        WHEN vpdc.coverage_status = 'Stale' THEN 'Fair'
        ELSE 'Poor'
    END as data_quality_rating

FROM stocks s
LEFT JOIN v_price_data_coverage vpdc ON s.id = vpdc.stock_id
LEFT JOIN (
    SELECT
        stock_id,
        COUNT(*) as total_periods,
        SUM(actual_data_points) as total_data_points,
        AVG(coverage_percentage) as avg_coverage,
        SUM(CASE WHEN has_gaps THEN 1 ELSE 0 END) as periods_with_gaps
    FROM price_data_coverage
    GROUP BY stock_id
) pdc ON s.id = pdc.stock_id
WHERE s.is_sp500 = 1
ORDER BY s.symbol;