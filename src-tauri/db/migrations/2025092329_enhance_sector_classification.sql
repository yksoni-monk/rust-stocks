-- Add GICS sector classification to stocks table for comprehensive sector-based analysis
-- This enhances the existing sector field with detailed GICS hierarchy

-- Add GICS classification columns
ALTER TABLE stocks ADD COLUMN gics_sector TEXT;
ALTER TABLE stocks ADD COLUMN gics_industry_group TEXT;
ALTER TABLE stocks ADD COLUMN gics_industry TEXT;
ALTER TABLE stocks ADD COLUMN gics_sub_industry TEXT;

-- Create EDGAR field mapping tracking table
CREATE TABLE edgar_field_mappings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sector TEXT NOT NULL,
    statement_type TEXT NOT NULL, -- 'balance_sheet', 'income_statement', 'cash_flow'
    our_field_name TEXT NOT NULL,
    edgar_gaap_field TEXT NOT NULL,
    success_rate REAL DEFAULT 0.0,
    companies_tested INTEGER DEFAULT 0,
    companies_successful INTEGER DEFAULT 0,
    last_verified DATETIME DEFAULT CURRENT_TIMESTAMP,
    notes TEXT,

    UNIQUE(sector, statement_type, our_field_name, edgar_gaap_field)
);

-- Create indexes for efficient sector-based queries
CREATE INDEX idx_stocks_gics_sector ON stocks(gics_sector);
CREATE INDEX idx_stocks_gics_industry ON stocks(gics_industry);
CREATE INDEX idx_edgar_mappings_sector ON edgar_field_mappings(sector, statement_type);
CREATE INDEX idx_edgar_mappings_success ON edgar_field_mappings(success_rate DESC);

-- Copy existing sector data to gics_sector for backward compatibility
UPDATE stocks SET gics_sector = sector WHERE sector IS NOT NULL;

-- Insert known successful field mappings from current implementation
INSERT OR IGNORE INTO edgar_field_mappings (sector, statement_type, our_field_name, edgar_gaap_field, notes) VALUES
-- Standard GAAP mappings that work for most sectors
('General', 'balance_sheet', 'total_assets', 'Assets', 'Standard GAAP field'),
('General', 'balance_sheet', 'total_liabilities', 'Liabilities', 'Standard GAAP field'),
('General', 'balance_sheet', 'total_equity', 'StockholdersEquity', 'Standard GAAP field'),
('General', 'balance_sheet', 'cash_and_equivalents', 'CashAndCashEquivalentsAtCarryingValue', 'Standard GAAP field'),
('General', 'income_statement', 'revenue', 'Revenues', 'Standard GAAP field'),
('General', 'income_statement', 'net_income', 'NetIncomeLoss', 'Standard GAAP field'),
('General', 'cash_flow', 'operating_cash_flow', 'NetCashProvidedByUsedInOperatingActivities', 'Standard GAAP field');

-- Insert sector-specific mappings that need to be tested
-- Financial sector variations
INSERT OR IGNORE INTO edgar_field_mappings (sector, statement_type, our_field_name, edgar_gaap_field, notes) VALUES
('Financials', 'balance_sheet', 'current_assets', 'CashAndCashEquivalentsAtCarryingValue', 'Banks: Cash is primary current asset'),
('Financials', 'balance_sheet', 'current_assets', 'LoansAndAdvancesToCustomers', 'Banks: Customer loans as current assets'),
('Financials', 'balance_sheet', 'current_assets', 'SecuritiesOwnedAtFairValue', 'Investment firms: Trading securities'),
('Financials', 'balance_sheet', 'current_liabilities', 'DepositsAtCarryingValue', 'Banks: Customer deposits'),
('Financials', 'balance_sheet', 'current_liabilities', 'CustomerPayablesAndAccruedLiabilities', 'Financial services payables');

-- Real Estate sector variations
INSERT OR IGNORE INTO edgar_field_mappings (sector, statement_type, our_field_name, edgar_gaap_field, notes) VALUES
('Real Estate', 'balance_sheet', 'current_assets', 'RealEstateInvestments', 'REITs: Real estate as current assets'),
('Real Estate', 'balance_sheet', 'current_assets', 'InvestmentInRealEstate', 'REITs: Alternative real estate field'),
('Real Estate', 'balance_sheet', 'total_assets', 'RealEstateInvestmentPropertyAtCost', 'REITs: Property at cost');

-- Consumer Discretionary (Homebuilders) variations
INSERT OR IGNORE INTO edgar_field_mappings (sector, statement_type, our_field_name, edgar_gaap_field, notes) VALUES
('Consumer Discretionary', 'balance_sheet', 'current_assets', 'InventoryRealEstate', 'Homebuilders: Land and homes inventory'),
('Consumer Discretionary', 'balance_sheet', 'current_assets', 'LandAndDevelopmentCosts', 'Homebuilders: Development inventory'),
('Consumer Discretionary', 'balance_sheet', 'current_assets', 'ConstructionInProgress', 'Homebuilders: WIP inventory');

-- Create view for sector-based data quality analysis
CREATE VIEW sector_data_quality AS
SELECT
    s.gics_sector,
    COUNT(*) as total_companies,
    COUNT(CASE WHEN s.is_sp500 = 1 THEN 1 END) as sp500_companies,
    -- Balance sheet data quality
    COUNT(CASE WHEN bs.current_assets IS NOT NULL AND bs.fiscal_year >= 2019 THEN 1 END) as has_current_assets,
    COUNT(CASE WHEN bs.current_liabilities IS NOT NULL AND bs.fiscal_year >= 2019 THEN 1 END) as has_current_liabilities,
    COUNT(CASE WHEN bs.total_debt IS NOT NULL AND bs.fiscal_year >= 2019 THEN 1 END) as has_total_debt,
    COUNT(CASE WHEN bs.total_equity IS NOT NULL AND bs.fiscal_year >= 2019 THEN 1 END) as has_total_equity,
    -- Income statement data quality
    COUNT(CASE WHEN is_.net_income IS NOT NULL AND is_.fiscal_year >= 2019 THEN 1 END) as has_net_income,
    COUNT(CASE WHEN is_.revenue IS NOT NULL AND is_.fiscal_year >= 2019 THEN 1 END) as has_revenue,
    -- Cash flow data quality
    COUNT(CASE WHEN cf.operating_cash_flow IS NOT NULL AND cf.fiscal_year >= 2019 THEN 1 END) as has_operating_cf
FROM stocks s
LEFT JOIN balance_sheets bs ON s.id = bs.stock_id AND bs.period_type = 'Annual'
LEFT JOIN income_statements is_ ON s.id = is_.stock_id AND is_.period_type = 'Annual'
LEFT JOIN cash_flow_statements cf ON s.id = cf.stock_id AND cf.period_type = 'Annual'
WHERE s.is_sp500 = 1
GROUP BY s.gics_sector
ORDER BY total_companies DESC;