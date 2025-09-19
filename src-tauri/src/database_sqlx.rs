use anyhow::Result;
use chrono::{NaiveDate, DateTime, Utc};
use sqlx::{sqlite::{SqlitePoolOptions, SqliteConnectOptions}, SqlitePool, Row};
use std::collections::HashMap;
use crate::models::{Stock, DailyPrice, StockStatus, StockDataStats};

/// SQLX-based database manager for the Rust Stocks TUI
#[derive(Clone)]
pub struct DatabaseManagerSqlx {
    pool: SqlitePool,
}

impl DatabaseManagerSqlx {
    /// Create a new database manager with SQLX
    pub async fn new(database_url: &str) -> Result<Self> {
        // Ensure the connection string is properly formatted for SQLite
        let connection_string = if database_url.starts_with("sqlite:") {
            database_url.to_string()
        } else {
            format!("sqlite:{}", database_url)
        };
        
        println!("Connecting to database: {}", connection_string);
        
        let pool = SqlitePoolOptions::new()
            .max_connections(50) // Increased for parallel processing
            .acquire_timeout(std::time::Duration::from_secs(30))
            .connect_with(SqliteConnectOptions::new().filename(database_url).create_if_missing(true))
            .await?;

        // Enable WAL mode for better concurrency
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&pool)
            .await?;

