-- Migration: Create data_imports table for centralized import tracking
-- Risk: ZERO RISK (creating new table only)
-- Impact: HIGH IMPACT (eliminates import metadata duplication)

-- Create centralized import tracking table
CREATE TABLE data_imports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    import_type TEXT NOT NULL,                    -- 'sec_edgar', 'schwab', 'manual', etc.
    import_date DATE NOT NULL,                    -- When the import happened
    source_file TEXT,                             -- Which file was imported (optional)
    records_imported INTEGER,                     -- How many records were imported
    status TEXT DEFAULT 'completed',              -- 'completed', 'failed', 'partial'
    
    -- Additional metadata
    error_message TEXT,                           -- If status is 'failed'
    processing_time_ms INTEGER,                   -- How long the import took
    
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    -- Constraints
    UNIQUE(import_type, import_date),
    CHECK(status IN ('completed', 'failed', 'partial'))
);

-- Create indexes for better performance
CREATE INDEX idx_data_imports_type ON data_imports(import_type);
CREATE INDEX idx_data_imports_date ON data_imports(import_date);
CREATE INDEX idx_data_imports_status ON data_imports(status);
CREATE INDEX idx_data_imports_type_date ON data_imports(import_type, import_date);