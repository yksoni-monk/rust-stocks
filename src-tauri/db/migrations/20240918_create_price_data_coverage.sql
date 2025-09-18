-- Migration: Create price_data_coverage table for tracking data gaps and completeness
-- Created: 2024-09-18
-- Purpose: Track price data coverage periods to enable efficient incremental updates

CREATE TABLE price_data_coverage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    symbol TEXT NOT NULL,

    -- Coverage period
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    total_trading_days INTEGER NOT NULL,
    actual_data_points INTEGER NOT NULL,

    -- Coverage metrics
    coverage_percentage REAL GENERATED ALWAYS AS (
        CAST(actual_data_points AS REAL) / CAST(total_trading_days AS REAL) * 100
    ) STORED,

    -- Data quality
    has_gaps BOOLEAN DEFAULT FALSE,
    gap_details TEXT,  -- JSON array of gap periods

    -- Metadata
    last_updated DATETIME DEFAULT CURRENT_TIMESTAMP,
    data_source TEXT DEFAULT 'schwab',

    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, period_start, period_end)
);

-- Indexes for efficient querying
CREATE INDEX idx_price_coverage_stock_id ON price_data_coverage(stock_id);
CREATE INDEX idx_price_coverage_symbol ON price_data_coverage(symbol);
CREATE INDEX idx_price_coverage_period ON price_data_coverage(period_start, period_end);
CREATE INDEX idx_price_coverage_gaps ON price_data_coverage(has_gaps);
CREATE INDEX idx_price_coverage_updated ON price_data_coverage(last_updated);

-- Create view for easy coverage analysis
CREATE VIEW v_stock_data_coverage AS
SELECT
    s.symbol,
    s.company_name,
    cm.ipo_date,
    cm.listing_date,
    cm.earliest_data_date,
    cm.latest_data_date,
    cm.total_trading_days as metadata_trading_days,

    -- Coverage summary from price_data_coverage
    COALESCE(pdc.total_periods, 0) as coverage_periods,
    COALESCE(pdc.total_data_points, 0) as total_data_points,
    COALESCE(pdc.avg_coverage, 0) as average_coverage_percentage,
    COALESCE(pdc.periods_with_gaps, 0) as periods_with_gaps,

    -- Data quality flags
    CASE
        WHEN cm.earliest_data_date IS NULL THEN 'No Data'
        WHEN pdc.avg_coverage >= 95 THEN 'Excellent'
        WHEN pdc.avg_coverage >= 85 THEN 'Good'
        WHEN pdc.avg_coverage >= 70 THEN 'Fair'
        ELSE 'Poor'
    END as data_quality_rating

FROM stocks s
LEFT JOIN company_metadata cm ON s.id = cm.stock_id
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