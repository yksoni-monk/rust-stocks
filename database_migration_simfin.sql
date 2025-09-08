-- SimFin Database Migration Script
-- Enhances existing schema for SimFin CSV data import
-- Run this before importing SimFin data

-- Enable WAL mode for better concurrent access
PRAGMA journal_mode=WAL;

-- Enhance stocks table for SimFin data (only add missing columns)
ALTER TABLE stocks ADD COLUMN simfin_id INTEGER;

-- Create quarterly_financials table for comprehensive financial data
CREATE TABLE IF NOT EXISTS quarterly_financials (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER NOT NULL,
    simfin_id INTEGER NOT NULL,
    currency TEXT NOT NULL DEFAULT 'USD',
    fiscal_year INTEGER NOT NULL,
    fiscal_period TEXT NOT NULL, -- Q1, Q2, Q3, Q4
    report_date DATE NOT NULL,
    publish_date DATE,
    restated_date DATE,
    
    -- Share Information
    shares_basic INTEGER,
    shares_diluted INTEGER,
    
    -- Income Statement Core Metrics
    revenue REAL,
    cost_of_revenue REAL,
    gross_profit REAL,
    operating_expenses REAL,
    selling_general_admin REAL,
    research_development REAL,
    depreciation_amortization REAL,
    operating_income REAL,
    non_operating_income REAL,
    interest_expense_net REAL,
    pretax_income_adj REAL,
    pretax_income REAL,
    income_tax_expense REAL,
    income_continuing_ops REAL,
    net_extraordinary_gains REAL,
    net_income REAL,
    net_income_common REAL,
    
    -- Calculated Metrics (will be computed after import)
    eps_basic REAL,
    eps_diluted REAL,
    eps_calculated REAL, -- Net Income / Diluted Shares Outstanding
    eps_calculation_date DATETIME,
    
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, fiscal_year, fiscal_period)
);

-- Create indexes for performance (after import completion)
-- These will be created by the import tool for optimal performance

-- Verification queries to check migration success
-- Run these after migration to verify schema

-- Check stocks table enhancement
-- SELECT name FROM pragma_table_info('stocks') WHERE name IN ('simfin_id', 'sector', 'industry');

-- Check daily_prices enhancement  
-- SELECT name FROM pragma_table_info('daily_prices') WHERE name IN ('data_source', 'last_updated');

-- Check quarterly_financials table creation
-- SELECT name FROM sqlite_master WHERE type='table' AND name='quarterly_financials';

-- Count existing data before import
-- SELECT 'stocks' as table_name, COUNT(*) as record_count FROM stocks
-- UNION ALL
-- SELECT 'daily_prices' as table_name, COUNT(*) as record_count FROM daily_prices;