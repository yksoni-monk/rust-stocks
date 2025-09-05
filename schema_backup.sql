CREATE TABLE stocks (
                id INTEGER PRIMARY KEY,
                symbol TEXT UNIQUE NOT NULL,
                company_name TEXT NOT NULL,
                sector TEXT,
                industry TEXT,
                market_cap REAL,
                status TEXT DEFAULT 'active',
                first_trading_date DATE,
                last_updated DATETIME,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
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
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (stock_id) REFERENCES stocks(id),
                UNIQUE(stock_id, date)
            );
CREATE TABLE metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
CREATE INDEX idx_daily_prices_stock_date ON daily_prices(stock_id, date);
CREATE INDEX idx_daily_prices_date ON daily_prices(date);
CREATE INDEX idx_stocks_symbol ON stocks(symbol);
CREATE INDEX idx_stocks_company_name ON stocks(company_name);
CREATE TABLE _sqlx_migrations (
    version BIGINT PRIMARY KEY,
    description TEXT NOT NULL,
    installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    success BOOLEAN NOT NULL,
    checksum BLOB NOT NULL,
    execution_time BIGINT NOT NULL
);
