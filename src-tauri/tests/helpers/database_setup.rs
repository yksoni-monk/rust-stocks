use sqlx::{SqlitePool, Row};
use std::path::Path;
use std::fs;
use std::str::FromStr;
use anyhow::Result;
use crate::helpers::test_config::TestConfig;
use crate::helpers::sync_report::SyncReport;

/// Test database setup utilities
pub struct TestDatabase {
    pub pool: SqlitePool,
    pub config: TestConfig,
    pub is_copy: bool, // true if we copied from production
}

impl TestDatabase {
    /// PHASE 1: Perform intelligent database synchronization (run BEFORE tests)
    /// This should be called once before any tests run, not during test setup
    pub async fn intelligent_sync() -> Result<SyncReport> {
        let config = TestConfig::from_env();
        println!("🔄 Starting intelligent database synchronization...");
        
        // Skip sync if using production database directly
        if config.use_production_db {
            println!("⚠️  Using production database directly - no sync needed");
            return Ok(SyncReport::production_direct());
        }

        let production_db = "db/stocks.db";
        let test_db_path = &config.test_db_path;

        // Check if production database exists
        if !Path::new(production_db).exists() {
            return Err(anyhow::anyhow!("Production database not found at {}", production_db));
        }

        // Ensure test db directory exists
        if let Some(parent) = Path::new(test_db_path).parent() {
            fs::create_dir_all(parent)?;
        }

        let start_time = std::time::Instant::now();
        
        // Simple intelligent sync using ATTACH DATABASE
        let sync_report = Self::attach_database_sync(production_db, test_db_path).await?;
        
        let duration_ms = start_time.elapsed().as_millis();
        println!("✅ Intelligent sync completed in {}ms!", duration_ms);
        
        Ok(SyncReport {
            sync_strategy: "attach_database_sync".to_string(),
            total_duration_ms: duration_ms,
            stocks_synced: sync_report.stocks_synced,
            daily_prices_synced: sync_report.daily_prices_synced,
            earnings_synced: sync_report.earnings_synced,
            metadata_synced: sync_report.metadata_synced,
            schema_changes_applied: sync_report.schema_changes_applied,
        })
    }

    /// PHASE 2: Create a test database connection (assumes sync already completed)  
    /// This should be called by tests after intelligent_sync() has run
    pub async fn new() -> Result<Self> {
        let config = TestConfig::from_env();
        println!("🧪 Connecting to test database: {}", config.get_description());
        
        let is_copy = true; // Always true since intelligent_sync makes test.db identical to production
        
        if config.use_production_db {
            println!("⚠️  Using production database directly - BE CAREFUL!");
        } else {
            // Test database should exist after intelligent_sync
            if !Path::new(&config.test_db_path).exists() {
                return Err(anyhow::anyhow!("Test database not found - intelligent_sync should run first"));
            }
            println!("✅ Using synchronized test database (identical to production)");
        }
        
        // Connect to the database with WAL mode and connection pool for proper concurrency
        let database_url = config.get_database_url();
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(10) // Allow multiple concurrent connections
            .min_connections(2)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .idle_timeout(Some(std::time::Duration::from_secs(300)))
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::from_str(&database_url)?
                    .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
                    .busy_timeout(std::time::Duration::from_secs(30))
                    .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            ).await?;
        
        println!("✅ Connected to test database: {}", database_url);
        
