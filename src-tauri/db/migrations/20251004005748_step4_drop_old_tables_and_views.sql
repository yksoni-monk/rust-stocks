-- Migration Step 4: Drop old tables and views, rename new ones
-- Risk: LOW (dropping old objects, no data loss)
-- Impact: Frees up disk space and completes the migration

-- Step 1: Drop ALL old views first (they depend on old tables)
DROP VIEW IF EXISTS revenue_growth_analysis;
DROP VIEW IF EXISTS enterprise_value_analysis;
DROP VIEW IF EXISTS revenue_data_validation;
DROP VIEW IF EXISTS balance_sheet_validation;
DROP VIEW IF EXISTS sector_data_quality;
DROP VIEW IF EXISTS ebitda_calculations;
DROP VIEW IF EXISTS book_value_calculations;
DROP VIEW IF EXISTS piotroski_multi_year_data;
DROP VIEW IF EXISTS piotroski_f_score_complete;
DROP VIEW IF EXISTS oshaughnessy_value_composite;
DROP VIEW IF EXISTS piotroski_screening_results;
DROP VIEW IF EXISTS oshaughnessy_ranking;

-- Step 2: Drop old tables
DROP TABLE IF EXISTS income_statements;
DROP TABLE IF EXISTS balance_sheets;
DROP TABLE IF EXISTS cash_flow_statements;

-- Step 3: Rename new tables to original names
ALTER TABLE income_statements_new RENAME TO income_statements;
ALTER TABLE balance_sheets_new RENAME TO balance_sheets;
ALTER TABLE cash_flow_statements_new RENAME TO cash_flow_statements;

-- Step 4: Drop new views and recreate with original names
DROP VIEW IF EXISTS revenue_growth_analysis_new;
DROP VIEW IF EXISTS enterprise_value_analysis_new;
DROP VIEW IF EXISTS revenue_data_validation_new;
DROP VIEW IF EXISTS balance_sheet_validation_new;
DROP VIEW IF EXISTS sector_data_quality_new;
DROP VIEW IF EXISTS ebitda_calculations_new;
DROP VIEW IF EXISTS book_value_calculations_new;
DROP VIEW IF EXISTS piotroski_multi_year_data_new;
DROP VIEW IF EXISTS oshaughnessy_value_composite_new;
DROP VIEW IF EXISTS piotroski_screening_results_new;
DROP VIEW IF EXISTS oshaughnessy_ranking_new;