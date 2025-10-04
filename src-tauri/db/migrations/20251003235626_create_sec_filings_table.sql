-- Migration: Create sec_filings table for EDGAR metadata consolidation
-- Risk: LOW (new table creation)
-- Impact: Normalizes filing metadata across financial statements

CREATE TABLE sec_filings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    accession_number TEXT NOT NULL,
    form_type TEXT NOT NULL,
    filed_date DATE NOT NULL,
    fiscal_period TEXT,
    fiscal_year INTEGER NOT NULL,
    report_date DATE NOT NULL,
    
    -- Additional metadata
    file_size_bytes INTEGER,
    document_count INTEGER,
    is_amended BOOLEAN DEFAULT 0,
    
    -- Audit fields
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    -- Foreign key constraint
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    
    -- Unique constraints
    UNIQUE(stock_id, accession_number),
    UNIQUE(stock_id, form_type, report_date, fiscal_year)
);

-- Create indexes for performance
CREATE INDEX idx_sec_filings_stock_id ON sec_filings(stock_id);
CREATE INDEX idx_sec_filings_filed_date ON sec_filings(filed_date);
CREATE INDEX idx_sec_filings_form_type ON sec_filings(form_type);
CREATE INDEX idx_sec_filings_accession ON sec_filings(accession_number);
CREATE INDEX idx_sec_filings_report_date ON sec_filings(report_date);