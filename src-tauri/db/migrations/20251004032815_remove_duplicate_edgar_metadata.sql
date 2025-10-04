-- Migration: Remove duplicate EDGAR metadata columns
-- Risk: MEDIUM (recreating tables)
-- Impact: Eliminates data duplication, reduces storage
-- Dependencies: Phase 1 complete (sec_filings table exists)

-- Step 1: Create backup tables (safety measure)
CREATE TABLE income_statements_backup AS SELECT * FROM income_statements;
CREATE TABLE balance_sheets_backup AS SELECT * FROM balance_sheets;
CREATE TABLE cash_flow_statements_backup AS SELECT * FROM cash_flow_statements;

-- Step 2: Recreate income_statements without duplicate EDGAR metadata columns
CREATE TABLE income_statements_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    sec_filing_id INTEGER,
    period_type TEXT NOT NULL,
    report_date DATE NOT NULL,
    fiscal_year INTEGER,
    
    -- Financial data only (no duplicate EDGAR metadata)
    revenue REAL,
    gross_profit REAL,
    operating_income REAL,
    net_income REAL,
    shares_basic REAL,
    shares_diluted REAL,
    cost_of_revenue REAL,
    research_development REAL,
    selling_general_admin REAL,
    depreciation_expense REAL,
    amortization_expense REAL,
    interest_expense REAL,
    publish_date DATE,
    
    -- Import metadata (keep minimal)
    currency TEXT DEFAULT 'USD',
    data_source TEXT DEFAULT 'sec_edgar',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    FOREIGN KEY (sec_filing_id) REFERENCES sec_filings(id),
    UNIQUE(stock_id, period_type, report_date)
);

-- Step 3: Copy data from old income_statements table (excluding duplicate columns)
INSERT INTO income_statements_new 
SELECT 
    id, stock_id, sec_filing_id, period_type, report_date, fiscal_year,
    revenue, gross_profit, operating_income, net_income, shares_basic, shares_diluted,
    cost_of_revenue, research_development, selling_general_admin, depreciation_expense,
    amortization_expense, interest_expense, publish_date, currency, data_source, created_at
FROM income_statements;

-- Step 4: Recreate balance_sheets without duplicate EDGAR metadata columns
CREATE TABLE balance_sheets_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    sec_filing_id INTEGER,
    period_type TEXT NOT NULL,
    report_date DATE NOT NULL,
    fiscal_year INTEGER,
    
    -- Financial data only (no duplicate EDGAR metadata)
    cash_and_equivalents REAL,
    short_term_debt REAL,
    long_term_debt REAL,
    total_debt REAL,
    total_assets REAL,
    total_liabilities REAL,
    total_equity REAL,
    shares_outstanding REAL,
    current_assets REAL,
    current_liabilities REAL,
    inventory REAL,
    accounts_receivable REAL,
    accounts_payable REAL,
    working_capital REAL,
    share_repurchases REAL,
    
    -- Import metadata (keep minimal)
    currency TEXT DEFAULT 'USD',
    data_source TEXT DEFAULT 'sec_edgar',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    FOREIGN KEY (sec_filing_id) REFERENCES sec_filings(id),
    UNIQUE(stock_id, period_type, report_date)
);

-- Step 5: Copy data from old balance_sheets table (excluding duplicate columns)
INSERT INTO balance_sheets_new 
SELECT 
    id, stock_id, sec_filing_id, period_type, report_date, fiscal_year,
    cash_and_equivalents, short_term_debt, long_term_debt, total_debt, total_assets,
    total_liabilities, total_equity, shares_outstanding, current_assets, current_liabilities,
    inventory, accounts_receivable, accounts_payable, working_capital, share_repurchases,
    currency, data_source, created_at
FROM balance_sheets;

-- Step 6: Recreate cash_flow_statements without duplicate EDGAR metadata columns
CREATE TABLE cash_flow_statements_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    sec_filing_id INTEGER,
    period_type TEXT NOT NULL,
    report_date DATE NOT NULL,
    fiscal_year INTEGER,
    
    -- Financial data only (no duplicate EDGAR metadata)
    operating_cash_flow REAL,
    depreciation_amortization REAL,
    depreciation_expense REAL,
    amortization_expense REAL,
    investing_cash_flow REAL,
    capital_expenditures REAL,
    financing_cash_flow REAL,
    dividends_paid REAL,
    share_repurchases REAL,
    net_cash_flow REAL,
    
    -- Import metadata (keep minimal)
    currency TEXT DEFAULT 'USD',
    data_source TEXT DEFAULT 'sec_edgar',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    FOREIGN KEY (sec_filing_id) REFERENCES sec_filings(id),
    UNIQUE(stock_id, period_type, report_date)
);

-- Step 7: Copy data from old cash_flow_statements table (excluding duplicate columns)
INSERT INTO cash_flow_statements_new 
SELECT 
    id, stock_id, sec_filing_id, period_type, report_date, fiscal_year,
    operating_cash_flow, depreciation_amortization, depreciation_expense, amortization_expense,
    investing_cash_flow, capital_expenditures, financing_cash_flow, dividends_paid,
    share_repurchases, net_cash_flow, currency, data_source, created_at
FROM cash_flow_statements;

-- Step 8: Create indexes for new tables
CREATE INDEX idx_income_statements_new_stock_id ON income_statements_new(stock_id);
CREATE INDEX idx_income_statements_new_sec_filing_id ON income_statements_new(sec_filing_id);
CREATE INDEX idx_income_statements_new_period_type ON income_statements_new(period_type);
CREATE INDEX idx_income_statements_new_report_date ON income_statements_new(report_date);

CREATE INDEX idx_balance_sheets_new_stock_id ON balance_sheets_new(stock_id);
CREATE INDEX idx_balance_sheets_new_sec_filing_id ON balance_sheets_new(sec_filing_id);
CREATE INDEX idx_balance_sheets_new_period_type ON balance_sheets_new(period_type);
CREATE INDEX idx_balance_sheets_new_report_date ON balance_sheets_new(report_date);

CREATE INDEX idx_cash_flow_statements_new_stock_id ON cash_flow_statements_new(stock_id);
CREATE INDEX idx_cash_flow_statements_new_sec_filing_id ON cash_flow_statements_new(sec_filing_id);
CREATE INDEX idx_cash_flow_statements_new_period_type ON cash_flow_statements_new(period_type);
CREATE INDEX idx_cash_flow_statements_new_report_date ON cash_flow_statements_new(report_date);

-- Step 9: Drop old tables and rename new ones
DROP TABLE income_statements;
DROP TABLE balance_sheets;
DROP TABLE cash_flow_statements;

ALTER TABLE income_statements_new RENAME TO income_statements;
ALTER TABLE balance_sheets_new RENAME TO balance_sheets;
ALTER TABLE cash_flow_statements_new RENAME TO cash_flow_statements;

-- Step 10: Verify data integrity (will be done in testing phase)
-- This migration removes duplicate EDGAR metadata columns:
-- - accession_number (now in sec_filings)
-- - form_type (now in sec_filings)  
-- - filed_date (now in sec_filings)
-- - fiscal_period (now in sec_filings)
-- - edgar_accession (duplicate of accession_number)
-- - edgar_form (duplicate of form_type)
-- - edgar_filed_date (duplicate of filed_date)