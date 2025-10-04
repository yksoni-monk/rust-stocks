-- Migration Step 1: Create new tables with correct DATE types and copy data
-- Risk: LOW (creating new tables, copying data)
-- Impact: Fixes root cause of sqlx type inference issues

-- Step 1: Create new income_statements table with correct DATE types
CREATE TABLE income_statements_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    period_type TEXT NOT NULL,
    report_date DATE NOT NULL,  -- Fixed: NUM -> DATE
    fiscal_year INTEGER,
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
    currency TEXT,
    simfin_id INTEGER,
    publish_date NUM,
    data_source TEXT,
    created_at NUM,
    edgar_accession TEXT,
    edgar_form TEXT,
    edgar_filed_date DATE,  -- Fixed: NUM -> DATE
    accession_number TEXT,
    form_type TEXT,
    filed_date DATE,  -- Fixed: NUM -> DATE
    fiscal_period TEXT,
    sec_filing_id INTEGER,
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    FOREIGN KEY (sec_filing_id) REFERENCES sec_filings(id)
);

-- Copy data from old table
INSERT INTO income_statements_new SELECT * FROM income_statements;

-- Step 2: Create new balance_sheets table with correct DATE types
CREATE TABLE balance_sheets_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    period_type TEXT NOT NULL,
    report_date DATE NOT NULL,  -- Fixed: NUM -> DATE
    fiscal_year INTEGER,
    cash_and_equivalents REAL,
    short_term_debt REAL,
    long_term_debt REAL,
    total_debt REAL,
    total_assets REAL,
    total_liabilities REAL,
    total_equity REAL,
    shares_outstanding REAL,
    currency TEXT,
    simfin_id INTEGER,
    data_source TEXT,
    created_at NUM,
    current_assets REAL,
    current_liabilities REAL,
    inventory REAL,
    accounts_receivable REAL,
    accounts_payable REAL,
    working_capital REAL,
    edgar_accession TEXT,
    edgar_form TEXT,
    edgar_filed_date DATE,  -- Fixed: NUM -> DATE
    share_repurchases REAL,
    accession_number TEXT,
    form_type TEXT,
    filed_date DATE,  -- Fixed: NUM -> DATE
    fiscal_period TEXT,
    sec_filing_id INTEGER,
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    FOREIGN KEY (sec_filing_id) REFERENCES sec_filings(id)
);

-- Copy data from old table
INSERT INTO balance_sheets_new SELECT * FROM balance_sheets;

-- Step 3: Create new cash_flow_statements table with correct DATE types
CREATE TABLE cash_flow_statements_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    period_type TEXT NOT NULL,
    report_date DATE NOT NULL,  -- Fixed: NUM -> DATE
    fiscal_year INTEGER,
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
    edgar_accession TEXT,
    edgar_form TEXT,
    edgar_filed_date DATE,  -- Fixed: NUM -> DATE
    data_source TEXT,
    created_at NUM,
    accession_number TEXT,
    form_type TEXT,
    filed_date DATE,  -- Fixed: NUM -> DATE
    fiscal_period TEXT,
    sec_filing_id INTEGER,
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    FOREIGN KEY (sec_filing_id) REFERENCES sec_filings(id)
);

-- Copy data from old table
INSERT INTO cash_flow_statements_new SELECT * FROM cash_flow_statements;

-- Step 4: Create indexes for new tables
CREATE INDEX idx_income_statements_new_stock_id ON income_statements_new(stock_id);
CREATE INDEX idx_income_statements_new_report_date ON income_statements_new(report_date);
CREATE INDEX idx_income_statements_new_fiscal_year ON income_statements_new(fiscal_year);
CREATE INDEX idx_income_statements_new_sec_filing_id ON income_statements_new(sec_filing_id);

CREATE INDEX idx_balance_sheets_new_stock_id ON balance_sheets_new(stock_id);
CREATE INDEX idx_balance_sheets_new_report_date ON balance_sheets_new(report_date);
CREATE INDEX idx_balance_sheets_new_fiscal_year ON balance_sheets_new(fiscal_year);
CREATE INDEX idx_balance_sheets_new_sec_filing_id ON balance_sheets_new(sec_filing_id);

CREATE INDEX idx_cash_flow_statements_new_stock_id ON cash_flow_statements_new(stock_id);
CREATE INDEX idx_cash_flow_statements_new_report_date ON cash_flow_statements_new(report_date);
CREATE INDEX idx_cash_flow_statements_new_fiscal_year ON cash_flow_statements_new(fiscal_year);
CREATE INDEX idx_cash_flow_statements_new_sec_filing_id ON cash_flow_statements_new(sec_filing_id);