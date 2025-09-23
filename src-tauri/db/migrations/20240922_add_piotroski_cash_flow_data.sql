-- Migration: 20240922_add_piotroski_cash_flow_data.sql
-- Add cash flow statements table for complete Piotroski F-Score implementation

CREATE TABLE cash_flow_statements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    period_type TEXT NOT NULL, -- 'TTM', 'Annual', 'Quarterly'
    report_date DATE NOT NULL,
    fiscal_year INTEGER,
    fiscal_period TEXT,
    
    -- Cash Flow Metrics (Critical for Piotroski)
    operating_cash_flow REAL,        -- For "Positive Operating Cash Flow" criterion
    investing_cash_flow REAL,
    financing_cash_flow REAL,
    net_cash_flow REAL,
    
    -- Additional Cash Flow Details
    depreciation_expense REAL,       -- For EBITDA calculation
    amortization_expense REAL,
    stock_repurchases REAL,          -- For share dilution analysis
    stock_issuance REAL,            -- For share dilution analysis
    
    -- Import metadata
    currency TEXT DEFAULT 'USD',
    data_source TEXT DEFAULT 'edgar',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, period_type, report_date)
);

-- Add indexes for performance
CREATE INDEX idx_cash_flow_stock_period ON cash_flow_statements(stock_id, period_type);
CREATE INDEX idx_cash_flow_report_date ON cash_flow_statements(report_date);
CREATE INDEX idx_cash_flow_operating ON cash_flow_statements(operating_cash_flow);

-- Add missing balance sheet fields for Piotroski criteria
ALTER TABLE balance_sheets ADD COLUMN current_assets REAL;
ALTER TABLE balance_sheets ADD COLUMN current_liabilities REAL;
ALTER TABLE balance_sheets ADD COLUMN short_term_debt REAL;
ALTER TABLE balance_sheets ADD COLUMN long_term_debt REAL;

-- Add missing income statement fields for Piotroski criteria
ALTER TABLE income_statements ADD COLUMN cost_of_revenue REAL;  -- For gross margin calculation
ALTER TABLE income_statements ADD COLUMN interest_expense REAL; -- For interest coverage
ALTER TABLE income_statements ADD COLUMN depreciation_expense REAL;
ALTER TABLE income_statements ADD COLUMN amortization_expense REAL;

-- Add indexes for new fields
CREATE INDEX idx_balance_current_assets ON balance_sheets(current_assets);
CREATE INDEX idx_balance_current_liabilities ON balance_sheets(current_liabilities);
CREATE INDEX idx_income_cost_of_revenue ON income_statements(cost_of_revenue);
CREATE INDEX idx_income_interest_expense ON income_statements(interest_expense);
