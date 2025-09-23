-- Migration: Create dividend history table for shareholder yield calculation
-- Purpose: Store dividend payments for O'Shaughnessy shareholder yield metric

CREATE TABLE dividend_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    ex_date DATE NOT NULL,
    payment_date DATE,
    record_date DATE,
    dividend_per_share REAL,
    dividend_type TEXT DEFAULT 'regular', -- 'regular', 'special', 'stock'

    -- Calculated fields
    annualized_dividend REAL,
    yield_at_ex_date REAL,

    -- EDGAR metadata
    edgar_accession TEXT,
    fiscal_year INTEGER,
    fiscal_period TEXT,
    data_source TEXT DEFAULT 'edgar',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, ex_date)
);

-- Indexes for performance
CREATE INDEX idx_dividend_stock_date ON dividend_history(stock_id, ex_date DESC);
CREATE INDEX idx_dividend_fiscal_year ON dividend_history(fiscal_year DESC);
CREATE INDEX idx_dividend_ex_date ON dividend_history(ex_date DESC);