-- Migration: Add Graham Value Screening Infrastructure
-- Created: 2025-09-17
-- Purpose: Implement Benjamin Graham-inspired value screening with comprehensive metrics

-- Graham screening results table
CREATE TABLE graham_screening_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    symbol TEXT NOT NULL,
    screening_date DATE NOT NULL,
    
    -- Core Graham Metrics
    pe_ratio REAL,
    pb_ratio REAL,
    pe_pb_product REAL,
    dividend_yield REAL,
    debt_to_equity REAL,
    profit_margin REAL,
    revenue_growth_1y REAL,
    revenue_growth_3y REAL,
    
    -- Additional Quality Metrics
    current_ratio REAL,
    quick_ratio REAL,
    interest_coverage_ratio REAL,
    return_on_equity REAL,
    return_on_assets REAL,
    
    -- Screening Filter Results
    passes_earnings_filter BOOLEAN DEFAULT 0,
    passes_pe_filter BOOLEAN DEFAULT 0,
    passes_pb_filter BOOLEAN DEFAULT 0,
    passes_pe_pb_combined BOOLEAN DEFAULT 0,
    passes_dividend_filter BOOLEAN DEFAULT 0,
    passes_debt_filter BOOLEAN DEFAULT 0,
    passes_quality_filter BOOLEAN DEFAULT 0,
    passes_growth_filter BOOLEAN DEFAULT 0,
    passes_all_filters BOOLEAN DEFAULT 0,
    
    -- Scoring and Ranking
    graham_score REAL,
    value_rank INTEGER,
    quality_score REAL,
    safety_score REAL,
    
    -- Financial Data Snapshot (for historical tracking)
    current_price REAL,
    market_cap REAL,
    shares_outstanding REAL,
    net_income REAL,
    total_equity REAL,
    total_debt REAL,
    revenue REAL,
    
    -- Reasoning and Context
    reasoning TEXT,
    sector TEXT,
    industry TEXT,
    
    -- Metadata
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, screening_date)
);

-- Graham screening criteria presets table
CREATE TABLE graham_screening_presets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    
    -- Core Criteria
    max_pe_ratio REAL NOT NULL DEFAULT 15.0,
    max_pb_ratio REAL NOT NULL DEFAULT 1.5,
    max_pe_pb_product REAL NOT NULL DEFAULT 22.5,
    min_dividend_yield REAL NOT NULL DEFAULT 2.0,
    max_debt_to_equity REAL NOT NULL DEFAULT 1.0,
    min_profit_margin REAL NOT NULL DEFAULT 5.0,
    min_revenue_growth_1y REAL NOT NULL DEFAULT 0.0,
    min_revenue_growth_3y REAL NOT NULL DEFAULT 0.0,
    
    -- Additional Quality Filters
    min_current_ratio REAL DEFAULT 2.0,
    min_interest_coverage REAL DEFAULT 2.5,
    min_roe REAL DEFAULT 10.0,
    require_positive_earnings BOOLEAN DEFAULT 1,
    require_dividend BOOLEAN DEFAULT 0,
    
    -- Market Cap Filters
    min_market_cap REAL DEFAULT 1000000000.0, -- $1B minimum
    max_market_cap REAL, -- NULL = no maximum
    
    -- Sector Exclusions (JSON array of sector names)
    excluded_sectors TEXT DEFAULT '[]',
    
    -- Metadata
    is_default BOOLEAN DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Performance indexes for efficient querying
CREATE INDEX idx_graham_screening_symbol_date ON graham_screening_results(symbol, screening_date);
CREATE INDEX idx_graham_screening_date ON graham_screening_results(screening_date);
CREATE INDEX idx_graham_screening_passes_all ON graham_screening_results(passes_all_filters, graham_score);
CREATE INDEX idx_graham_screening_score ON graham_screening_results(graham_score DESC) WHERE passes_all_filters = 1;
CREATE INDEX idx_graham_screening_pe_ratio ON graham_screening_results(pe_ratio) WHERE pe_ratio IS NOT NULL;
CREATE INDEX idx_graham_screening_pb_ratio ON graham_screening_results(pb_ratio) WHERE pb_ratio IS NOT NULL;
CREATE INDEX idx_graham_screening_stock_id ON graham_screening_results(stock_id);

-- Index for preset lookups
CREATE INDEX idx_graham_presets_name ON graham_screening_presets(name);
CREATE INDEX idx_graham_presets_default ON graham_screening_presets(is_default) WHERE is_default = 1;

