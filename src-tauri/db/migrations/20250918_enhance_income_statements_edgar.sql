-- Migration: Enhance income_statements table with EDGAR data fields
-- Purpose: Add missing income statement fields for accurate EBITDA and expense breakdowns

-- Add expense breakdowns for better analysis
ALTER TABLE income_statements ADD COLUMN cost_of_revenue REAL; -- CostOfRevenue
ALTER TABLE income_statements ADD COLUMN research_development REAL; -- ResearchAndDevelopmentExpense
ALTER TABLE income_statements ADD COLUMN selling_general_admin REAL; -- SellingGeneralAndAdministrativeExpense

-- Add depreciation and amortization for EBITDA calculation
ALTER TABLE income_statements ADD COLUMN depreciation_expense REAL; -- DepreciationAndAmortization
ALTER TABLE income_statements ADD COLUMN amortization_expense REAL; -- AmortizationOfIntangibleAssets
ALTER TABLE income_statements ADD COLUMN interest_expense REAL; -- InterestExpense

-- Add EDGAR metadata for traceability
ALTER TABLE income_statements ADD COLUMN edgar_accession TEXT;
ALTER TABLE income_statements ADD COLUMN edgar_form TEXT; -- '10-K', '10-Q'
ALTER TABLE income_statements ADD COLUMN edgar_filed_date DATE;

-- Update existing records to set data_source
UPDATE income_statements SET data_source = 'simfin' WHERE data_source IS NULL;