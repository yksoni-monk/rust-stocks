-- Migration: Fix import linking for actual data sources
-- Risk: LOW RISK (updating existing data)
-- Impact: Links financial records to correct import sessions

-- First, create import sessions for the actual data sources found
INSERT OR IGNORE INTO data_imports (import_type, import_date, records_imported, status)
SELECT 
    CASE 
        WHEN data_source = 'edgar' THEN 'sec_edgar'
        WHEN data_source = 'sec_edgar_json' THEN 'sec_edgar'
        WHEN data_source = 'unknown' THEN 'manual'
        ELSE data_source
    END as import_type,
    DATE(created_at) as import_date,
    COUNT(*) as records_imported,
    'completed' as status
FROM income_statements 
GROUP BY data_source, DATE(created_at);

-- Link income_statements to their import sessions
UPDATE income_statements 
SET import_id = (
    SELECT id FROM data_imports 
    WHERE import_type = CASE 
        WHEN income_statements.data_source = 'edgar' THEN 'sec_edgar'
        WHEN income_statements.data_source = 'sec_edgar_json' THEN 'sec_edgar'
        WHEN income_statements.data_source = 'unknown' THEN 'manual'
        ELSE income_statements.data_source
    END
    AND import_date = DATE(income_statements.created_at)
    LIMIT 1
)
WHERE import_id IS NULL;

-- Do the same for balance_sheets
INSERT OR IGNORE INTO data_imports (import_type, import_date, records_imported, status)
SELECT 
    CASE 
        WHEN data_source = 'edgar' THEN 'sec_edgar'
        WHEN data_source = 'sec_edgar_json' THEN 'sec_edgar'
        WHEN data_source = 'unknown' THEN 'manual'
        ELSE data_source
    END as import_type,
    DATE(created_at) as import_date,
    COUNT(*) as records_imported,
    'completed' as status
FROM balance_sheets 
GROUP BY data_source, DATE(created_at);

UPDATE balance_sheets 
SET import_id = (
    SELECT id FROM data_imports 
    WHERE import_type = CASE 
        WHEN balance_sheets.data_source = 'edgar' THEN 'sec_edgar'
        WHEN balance_sheets.data_source = 'sec_edgar_json' THEN 'sec_edgar'
        WHEN balance_sheets.data_source = 'unknown' THEN 'manual'
        ELSE balance_sheets.data_source
    END
    AND import_date = DATE(balance_sheets.created_at)
    LIMIT 1
)
WHERE import_id IS NULL;

-- Do the same for cash_flow_statements
INSERT OR IGNORE INTO data_imports (import_type, import_date, records_imported, status)
SELECT 
    CASE 
        WHEN data_source = 'edgar' THEN 'sec_edgar'
        WHEN data_source = 'sec_edgar_json' THEN 'sec_edgar'
        WHEN data_source = 'unknown' THEN 'manual'
        ELSE data_source
    END as import_type,
    DATE(created_at) as import_date,
    COUNT(*) as records_imported,
    'completed' as status
FROM cash_flow_statements 
GROUP BY data_source, DATE(created_at);

UPDATE cash_flow_statements 
SET import_id = (
    SELECT id FROM data_imports 
    WHERE import_type = CASE 
        WHEN cash_flow_statements.data_source = 'edgar' THEN 'sec_edgar'
        WHEN cash_flow_statements.data_source = 'sec_edgar_json' THEN 'sec_edgar'
        WHEN cash_flow_statements.data_source = 'unknown' THEN 'manual'
        ELSE cash_flow_statements.data_source
    END
    AND import_date = DATE(cash_flow_statements.created_at)
    LIMIT 1
)
WHERE import_id IS NULL;