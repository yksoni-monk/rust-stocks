use anyhow::Result;
use chrono::{NaiveDate, Utc};
use rusqlite::{Connection, params};
use std::sync::{Arc, Mutex};
use tracing::info;

use crate::models::{Stock, DailyPrice, StockStatus, DatabaseStats, CollectionProgress, 
                    StockProgress, DataCoverage, OverallProgress, StockCollectionStatus, 
                    CollectionStatus, StockDataStats};

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
    #[allow(dead_code)]
    pub fn upsert_stock(&self, stock: &Stock) -> Result<i64> {
        let conn = self.connection.lock().unwrap();
        
        let status_str = match stock.status {
            StockStatus::Active => "active",
            StockStatus::Delisted => "delisted", 
            StockStatus::Suspended => "suspended",
        };

        // Check if stock already exists by symbol
        let existing_stock = conn.query_row(
            "SELECT id FROM stocks WHERE symbol = ?1",
            params![stock.symbol],
            |row| Ok(row.get::<_, i64>(0)?)
        );

        match existing_stock {
            Ok(existing_id) => {
                // Stock exists, update it
                conn.execute(
                    "UPDATE stocks SET 
                        company_name = ?2, sector = ?3, industry = ?4, market_cap = ?5, 
                        status = ?6, first_trading_date = ?7, last_updated = ?8
                     WHERE id = ?1",
                    params![
                        existing_id,
                        stock.company_name,
                        stock.sector,
                        stock.industry,
                        stock.market_cap,
                        status_str,
                        stock.first_trading_date,
                        stock.last_updated
                    ],
                )?;
                Ok(existing_id)
            },
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // Stock doesn't exist, insert new one
                conn.execute(
                    "INSERT INTO stocks (
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
            },
            Err(e) => Err(e.into()),
        }
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
    #[allow(dead_code)]
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

    /// Count existing records for a stock in a date range
    pub fn count_existing_records(&self, stock_id: i64, start_date: NaiveDate, end_date: NaiveDate) -> Result<usize> {
        let conn = self.connection.lock().unwrap();
        
        let count: usize = conn.query_row(
            "SELECT COUNT(*) FROM daily_prices WHERE stock_id = ?1 AND date BETWEEN ?2 AND ?3",
            params![stock_id, start_date, end_date],
            |row| Ok(row.get(0)?)
        )?;

        Ok(count)
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

    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn set_last_update_date(&self, date: NaiveDate) -> Result<()> {
        self.set_metadata("last_update_date", &date.format("%Y-%m-%d").to_string())
    }

    /// Get database statistics
    #[allow(dead_code)]
    pub fn get_stats(&self) -> Result<(usize, usize, Option<NaiveDate>)> {
        let conn = self.connection.lock().unwrap();
        
        let stock_count: usize = conn.query_row("SELECT COUNT(*) FROM stocks", [], |row| {
            Ok(row.get(0)?)
        })?;

        let price_count: usize = conn.query_row("SELECT COUNT(*) FROM daily_prices", [], |row| {
            Ok(row.get(0)?)
        })?;

        // Get last update date directly from the same connection to avoid recursive locking
        let last_update: Option<NaiveDate> = {
            let mut stmt = conn.prepare("SELECT value FROM metadata WHERE key = 'last_update_date'")?;
            let value = stmt.query_row([], |row| {
                Ok(row.get::<_, String>(0)?)
            });

            match value {
                Ok(date_str) => Some(NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?),
                Err(rusqlite::Error::QueryReturnedNoRows) => None,
                Err(e) => return Err(e.into()),
            }
        };

        Ok((stock_count, price_count, last_update))
    }

    /// Clear all stocks from database
    #[allow(dead_code)]
    pub fn clear_stocks(&self) -> Result<()> {
        let conn = self.connection.lock().unwrap();
        conn.execute("DELETE FROM daily_prices", [])?;
        conn.execute("DELETE FROM stocks", [])?;
        info!("🗑️  Cleared all stocks and price data");
        Ok(())
    }

    // ============================================================================
    // Enhanced Progress Analysis Methods for TUI Application
    // ============================================================================

    /// Get comprehensive database statistics
    #[allow(dead_code)]
    pub fn get_database_stats(&self) -> Result<DatabaseStats> {
        let conn = self.connection.lock().unwrap();
        
        let total_stocks: usize = conn.query_row(
            "SELECT COUNT(*) FROM stocks", 
            [], 
            |row| Ok(row.get(0)?)
        )?;

        let total_price_records: usize = conn.query_row(
            "SELECT COUNT(*) FROM daily_prices", 
            [], 
            |row| Ok(row.get(0)?)
        )?;

        let oldest_data_date: Option<NaiveDate> = conn.query_row(
            "SELECT MIN(date) FROM daily_prices",
            [],
            |row| Ok(row.get(0)?)
        ).ok();

        let stocks_with_data: usize = conn.query_row(
            "SELECT COUNT(DISTINCT stock_id) FROM daily_prices",
            [],
            |row| Ok(row.get(0)?)
        ).unwrap_or(0);

        let data_coverage_percentage = if total_stocks > 0 {
            (stocks_with_data as f64 / total_stocks as f64) * 100.0
        } else {
            0.0
        };

        // Get last update date directly from the same connection to avoid recursive locking
        let last_update_date: Option<NaiveDate> = {
            let mut stmt = conn.prepare("SELECT value FROM metadata WHERE key = 'last_update_date'")?;
            let value = stmt.query_row([], |row| {
                Ok(row.get::<_, String>(0)?)
            });

            match value {
                Ok(date_str) => Some(NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?),
                Err(rusqlite::Error::QueryReturnedNoRows) => None,
                Err(e) => return Err(e.into()),
            }
        };

        Ok(DatabaseStats {
            total_stocks,
            total_price_records,
            data_coverage_percentage,
            last_update_date,
            oldest_data_date,
        })
    }

    /// Get collection progress toward target date
    #[allow(dead_code)]
    pub fn get_collection_progress(&self, target_date: NaiveDate) -> Result<CollectionProgress> {
        let conn = self.connection.lock().unwrap();
        
        let total_stocks: usize = conn.query_row(
            "SELECT COUNT(*) FROM stocks WHERE status = 'active'", 
            [], 
            |row| Ok(row.get(0)?)
        )?;

        let stocks_with_data: usize = conn.query_row(
            "SELECT COUNT(DISTINCT s.id) FROM stocks s 
             JOIN daily_prices dp ON s.id = dp.stock_id 
             WHERE s.status = 'active' AND dp.date >= ?1",
            params![target_date],
            |row| Ok(row.get(0)?)
        ).unwrap_or(0);

        let stocks_missing_data = total_stocks.saturating_sub(stocks_with_data);
        
        let completion_percentage = if total_stocks > 0 {
            (stocks_with_data as f64 / total_stocks as f64) * 100.0
        } else {
            0.0
        };

        // Estimate remaining records needed (assuming ~1400 records per stock on average)
        let estimated_records_remaining = stocks_missing_data * 1400;

        Ok(CollectionProgress {
            stocks_with_data,
            stocks_missing_data,
            target_start_date: target_date,
            completion_percentage,
            estimated_records_remaining,
        })
    }

    /// Get overall progress analysis
    #[allow(dead_code)]
    pub fn get_overall_progress(&self, target_date: NaiveDate) -> Result<OverallProgress> {
        let conn = self.connection.lock().unwrap();
        
        let target_records_per_stock = 1400; // Approximate trading days from 2020-01-01 to today
        
        let total_stocks: usize = conn.query_row(
            "SELECT COUNT(*) FROM stocks WHERE status = 'active'",
            [],
            |row| Ok(row.get(0)?)
        )?;
        
        let total_target_records = total_stocks * target_records_per_stock;
        
        let current_records: usize = conn.query_row(
            "SELECT COUNT(*) FROM daily_prices dp 
             JOIN stocks s ON dp.stock_id = s.id 
             WHERE s.status = 'active' AND dp.date >= ?1",
            params![target_date],
            |row| Ok(row.get(0)?)
        ).unwrap_or(0);

        // Count stocks by completion status
        let stocks_completed: usize = conn.query_row(
            "SELECT COUNT(*) FROM stocks s WHERE s.status = 'active' AND 
             EXISTS (SELECT 1 FROM daily_prices dp WHERE dp.stock_id = s.id AND dp.date = ?1)",
            params![target_date],
            |row| Ok(row.get(0)?)
        ).unwrap_or(0);

        let stocks_with_some_data: usize = conn.query_row(
            "SELECT COUNT(DISTINCT s.id) FROM stocks s 
             JOIN daily_prices dp ON s.id = dp.stock_id 
             WHERE s.status = 'active' AND dp.date >= ?1",
            params![target_date],
            |row| Ok(row.get(0)?)
        ).unwrap_or(0);

        let stocks_partial = stocks_with_some_data.saturating_sub(stocks_completed);
        let stocks_missing = total_stocks.saturating_sub(stocks_with_some_data);
        
        let completion_percentage = if total_target_records > 0 {
            (current_records as f64 / total_target_records as f64) * 100.0
        } else {
            0.0
        };

        Ok(OverallProgress {
            target_start_date: target_date,
            total_target_records,
            current_records,
            completion_percentage,
            stocks_completed,
            stocks_partial,
            stocks_missing,
        })
    }

    /// Get data coverage for a specific stock
    #[allow(dead_code)]
    pub fn get_stock_data_coverage(&self, stock_id: i64, target_date: NaiveDate) -> Result<DataCoverage> {
        let conn = self.connection.lock().unwrap();
        
        let mut stmt = conn.prepare(
            "SELECT MIN(date) as earliest, MAX(date) as latest, COUNT(*) as total
             FROM daily_prices WHERE stock_id = ?1"
        )?;

        let (earliest_date, latest_date, total_records) = stmt.query_row(
            params![stock_id],
            |row| {
                Ok((
                    row.get::<_, Option<NaiveDate>>(0)?,
                    row.get::<_, Option<NaiveDate>>(1)?,
                    row.get::<_, usize>(2)?
                ))
            }
        ).unwrap_or((None, None, 0));

        // Calculate coverage percentage based on target date
        let coverage_percentage = if let (Some(earliest), Some(latest)) = (earliest_date, latest_date) {
            if earliest <= target_date {
                let total_possible_days = (latest - target_date.max(earliest)).num_days() + 1;
                if total_possible_days > 0 {
                    (total_records as f64 / total_possible_days as f64 * 5.0 / 7.0) * 100.0 // Adjust for weekends
                } else {
                    0.0
                }
            } else {
                0.0
            }
        } else {
            0.0
        };

        // TODO: Implement missing_ranges calculation
        let missing_ranges = Vec::new();

        Ok(DataCoverage {
            earliest_date,
            latest_date,
            total_records,
            missing_ranges,
            coverage_percentage: coverage_percentage.min(100.0),
        })
    }

    /// Get stock progress list for all stocks
    #[allow(dead_code)]
    pub fn get_all_stock_progress(&self, target_date: NaiveDate) -> Result<Vec<StockProgress>> {
        let stocks = self.get_active_stocks()?;
        let mut progress_list = Vec::new();
        
        for stock in stocks {
            if let Some(stock_id) = stock.id {
                let coverage = self.get_stock_data_coverage(stock_id, target_date)?;
                
                let data_range = if let (Some(earliest), Some(latest)) = (coverage.earliest_date, coverage.latest_date) {
                    Some((earliest, latest))
                } else {
                    None
                };

                // Estimate expected records (approximately 250 trading days per year)
                let years_since_target = (chrono::Utc::now().date_naive() - target_date).num_days() / 365;
                let expected_records = (years_since_target as usize + 1) * 250;

                // Calculate priority score (higher = needs more attention)
                let priority_score = if coverage.total_records == 0 {
                    100.0 // Highest priority for stocks with no data
                } else {
                    100.0 - coverage.coverage_percentage
                };

                progress_list.push(StockProgress {
                    stock,
                    data_range,
                    record_count: coverage.total_records,
                    expected_records,
                    missing_ranges: coverage.missing_ranges,
                    priority_score,
                });
            }
        }

        // Sort by priority score (descending)
        progress_list.sort_by(|a, b| b.priority_score.partial_cmp(&a.priority_score).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(progress_list)
    }

    /// Get stock collection status for UI display
    #[allow(dead_code)]
    pub fn get_stock_collection_status(&self, target_date: NaiveDate) -> Result<Vec<StockCollectionStatus>> {
        let stocks = self.get_active_stocks()?;
        let mut status_list = Vec::new();
        
        for stock in stocks {
            if let Some(stock_id) = stock.id {
                let coverage = self.get_stock_data_coverage(stock_id, target_date)?;
                
                let status = if coverage.total_records == 0 {
                    CollectionStatus::NotStarted
                } else if coverage.coverage_percentage >= 95.0 {
                    CollectionStatus::Completed
                } else if !coverage.missing_ranges.is_empty() {
                    CollectionStatus::PartialData { 
                        gaps: coverage.missing_ranges.clone() 
                    }
                } else {
                    CollectionStatus::Completed
                };

                let date_range = if let (Some(earliest), Some(latest)) = (coverage.earliest_date, coverage.latest_date) {
                    Some((earliest, latest))
                } else {
                    None
                };

                status_list.push(StockCollectionStatus {
                    symbol: stock.symbol,
                    company_name: stock.company_name,
                    status,
                    date_range,
                    record_count: coverage.total_records,
                    progress_percentage: coverage.coverage_percentage,
                });
            }
        }

        Ok(status_list)
    }

    /// Get total number of price records in database
    pub fn get_total_price_records(&self) -> Result<usize> {
        let conn = self.connection.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM daily_prices",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Get the oldest data date in the database
    pub fn get_oldest_data_date(&self) -> Result<Option<NaiveDate>> {
        let conn = self.connection.lock().unwrap();
        let result = conn.query_row(
            "SELECT MIN(date) FROM daily_prices",
            [],
            |row| {
                let date_str: String = row.get(0)?;
                Ok(NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").unwrap_or_default())
            },
        );
        
        match result {
            Ok(date) => Ok(Some(date)),
            Err(_) => Ok(None),
        }
    }

    /// Get stock data statistics
    pub fn get_stock_data_stats(&self, stock_id: i64) -> Result<StockDataStats> {
        let conn = self.connection.lock().unwrap();
        
        // Get data points count
        let data_points: i64 = conn.query_row(
            "SELECT COUNT(*) FROM daily_prices WHERE stock_id = ?",
            params![stock_id],
            |row| row.get(0),
        )?;
        
        // Get earliest date
        let earliest_date: Option<NaiveDate> = conn.query_row(
            "SELECT MIN(date) FROM daily_prices WHERE stock_id = ?",
            params![stock_id],
            |row| {
                let date_str: Option<String> = row.get(0)?;
                Ok(date_str.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()))
            },
        ).ok().flatten();
        
        // Get latest date
        let latest_date: Option<NaiveDate> = conn.query_row(
            "SELECT MAX(date) FROM daily_prices WHERE stock_id = ?",
            params![stock_id],
            |row| {
                let date_str: Option<String> = row.get(0)?;
                Ok(date_str.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()))
            },
        ).ok().flatten();
        
        Ok(StockDataStats {
            data_points: data_points as usize,
            earliest_date,
            latest_date,
        })
    }

    /// Get P/E ratio for a stock on a specific date
    pub fn get_pe_ratio_on_date(&self, stock_id: i64, date: NaiveDate) -> Result<Option<f64>> {
        let conn = self.connection.lock().unwrap();
        let result = conn.query_row(
            "SELECT pe_ratio FROM daily_prices WHERE stock_id = ? AND date = ?",
            params![stock_id, date.format("%Y-%m-%d").to_string()],
            |row| row.get(0),
        );
        
        match result {
            Ok(pe_ratio) => Ok(Some(pe_ratio)),
            Err(_) => Ok(None),
        }
    }

    /// Get market cap for a stock on a specific date
    pub fn get_market_cap_on_date(&self, stock_id: i64, date: NaiveDate) -> Result<Option<f64>> {
        let conn = self.connection.lock().unwrap();
        let result = conn.query_row(
            "SELECT market_cap FROM daily_prices WHERE stock_id = ? AND date = ?",
            params![stock_id, date.format("%Y-%m-%d").to_string()],
            |row| row.get(0),
        );
        
        match result {
            Ok(market_cap) => Ok(Some(market_cap)),
            Err(_) => Ok(None),
        }
    }


}