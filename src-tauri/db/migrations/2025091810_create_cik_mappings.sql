-- Create CIK mappings table for EDGAR data extraction
CREATE TABLE IF NOT EXISTS cik_mappings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cik TEXT NOT NULL UNIQUE,
    stock_id INTEGER NOT NULL,
    symbol TEXT NOT NULL,
    entity_name TEXT,
    mapping_source TEXT DEFAULT 'manual',  -- 'manual', 'fuzzy', 'api'
    confidence_score REAL DEFAULT 1.0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (stock_id) REFERENCES stocks(id)
);

-- Create indices for performance
CREATE INDEX IF NOT EXISTS idx_cik_mappings_cik ON cik_mappings(cik);
CREATE INDEX IF NOT EXISTS idx_cik_mappings_stock_id ON cik_mappings(stock_id);
CREATE INDEX IF NOT EXISTS idx_cik_mappings_symbol ON cik_mappings(symbol);

-- Insert manual CIK mappings for major S&P 500 companies
INSERT OR IGNORE INTO cik_mappings (cik, stock_id, symbol, entity_name, mapping_source, confidence_score)
SELECT 
    cik_data.cik,
    s.id,
    s.symbol,
    cik_data.entity_name,
    'manual',
    1.0
FROM (
    SELECT '320193' AS cik, 'AAPL' AS symbol, 'Apple Inc.' AS entity_name
    UNION ALL SELECT '789019', 'MSFT', 'Microsoft Corporation'
    UNION ALL SELECT '1652044', 'GOOGL', 'Alphabet Inc.'
    UNION ALL SELECT '1018724', 'AMZN', 'Amazon.com Inc.'
    UNION ALL SELECT '1045810', 'NVDA', 'NVIDIA Corporation'
    UNION ALL SELECT '1318605', 'TSLA', 'Tesla Inc.'
    UNION ALL SELECT '1326801', 'META', 'Meta Platforms Inc.'
    UNION ALL SELECT '1065280', 'NFLX', 'Netflix Inc.'
    UNION ALL SELECT '2488', 'AMD', 'Advanced Micro Devices Inc.'
    UNION ALL SELECT '1108524', 'CRM', 'Salesforce Inc.'
    UNION ALL SELECT '796343', 'ADBE', 'Adobe Inc.'
    UNION ALL SELECT '886982', 'ORCL', 'Oracle Corporation'
    UNION ALL SELECT '51143', 'IBM', 'International Business Machines Corporation'
    UNION ALL SELECT '858877', 'INTC', 'Intel Corporation'
    UNION ALL SELECT '1341439', 'CSCO', 'Cisco Systems Inc.'
    UNION ALL SELECT '201527', 'TXN', 'Texas Instruments Inc.'
    UNION ALL SELECT '40545', 'QCOM', 'Qualcomm Inc.'
    UNION ALL SELECT '19617', 'JPM', 'JPMorgan Chase & Co.'
    UNION ALL SELECT '70858', 'BAC', 'Bank of America Corporation'
    UNION ALL SELECT '72971', 'WFC', 'Wells Fargo & Company'
    UNION ALL SELECT '886982', 'GS', 'Goldman Sachs Group Inc.'
    UNION ALL SELECT '1067983', 'BRK.A', 'Berkshire Hathaway Inc.'
    UNION ALL SELECT '200406', 'JNJ', 'Johnson & Johnson'
    UNION ALL SELECT '78003', 'PFE', 'Pfizer Inc.'
    UNION ALL SELECT '1551152', 'ABBV', 'AbbVie Inc.'
    UNION ALL SELECT '59478', 'MRK', 'Merck & Co. Inc.'
    UNION ALL SELECT '731766', 'UNH', 'UnitedHealth Group Inc.'
    UNION ALL SELECT '1800', 'ABT', 'Abbott Laboratories'
    UNION ALL SELECT '97745', 'TMO', 'Thermo Fisher Scientific Inc.'
    UNION ALL SELECT '21344', 'KO', 'Coca-Cola Company'
    UNION ALL SELECT '77476', 'PEP', 'PepsiCo Inc.'
    UNION ALL SELECT '104169', 'WMT', 'Walmart Inc.'
    UNION ALL SELECT '80424', 'PG', 'Procter & Gamble Company'
    UNION ALL SELECT '354950', 'HD', 'Home Depot Inc.'
    UNION ALL SELECT '63908', 'MCD', 'McDonald''s Corporation'
    UNION ALL SELECT '12927', 'BA', 'Boeing Company'
    UNION ALL SELECT '18230', 'CAT', 'Caterpillar Inc.'
    UNION ALL SELECT '40545', 'GE', 'General Electric Company'
    UNION ALL SELECT '34088', 'XOM', 'Exxon Mobil Corporation'
    UNION ALL SELECT '93410', 'CVX', 'Chevron Corporation'
    UNION ALL SELECT '732712', 'VZ', 'Verizon Communications Inc.'
    UNION ALL SELECT '732717', 'T', 'AT&T Inc.'
    UNION ALL SELECT '1364742', 'PYPL', 'PayPal Holdings Inc.'
) AS cik_data
INNER JOIN stocks s ON s.symbol = cik_data.symbol
WHERE s.is_sp500 = 1;