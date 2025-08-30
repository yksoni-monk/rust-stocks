use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{Connection, params};
use std::sync::{Arc, Mutex};
use tracing::{info, warn, error};

use crate::models::{Stock, DailyPrice, StockStatus, SystemMetadata};

#[derive(Clone)]
pub struct DatabaseManager {
    connection: Arc<Mutex<Connection>>,
}

impl DatabaseManager {
    /// Create a new DatabaseManager with the given database path
    pub fn new(database_path: &str) -> Result<Self> {
        let conn = Connection::open(database_path)?;
        let db = DatabaseManager {
            connection: Arc::new(Mutex::new(conn)),
        };
        
        // Run migrations
        db.run_migrations()?;
        info!("Database initialized at {}", database_path);
        
        Ok(db)
    }

    /// Run database migrations
    fn run_migrations(&self) -> Result<()> {
        let conn = self.connection.lock().unwrap();
        
        // Create stocks table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS stocks (
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
            )",
            [],
        )?;

        // Create daily_prices table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS daily_prices (
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
            )",
            [],
        )?;

        // Create metadata table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // Create indexes for performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_daily_prices_stock_date ON daily_prices(stock_id, date)",
            [],
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_daily_prices_date ON daily_prices(date)",
            [],
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_stocks_symbol ON stocks(symbol)",
            [],
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_stocks_company_name ON stocks(company_name)",
            [],
        )?;

        info!("Database migrations completed successfully");
        Ok(())
    }

    /// Insert or update a stock
    pub fn upsert_stock(&self, stock: &Stock) -> Result<i64> {
        let conn = self.connection.lock().unwrap();
        
        let status_str = match stock.status {
            StockStatus::Active => "active",
            StockStatus::Delisted => "delisted", 
            StockStatus::Suspended => "suspended",
        };

        conn.execute(
            "INSERT OR REPLACE INTO stocks (
                symbol, company_name, sector, industry, market_cap, status,
                first_trading_date, last_updated
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                stock.symbol,
                stock.company_name,
                stock.sector,
                stock.industry,
                stock.market_cap,
                status_str,
                stock.first_trading_date,
                stock.last_updated
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Get stock by symbol
    pub fn get_stock_by_symbol(&self, symbol: &str) -> Result<Option<Stock>> {
        let conn = self.connection.lock().unwrap();
        
        let mut stmt = conn.prepare(
            "SELECT id, symbol, company_name, sector, industry, market_cap, 
                    status, first_trading_date, last_updated 
             FROM stocks WHERE symbol = ?1"
        )?;

        let stock = stmt.query_row(params![symbol], |row| {
            let status_str: String = row.get(6)?;
            let status = match status_str.as_str() {
                "active" => StockStatus::Active,
                "delisted" => StockStatus::Delisted,
                "suspended" => StockStatus::Suspended,
                _ => StockStatus::Active,
            };

            Ok(Stock {
                id: Some(row.get(0)?),
                symbol: row.get(1)?,
                company_name: row.get(2)?,
                sector: row.get(3)?,
                industry: row.get(4)?,
                market_cap: row.get(5)?,
                status,
                first_trading_date: row.get(7)?,
                last_updated: row.get(8)?,
            })
        });

        match stock {
            Ok(stock) => Ok(Some(stock)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get all active stocks
    pub fn get_active_stocks(&self) -> Result<Vec<Stock>> {
        let conn = self.connection.lock().unwrap();
        
        let mut stmt = conn.prepare(
            "SELECT id, symbol, company_name, sector, industry, market_cap, 
                    status, first_trading_date, last_updated 
             FROM stocks WHERE status = 'active'
             ORDER BY symbol"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Stock {
                id: Some(row.get(0)?),
                symbol: row.get(1)?,
                company_name: row.get(2)?,
                sector: row.get(3)?,
                industry: row.get(4)?,
                market_cap: row.get(5)?,
                status: StockStatus::Active,
                first_trading_date: row.get(7)?,
                last_updated: row.get(8)?,
            })
        })?;

        let mut stocks = Vec::new();
        for row in rows {
            stocks.push(row?);
        }

        Ok(stocks)
    }

    /// Insert daily price data
    pub fn insert_daily_price(&self, price: &DailyPrice) -> Result<()> {
        let conn = self.connection.lock().unwrap();
        
        conn.execute(
            "INSERT OR REPLACE INTO daily_prices (
                stock_id, date, open_price, high_price, low_price, close_price,
                volume, pe_ratio, market_cap, dividend_yield
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                price.stock_id,
                price.date,
                price.open_price,
                price.high_price,
                price.low_price,
                price.close_price,
                price.volume,
                price.pe_ratio,
                price.market_cap,
                price.dividend_yield
            ],
        )?;

        Ok(())
    }

    /// Get latest price for a stock
    pub fn get_latest_price(&self, stock_id: i64) -> Result<Option<DailyPrice>> {
        let conn = self.connection.lock().unwrap();
        
        let mut stmt = conn.prepare(
            "SELECT id, stock_id, date, open_price, high_price, low_price, close_price,
                    volume, pe_ratio, market_cap, dividend_yield
             FROM daily_prices 
             WHERE stock_id = ?1 
             ORDER BY date DESC 
             LIMIT 1"
        )?;

        let price = stmt.query_row(params![stock_id], |row| {
            Ok(DailyPrice {
                id: Some(row.get(0)?),
                stock_id: row.get(1)?,
                date: row.get(2)?,
                open_price: row.get(3)?,
                high_price: row.get(4)?,
                low_price: row.get(5)?,
                close_price: row.get(6)?,
                volume: row.get(7)?,
                pe_ratio: row.get(8)?,
                market_cap: row.get(9)?,
                dividend_yield: row.get(10)?,
            })
        });

        match price {
            Ok(price) => Ok(Some(price)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get price for a specific date
    pub fn get_price_on_date(&self, stock_id: i64, date: NaiveDate) -> Result<Option<DailyPrice>> {
        let conn = self.connection.lock().unwrap();
        
        let mut stmt = conn.prepare(
            "SELECT id, stock_id, date, open_price, high_price, low_price, close_price,
                    volume, pe_ratio, market_cap, dividend_yield
             FROM daily_prices 
             WHERE stock_id = ?1 AND date = ?2"
        )?;

        let price = stmt.query_row(params![stock_id, date], |row| {
            Ok(DailyPrice {
                id: Some(row.get(0)?),
                stock_id: row.get(1)?,
                date: row.get(2)?,
                open_price: row.get(3)?,
                high_price: row.get(4)?,
                low_price: row.get(5)?,
                close_price: row.get(6)?,
                volume: row.get(7)?,
                pe_ratio: row.get(8)?,
                market_cap: row.get(9)?,
                dividend_yield: row.get(10)?,
            })
        });

        match price {
            Ok(price) => Ok(Some(price)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get or set system metadata
    pub fn get_metadata(&self, key: &str) -> Result<Option<String>> {
        let conn = self.connection.lock().unwrap();
        
        let mut stmt = conn.prepare("SELECT value FROM metadata WHERE key = ?1")?;
        
        let value = stmt.query_row(params![key], |row| {
            Ok(row.get::<_, String>(0)?)
        });

        match value {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set_metadata(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.connection.lock().unwrap();
        
        conn.execute(
            "INSERT OR REPLACE INTO metadata (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params![key, value, Utc::now()],
        )?;

        Ok(())
    }

    /// Get last update date from metadata
    pub fn get_last_update_date(&self) -> Result<Option<NaiveDate>> {
        if let Some(date_str) = self.get_metadata("last_update_date")? {
            Ok(Some(NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?))
        } else {
            Ok(None)
        }
    }

    /// Set last update date
    pub fn set_last_update_date(&self, date: NaiveDate) -> Result<()> {
        self.set_metadata("last_update_date", &date.format("%Y-%m-%d").to_string())
    }

    /// Get database statistics
    pub fn get_stats(&self) -> Result<(usize, usize, Option<NaiveDate>)> {
        let conn = self.connection.lock().unwrap();
        
        let stock_count: usize = conn.query_row("SELECT COUNT(*) FROM stocks", [], |row| {
            Ok(row.get(0)?)
        })?;

        let price_count: usize = conn.query_row("SELECT COUNT(*) FROM daily_prices", [], |row| {
            Ok(row.get(0)?)
        })?;

        let last_update = self.get_last_update_date()?;

        Ok((stock_count, price_count, last_update))
    }

    /// Clear all stocks from database
    pub fn clear_stocks(&self) -> Result<()> {
        let conn = self.connection.lock().unwrap();
        conn.execute("DELETE FROM daily_prices", [])?;
        conn.execute("DELETE FROM stocks", [])?;
        info!("🗑️  Cleared all stocks and price data");
        Ok(())
    }

}