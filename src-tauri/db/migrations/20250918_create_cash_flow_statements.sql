-- Migration: Create cash flow statements table for EDGAR data
-- Purpose: Store operating, investing, and financing cash flow data from EDGAR company facts

CREATE TABLE cash_flow_statements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    period_type TEXT NOT NULL, -- 'TTM', 'Annual', 'Quarterly'
    report_date DATE NOT NULL,
    fiscal_year INTEGER,
    fiscal_period TEXT,

    -- Operating Activities
    operating_cash_flow REAL, -- NetCashProvidedByUsedInOperatingActivities
    depreciation_amortization REAL, -- DepreciationDepletionAndAmortization
    depreciation_expense REAL, -- DepreciationAndAmortization
    amortization_expense REAL, -- AmortizationOfIntangibleAssets

    -- Investing Activities
    investing_cash_flow REAL, -- NetCashProvidedByUsedInInvestingActivities
    capital_expenditures REAL, -- PaymentsToAcquirePropertyPlantAndEquipment

    -- Financing Activities
    financing_cash_flow REAL, -- NetCashProvidedByUsedInFinancingActivities
    dividends_paid REAL, -- PaymentsOfDividends
    share_repurchases REAL, -- PaymentsForRepurchaseOfCommonStock
    net_cash_flow REAL, -- Total net change in cash

    -- EDGAR metadata
    edgar_accession TEXT,
    edgar_form TEXT, -- '10-K', '10-Q'
    edgar_filed_date DATE,
    data_source TEXT DEFAULT 'edgar',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, period_type, report_date)
);

-- Indexes for performance
CREATE INDEX idx_cash_flow_stock_period ON cash_flow_statements(stock_id, period_type);
CREATE INDEX idx_cash_flow_report_date ON cash_flow_statements(report_date DESC);
CREATE INDEX idx_cash_flow_fiscal_year ON cash_flow_statements(fiscal_year DESC);