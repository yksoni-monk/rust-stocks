-- Migration: Fix v_price_data_coverage view to remove company_metadata reference
-- Risk: ZERO RISK (fixing broken view)
-- Impact: Fixes view that references dropped table

-- Drop the broken view
DROP VIEW IF EXISTS v_price_data_coverage;

-- Recreate the view without company_metadata reference
CREATE VIEW v_price_data_coverage AS
SELECT 
    s.id as stock_id,
    s.symbol,
    s.company_name,
    s.is_sp500,
    s.sector,
    
    -- Price data coverage (computed from daily_prices)
    MIN(dp.date) as earliest_data_date,
    MAX(dp.date) as latest_data_date,
    COUNT(dp.date) as total_trading_days,
    COUNT(DISTINCT dp.date) as unique_trading_days,
    
    -- Additional useful metrics
    AVG(dp.close_price) as avg_close_price,
    MAX(dp.close_price) as highest_close_price,
    MIN(dp.close_price) as lowest_close_price,
    SUM(dp.volume) as total_volume,
    
    -- Data freshness
    MAX(dp.last_updated) as last_price_update,
    
    -- Coverage status
    CASE 
        WHEN COUNT(dp.date) = 0 THEN 'No Data'
        WHEN MAX(dp.date) >= DATE('now', '-7 days') THEN 'Current'
        WHEN MAX(dp.date) >= DATE('now', '-30 days') THEN 'Recent'
        ELSE 'Stale'
    END as coverage_status
    
FROM stocks s
LEFT JOIN daily_prices dp ON s.id = dp.stock_id
GROUP BY s.id, s.symbol, s.company_name, s.is_sp500, s.sector;
