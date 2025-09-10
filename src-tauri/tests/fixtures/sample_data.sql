-- Test database setup with minimal dataset for fast execution
-- ⚠️  WARNING: THIS IS ONLY FOR ISOLATED TEST DATABASES - NEVER RUN ON PRODUCTION
-- This file should only be executed on temporary test databases created in /tmp

-- This assumes we're working with a FRESH, EMPTY test database created by TestDatabase::new()
-- The migrations have already created the schema, so we just insert test data

-- Insert test stocks (10 stocks - mix of complete/incomplete data)
INSERT INTO stocks (id, symbol, company_name, sector, industry, simfin_id, created_at) VALUES
(1, 'AAPL', 'Apple Inc.', 'Technology', 'Consumer Electronics', 111052, '2023-01-01 00:00:00'),
(2, 'MSFT', 'Microsoft Corporation', 'Technology', 'Software', 114991, '2023-01-01 00:00:00'),
(3, 'GOOGL', 'Alphabet Inc.', 'Technology', 'Internet Services', 101302, '2023-01-01 00:00:00'),
(4, 'TSLA', 'Tesla Inc.', 'Consumer Cyclical', 'Auto Manufacturers', 417688, '2023-01-01 00:00:00'),
(5, 'AMZN', 'Amazon.com Inc.', 'Consumer Cyclical', 'Internet Retail', 100332, '2023-01-01 00:00:00'),
(6, 'NVDA', 'NVIDIA Corporation', 'Technology', 'Semiconductors', 110705, '2023-01-01 00:00:00'),
(7, 'META', 'Meta Platforms Inc.', 'Technology', 'Internet Services', 110341, '2023-01-01 00:00:00'),
(8, 'NFLX', 'Netflix Inc.', 'Communication Services', 'Entertainment', 114285, '2023-01-01 00:00:00'),
(9, 'UNPROFITABLE', 'Unprofitable Corp', 'Technology', 'Software', 999001, '2023-01-01 00:00:00'),
(10, 'MINIMAL', 'Minimal Data Corp', 'Technology', 'Software', 999002, '2023-01-01 00:00:00');

-- Insert S&P 500 symbols (first 8 are S&P 500, last 2 are not)
INSERT INTO sp500_symbols (symbol, created_at) VALUES
('AAPL', '2023-01-01 00:00:00'),
('MSFT', '2023-01-01 00:00:00'),
('GOOGL', '2023-01-01 00:00:00'),
('TSLA', '2023-01-01 00:00:00'),
('AMZN', '2023-01-01 00:00:00'),
('NVDA', '2023-01-01 00:00:00'),
('META', '2023-01-01 00:00:00'),
('NFLX', '2023-01-01 00:00:00');

-- Insert daily prices (50 records per stock for 2024)
-- AAPL prices (complete data)
INSERT INTO daily_prices (stock_id, date, open_price, high_price, low_price, close_price, volume, pe_ratio, shares_outstanding) VALUES
(1, '2024-01-01', 185.50, 186.20, 184.80, 185.90, 45000000, 28.5, 15500000000),
(1, '2024-01-02', 186.00, 187.50, 185.90, 187.20, 42000000, 28.7, 15500000000),
(1, '2024-01-03', 187.20, 188.00, 186.50, 187.80, 38000000, 28.8, 15500000000),
(1, '2024-01-04', 187.80, 189.20, 187.10, 188.50, 41000000, 28.9, 15500000000),
(1, '2024-01-05', 188.50, 190.00, 187.80, 189.70, 44000000, 29.1, 15500000000);

-- MSFT prices (complete data)
INSERT INTO daily_prices (stock_id, date, open_price, high_price, low_price, close_price, volume, pe_ratio, shares_outstanding) VALUES
(2, '2024-01-01', 372.50, 374.20, 371.80, 373.90, 25000000, 32.5, 7400000000),
(2, '2024-01-02', 374.00, 376.50, 373.90, 375.20, 23000000, 32.6, 7400000000),
(2, '2024-01-03', 375.20, 377.00, 374.50, 376.80, 21000000, 32.8, 7400000000),
(2, '2024-01-04', 376.80, 378.20, 375.10, 377.50, 24000000, 32.9, 7400000000),
(2, '2024-01-05', 377.50, 380.00, 376.80, 379.70, 26000000, 33.1, 7400000000);

-- TSLA prices (complete data but volatile)
INSERT INTO daily_prices (stock_id, date, open_price, high_price, low_price, close_price, volume, pe_ratio, shares_outstanding) VALUES
(4, '2024-01-01', 238.50, 245.20, 235.80, 242.90, 85000000, 45.2, 3170000000),
(4, '2024-01-02', 243.00, 248.50, 240.90, 246.20, 82000000, 45.8, 3170000000),
(4, '2024-01-03', 246.20, 250.00, 244.50, 248.80, 78000000, 46.3, 3170000000),
(4, '2024-01-04', 248.80, 252.20, 246.10, 249.50, 81000000, 46.4, 3170000000),
(4, '2024-01-05', 249.50, 255.00, 247.80, 253.70, 86000000, 47.2, 3170000000);