        // Optimize SQLite settings for performance
        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&pool)
            .await?;

        sqlx::query("PRAGMA cache_size = 10000")
            .execute(&pool)
            .await?;

        sqlx::query("PRAGMA temp_store = memory")
            .execute(&pool)
            .await?;
        
        println!("Connected successfully, creating schema...");
        
        // Create tables directly instead of using migrations
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS stocks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                symbol TEXT UNIQUE NOT NULL,
                company_name TEXT NOT NULL,
                sector TEXT,
                industry TEXT,
                market_cap REAL,
                status TEXT NOT NULL DEFAULT 'active',
                first_trading_date DATE,
                last_updated DATETIME NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#
        ).execute(&pool).await?;
        
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS daily_prices (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                stock_id INTEGER NOT NULL,
                date DATE NOT NULL,
                open_price REAL,
                high_price REAL,
                low_price REAL,
                close_price REAL NOT NULL,
                volume INTEGER,
                pe_ratio REAL,
                market_cap REAL,
                dividend_yield REAL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (stock_id) REFERENCES stocks(id),
                UNIQUE(stock_id, date)
            )
            "#
        ).execute(&pool).await?;
        
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#
        ).execute(&pool).await?;
        
        Ok(Self { pool })
    }

    /// Upsert a stock (insert or update) - using raw SQL for flexibility
    #[allow(dead_code)]
    pub async fn upsert_stock(&self, stock: &Stock) -> Result<i64> {
        let status_str = match stock.status {
            StockStatus::Active => "active",
            StockStatus::Delisted => "delisted",
            StockStatus::Suspended => "suspended",
        };

        let last_updated = stock.last_updated.map(|dt| dt.naive_utc()).unwrap_or_else(|| Utc::now().naive_utc());

        let result = sqlx::query(
            r#"
            INSERT INTO stocks (symbol, company_name, sector, industry, market_cap, status, first_trading_date, last_updated)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(symbol) DO UPDATE SET
                company_name = excluded.company_name,
                sector = excluded.sector,
                industry = excluded.industry,
                market_cap = excluded.market_cap,
                status = excluded.status,
                first_trading_date = excluded.first_trading_date,
                last_updated = excluded.last_updated
            RETURNING id
            "#
        )
        .bind(&stock.symbol)
        .bind(&stock.company_name)
        .bind(&stock.sector)
        .bind(&stock.industry)
        .bind(stock.market_cap)
        .bind(status_str)
        .bind(stock.first_trading_date)
        .bind(last_updated)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.get::<i64, _>("id"))
    }

    /// Get stock by symbol - using raw SQL
    pub async fn get_stock_by_symbol(&self, symbol: &str) -> Result<Option<Stock>> {
        let row = sqlx::query(
            r#"
            SELECT id, symbol, company_name, sector, industry, market_cap, status, first_trading_date, last_updated, created_at
            FROM stocks
            WHERE symbol = ?
            "#
        )
        .bind(symbol)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| {
            let status = match r.get::<Option<String>, _>("status").as_deref() {
                Some("active") => StockStatus::Active,
                Some("delisted") => StockStatus::Delisted,
                Some("suspended") => StockStatus::Suspended,
                _ => StockStatus::Active,
            };

            Stock {
                id: Some(r.get::<i64, _>("id")),
                symbol: r.get::<String, _>("symbol"),
                company_name: r.get::<String, _>("company_name"),
                sector: r.get::<Option<String>, _>("sector"),
                industry: r.get::<Option<String>, _>("industry"),
                market_cap: r.get::<Option<f64>, _>("market_cap"),
                status,
                first_trading_date: r.get::<Option<NaiveDate>, _>("first_trading_date"),
                last_updated: r.get::<Option<DateTime<Utc>>, _>("last_updated"),
            }
        }))
    }

    /// Get all active stocks - using raw SQL
    pub async fn get_active_stocks(&self) -> Result<Vec<Stock>> {
        let rows = sqlx::query(
            r#"
            SELECT id, symbol, company_name, sector, industry, market_cap, status, first_trading_date, last_updated, created_at
            FROM stocks
            WHERE status = 'active'
            ORDER BY symbol
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| {
            let status = match r.get::<Option<String>, _>("status").as_deref() {
                Some("active") => StockStatus::Active,
                Some("delisted") => StockStatus::Delisted,
                Some("suspended") => StockStatus::Suspended,
                _ => StockStatus::Active,
            };

            Stock {
                id: Some(r.get::<i64, _>("id")),
                symbol: r.get::<String, _>("symbol"),
                company_name: r.get::<String, _>("company_name"),
                sector: r.get::<Option<String>, _>("sector"),
                industry: r.get::<Option<String>, _>("industry"),
                market_cap: r.get::<Option<f64>, _>("market_cap"),
                status,
                first_trading_date: r.get::<Option<NaiveDate>, _>("first_trading_date"),
                last_updated: r.get::<Option<DateTime<Utc>>, _>("last_updated"),
            }
        }).collect())
    }

    /// Insert daily price - using raw SQL
    pub async fn insert_daily_price(&self, price: &DailyPrice) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO daily_prices (stock_id, date, open_price, high_price, low_price, close_price, volume, pe_ratio, market_cap, dividend_yield)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(stock_id, date) DO UPDATE SET
                open_price = excluded.open_price,
                high_price = excluded.high_price,
                low_price = excluded.low_price,
                close_price = excluded.close_price,
                volume = excluded.volume,
                pe_ratio = excluded.pe_ratio,
                market_cap = excluded.market_cap,
                dividend_yield = excluded.dividend_yield
            RETURNING id
            "#
        )
        .bind(price.stock_id)
        .bind(price.date)
        .bind(price.open_price)
        .bind(price.high_price)
        .bind(price.low_price)
        .bind(price.close_price)
        .bind(price.volume)
        .bind(price.pe_ratio)
        .bind(price.market_cap)
        .bind(price.dividend_yield)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.get::<i64, _>("id"))
    }

    /// Get price on specific date - using raw SQL
    pub async fn get_price_on_date(&self, stock_id: i64, date: NaiveDate) -> Result<Option<DailyPrice>> {
        let row = sqlx::query(
            r#"
            SELECT id, stock_id, date, open_price, high_price, low_price, close_price, volume, pe_ratio, market_cap, dividend_yield, created_at
            FROM daily_prices
            WHERE stock_id = ? AND date = ?
            "#
        )
        .bind(stock_id)
        .bind(date)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| DailyPrice {
            id: Some(r.get::<i64, _>("id")),
            stock_id: r.get::<i64, _>("stock_id"),
            date: r.get::<NaiveDate, _>("date"),
            open_price: r.get::<f64, _>("open_price"),
            high_price: r.get::<f64, _>("high_price"),
            low_price: r.get::<f64, _>("low_price"),
            close_price: r.get::<f64, _>("close_price"),
            volume: r.get::<Option<i64>, _>("volume"),
            pe_ratio: r.get::<Option<f64>, _>("pe_ratio"),
            market_cap: r.get::<Option<f64>, _>("market_cap"),
            dividend_yield: r.get::<Option<f64>, _>("dividend_yield"),
        }))
    }

    /// Count existing records for a date range - using raw SQL
    pub async fn count_existing_records(&self, stock_id: i64, start_date: NaiveDate, end_date: NaiveDate) -> Result<i64> {
        let result = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM daily_prices
            WHERE stock_id = ? AND date BETWEEN ? AND ?
            "#
        )
        .bind(stock_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.get::<i64, _>("count"))
    }

    /// Get latest price for a stock - using raw SQL
    pub async fn get_latest_price(&self, stock_id: i64) -> Result<Option<DailyPrice>> {
        let row = sqlx::query(
            r#"
            SELECT id, stock_id, date, open_price, high_price, low_price, close_price, volume, pe_ratio, market_cap, dividend_yield, created_at
            FROM daily_prices
            WHERE stock_id = ?
            ORDER BY date DESC
            LIMIT 1
            "#
        )
        .bind(stock_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| DailyPrice {
            id: Some(r.get::<i64, _>("id")),
            stock_id: r.get::<i64, _>("stock_id"),
            date: r.get::<NaiveDate, _>("date"),
            open_price: r.get::<f64, _>("open_price"),
            high_price: r.get::<f64, _>("high_price"),
            low_price: r.get::<f64, _>("low_price"),
            close_price: r.get::<f64, _>("close_price"),
            volume: r.get::<Option<i64>, _>("volume"),
            pe_ratio: r.get::<Option<f64>, _>("pe_ratio"),
            market_cap: r.get::<Option<f64>, _>("market_cap"),
            dividend_yield: r.get::<Option<f64>, _>("dividend_yield"),
        }))
    }

    /// Get stock data statistics
    pub async fn get_stock_data_stats(&self, stock_id: i64) -> Result<StockDataStats> {
        let row = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as data_points,
                MIN(date) as oldest_date,
                MAX(date) as newest_date,
                AVG(close_price) as avg_price
            FROM daily_prices 
            WHERE stock_id = ?
            "#
        )
        .bind(stock_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(StockDataStats {
            data_points: row.get::<i64, _>("data_points") as usize,
            earliest_date: row.get::<Option<NaiveDate>, _>("oldest_date"),
            latest_date: row.get::<Option<NaiveDate>, _>("newest_date"),
        })
    }

    /// Get metadata value - using raw SQL
    pub async fn get_metadata(&self, key: &str) -> Result<Option<String>> {
        let row = sqlx::query(
            r#"
            SELECT value
            FROM metadata
            WHERE key = ?
            "#
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.get::<String, _>("value")))
    }

    /// Set metadata value - using raw SQL
    pub async fn set_metadata(&self, key: &str, value: &str) -> Result<()> {
        let now = Utc::now().naive_utc();
        sqlx::query(
            r#"
            INSERT INTO metadata (key, value, updated_at)
            VALUES (?, ?, ?)
            ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                updated_at = excluded.updated_at
            "#
        )
        .bind(key)
        .bind(value)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get database statistics - using raw SQL
    pub async fn get_stats(&self) -> Result<HashMap<String, i64>> {
        let mut stats = HashMap::new();

        // Count stocks
        let stock_count = sqlx::query("SELECT COUNT(*) as count FROM stocks WHERE status = 'active'")
            .fetch_one(&self.pool)
            .await?;
        stats.insert("total_stocks".to_string(), stock_count.get::<i64, _>("count"));

        // Count prices
        let price_count = sqlx::query("SELECT COUNT(*) as count FROM daily_prices")
            .fetch_one(&self.pool)
            .await?;
        stats.insert("total_price_records".to_string(), price_count.get::<i64, _>("count"));

        // Count unique dates
        let date_count = sqlx::query("SELECT COUNT(DISTINCT date) as count FROM daily_prices")
            .fetch_one(&self.pool)
            .await?;
        stats.insert("unique_dates".to_string(), date_count.get::<i64, _>("count"));

        Ok(stats)
    }

    /// Clear all stocks and related data - using raw SQL
    #[allow(dead_code)]
    pub async fn clear_stocks(&self) -> Result<()> {
        sqlx::query("DELETE FROM daily_prices").execute(&self.pool).await?;
        sqlx::query("DELETE FROM stocks").execute(&self.pool).await?;
        Ok(())
    }

    /// Set the last update date
    #[allow(dead_code)]
    pub async fn set_last_update_date(&self, date: NaiveDate) -> Result<()> {
        self.set_metadata("last_update_date", &date.format("%Y-%m-%d").to_string()).await
    }

    /// Get the last update date
    #[allow(dead_code)]
    pub async fn get_last_update_date(&self) -> Result<Option<NaiveDate>> {
        if let Some(date_str) = self.get_metadata("last_update_date").await? {
            NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .map(Some)
                .map_err(|e| anyhow::anyhow!("Failed to parse date: {}", e))
        } else {
            Ok(None)
        }
    }

    /// Get P/E ratio on a specific date
    #[allow(dead_code)]
    pub async fn get_pe_ratio_on_date(&self, stock_id: i64, date: NaiveDate) -> Result<Option<f64>> {
        let row = sqlx::query(
            "SELECT pe_ratio FROM daily_prices WHERE stock_id = ? AND date = ?"
        )
        .bind(stock_id)
        .bind(date)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.get::<Option<f64>, _>("pe_ratio")).flatten())
    }

    /// Get market cap on a specific date
    #[allow(dead_code)]
    pub async fn get_market_cap_on_date(&self, stock_id: i64, date: NaiveDate) -> Result<Option<f64>> {
        let row = sqlx::query(
            "SELECT market_cap FROM daily_prices WHERE stock_id = ? AND date = ?"
        )
        .bind(stock_id)
        .bind(date)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.get::<Option<f64>, _>("market_cap")).flatten())
    }

    /// Get the oldest date in the database
    pub async fn get_oldest_data_date(&self) -> Result<Option<NaiveDate>> {
        let row = sqlx::query("SELECT MIN(date) as oldest_date FROM daily_prices")
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.and_then(|r| r.get::<Option<NaiveDate>, _>("oldest_date")))
    }

    /// Get the newest date in the database  
    pub async fn get_newest_data_date(&self) -> Result<Option<NaiveDate>> {
        let row = sqlx::query("SELECT MAX(date) as newest_date FROM daily_prices")
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.and_then(|r| r.get::<Option<NaiveDate>, _>("newest_date")))
    }

    /// Close the database connection pool
    #[allow(dead_code)]
    pub async fn close(self) -> Result<()> {
        self.pool.close().await;
        Ok(())
    }
}
