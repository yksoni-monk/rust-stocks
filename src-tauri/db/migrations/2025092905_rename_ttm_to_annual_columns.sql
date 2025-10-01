-- Migration: Rename TTM columns to Annual columns in daily_valuation_ratios
-- Purpose: Convert all TTM data usage to Annual data for consistency

-- Create new table with Annual column names
CREATE TABLE daily_valuation_ratios_new (
    stock_id INT NOT NULL,
    date NUM NOT NULL,
    price REAL,
    market_cap REAL,
    enterprise_value REAL,
    ps_ratio_annual REAL,
    evs_ratio_annual REAL,
    pb_ratio_annual REAL,
    pcf_ratio_annual REAL,
    ev_ebitda_ratio_annual REAL,
    shareholder_yield_annual REAL,
    revenue_annual REAL,
    data_completeness_score INT,
    last_financial_update NUM,
    PRIMARY KEY (stock_id, date),
    FOREIGN KEY (stock_id) REFERENCES stocks(id)
);

-- Copy data from old table to new table
INSERT INTO daily_valuation_ratios_new (
    stock_id, date, price, market_cap, enterprise_value,
    ps_ratio_annual, evs_ratio_annual, pb_ratio_annual, pcf_ratio_annual, 
    ev_ebitda_ratio_annual, shareholder_yield_annual, revenue_annual,
    data_completeness_score, last_financial_update
)
SELECT 
    stock_id, date, price, market_cap, enterprise_value,
    ps_ratio_ttm, evs_ratio_ttm, pb_ratio_ttm, pcf_ratio_annual, 
    ev_ebitda_ratio_annual, shareholder_yield_annual, revenue_ttm,
    data_completeness_score, last_financial_update
FROM daily_valuation_ratios;

-- Drop old table
DROP TABLE daily_valuation_ratios;

-- Rename new table
ALTER TABLE daily_valuation_ratios_new RENAME TO daily_valuation_ratios;

-- Recreate indexes
CREATE INDEX IF NOT EXISTS idx_daily_ratios_stock_date ON daily_valuation_ratios(stock_id, date);
CREATE INDEX IF NOT EXISTS idx_daily_ratios_ps_annual ON daily_valuation_ratios(ps_ratio_annual);
CREATE INDEX IF NOT EXISTS idx_daily_ratios_evs_annual ON daily_valuation_ratios(evs_ratio_annual);
CREATE INDEX IF NOT EXISTS idx_daily_ratios_pb_annual ON daily_valuation_ratios(pb_ratio_annual);
CREATE INDEX IF NOT EXISTS idx_daily_ratios_pcf_annual ON daily_valuation_ratios(pcf_ratio_annual);
CREATE INDEX IF NOT EXISTS idx_daily_ratios_ev_ebitda_annual ON daily_valuation_ratios(ev_ebitda_ratio_annual);
CREATE INDEX IF NOT EXISTS idx_daily_ratios_shareholder_yield_annual ON daily_valuation_ratios(shareholder_yield_annual);

