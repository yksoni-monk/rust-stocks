-- Migration: Create company_metadata table for IPO/listing dates and metadata tracking
-- Created: 2024-09-18
-- Purpose: Store company IPO dates, listing dates, and data coverage metadata

CREATE TABLE company_metadata (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL UNIQUE,
    symbol TEXT NOT NULL,
    company_name TEXT NOT NULL,

    -- Key dates for data range optimization
    ipo_date DATE,                    -- Initial public offering date
    listing_date DATE,                -- First trading date (may differ from IPO)
    delisting_date DATE,              -- If delisted (for historical symbols)
    spinoff_date DATE,                -- If spun off from parent company
    spinoff_parent TEXT,              -- Parent company symbol if spinoff

    -- Data coverage tracking
    earliest_data_date DATE,          -- Earliest available price data
    latest_data_date DATE,            -- Latest available price data
    total_trading_days INTEGER,       -- Total trading days with data

    -- Metadata
    exchange TEXT,                    -- Primary exchange (NYSE, NASDAQ, etc.)
    sector TEXT,                     -- Business sector
    market_cap_category TEXT,        -- Large/Mid/Small cap
    data_source TEXT DEFAULT 'schwab', -- Primary data source

    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (stock_id) REFERENCES stocks(id)
);

-- Indexes for performance
CREATE INDEX idx_company_metadata_symbol ON company_metadata(symbol);
CREATE INDEX idx_company_metadata_stock_id ON company_metadata(stock_id);
CREATE INDEX idx_company_metadata_ipo_date ON company_metadata(ipo_date);
CREATE INDEX idx_company_metadata_listing_date ON company_metadata(listing_date);
CREATE INDEX idx_company_metadata_updated_at ON company_metadata(updated_at);

-- Insert known IPO/listing dates based on our analysis
INSERT INTO company_metadata (stock_id, symbol, company_name, ipo_date, listing_date, spinoff_date, spinoff_parent)
SELECT s.id, s.symbol, s.company_name,
    CASE s.symbol
        -- Recent IPOs
        WHEN 'COIN' THEN '2021-04-14'
        WHEN 'PLTR' THEN '2020-09-30'
        WHEN 'UBER' THEN '2019-05-10'

        -- Spinoffs and re-listings
        WHEN 'KVUE' THEN '2023-05-04'
        WHEN 'CEG' THEN '2022-02-02'
        WHEN 'CARR' THEN '2020-04-03'
        WHEN 'OTIS' THEN '2020-04-03'
        WHEN 'GEV' THEN '2024-04-02'
        WHEN 'GEHC' THEN '2023-01-04'
        WHEN 'SOLV' THEN '2024-04-01'
        WHEN 'VLTO' THEN '2023-10-02'
        WHEN 'TKO' THEN '2023-09-12'
        WHEN 'DELL' THEN '2018-12-28'

        -- Conservative estimate for established companies
        ELSE '2015-01-01'
    END as ipo_date,

    CASE s.symbol
        -- Recent IPOs (same as IPO date)
        WHEN 'COIN' THEN '2021-04-14'
        WHEN 'PLTR' THEN '2020-09-30'
        WHEN 'UBER' THEN '2019-05-10'

        -- Spinoffs (listing date = spinoff date)
        WHEN 'KVUE' THEN '2023-05-04'
        WHEN 'CEG' THEN '2022-02-02'
        WHEN 'CARR' THEN '2020-04-03'
        WHEN 'OTIS' THEN '2020-04-03'
        WHEN 'GEV' THEN '2024-04-02'
        WHEN 'GEHC' THEN '2023-01-04'
        WHEN 'SOLV' THEN '2024-04-01'
        WHEN 'VLTO' THEN '2023-10-02'
        WHEN 'TKO' THEN '2023-09-12'
        WHEN 'DELL' THEN '2018-12-28'

        -- Use our data start date for established companies
        ELSE '2015-01-01'
    END as listing_date,

    CASE s.symbol
        -- Spinoffs
        WHEN 'KVUE' THEN '2023-05-04'
        WHEN 'CEG' THEN '2022-02-02'
        WHEN 'CARR' THEN '2020-04-03'
        WHEN 'OTIS' THEN '2020-04-03'
        WHEN 'GEV' THEN '2024-04-02'
        WHEN 'GEHC' THEN '2023-01-04'
        WHEN 'SOLV' THEN '2024-04-01'
        WHEN 'VLTO' THEN '2023-10-02'
        ELSE NULL
    END as spinoff_date,

    CASE s.symbol
        -- Spinoff parents
        WHEN 'KVUE' THEN 'JNJ'
        WHEN 'CEG' THEN 'EXC'
        WHEN 'CARR' THEN 'UTC'
        WHEN 'OTIS' THEN 'UTC'
        WHEN 'GEV' THEN 'GE'
        WHEN 'GEHC' THEN 'GE'
        WHEN 'SOLV' THEN 'MMM'
        WHEN 'VLTO' THEN 'DHR'
        ELSE NULL
    END as spinoff_parent

FROM stocks s
WHERE s.is_sp500 = 1;

-- Update data coverage information based on existing daily_prices data
UPDATE company_metadata
SET
    earliest_data_date = (
        SELECT MIN(dp.date)
        FROM daily_prices dp
        WHERE dp.stock_id = company_metadata.stock_id
    ),
    latest_data_date = (
        SELECT MAX(dp.date)
        FROM daily_prices dp
        WHERE dp.stock_id = company_metadata.stock_id
    ),
    total_trading_days = (
        SELECT COUNT(*)
        FROM daily_prices dp
        WHERE dp.stock_id = company_metadata.stock_id
    );