-- UNPROFITABLE stock (has prices but negative earnings)
INSERT INTO daily_prices (stock_id, date, open_price, high_price, low_price, close_price, volume, pe_ratio, shares_outstanding) VALUES
(9, '2024-01-01', 12.50, 13.20, 12.20, 12.90, 5000000, NULL, 100000000),
(9, '2024-01-02', 12.90, 13.50, 12.80, 13.20, 4500000, NULL, 100000000),
(9, '2024-01-03', 13.20, 13.80, 12.90, 13.50, 4800000, NULL, 100000000);

-- MINIMAL stock (only 1 day of data)
INSERT INTO daily_prices (stock_id, date, open_price, high_price, low_price, close_price, volume, pe_ratio, shares_outstanding) VALUES
(10, '2024-01-01', 5.50, 5.80, 5.40, 5.70, 1000000, 12.5, 50000000);

-- Insert TTM income statements (for P/S and EV/S calculations)
INSERT INTO income_statements (stock_id, period_type, report_date, fiscal_year, revenue, net_income, shares_diluted, currency, simfin_id) VALUES
(1, 'TTM', '2023-12-31', 2023, 394328000000, 96995000000, 15550321000, 'USD', 111052), -- AAPL: profitable
(2, 'TTM', '2023-12-31', 2023, 211915000000, 72361000000, 7430000000, 'USD', 114991), -- MSFT: profitable  
(4, 'TTM', '2023-12-31', 2023, 96773000000, 14997000000, 3178000000, 'USD', 417688), -- TSLA: profitable
(9, 'TTM', '2023-12-31', 2023, 50000000, -10000000, 100000000, 'USD', 999001); -- UNPROFITABLE: negative earnings

-- Insert TTM balance sheets (for EV/S calculations)
INSERT INTO balance_sheets (stock_id, period_type, report_date, fiscal_year, cash_and_equivalents, total_debt, currency, simfin_id) VALUES
(1, 'TTM', '2023-12-31', 2023, 29965000000, 123930000000, 'USD', 111052), -- AAPL
(2, 'TTM', '2023-12-31', 2023, 34704000000, 97818000000, 'USD', 114991), -- MSFT
(4, 'TTM', '2023-12-31', 2023, 13290000000, 9548000000, 'USD', 417688), -- TSLA
(9, 'TTM', '2023-12-31', 2023, 5000000, 25000000, 'USD', 999001); -- UNPROFITABLE

-- Insert calculated daily valuation ratios (P/S, EV/S)
INSERT INTO daily_valuation_ratios (stock_id, date, price, market_cap, enterprise_value, ps_ratio_ttm, evs_ratio_ttm, revenue_ttm, data_completeness_score, last_financial_update) VALUES
-- AAPL ratios (market cap = 185.90 * 15.5B = $2.88T)
(1, '2024-01-01', 185.90, 2881450000000, 2975415000000, 7.31, 7.54, 394328000000, 100, '2023-12-31'),
(1, '2024-01-02', 187.20, 2901600000000, 2995565000000, 7.36, 7.59, 394328000000, 100, '2023-12-31'),
(1, '2024-01-05', 189.70, 2940350000000, 3034300000000, 7.46, 7.69, 394328000000, 100, '2023-12-31'),

-- MSFT ratios (market cap = 373.90 * 7.4B = $2.77T)
(2, '2024-01-01', 373.90, 2766860000000, 2829974000000, 13.06, 13.36, 211915000000, 100, '2023-12-31'),
(2, '2024-01-02', 375.20, 2776480000000, 2839594000000, 13.10, 13.40, 211915000000, 100, '2023-12-31'),
(2, '2024-01-05', 379.70, 2809780000000, 2872894000000, 13.26, 13.56, 211915000000, 100, '2023-12-31'),

-- TSLA ratios (market cap = 242.90 * 3.17B = $770B)  
(4, '2024-01-01', 242.90, 770593000000, 776851000000, 7.96, 8.03, 96773000000, 100, '2023-12-31'),
(4, '2024-01-02', 246.20, 780454000000, 786712000000, 8.06, 8.13, 96773000000, 100, '2023-12-31'),
(4, '2024-01-05', 253.70, 804723000000, 810981000000, 8.31, 8.38, 96773000000, 100, '2023-12-31'),

-- UNPROFITABLE ratios (market cap = 12.90 * 100M = $1.29B)
(9, '2024-01-01', 12.90, 1290000000, 1310000000, 25.80, 26.20, 50000000, 75, '2023-12-31'),
(9, '2024-01-02', 13.20, 1320000000, 1340000000, 26.40, 26.80, 50000000, 75, '2023-12-31');

-- Note: No ratios for MINIMAL stock due to missing financial data