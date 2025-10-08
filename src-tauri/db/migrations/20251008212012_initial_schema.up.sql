-- Initial schema migration
-- Created from existing database schema
-- This represents the complete schema at the time of migration reset

CREATE TABLE daily_prices (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER NOT NULL,
    date DATE NOT NULL,
    open_price REAL NOT NULL,
    high_price REAL NOT NULL,
    low_price REAL NOT NULL,
    close_price REAL NOT NULL,
    volume INTEGER,
    pe_ratio REAL,
    market_cap REAL,
    dividend_yield REAL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP, eps REAL, beta REAL, week_52_high REAL, week_52_low REAL, pb_ratio REAL, ps_ratio REAL, shares_outstanding REAL, profit_margin REAL, operating_margin REAL, return_on_equity REAL, return_on_assets REAL, debt_to_equity REAL, dividend_per_share REAL, data_source TEXT DEFAULT 'alpha_vantage', last_updated DATETIME,
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, date)
);
CREATE TABLE metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE processing_status (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER NOT NULL,
    data_type TEXT NOT NULL CHECK (data_type IN ('prices', 'earnings', 'fundamentals')),
    status TEXT NOT NULL CHECK (status IN ('pending', 'processing', 'completed', 'failed')),
    fetch_mode TEXT CHECK (fetch_mode IN ('compact', 'full')),
    records_processed INTEGER DEFAULT 0,
    total_records INTEGER DEFAULT 0,
    error_message TEXT,
    started_at DATETIME,
    completed_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (stock_id) REFERENCES stocks(id) ON DELETE CASCADE,
    UNIQUE(stock_id, data_type)
);
CREATE TABLE sp500_symbols (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL UNIQUE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE refresh_progress (
    session_id TEXT PRIMARY KEY,
    operation_type TEXT NOT NULL,               -- 'quick', 'standard', 'full'
    start_time DATETIME DEFAULT CURRENT_TIMESTAMP,
    end_time DATETIME,
    total_steps INTEGER NOT NULL,
    completed_steps INTEGER DEFAULT 0,
    current_step_name TEXT,
    current_step_progress REAL DEFAULT 0.0,    -- 0.0 to 100.0
    estimated_completion DATETIME,
    status TEXT DEFAULT 'running',              -- 'running', 'completed', 'error', 'cancelled'
    error_details TEXT,
    initiated_by TEXT,                          -- 'user', 'scheduled', 'auto'
    data_sources_refreshed TEXT,                -- JSON array of data sources included
    total_records_processed INTEGER DEFAULT 0,
    performance_metrics TEXT                    -- JSON object with performance data
);
CREATE TABLE refresh_configuration (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    data_source TEXT NOT NULL UNIQUE,
    refresh_frequency_hours INTEGER DEFAULT 24, -- How often to refresh (in hours)
    max_staleness_days INTEGER DEFAULT 7,       -- Maximum acceptable staleness
    auto_refresh_enabled BOOLEAN DEFAULT TRUE,
    refresh_priority INTEGER DEFAULT 5,         -- 1-10 priority for ordering
    dependency_sources TEXT,                     -- JSON array of required predecessor sources
    refresh_command TEXT,                        -- Command/function to execute refresh
    estimated_duration_minutes INTEGER DEFAULT 10,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS "stocks" (id INTEGER PRIMARY KEY AUTOINCREMENT, symbol TEXT UNIQUE NOT NULL, company_name TEXT NOT NULL, cik TEXT UNIQUE, sector TEXT, last_updated DATETIME, created_at DATETIME DEFAULT CURRENT_TIMESTAMP, is_sp500 BOOLEAN DEFAULT 0);
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
CREATE TABLE IF NOT EXISTS "income_statements" (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    period_type TEXT NOT NULL,
    report_date DATE NOT NULL,  -- Fixed: NUM -> DATE
    fiscal_year INTEGER,
    revenue REAL,
    gross_profit REAL,
    operating_income REAL,
    net_income REAL,
    shares_basic REAL,
    shares_diluted REAL,
    cost_of_revenue REAL,
    research_development REAL,
    selling_general_admin REAL,
    depreciation_expense REAL,
    amortization_expense REAL,
    interest_expense REAL,
    currency TEXT,
    simfin_id INTEGER,
    publish_date NUM,
    sec_filing_id INTEGER, import_id INTEGER,
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    FOREIGN KEY (sec_filing_id) REFERENCES sec_filings(id)
);
CREATE TABLE IF NOT EXISTS "balance_sheets" (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    period_type TEXT NOT NULL,
    report_date DATE NOT NULL,  -- Fixed: NUM -> DATE
    fiscal_year INTEGER,
    cash_and_equivalents REAL,
    short_term_debt REAL,
    long_term_debt REAL,
    total_debt REAL,
    total_assets REAL,
    total_liabilities REAL,
    total_equity REAL,
    shares_outstanding REAL,
    currency TEXT,
    simfin_id INTEGER,
    current_assets REAL,
    current_liabilities REAL,
    inventory REAL,
    accounts_receivable REAL,
    accounts_payable REAL,
    working_capital REAL,
    share_repurchases REAL,
    sec_filing_id INTEGER, import_id INTEGER,
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    FOREIGN KEY (sec_filing_id) REFERENCES sec_filings(id)
);
CREATE TABLE IF NOT EXISTS "cash_flow_statements" (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    period_type TEXT NOT NULL,
    report_date DATE NOT NULL,  -- Fixed: NUM -> DATE
    fiscal_year INTEGER,
    operating_cash_flow REAL,
    depreciation_amortization REAL,
    depreciation_expense REAL,
    amortization_expense REAL,
    investing_cash_flow REAL,
    capital_expenditures REAL,
    financing_cash_flow REAL,
    dividends_paid REAL,
    share_repurchases REAL,
    net_cash_flow REAL,
    sec_filing_id INTEGER, import_id INTEGER,
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    FOREIGN KEY (sec_filing_id) REFERENCES sec_filings(id)
);
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
CREATE INDEX idx_daily_prices_stock_date ON daily_prices(stock_id, date);
CREATE INDEX idx_daily_prices_date ON daily_prices(date);
CREATE INDEX idx_daily_prices_pe_ratio ON daily_prices(pe_ratio);
CREATE INDEX idx_daily_prices_market_cap ON daily_prices(market_cap);
CREATE INDEX idx_daily_prices_dividend_yield ON daily_prices(dividend_yield);
CREATE INDEX idx_daily_prices_eps ON daily_prices(eps);
CREATE INDEX idx_daily_prices_beta ON daily_prices(beta);
CREATE INDEX idx_processing_stock_type ON processing_status(stock_id, data_type);
CREATE INDEX idx_processing_status ON processing_status(status);
CREATE INDEX idx_processing_stock_status ON processing_status(stock_id, status);
CREATE INDEX idx_daily_prices_source ON daily_prices(data_source);
CREATE INDEX idx_daily_prices_source_date ON daily_prices(data_source, date);
CREATE INDEX idx_daily_prices_updated ON daily_prices(last_updated);
CREATE INDEX idx_daily_prices_simfin ON daily_prices(data_source);
CREATE INDEX idx_daily_prices_stock_count ON daily_prices(stock_id);
CREATE INDEX idx_sp500_symbols_symbol ON sp500_symbols(symbol);
CREATE INDEX idx_daily_prices_stock_id ON daily_prices(stock_id);
CREATE INDEX idx_daily_prices_stock_pe ON daily_prices(stock_id, pe_ratio);
CREATE INDEX idx_sp500_symbol ON sp500_symbols(symbol);
CREATE INDEX idx_refresh_progress_status ON refresh_progress(status);
CREATE INDEX idx_refresh_progress_start_time ON refresh_progress(start_time);
CREATE INDEX idx_refresh_config_source ON refresh_configuration(data_source);
CREATE INDEX idx_refresh_config_priority ON refresh_configuration(refresh_priority);
CREATE INDEX idx_stocks_symbol ON stocks(symbol);
CREATE INDEX idx_stocks_cik ON stocks(cik);
CREATE INDEX idx_stocks_sp500 ON stocks(is_sp500);
CREATE INDEX idx_stocks_sector ON stocks(sector);
CREATE INDEX idx_sec_filings_stock_id ON sec_filings(stock_id);
CREATE INDEX idx_sec_filings_filed_date ON sec_filings(filed_date);
CREATE INDEX idx_sec_filings_form_type ON sec_filings(form_type);
CREATE INDEX idx_sec_filings_accession ON sec_filings(accession_number);
CREATE INDEX idx_sec_filings_report_date ON sec_filings(report_date);
CREATE INDEX idx_income_statements_new_stock_id ON "income_statements"(stock_id);
CREATE INDEX idx_income_statements_new_report_date ON "income_statements"(report_date);
CREATE INDEX idx_income_statements_new_fiscal_year ON "income_statements"(fiscal_year);
CREATE INDEX idx_income_statements_new_sec_filing_id ON "income_statements"(sec_filing_id);
CREATE INDEX idx_balance_sheets_new_stock_id ON "balance_sheets"(stock_id);
CREATE INDEX idx_balance_sheets_new_report_date ON "balance_sheets"(report_date);
CREATE INDEX idx_balance_sheets_new_fiscal_year ON "balance_sheets"(fiscal_year);
CREATE INDEX idx_balance_sheets_new_sec_filing_id ON "balance_sheets"(sec_filing_id);
CREATE INDEX idx_cash_flow_statements_new_stock_id ON "cash_flow_statements"(stock_id);
CREATE INDEX idx_cash_flow_statements_new_report_date ON "cash_flow_statements"(report_date);
CREATE INDEX idx_cash_flow_statements_new_fiscal_year ON "cash_flow_statements"(fiscal_year);
CREATE INDEX idx_cash_flow_statements_new_sec_filing_id ON "cash_flow_statements"(sec_filing_id);
CREATE INDEX idx_data_imports_type ON data_imports(import_type);
CREATE INDEX idx_data_imports_date ON data_imports(import_date);
CREATE INDEX idx_data_imports_status ON data_imports(status);
CREATE INDEX idx_data_imports_type_date ON data_imports(import_type, import_date);
CREATE INDEX idx_income_statements_import_id ON income_statements(import_id);
CREATE INDEX idx_balance_sheets_import_id ON balance_sheets(import_id);
CREATE INDEX idx_cash_flow_statements_import_id ON cash_flow_statements(import_id);
CREATE TRIGGER update_processing_timestamp 
    AFTER UPDATE ON processing_status
    FOR EACH ROW
BEGIN
    UPDATE processing_status SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
CREATE TRIGGER update_daily_prices_timestamp 
    AFTER UPDATE ON daily_prices
    FOR EACH ROW
BEGIN
    UPDATE daily_prices SET last_updated = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
CREATE VIEW piotroski_f_score_complete AS
SELECT
    fd.*,
    NULL as pb_ratio,

    -- PROFITABILITY (4 criteria)
    CASE WHEN current_net_income > 0 THEN 1 ELSE 0 END as criterion_positive_net_income,
    CASE WHEN current_operating_cash_flow > 0 THEN 1 ELSE 0 END as criterion_positive_operating_cash_flow,
    CASE
        WHEN current_net_income / NULLIF(current_assets, 0) >
             prior_net_income / NULLIF(prior_assets, 0)
             AND current_net_income IS NOT NULL AND prior_net_income IS NOT NULL
             AND current_assets IS NOT NULL AND prior_assets IS NOT NULL THEN 1
        ELSE 0
    END as criterion_improving_roa,
    CASE
        WHEN current_operating_cash_flow > current_net_income
             AND current_operating_cash_flow IS NOT NULL
             AND current_net_income IS NOT NULL THEN 1
        ELSE 0
    END as criterion_cash_flow_quality,

    -- LEVERAGE/LIQUIDITY (3 criteria)
    CASE
        WHEN current_debt / NULLIF(current_assets, 0) <
             prior_debt / NULLIF(prior_assets, 0)
             AND current_debt IS NOT NULL AND prior_debt IS NOT NULL
             AND current_assets IS NOT NULL AND prior_assets IS NOT NULL THEN 1
        ELSE 0
    END as criterion_decreasing_debt_ratio,
    CASE
        WHEN current_current_assets / NULLIF(current_current_liabilities, 0) >
             prior_current_assets / NULLIF(prior_current_liabilities, 0)
             AND current_current_assets IS NOT NULL AND prior_current_assets IS NOT NULL
             AND current_current_liabilities IS NOT NULL AND prior_current_liabilities IS NOT NULL THEN 1
        ELSE 0
    END as criterion_improving_current_ratio,
    CASE
        WHEN COALESCE(current_shares_outstanding_bs, current_shares_outstanding) <=
             COALESCE(prior_shares_outstanding_bs, prior_shares_outstanding)
             AND COALESCE(current_shares_outstanding_bs, current_shares_outstanding) IS NOT NULL
             AND COALESCE(prior_shares_outstanding_bs, prior_shares_outstanding) IS NOT NULL THEN 1
        ELSE 0
    END as criterion_no_dilution,

    -- OPERATING EFFICIENCY (2 criteria) - USING NET MARGIN
    CASE
        WHEN current_net_income / NULLIF(current_revenue, 0) >
             prior_net_income / NULLIF(prior_revenue, 0)
             AND current_net_income IS NOT NULL AND prior_net_income IS NOT NULL
             AND current_revenue IS NOT NULL AND prior_revenue IS NOT NULL THEN 1
        ELSE 0
    END as criterion_improving_gross_margin,
    CASE
        WHEN current_revenue / NULLIF(current_assets, 0) >
             prior_revenue / NULLIF(prior_assets, 0)
             AND current_revenue IS NOT NULL AND prior_revenue IS NOT NULL
             AND current_assets IS NOT NULL AND prior_assets IS NOT NULL THEN 1
        ELSE 0
    END as criterion_improving_asset_turnover,

    -- TOTAL F-SCORE CALCULATION (0-9) - USING NET MARGIN
    (CASE WHEN current_net_income > 0 THEN 1 ELSE 0 END +
     CASE WHEN current_operating_cash_flow > 0 THEN 1 ELSE 0 END +
     CASE
         WHEN current_net_income / NULLIF(current_assets, 0) >
              prior_net_income / NULLIF(prior_assets, 0)
              AND current_net_income IS NOT NULL AND prior_net_income IS NOT NULL
              AND current_assets IS NOT NULL AND prior_assets IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN current_operating_cash_flow > current_net_income
              AND current_operating_cash_flow IS NOT NULL
              AND current_net_income IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN current_debt / NULLIF(current_assets, 0) <
              prior_debt / NULLIF(prior_assets, 0)
              AND current_debt IS NOT NULL AND prior_debt IS NOT NULL
              AND current_assets IS NOT NULL AND prior_assets IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN current_current_assets / NULLIF(current_current_liabilities, 0) >
              prior_current_assets / NULLIF(prior_current_liabilities, 0)
              AND current_current_assets IS NOT NULL AND prior_current_assets IS NOT NULL
              AND current_current_liabilities IS NOT NULL AND prior_current_liabilities IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN COALESCE(current_shares_outstanding_bs, current_shares_outstanding) <=
              COALESCE(prior_shares_outstanding_bs, prior_shares_outstanding)
              AND COALESCE(current_shares_outstanding_bs, current_shares_outstanding) IS NOT NULL
              AND COALESCE(prior_shares_outstanding_bs, prior_shares_outstanding) IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN current_net_income / NULLIF(current_revenue, 0) >
              prior_net_income / NULLIF(prior_revenue, 0)
              AND current_net_income IS NOT NULL AND prior_net_income IS NOT NULL
              AND current_revenue IS NOT NULL AND prior_revenue IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN current_revenue / NULLIF(current_assets, 0) >
              prior_revenue / NULLIF(prior_assets, 0)
              AND current_revenue IS NOT NULL AND prior_revenue IS NOT NULL
              AND current_assets IS NOT NULL AND prior_assets IS NOT NULL THEN 1
         ELSE 0
     END) as f_score_complete,

    -- DATA COMPLETENESS CALCULATION (Updated for net margin)
    (
        CASE WHEN current_net_income IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_operating_cash_flow IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_assets IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_debt IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_current_assets IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_current_liabilities IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN COALESCE(current_shares_outstanding_bs, current_shares_outstanding) IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_net_income IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_revenue IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_net_income IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_assets IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_debt IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_current_assets IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_current_liabilities IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN COALESCE(prior_shares_outstanding_bs, prior_shares_outstanding) IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_net_income IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_revenue IS NOT NULL THEN 1 ELSE 0 END
    ) * 100 / 17 as data_completeness_score,

    -- Additional calculated metrics for display
    CASE
        WHEN current_assets > 0
        THEN current_net_income / current_assets
        ELSE NULL
    END as current_roa,

    CASE
        WHEN current_assets > 0
        THEN current_debt / current_assets
        ELSE NULL
    END as current_debt_ratio,

    CASE
        WHEN current_current_liabilities > 0
        THEN current_current_assets / current_current_liabilities
        ELSE NULL
    END as current_current_ratio,

    -- NET MARGIN (instead of gross margin)
    CASE
        WHEN current_revenue > 0 AND current_net_income IS NOT NULL
        THEN current_net_income / current_revenue
        ELSE NULL
    END as current_gross_margin,

    CASE
        WHEN current_assets > 0
        THEN current_revenue / current_assets
        ELSE NULL
    END as current_asset_turnover

FROM piotroski_multi_year_data fd
WHERE fd.current_net_income IS NOT NULL
  OR fd.current_operating_cash_flow IS NOT NULL;
CREATE VIEW oshaughnessy_value_composite AS
SELECT
  s.id as stock_id,
  s.symbol,
  s.sector,
  (SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) as current_price,
  (SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) * b.shares_outstanding as market_cap,
  i.net_income,
  i.revenue,
  i.operating_income,
  b.total_equity,
  b.shares_outstanding,
  b.total_debt,
  b.cash_and_equivalents,
  cf.dividends_paid,
  cf.share_repurchases,
  cf.depreciation_expense,
  cf.amortization_expense,
  (((SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) * b.shares_outstanding) + COALESCE(b.total_debt, 0) - COALESCE(b.cash_and_equivalents, 0)) as enterprise_value,
  (COALESCE(i.operating_income, 0) + COALESCE(cf.depreciation_expense, 0) + COALESCE(cf.amortization_expense, 0)) as ebitda,
  CASE WHEN i.net_income > 0 AND b.shares_outstanding > 0 THEN ((SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) * b.shares_outstanding) / i.net_income ELSE NULL END as pe_ratio,
  CASE WHEN b.total_equity > 0 AND b.shares_outstanding > 0 THEN ((SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) * b.shares_outstanding) / b.total_equity ELSE NULL END as pb_ratio,
  CASE WHEN i.revenue > 0 AND b.shares_outstanding > 0 THEN ((SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) * b.shares_outstanding) / i.revenue ELSE NULL END as ps_ratio,
  CASE WHEN i.revenue > 0 AND b.shares_outstanding > 0 THEN (((SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) * b.shares_outstanding) + COALESCE(b.total_debt, 0) - COALESCE(b.cash_and_equivalents, 0)) / i.revenue ELSE NULL END as evs_ratio,
  CASE WHEN (COALESCE(i.operating_income, 0) + COALESCE(cf.depreciation_expense, 0) + COALESCE(cf.amortization_expense, 0)) > 0 AND b.shares_outstanding > 0 THEN (((SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) * b.shares_outstanding) + COALESCE(b.total_debt, 0) - COALESCE(b.cash_and_equivalents, 0)) / (COALESCE(i.operating_income, 0) + COALESCE(cf.depreciation_expense, 0) + COALESCE(cf.amortization_expense, 0)) ELSE NULL END as ev_ebitda_ratio,
  CASE WHEN b.shares_outstanding > 0 AND ((SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) * b.shares_outstanding) > 0 THEN (COALESCE(cf.dividends_paid, 0) + COALESCE(cf.share_repurchases, 0)) / ((SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) * b.shares_outstanding) ELSE NULL END as shareholder_yield,
  ((CASE WHEN i.net_income > 0 AND b.shares_outstanding > 0 THEN 1 ELSE 0 END) + (CASE WHEN b.total_equity > 0 AND b.shares_outstanding > 0 THEN 1 ELSE 0 END) + (CASE WHEN i.revenue > 0 AND b.shares_outstanding > 0 THEN 1 ELSE 0 END) + (CASE WHEN i.revenue > 0 AND b.shares_outstanding > 0 THEN 1 ELSE 0 END) + (CASE WHEN (COALESCE(i.operating_income, 0) + COALESCE(cf.depreciation_expense, 0) + COALESCE(cf.amortization_expense, 0)) > 0 AND b.shares_outstanding > 0 THEN 1 ELSE 0 END) + (CASE WHEN b.shares_outstanding > 0 AND (SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) > 0 THEN 1 ELSE 0 END)) * 16.67 as data_completeness_score
FROM stocks s
LEFT JOIN (SELECT stock_id, net_income, revenue, operating_income, report_date, ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn FROM income_statements WHERE period_type = 'Annual' AND revenue IS NOT NULL) i ON s.id = i.stock_id AND i.rn = 1
LEFT JOIN (SELECT stock_id, total_equity, shares_outstanding, total_debt, cash_and_equivalents, report_date, ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn FROM balance_sheets WHERE period_type = 'Annual' AND total_equity IS NOT NULL) b ON s.id = b.stock_id AND b.rn = 1
LEFT JOIN (SELECT stock_id, dividends_paid, share_repurchases, depreciation_expense, amortization_expense, report_date, ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn FROM cash_flow_statements WHERE period_type = 'Annual' AND operating_cash_flow IS NOT NULL) cf ON s.id = cf.stock_id AND cf.rn = 1
WHERE s.is_sp500 = 1
/* oshaughnessy_value_composite(stock_id,symbol,sector,current_price,market_cap,net_income,revenue,operating_income,total_equity,shares_outstanding,total_debt,cash_and_equivalents,dividends_paid,share_repurchases,depreciation_expense,amortization_expense,enterprise_value,ebitda,pe_ratio,pb_ratio,ps_ratio,evs_ratio,ev_ebitda_ratio,shareholder_yield,data_completeness_score) */;
CREATE VIEW oshaughnessy_ranking AS
WITH ranked AS (
  SELECT *, RANK() OVER (ORDER BY pe_ratio ASC) as pe_rank, RANK() OVER (ORDER BY pb_ratio ASC) as pb_rank, RANK() OVER (ORDER BY ps_ratio ASC) as ps_rank, RANK() OVER (ORDER BY evs_ratio ASC) as evs_rank, RANK() OVER (ORDER BY ev_ebitda_ratio ASC) as ebitda_rank, RANK() OVER (ORDER BY shareholder_yield DESC) as yield_rank, COUNT(*) OVER () as total_stocks
  FROM oshaughnessy_value_composite
  WHERE pe_ratio IS NOT NULL AND pb_ratio IS NOT NULL AND ps_ratio IS NOT NULL AND evs_ratio IS NOT NULL AND ev_ebitda_ratio IS NOT NULL AND shareholder_yield IS NOT NULL
)
SELECT *, CAST((pe_rank + pb_rank + ps_rank + evs_rank + ebitda_rank + yield_rank) / 6.0 AS REAL) as composite_score, CAST(ROUND(((pe_rank + pb_rank + ps_rank + evs_rank + ebitda_rank + yield_rank) / 6.0 / total_stocks) * 100, 1) AS REAL) as composite_percentile, RANK() OVER (ORDER BY (pe_rank + pb_rank + ps_rank + evs_rank + ebitda_rank + yield_rank) / 6.0 ASC) as overall_rank, CASE WHEN RANK() OVER (ORDER BY (pe_rank + pb_rank + ps_rank + evs_rank + ebitda_rank + yield_rank) / 6.0 ASC) <= 10 THEN 1 ELSE 0 END as passes_screening, 6 as metrics_available
FROM ranked
ORDER BY composite_score ASC
/* oshaughnessy_ranking(stock_id,symbol,sector,current_price,market_cap,net_income,revenue,operating_income,total_equity,shares_outstanding,total_debt,cash_and_equivalents,dividends_paid,share_repurchases,depreciation_expense,amortization_expense,enterprise_value,ebitda,pe_ratio,pb_ratio,ps_ratio,evs_ratio,ev_ebitda_ratio,shareholder_yield,data_completeness_score,pe_rank,pb_rank,ps_rank,evs_rank,ebitda_rank,yield_rank,total_stocks,composite_score,composite_percentile,overall_rank,passes_screening,metrics_available) */;
CREATE VIEW piotroski_multi_year_data AS
WITH financial_data AS (
    SELECT
        s.id as stock_id,
        s.symbol,
        s.sector,
        s.sector as industry, -- Use sector as industry for compatibility

        -- Current TTM data (most recent)
        current_income.net_income as current_net_income,
        current_income.revenue as current_revenue,
        current_income.gross_profit as current_gross_profit,
        current_income.cost_of_revenue as current_cost_of_revenue,
        current_income.interest_expense as current_interest_expense,
        current_income.shares_diluted as current_shares,
        current_income.shares_diluted as current_shares_outstanding,

        -- Current TTM balance data
        current_balance.total_assets as current_assets,
        current_balance.total_debt as current_debt,
        current_balance.total_equity as current_equity,
        current_balance.current_assets as current_current_assets,
        current_balance.current_liabilities as current_current_liabilities,
        current_balance.short_term_debt as current_short_term_debt,
        current_balance.long_term_debt as current_long_term_debt,

        -- Prior year TTM data (for year-over-year comparisons)
        prior_income.net_income as prior_net_income,
        prior_income.revenue as prior_revenue,
        prior_income.gross_profit as prior_gross_profit,
        prior_income.cost_of_revenue as prior_cost_of_revenue,
        prior_income.interest_expense as prior_interest_expense,
        prior_income.shares_diluted as prior_shares,
        prior_income.shares_diluted as prior_shares_outstanding,

        -- Prior year balance data
        prior_balance.total_assets as prior_assets,
        prior_balance.total_debt as prior_debt,
        prior_balance.total_equity as prior_equity,
        prior_balance.current_assets as prior_current_assets,
        prior_balance.current_liabilities as prior_current_liabilities,
        prior_balance.short_term_debt as prior_short_term_debt,
        prior_balance.long_term_debt as prior_long_term_debt,

        -- Current TTM cash flow data
        current_cashflow.operating_cash_flow as current_operating_cash_flow,
        current_cashflow.investing_cash_flow as current_investing_cash_flow,
        current_cashflow.financing_cash_flow as current_financing_cash_flow,
        current_cashflow.net_cash_flow as current_net_cash_flow,

        -- Prior year cash flow data
        prior_cashflow.operating_cash_flow as prior_operating_cash_flow,
        prior_cashflow.investing_cash_flow as prior_investing_cash_flow,
        prior_cashflow.financing_cash_flow as prior_financing_cash_flow,
        prior_cashflow.net_cash_flow as prior_net_cash_flow

    FROM stocks s

    -- Current TTM income data (most recent)
    LEFT JOIN (
        SELECT stock_id, net_income, revenue, gross_profit, cost_of_revenue,
               interest_expense, shares_diluted, report_date, fiscal_year,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY fiscal_year DESC, report_date DESC) as rn
        FROM income_statements
        WHERE period_type = 'TTM'
    ) current_income ON s.id = current_income.stock_id AND current_income.rn = 1

    -- Prior year TTM income data (previous year)
    LEFT JOIN (
        SELECT stock_id, net_income, revenue, gross_profit, cost_of_revenue,
               interest_expense, shares_diluted, report_date, fiscal_year,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY fiscal_year DESC, report_date DESC) as rn
        FROM income_statements
        WHERE period_type = 'TTM'
    ) prior_income ON s.id = prior_income.stock_id AND prior_income.rn = 2

    -- Current TTM balance data (most recent)
    LEFT JOIN (
        SELECT stock_id, total_assets, total_debt, total_equity,
               current_assets, current_liabilities, short_term_debt, long_term_debt, report_date, fiscal_year,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY fiscal_year DESC, report_date DESC) as rn
        FROM balance_sheets
        WHERE period_type = 'TTM'
    ) current_balance ON s.id = current_balance.stock_id AND current_balance.rn = 1

    -- Prior year TTM balance data (previous year)
    LEFT JOIN (
        SELECT stock_id, total_assets, total_debt, total_equity,
               current_assets, current_liabilities, short_term_debt, long_term_debt, report_date, fiscal_year,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY fiscal_year DESC, report_date DESC) as rn
        FROM balance_sheets
        WHERE period_type = 'TTM'
    ) prior_balance ON s.id = prior_balance.stock_id AND prior_balance.rn = 2

    -- Current TTM cash flow data (most recent)
    LEFT JOIN (
        SELECT stock_id, operating_cash_flow, investing_cash_flow,
               financing_cash_flow, net_cash_flow, report_date, fiscal_year,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY fiscal_year DESC, report_date DESC) as rn
        FROM cash_flow_statements
        WHERE period_type = 'TTM'
    ) current_cashflow ON s.id = current_cashflow.stock_id AND current_cashflow.rn = 1

    -- Prior year TTM cash flow data (previous year)
    LEFT JOIN (
        SELECT stock_id, operating_cash_flow, investing_cash_flow,
               financing_cash_flow, net_cash_flow, report_date, fiscal_year,
               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY fiscal_year DESC, report_date DESC) as rn
        FROM cash_flow_statements
        WHERE period_type = 'TTM'
    ) prior_cashflow ON s.id = prior_cashflow.stock_id AND prior_cashflow.rn = 2
)
SELECT
    fd.*,
    NULL as pb_ratio,

    -- PROFITABILITY (4 criteria)
    CASE WHEN current_net_income > 0 THEN 1 ELSE 0 END as criterion_positive_net_income,
    CASE WHEN current_operating_cash_flow > 0 THEN 1 ELSE 0 END as criterion_positive_operating_cash_flow,
    CASE
        WHEN current_net_income / NULLIF(current_assets, 0) >
             prior_net_income / NULLIF(prior_assets, 0)
             AND current_net_income IS NOT NULL AND prior_net_income IS NOT NULL
             AND current_assets IS NOT NULL AND prior_assets IS NOT NULL THEN 1
        ELSE 0
    END as criterion_improving_roa,
    CASE
        WHEN current_operating_cash_flow > current_net_income
             AND current_operating_cash_flow IS NOT NULL
             AND current_net_income IS NOT NULL THEN 1
        ELSE 0
    END as criterion_cash_flow_quality,

    -- LEVERAGE/LIQUIDITY (3 criteria)
    CASE
        WHEN current_debt / NULLIF(current_assets, 0) <
             prior_debt / NULLIF(prior_assets, 0)
             AND current_debt IS NOT NULL AND prior_debt IS NOT NULL
             AND current_assets IS NOT NULL AND prior_assets IS NOT NULL THEN 1
        ELSE 0
    END as criterion_decreasing_debt_ratio,
    CASE
        WHEN current_current_assets / NULLIF(current_current_liabilities, 0) >
             prior_current_assets / NULLIF(prior_current_liabilities, 0)
             AND current_current_assets IS NOT NULL AND prior_current_assets IS NOT NULL
             AND current_current_liabilities IS NOT NULL AND prior_current_liabilities IS NOT NULL THEN 1
        ELSE 0
    END as criterion_improving_current_ratio,
    CASE
        WHEN current_shares_outstanding <= prior_shares_outstanding
             AND current_shares_outstanding IS NOT NULL
             AND prior_shares_outstanding IS NOT NULL THEN 1
        ELSE 0
    END as criterion_no_dilution,

    -- OPERATING EFFICIENCY (2 criteria)
    CASE
        WHEN current_gross_profit / NULLIF(current_revenue, 0) >
             prior_gross_profit / NULLIF(prior_revenue, 0)
             AND current_gross_profit IS NOT NULL AND prior_gross_profit IS NOT NULL
             AND current_revenue IS NOT NULL AND prior_revenue IS NOT NULL THEN 1
        ELSE 0
    END as criterion_improving_gross_margin,
    CASE
        WHEN current_revenue / NULLIF(current_assets, 0) >
             prior_revenue / NULLIF(prior_assets, 0)
             AND current_revenue IS NOT NULL AND prior_revenue IS NOT NULL
             AND current_assets IS NOT NULL AND prior_assets IS NOT NULL THEN 1
        ELSE 0
    END as criterion_improving_asset_turnover,

    -- F-SCORE CALCULATION
    (CASE WHEN current_net_income > 0 THEN 1 ELSE 0 END +
     CASE WHEN current_operating_cash_flow > 0 THEN 1 ELSE 0 END +
     CASE
         WHEN current_net_income / NULLIF(current_assets, 0) >
              prior_net_income / NULLIF(prior_assets, 0)
              AND current_net_income IS NOT NULL AND prior_net_income IS NOT NULL
              AND current_assets IS NOT NULL AND prior_assets IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN current_operating_cash_flow > current_net_income
              AND current_operating_cash_flow IS NOT NULL
              AND current_net_income IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN current_debt / NULLIF(current_assets, 0) <
              prior_debt / NULLIF(prior_assets, 0)
              AND current_debt IS NOT NULL AND prior_debt IS NOT NULL
              AND current_assets IS NOT NULL AND prior_assets IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN current_current_assets / NULLIF(current_current_liabilities, 0) >
              prior_current_assets / NULLIF(prior_current_liabilities, 0)
              AND current_current_assets IS NOT NULL AND prior_current_assets IS NOT NULL
              AND current_current_liabilities IS NOT NULL AND prior_current_liabilities IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN current_shares_outstanding <= prior_shares_outstanding
              AND current_shares_outstanding IS NOT NULL
              AND prior_shares_outstanding IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN current_gross_profit / NULLIF(current_revenue, 0) >
              prior_gross_profit / NULLIF(prior_revenue, 0)
              AND current_gross_profit IS NOT NULL AND prior_gross_profit IS NOT NULL
              AND current_revenue IS NOT NULL AND prior_revenue IS NOT NULL THEN 1
         ELSE 0
     END +
     CASE
         WHEN current_revenue / NULLIF(current_assets, 0) >
              prior_revenue / NULLIF(prior_assets, 0)
              AND current_revenue IS NOT NULL AND prior_revenue IS NOT NULL
              AND current_assets IS NOT NULL AND prior_assets IS NOT NULL THEN 1
         ELSE 0
     END) as f_score_complete,

    -- DATA COMPLETENESS CALCULATION
    (
        CASE WHEN current_net_income IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_operating_cash_flow IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_assets IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_debt IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_current_assets IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_current_liabilities IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_shares_outstanding IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_gross_profit IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN current_revenue IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_net_income IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_assets IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_debt IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_current_assets IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_current_liabilities IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_shares_outstanding IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_gross_profit IS NOT NULL THEN 1 ELSE 0 END +
        CASE WHEN prior_revenue IS NOT NULL THEN 1 ELSE 0 END
    ) * 100 / 17 as data_completeness_score,

    -- Additional calculated metrics
    CASE WHEN current_assets > 0 THEN current_net_income / current_assets ELSE NULL END as current_roa,
    CASE WHEN current_assets > 0 THEN current_debt / current_assets ELSE NULL END as current_debt_ratio,
    CASE WHEN current_current_liabilities > 0 THEN current_current_assets / current_current_liabilities ELSE NULL END as current_current_ratio,
    CASE WHEN current_revenue > 0 AND current_gross_profit IS NOT NULL THEN current_gross_profit / current_revenue ELSE NULL END as current_gross_margin,
    CASE WHEN current_assets > 0 THEN current_revenue / current_assets ELSE NULL END as current_asset_turnover

