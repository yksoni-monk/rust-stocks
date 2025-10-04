-- Migration: Create v_market_cap_calculated SQL view
-- Risk: ZERO RISK (creating view only)
-- Impact: Provides real-time market cap calculations

-- Create market cap calculation view
CREATE VIEW v_market_cap_calculated AS
SELECT 
    s.id as stock_id,
    s.symbol,
    s.company_name,
    s.sector,
    s.is_sp500,
    
    -- Latest price data
    dp.close_price,
    dp.date as price_date,
    
    -- Latest shares outstanding
    bs.shares_outstanding,
    bs.report_date as shares_date,
    
    -- Calculated market cap
    CASE 
        WHEN dp.close_price IS NOT NULL AND bs.shares_outstanding IS NOT NULL 
        THEN dp.close_price * bs.shares_outstanding
        ELSE NULL
    END as market_cap,
    
    -- Market cap category
    CASE 
        WHEN dp.close_price IS NULL OR bs.shares_outstanding IS NULL THEN 'Unknown'
        WHEN dp.close_price * bs.shares_outstanding >= 200000000000 THEN 'Mega Cap'  -- $200B+
        WHEN dp.close_price * bs.shares_outstanding >= 10000000000 THEN 'Large Cap'   -- $10B+
        WHEN dp.close_price * bs.shares_outstanding >= 2000000000 THEN 'Mid Cap'      -- $2B+
        WHEN dp.close_price * bs.shares_outstanding >= 300000000 THEN 'Small Cap'     -- $300M+
        WHEN dp.close_price * bs.shares_outstanding >= 50000000 THEN 'Micro Cap'       -- $50M+
        ELSE 'Nano Cap'                                                               -- <$50M
    END as market_cap_category,
    
    -- Data freshness
    CASE 
        WHEN dp.date IS NULL THEN 'No Price Data'
        WHEN dp.date >= DATE('now', '-7 days') THEN 'Current'
        WHEN dp.date >= DATE('now', '-30 days') THEN 'Recent'
        ELSE 'Stale'
    END as price_freshness,
    
    CASE 
        WHEN bs.report_date IS NULL THEN 'No Shares Data'
        WHEN bs.report_date >= DATE('now', '-90 days') THEN 'Current'
        WHEN bs.report_date >= DATE('now', '-365 days') THEN 'Recent'
        ELSE 'Stale'
    END as shares_freshness

FROM stocks s
LEFT JOIN (
    -- Get latest price for each stock
    SELECT 
        stock_id,
        close_price,
        date,
        ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY date DESC) as rn
    FROM daily_prices
    WHERE close_price IS NOT NULL
) dp ON s.id = dp.stock_id AND dp.rn = 1
LEFT JOIN (
    -- Get latest shares outstanding for each stock
    SELECT 
        stock_id,
        shares_outstanding,
        report_date,
        ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
    FROM balance_sheets
    WHERE shares_outstanding IS NOT NULL
) bs ON s.id = bs.stock_id AND bs.rn = 1;