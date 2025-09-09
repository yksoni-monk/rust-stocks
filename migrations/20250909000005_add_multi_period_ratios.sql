-- Add Multi-Period Valuation Ratios System (P/S & EV/S)
-- This migration is ADDITIVE only - no data destruction
-- Adds support for TTM, Annual, and Quarterly financial data and ratios

-- Multi-period income statements table
CREATE TABLE IF NOT EXISTS income_statements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    period_type TEXT NOT NULL, -- 'TTM', 'Annual', 'Quarterly'
    report_date DATE NOT NULL,
    fiscal_year INTEGER,
    fiscal_period TEXT, -- NULL for TTM/Annual, 'Q1'-'Q4' for quarterly
    
    -- Core income metrics
    revenue REAL,
    gross_profit REAL,
    operating_income REAL,
    net_income REAL,
    shares_basic REAL,
    shares_diluted REAL,
    
    -- Import metadata
    currency TEXT DEFAULT 'USD',
    simfin_id INTEGER,
    publish_date DATE,
    data_source TEXT DEFAULT 'simfin',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (stock_id) REFERENCES stocks (id),
    UNIQUE(stock_id, period_type, report_date)
);

-- Multi-period balance sheets table
CREATE TABLE IF NOT EXISTS balance_sheets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    period_type TEXT NOT NULL, -- 'TTM', 'Annual', 'Quarterly'
    report_date DATE NOT NULL,
    fiscal_year INTEGER,
    fiscal_period TEXT,
    
    -- Enterprise value components
    cash_and_equivalents REAL,
    short_term_debt REAL,
    long_term_debt REAL,
    total_debt REAL, -- Calculated: short_term + long_term
    
    -- Additional metrics
    total_assets REAL,
    total_liabilities REAL,
    total_equity REAL,
    shares_outstanding REAL,
    
    -- Import metadata
    currency TEXT DEFAULT 'USD',
    simfin_id INTEGER,
    data_source TEXT DEFAULT 'simfin',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (stock_id) REFERENCES stocks (id),
    UNIQUE(stock_id, period_type, report_date)
);

-- Enhanced daily ratios table for multi-period analysis
CREATE TABLE IF NOT EXISTS daily_valuation_ratios (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    date DATE NOT NULL,
    price REAL,
    
    -- Market metrics
    market_cap REAL, -- Stock Price × Shares Outstanding
    enterprise_value REAL, -- Market Cap + Total Debt - Cash
    
    -- Existing ratios (preserved)
    pe_ratio REAL,
    
    -- New multi-period ratios
    ps_ratio_ttm REAL,    -- PRIMARY: Standard P/S using TTM revenue
    ps_ratio_annual REAL, -- Annual P/S for trend analysis
    ps_ratio_quarterly REAL, -- Latest quarter P/S for momentum
    
    evs_ratio_ttm REAL,    -- PRIMARY: Standard EV/S using TTM revenue
    evs_ratio_annual REAL, -- Annual EV/S for trend analysis
    evs_ratio_quarterly REAL, -- Latest quarter EV/S for momentum
    
    -- Supporting data
    revenue_ttm REAL,      -- TTM revenue for calculations
    revenue_annual REAL,   -- Annual revenue
    revenue_quarterly REAL, -- Latest quarterly revenue
    
    -- Data quality tracking
    data_completeness_score INTEGER, -- 0-100 based on available ratios
    last_financial_update DATE,      -- Most recent financial data used
    
    FOREIGN KEY (stock_id) REFERENCES stocks (id),
    UNIQUE(stock_id, date)
);

-- Performance indexes for multi-period analysis
CREATE INDEX IF NOT EXISTS idx_income_statements_period_lookup 
ON income_statements(stock_id, period_type, report_date);

CREATE INDEX IF NOT EXISTS idx_balance_sheets_period_lookup 
ON balance_sheets(stock_id, period_type, report_date);

CREATE INDEX IF NOT EXISTS idx_daily_ratios_ps_ttm 
ON daily_valuation_ratios(ps_ratio_ttm);

CREATE INDEX IF NOT EXISTS idx_daily_ratios_evs_ttm 
ON daily_valuation_ratios(evs_ratio_ttm);

CREATE INDEX IF NOT EXISTS idx_daily_ratios_multi_period 
ON daily_valuation_ratios(stock_id, date, ps_ratio_ttm, evs_ratio_ttm);

-- Period type constraints to ensure data quality
CREATE INDEX IF NOT EXISTS idx_income_statements_period_type 
ON income_statements(period_type);

CREATE INDEX IF NOT EXISTS idx_balance_sheets_period_type 
ON balance_sheets(period_type);

-- Metadata tracking for migration
INSERT OR IGNORE INTO metadata (key, value) 
VALUES ('multi_period_ratios_schema_version', '1.0');

INSERT OR IGNORE INTO metadata (key, value) 
VALUES ('multi_period_ratios_created', datetime('now'));

-- Migrate existing P/E ratios to new table (preserve existing data)
INSERT OR IGNORE INTO daily_valuation_ratios (stock_id, date, price, pe_ratio)
SELECT stock_id, date, close_price, pe_ratio 
FROM daily_prices 
WHERE pe_ratio IS NOT NULL;