-- Insert default Graham screening presets
INSERT INTO graham_screening_presets (
    name, 
    description,
    max_pe_ratio,
    max_pb_ratio,
    max_pe_pb_product,
    min_dividend_yield,
    max_debt_to_equity,
    min_profit_margin,
    min_revenue_growth_1y,
    min_current_ratio,
    min_interest_coverage,
    min_roe,
    require_positive_earnings,
    require_dividend,
    min_market_cap,
    is_default
) VALUES 
(
    'Classic Graham',
    'Traditional Benjamin Graham criteria from The Intelligent Investor',
    15.0,      -- max P/E
    1.5,       -- max P/B  
    22.5,      -- max P/E × P/B
    2.0,       -- min dividend yield %
    1.0,       -- max debt/equity
    5.0,       -- min profit margin %
    0.0,       -- min revenue growth (stability over growth)
    2.0,       -- min current ratio
    2.5,       -- min interest coverage
    10.0,      -- min ROE %
    1,         -- require positive earnings
    1,         -- require dividend
    1000000000.0, -- $1B min market cap
    1          -- is default
),
(
    'Modern Graham',
    'Graham principles adapted for modern market conditions',
    20.0,      -- higher P/E tolerance
    2.0,       -- higher P/B tolerance
    30.0,      -- adjusted combined ratio
    1.5,       -- lower dividend requirement
    1.5,       -- higher debt tolerance
    3.0,       -- lower margin requirement
    5.0,       -- modest growth requirement
    1.5,       -- lower current ratio
    2.0,       -- lower interest coverage
    8.0,       -- lower ROE requirement
    1,         -- require positive earnings
    0,         -- don't require dividend
    500000000.0,  -- $500M min market cap
    0          -- not default
),
(
    'Defensive Investor',
    'Ultra-conservative approach for risk-averse investors',
    12.0,      -- very low P/E
    1.2,       -- very low P/B
    15.0,      -- very conservative combined
    3.0,       -- high dividend requirement
    0.5,       -- very low debt
    8.0,       -- high profit margins
    0.0,       -- no growth requirement (stability)
    2.5,       -- high current ratio
    5.0,       -- high interest coverage
    15.0,      -- high ROE requirement
    1,         -- require positive earnings
    1,         -- require dividend
    2000000000.0, -- $2B min market cap (large caps)
    0          -- not default
),
(
    'Enterprising Investor',
    'More aggressive Graham approach for active investors',
    25.0,      -- higher P/E for growth
    3.0,       -- higher P/B tolerance
    40.0,      -- higher combined tolerance
    1.0,       -- lower dividend requirement
    2.0,       -- higher debt tolerance
    2.0,       -- lower margin requirement
    10.0,      -- meaningful growth requirement
    1.2,       -- lower current ratio
    1.5,       -- lower interest coverage
    5.0,       -- lower ROE requirement
    1,         -- require positive earnings
    0,         -- don't require dividend
    100000000.0,  -- $100M min market cap
    0          -- not default
);

-- Create view for easy access to latest screening results with stock details
CREATE VIEW v_latest_graham_screening AS
SELECT 
    gsr.*,
    s.symbol as stock_symbol,
    s.company_name,
    s.sector,
    s.industry,
    s.is_sp500,
    sp.latest_price,
    sp.market_cap as current_market_cap
FROM graham_screening_results gsr
JOIN stocks s ON gsr.stock_id = s.id
LEFT JOIN stock_prices sp ON s.id = sp.stock_id
WHERE gsr.screening_date = (
    SELECT MAX(screening_date) 
    FROM graham_screening_results gsr2 
    WHERE gsr2.stock_id = gsr.stock_id
)
ORDER BY gsr.graham_score DESC;

-- Create view for Graham screening summary statistics
CREATE VIEW v_graham_screening_stats AS
SELECT 
    screening_date,
    COUNT(*) as total_screened,
    COUNT(CASE WHEN passes_all_filters = 1 THEN 1 END) as passed_all_filters,
    COUNT(CASE WHEN passes_earnings_filter = 1 THEN 1 END) as passed_earnings,
    COUNT(CASE WHEN passes_pe_filter = 1 THEN 1 END) as passed_pe,
    COUNT(CASE WHEN passes_pb_filter = 1 THEN 1 END) as passed_pb,
    COUNT(CASE WHEN passes_dividend_filter = 1 THEN 1 END) as passed_dividend,
    COUNT(CASE WHEN passes_debt_filter = 1 THEN 1 END) as passed_debt,
    COUNT(CASE WHEN passes_quality_filter = 1 THEN 1 END) as passed_quality,
    COUNT(CASE WHEN passes_growth_filter = 1 THEN 1 END) as passed_growth,
    AVG(pe_ratio) as avg_pe_ratio,
    AVG(pb_ratio) as avg_pb_ratio,
    AVG(graham_score) as avg_graham_score,
    MIN(graham_score) as min_graham_score,
    MAX(graham_score) as max_graham_score
FROM graham_screening_results
GROUP BY screening_date
ORDER BY screening_date DESC;