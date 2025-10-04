-- Migration: Link existing financial records to import sessions
-- Risk: LOW RISK (updating existing data)
-- Impact: Establishes relationships between records and imports

-- Link income_statements to their import sessions
UPDATE income_statements 
SET import_id = (
    SELECT id FROM data_imports 
    WHERE import_type = 'sec_edgar' 
    AND import_date = DATE(income_statements.created_at)
    LIMIT 1
)
WHERE data_source = 'sec_edgar' 
AND import_id IS NULL;

-- Link balance_sheets to their import sessions
UPDATE balance_sheets 
SET import_id = (
    SELECT id FROM data_imports 
    WHERE import_type = 'sec_edgar' 
    AND import_date = DATE(balance_sheets.created_at)
    LIMIT 1
)
WHERE data_source = 'sec_edgar' 
AND import_id IS NULL;

-- Link cash_flow_statements to their import sessions
UPDATE cash_flow_statements 
SET import_id = (
    SELECT id FROM data_imports 
    WHERE import_type = 'sec_edgar' 
    AND import_date = DATE(cash_flow_statements.created_at)
    LIMIT 1
)
WHERE data_source = 'sec_edgar' 
AND import_id IS NULL;