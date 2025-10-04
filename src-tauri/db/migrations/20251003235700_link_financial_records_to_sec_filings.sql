-- Migration: Link existing financial records to sec_filings table
-- Risk: LOW (UPDATE operations with proper WHERE clauses)
-- Impact: Establishes relationships between financial data and filing metadata

-- Link income_statements records to sec_filings
UPDATE income_statements 
SET sec_filing_id = (
    SELECT sf.id 
    FROM sec_filings sf 
    WHERE sf.stock_id = income_statements.stock_id 
        AND sf.accession_number = income_statements.accession_number 
        AND sf.form_type = income_statements.form_type 
        AND sf.filed_date = income_statements.filed_date
)
WHERE accession_number IS NOT NULL 
    AND form_type IS NOT NULL 
    AND filed_date IS NOT NULL;

-- Link balance_sheets records to sec_filings
UPDATE balance_sheets 
SET sec_filing_id = (
    SELECT sf.id 
    FROM sec_filings sf 
    WHERE sf.stock_id = balance_sheets.stock_id 
        AND sf.accession_number = balance_sheets.accession_number 
        AND sf.form_type = balance_sheets.form_type 
        AND sf.filed_date = balance_sheets.filed_date
)
WHERE accession_number IS NOT NULL 
    AND form_type IS NOT NULL 
    AND filed_date IS NOT NULL;

-- Link cash_flow_statements records to sec_filings
UPDATE cash_flow_statements 
SET sec_filing_id = (
    SELECT sf.id 
    FROM sec_filings sf 
    WHERE sf.stock_id = cash_flow_statements.stock_id 
        AND sf.accession_number = cash_flow_statements.accession_number 
        AND sf.form_type = cash_flow_statements.form_type 
        AND sf.filed_date = cash_flow_statements.filed_date
)
WHERE accession_number IS NOT NULL 
    AND form_type IS NOT NULL 
    AND filed_date IS NOT NULL;