FROM financial_data fd
WHERE fd.current_net_income IS NOT NULL
  AND fd.current_operating_cash_flow IS NOT NULL
/* piotroski_multi_year_data(stock_id,symbol,sector,industry,current_net_income,current_revenue,current_gross_profit,current_cost_of_revenue,current_interest_expense,current_shares,current_shares_outstanding,current_assets,current_debt,current_equity,current_current_assets,current_current_liabilities,current_short_term_debt,current_long_term_debt,prior_net_income,prior_revenue,prior_gross_profit,prior_cost_of_revenue,prior_interest_expense,prior_shares,prior_shares_outstanding,prior_assets,prior_debt,prior_equity,prior_current_assets,prior_current_liabilities,prior_short_term_debt,prior_long_term_debt,current_operating_cash_flow,current_investing_cash_flow,current_financing_cash_flow,current_net_cash_flow,prior_operating_cash_flow,prior_investing_cash_flow,prior_financing_cash_flow,prior_net_cash_flow,pb_ratio,criterion_positive_net_income,criterion_positive_operating_cash_flow,criterion_improving_roa,criterion_cash_flow_quality,criterion_decreasing_debt_ratio,criterion_improving_current_ratio,criterion_no_dilution,criterion_improving_gross_margin,criterion_improving_asset_turnover,f_score_complete,data_completeness_score,current_roa,current_debt_ratio,current_current_ratio,current_gross_margin,current_asset_turnover) */;
CREATE VIEW piotroski_screening_results AS
SELECT
    *,
    CASE
        WHEN f_score_complete >= 6
             AND data_completeness_score >= 60
        THEN 1
        ELSE 0
    END as passes_screening,
    CASE
        WHEN data_completeness_score >= 90 THEN 'High'
        WHEN data_completeness_score >= 70 THEN 'Medium'
        WHEN data_completeness_score >= 50 THEN 'Low'
        ELSE 'Very Low'
    END as confidence_level,
    CASE
        WHEN f_score_complete >= 7 THEN 'Excellent'
        WHEN f_score_complete >= 5 THEN 'Good'
        WHEN f_score_complete >= 3 THEN 'Average'
        WHEN f_score_complete >= 1 THEN 'Poor'
        ELSE 'Very Poor'
    END as f_score_interpretation
