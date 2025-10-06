-- Cleanup orphaned sec_filings records that have no associated financial data
-- Run this before: SELECT COUNT(*) FROM sec_filings; to record the count

-- First, let's see what we're about to delete (for logging)
-- Uncomment to preview:
-- SELECT sf.id, s.symbol, sf.filed_date, sf.accession_number
-- FROM sec_filings sf
-- JOIN stocks s ON s.id = sf.stock_id
-- WHERE NOT EXISTS (
--     SELECT 1 FROM balance_sheets bs WHERE bs.sec_filing_id = sf.id
-- )
-- OR NOT EXISTS (
--     SELECT 1 FROM income_statements inc WHERE inc.sec_filing_id = sf.id
-- )
-- OR NOT EXISTS (
--     SELECT 1 FROM cash_flow_statements cf WHERE cf.sec_filing_id = sf.id
-- );

-- Delete sec_filings that are missing ANY of the three required financial statements
DELETE FROM sec_filings
WHERE id IN (
    SELECT sf.id
    FROM sec_filings sf
    WHERE NOT EXISTS (
        SELECT 1 FROM balance_sheets bs WHERE bs.sec_filing_id = sf.id
    )
    OR NOT EXISTS (
        SELECT 1 FROM income_statements inc WHERE inc.sec_filing_id = sf.id
    )
    OR NOT EXISTS (
        SELECT 1 FROM cash_flow_statements cf WHERE cf.sec_filing_id = sf.id
    )
);

-- After: SELECT COUNT(*) FROM sec_filings; to verify cleanup
