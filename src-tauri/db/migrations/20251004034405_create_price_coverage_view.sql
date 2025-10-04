-- Migration: Create v_price_data_coverage SQL view
-- Risk: ZERO RISK (creating view only)
-- Impact: HIGH IMPACT (eliminates redundant computed data)

-- Create comprehensive price data coverage view
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
    END as coverage_status,
    
    -- Company lifecycle info (from company_metadata)
    cm.ipo_date,
    cm.listing_date,
    cm.delisting_date,
    cm.spinoff_date,
    cm.spinoff_parent,
    cm.exchange,
    cm.market_cap_category,
    cm.data_source,
    cm.created_at,
    cm.updated_at
    
FROM stocks s
LEFT JOIN daily_prices dp ON s.id = dp.stock_id
LEFT JOIN company_metadata cm ON s.id = cm.stock_id
GROUP BY s.id, s.symbol, s.company_name, s.is_sp500, s.sector,
         cm.ipo_date, cm.listing_date, cm.delisting_date, cm.spinoff_date,
         cm.spinoff_parent, cm.exchange, cm.market_cap_category, cm.data_source,
         cm.created_at, cm.updated_at;

-- Create index for better performance
CREATE INDEX IF NOT EXISTS idx_daily_prices_stock_date ON daily_prices(stock_id, date);
