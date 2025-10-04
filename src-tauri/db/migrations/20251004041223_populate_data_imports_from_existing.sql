-- Migration: Populate data_imports from existing financial data
-- Risk: LOW RISK (reading existing data only)
-- Impact: Creates import tracking from historical data

-- Extract unique import sessions from income_statements
INSERT OR IGNORE INTO data_imports (import_type, import_date, records_imported, status)
SELECT 
    'sec_edgar' as import_type,
    DATE(created_at) as import_date,
    COUNT(*) as records_imported,
    'completed' as status
FROM income_statements 
WHERE data_source = 'sec_edgar'
GROUP BY DATE(created_at);

-- Extract unique import sessions from balance_sheets
INSERT OR IGNORE INTO data_imports (import_type, import_date, records_imported, status)
SELECT 
    'sec_edgar' as import_type,
    DATE(created_at) as import_date,
    COUNT(*) as records_imported,
    'completed' as status
FROM balance_sheets 
WHERE data_source = 'sec_edgar'
GROUP BY DATE(created_at);

-- Extract unique import sessions from cash_flow_statements
INSERT OR IGNORE INTO data_imports (import_type, import_date, records_imported, status)
SELECT 
    'sec_edgar' as import_type,
    DATE(created_at) as import_date,
    COUNT(*) as records_imported,
    'completed' as status
FROM cash_flow_statements 
WHERE data_source = 'sec_edgar'
GROUP BY DATE(created_at);

-- Extract Schwab price data imports (if any exist)
INSERT OR IGNORE INTO data_imports (import_type, import_date, records_imported, status)
SELECT 
    'schwab' as import_type,
    DATE(last_updated) as import_date,
    COUNT(*) as records_imported,
    'completed' as status
FROM daily_prices 
WHERE last_updated IS NOT NULL
GROUP BY DATE(last_updated);