        Ok(TestDatabase { 
            pool, 
            config,
            is_copy,
        })
    }
    
    /// Connect to test database (full read-write access)
    async fn connect_to_test_database(db_path: &str) -> Result<SqlitePool, Box<dyn std::error::Error>> {
        let database_url = format!("sqlite:{}", db_path);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(10)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::from_str(&database_url)?
                    .create_if_missing(true)
                    .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
                    .busy_timeout(std::time::Duration::from_secs(30))
            ).await?;
        Ok(pool)
    }

    /// Simple intelligent sync using SQLite ATTACH DATABASE
    /// Makes test.db identical to stocks.db
    async fn attach_database_sync(production_db: &str, test_db_path: &str) -> Result<SyncReport, anyhow::Error> {
        println!("🔄 Using ATTACH DATABASE approach for intelligent sync...");
        
        let mut report = SyncReport::default();
        
        // If test.db doesn't exist, do full copy
        if !Path::new(test_db_path).exists() {
            println!("📋 Test database doesn't exist - performing full copy...");
            tokio::fs::copy(production_db, test_db_path).await?;
            
            // Count records for report
            let test_pool = Self::connect_to_test_database(test_db_path).await
                .map_err(|e| anyhow::anyhow!("Failed to connect to test database: {}", e))?;
            
            report.stocks_synced = sqlx::query("SELECT COUNT(*) as count FROM stocks")
                .fetch_one(&test_pool).await?
                .get::<i64, _>("count") as usize;
                
            report.daily_prices_synced = sqlx::query("SELECT COUNT(*) as count FROM daily_prices")
                .fetch_one(&test_pool).await?
                .get::<i64, _>("count") as usize;
                
            test_pool.close().await;
            println!("✅ Full copy completed: {} stocks, {} prices", report.stocks_synced, report.daily_prices_synced);
            return Ok(report);
        }
        
        // Test.db exists - check if sync needed
        let prod_modified = fs::metadata(production_db)?.modified()?;
        let test_modified = fs::metadata(test_db_path)?.modified()?;
        
        if test_modified >= prod_modified {
            println!("✨ Test database is up to date - no sync needed");
            return Ok(report);
        }
        
        println!("📊 Production database is newer - syncing changes...");
        
        // Connect to test database and attach production
        let test_pool = Self::connect_to_test_database(test_db_path).await
            .map_err(|e| anyhow::anyhow!("Failed to connect to test database: {}", e))?;
        
        // Attach production database
        sqlx::query(&format!("ATTACH DATABASE '{}' AS prod", production_db))
            .execute(&test_pool).await?;
        
        // Sync each table
        report.stocks_synced = Self::sync_table_with_attach(&test_pool, "stocks").await?;
        report.daily_prices_synced = Self::sync_table_with_attach(&test_pool, "daily_prices").await?;
        report.earnings_synced = Self::sync_table_with_attach(&test_pool, "income_statements").await?;
        Self::sync_table_with_attach(&test_pool, "balance_sheets").await?;
        Self::sync_table_with_attach(&test_pool, "sp500_symbols").await?;
        Self::sync_table_with_attach(&test_pool, "daily_valuation_ratios").await?;
        report.metadata_synced = 3;
        
        // Detach production database
        sqlx::query("DETACH DATABASE prod").execute(&test_pool).await?;
        test_pool.close().await;
        
        println!("✅ Incremental sync completed: {} stocks, {} prices, {} earnings updated", 
                report.stocks_synced, report.daily_prices_synced, report.earnings_synced);
        
        Ok(report)
    }
    
    /// Sync a single table using ATTACH DATABASE
    async fn sync_table_with_attach(test_pool: &SqlitePool, table_name: &str) -> Result<usize, anyhow::Error> {
        // Check if table exists in both databases
        let test_exists = sqlx::query("SELECT 1 FROM sqlite_master WHERE type='table' AND name=?")
            .bind(table_name)
            .fetch_optional(test_pool).await?.is_some();
            
        let prod_exists = sqlx::query("SELECT 1 FROM prod.sqlite_master WHERE type='table' AND name=?")
            .bind(table_name)
            .fetch_optional(test_pool).await?.is_some();
        
        if !prod_exists {
            return Ok(0);
        }
        
        if !test_exists {
            // Create table with production schema
            let create_sql: String = sqlx::query("SELECT sql FROM prod.sqlite_master WHERE type='table' AND name=?")
                .bind(table_name)
                .fetch_one(test_pool).await?
                .get("sql");
            sqlx::query(&create_sql).execute(test_pool).await?;
        }
        
        // Get counts
        let prod_count: i64 = sqlx::query(&format!("SELECT COUNT(*) as count FROM prod.{}", table_name))
            .fetch_one(test_pool).await?
            .get("count");
            
        let test_count: i64 = sqlx::query(&format!("SELECT COUNT(*) as count FROM {}", table_name))
            .fetch_one(test_pool).await?
            .get("count");
        
        if prod_count == test_count {
            return Ok(0);
        }
        
        // Simple approach: replace all data
        sqlx::query(&format!("DELETE FROM {}", table_name)).execute(test_pool).await?;
        sqlx::query(&format!("INSERT INTO {} SELECT * FROM prod.{}", table_name, table_name))
            .execute(test_pool).await?;
        
        Ok(prod_count as usize)
    }
    
    /// Get a reference to the database pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
    
    /// Verify database data is accessible and report statistics
    pub async fn verify_test_data(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Check stocks count
        let stock_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM stocks")
            .fetch_one(&self.pool)
            .await?
            .get("count");
        
        // Production database or copy - expect thousands of stocks
        assert!(stock_count > 1000, "Test database should have > 1000 stocks (production data), found {}", stock_count);
        println!("✅ Database verified: {} stocks (production data)", stock_count);
        
        // Check S&P 500 symbols count
        let sp500_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM sp500_symbols")
            .fetch_one(&self.pool)
            .await?
            .get("count");
        
        assert!(sp500_count > 400, "Test database should have > 400 S&P 500 symbols, found {}", sp500_count);
        println!("✅ S&P 500 symbols verified: {} (production data)", sp500_count);
        
        // Check daily prices count
        let price_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM daily_prices")
            .fetch_one(&self.pool)
            .await?
            .get("count");
        
        assert!(price_count > 1000000, "Test database should have > 1M price records, found {}", price_count);
        println!("✅ Price data verified: {} records (production data)", price_count);
        
        // Check income statements count
        let income_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM income_statements")
            .fetch_one(&self.pool)
            .await?
            .get("count");
        
        assert!(income_count > 1000, "Test database should have > 1000 income statements, found {}", income_count);
        println!("✅ Income statements verified: {} records (production data)", income_count);
        
        // Check valuation ratios count
        let ratios_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM daily_valuation_ratios")
            .fetch_one(&self.pool)
            .await?
            .get("count");
        
        assert!(ratios_count > 1000, "Test database should have > 1000 ratio records, found {}", ratios_count);
        println!("✅ Valuation ratios verified: {} records (production data)", ratios_count);
        
        println!("✅ Test database verified: {} stocks, {} S&P 500, {} prices, {} income, {} ratios (production data)", 
                stock_count, sp500_count, price_count, income_count, ratios_count);
        
        Ok(())
    }
    
    /// Clean up database (called automatically on drop)
    pub async fn cleanup(self) -> Result<(), Box<dyn std::error::Error>> {
        self.pool.close().await;
        
        // If it's not a production database connection, we can safely clean up
        if !self.config.use_production_db {
            println!("🧹 Test database cleanup completed");
        } else {
            println!("🧹 Production database connection closed (no cleanup needed)");
        }
        
        Ok(())
    }
}

