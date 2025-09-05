-- Enhanced schema migration for comprehensive stock data
-- Migration: 20250905000001_enhanced_schema.sql
-- Adds comprehensive fundamental metrics and real-time data support

-- Enable foreign keys
PRAGMA foreign_keys = ON;

-- Enhanced stocks table with comprehensive company data
CREATE TABLE IF NOT EXISTS stocks_enhanced (
    id INTEGER PRIMARY KEY,
    symbol TEXT UNIQUE NOT NULL,
    company_name TEXT,
    exchange TEXT,
    sector TEXT,
    industry TEXT,
    market_cap REAL,
    description TEXT,
    employees INTEGER,
    founded_year INTEGER,
    headquarters TEXT,
    website TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Enhanced daily prices with comprehensive fundamental metrics
CREATE TABLE IF NOT EXISTS daily_prices_enhanced (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER,
    date TEXT,
    open_price REAL,
    high_price REAL,
    low_price REAL,
    close_price REAL,
    adjusted_close REAL,
    volume INTEGER,
    average_volume INTEGER,
    
    -- Fundamental ratios
    pe_ratio REAL,
    pe_ratio_forward REAL,
    pb_ratio REAL,
    ps_ratio REAL,
    dividend_yield REAL,
    dividend_per_share REAL,
    eps REAL,
    eps_forward REAL,
    beta REAL,
    
    -- 52-week data
    week_52_high REAL,
    week_52_low REAL,
    week_52_change_percent REAL,
    
    -- Market metrics
    shares_outstanding REAL,
    float_shares REAL,
    revenue_ttm REAL,
    profit_margin REAL,
    operating_margin REAL,
    return_on_equity REAL,
    return_on_assets REAL,
    debt_to_equity REAL,
    
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (stock_id) REFERENCES stocks_enhanced (id),
    UNIQUE(stock_id, date)
);

-- Separate real-time quotes table for live data
CREATE TABLE IF NOT EXISTS real_time_quotes (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER,
    timestamp TIMESTAMP,
    bid_price REAL,
    bid_size INTEGER,
    ask_price REAL,
    ask_size INTEGER,
    last_price REAL,
    last_size INTEGER,
    volume INTEGER,
    change_amount REAL,
    change_percent REAL,
    day_high REAL,
    day_low REAL,
    FOREIGN KEY (stock_id) REFERENCES stocks_enhanced (id)
);

-- Intraday price data for detailed analysis
CREATE TABLE IF NOT EXISTS intraday_prices (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER,
    datetime TIMESTAMP,
    interval_type TEXT CHECK (interval_type IN ('1min', '5min', '15min', '30min', '1hour')),
    open_price REAL,
    high_price REAL,
    low_price REAL,
    close_price REAL,
    volume INTEGER,
    FOREIGN KEY (stock_id) REFERENCES stocks_enhanced (id)
);

-- Option chains data
CREATE TABLE IF NOT EXISTS option_chains (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER,
    expiration_date DATE,
    strike_price REAL,
    option_type TEXT CHECK (option_type IN ('CALL', 'PUT')),
    bid REAL,
    ask REAL,
    last_price REAL,
    volume INTEGER,
    open_interest INTEGER,
    implied_volatility REAL,
    delta REAL,
    gamma REAL,
    theta REAL,
    vega REAL,
    rho REAL,
    FOREIGN KEY (stock_id) REFERENCES stocks_enhanced (id)
);

-- Performance indexes for fast queries
CREATE INDEX IF NOT EXISTS idx_stocks_enhanced_symbol ON stocks_enhanced(symbol);
CREATE INDEX IF NOT EXISTS idx_stocks_enhanced_company_name ON stocks_enhanced(company_name);
CREATE INDEX IF NOT EXISTS idx_stocks_enhanced_sector ON stocks_enhanced(sector);
CREATE INDEX IF NOT EXISTS idx_daily_prices_enhanced_stock_date ON daily_prices_enhanced(stock_id, date);
CREATE INDEX IF NOT EXISTS idx_daily_prices_enhanced_date ON daily_prices_enhanced(date);
CREATE INDEX IF NOT EXISTS idx_real_time_quotes_stock_timestamp ON real_time_quotes(stock_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_intraday_prices_stock_datetime ON intraday_prices(stock_id, datetime);
CREATE INDEX IF NOT EXISTS idx_intraday_prices_interval ON intraday_prices(interval_type);
CREATE INDEX IF NOT EXISTS idx_option_chains_stock_expiration ON option_chains(stock_id, expiration_date);
CREATE INDEX IF NOT EXISTS idx_option_chains_type_strike ON option_chains(option_type, strike_price);

-- Update metadata for enhanced schema
INSERT OR REPLACE INTO metadata (key, value) VALUES 
    ('enhanced_schema_version', '2.0'),
    ('enhanced_schema_applied', CURRENT_TIMESTAMP),
    ('migration_20250905000001', 'completed');