FROM piotroski_multi_year_data
ORDER BY f_score_complete DESC, data_completeness_score DESC
/* piotroski_screening_results(stock_id,symbol,sector,industry,current_net_income,current_revenue,current_gross_profit,current_cost_of_revenue,current_interest_expense,current_shares,current_shares_outstanding,current_assets,current_debt,current_equity,current_current_assets,current_current_liabilities,current_short_term_debt,current_long_term_debt,prior_net_income,prior_revenue,prior_gross_profit,prior_cost_of_revenue,prior_interest_expense,prior_shares,prior_shares_outstanding,prior_assets,prior_debt,prior_equity,prior_current_assets,prior_current_liabilities,prior_short_term_debt,prior_long_term_debt,current_operating_cash_flow,current_investing_cash_flow,current_financing_cash_flow,current_net_cash_flow,prior_operating_cash_flow,prior_investing_cash_flow,prior_financing_cash_flow,prior_net_cash_flow,pb_ratio,criterion_positive_net_income,criterion_positive_operating_cash_flow,criterion_improving_roa,criterion_cash_flow_quality,criterion_decreasing_debt_ratio,criterion_improving_current_ratio,criterion_no_dilution,criterion_improving_gross_margin,criterion_improving_asset_turnover,f_score_complete,data_completeness_score,current_roa,current_debt_ratio,current_current_ratio,current_gross_margin,current_asset_turnover,passes_screening,confidence_level,f_score_interpretation) */;