/// Test assertions helper
pub struct TestAssertions;

impl TestAssertions {
    pub fn assert_stock_data_valid(stock: &rust_stocks_tauri_lib::commands::stocks::StockWithData) {
        assert!(!stock.symbol.is_empty(), "Stock symbol should not be empty");
        assert!(!stock.company_name.is_empty(), "Company name should not be empty");
        assert!(stock.id > 0, "Stock ID should be positive");
    }
    
    pub fn assert_price_data_valid(price: &rust_stocks_tauri_lib::commands::analysis::PriceData) {
        assert!(price.close > 0.0, "Close price should be positive");
        assert!(price.high >= price.low, "High should be >= Low");
        assert!(price.high >= price.close, "High should be >= Close");
        assert!(price.low <= price.close, "Low should be <= Close");
        assert!(!price.date.is_empty(), "Date should not be empty");
    }
    
    #[allow(dead_code)] // Available for future use in valuation ratio tests
    pub fn assert_valuation_ratios_valid(ratios: &rust_stocks_tauri_lib::commands::analysis::ValuationRatios) {
        if let Some(ps_ratio) = ratios.ps_ratio_ttm {
            assert!(ps_ratio > 0.0, "P/S ratio should be positive");
            assert!(ps_ratio < 1000.0, "P/S ratio should be reasonable");
        }
        
        if let Some(evs_ratio) = ratios.evs_ratio_ttm {
            assert!(evs_ratio > 0.0, "EV/S ratio should be positive");
            assert!(evs_ratio < 1000.0, "EV/S ratio should be reasonable");
        }
        
        if let Some(market_cap) = ratios.market_cap {
            assert!(market_cap > 0.0, "Market cap should be positive");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_database_setup() {
        let test_db = TestDatabase::new().await.unwrap();
        test_db.verify_test_data().await.unwrap();
        test_db.cleanup().await.unwrap();
    }
}