use sqlx::{SqlitePool, Row};
use std::path::Path;
use std::fs;
use crate::helpers::test_config::TestConfig;

/// Test database setup utilities
pub struct TestDatabase {
    pub pool: SqlitePool,
    pub config: TestConfig,
    pub is_copy: bool, // true if we copied from production
}

impl TestDatabase {
    /// Create a test database based on configuration
    /// - If USE_PRODUCTION_DB=true: Use production database directly (DANGEROUS)
    /// - Otherwise: Copy production db to test.db and use that (SAFE)
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config = TestConfig::from_env();
        println!("🧪 Setting up test database: {}", config.get_description());
        
        let mut is_copy = false;
        
        if config.use_production_db {
            println!("⚠️  Using production database directly - BE CAREFUL!");
        } else {
            // Copy production database to test database
            is_copy = Self::setup_test_database_copy(&config.test_db_path).await?;
        }
        
        // Connect to the database
        let database_url = config.get_database_url();
        let pool = SqlitePool::connect(&database_url).await?;
        
        println!("✅ Connected to test database: {}", database_url);
        
        Ok(TestDatabase { 
            pool, 
            config,
            is_copy,
        })
    }
    
    /// Copy production database to test database location with concurrency safety
    async fn setup_test_database_copy(test_db_path: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let production_db = "db/stocks.db";
        
        // Check if production database exists
        if !Path::new(production_db).exists() {
            println!("⚠️  Production database not found at {}, creating test database with sample data", production_db);
            return Self::create_test_database_with_sample_data(test_db_path).await;
        }
        
        // Check if test database already exists and is valid
        if Path::new(test_db_path).exists() {
            match Self::validate_existing_test_db(test_db_path).await {
                Ok(true) => {
                    println!("✅ Using existing valid test database at {}", test_db_path);
                    return Ok(true);
                }
                _ => {
                    println!("🔄 Removing invalid existing test database");
                    let _ = fs::remove_file(test_db_path);
                }
            }
        }
        
        // Get production database size for progress reporting
        let production_size = fs::metadata(production_db)?.len();
        let size_mb = production_size as f64 / 1024.0 / 1024.0;
        
        println!("📋 Copying production database ({:.1} MB) to test database...", size_mb);
        println!("   Source: {}", production_db);  
        println!("   Target: {}", test_db_path);
        
        // Create test db directory if it doesn't exist
        if let Some(parent) = Path::new(test_db_path).parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Use a unique temporary file name to avoid conflicts
        let temp_path = format!("{}.tmp.{}", test_db_path, std::process::id());
        
        // Copy to temporary file first
        match tokio::fs::copy(production_db, &temp_path).await {
            Ok(bytes_copied) => {
                if bytes_copied != production_size {
                    let _ = fs::remove_file(&temp_path);
                    return Err(format!("Copy size mismatch: {} vs {} bytes", bytes_copied, production_size).into());
                }
            }
            Err(e) => {
                let _ = fs::remove_file(&temp_path);
                return Err(format!("Failed to copy database: {}", e).into());
            }
        }
        
        // Verify the temporary file is valid SQLite
        match Self::validate_existing_test_db(&temp_path).await {
            Ok(true) => {
                // Atomically move temp file to final location
                fs::rename(&temp_path, test_db_path)?;
                println!("✅ Successfully copied production database to test database");
                Ok(true)
            }
            Ok(false) => {
                let _ = fs::remove_file(&temp_path);
                Err("Copied database is not valid SQLite".into())
            }
            Err(e) => {
                let _ = fs::remove_file(&temp_path);
                Err(format!("Database validation failed: {}", e).into())
            }
        }
    }
    
    /// Validate that a test database file is a valid SQLite database
    async fn validate_existing_test_db(db_path: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let database_url = format!("sqlite:{}", db_path);
        
        match SqlitePool::connect(&database_url).await {
            Ok(pool) => {
                // Try a simple query to verify the database is not corrupted
                match sqlx::query("SELECT COUNT(*) FROM sqlite_master").fetch_one(&pool).await {
                    Ok(_) => {
                        pool.close().await;
                        Ok(true)
                    }
                    Err(_) => {
                        pool.close().await;
                        Ok(false)
                    }
                }
            }
            Err(_) => Ok(false)
        }
    }
    
    /// Create test database with sample data (fallback if no production db)
    async fn create_test_database_with_sample_data(test_db_path: &str) -> Result<bool, Box<dyn std::error::Error>> {
        println!("🧪 Creating test database with sample data at: {}", test_db_path);
        
        // Create test db directory if it doesn't exist
        if let Some(parent) = Path::new(test_db_path).parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Connect to new database
        let database_url = format!("sqlite:{}", test_db_path);
        let pool = SqlitePool::connect(&database_url).await?;
        
        // Run migrations
        sqlx::migrate!("./db/migrations").run(&pool).await?;
        
        // Load sample data
        let test_data = include_str!("../fixtures/sample_data.sql");
        sqlx::raw_sql(test_data).execute(&pool).await?;
        
        pool.close().await;
        
        println!("✅ Created test database with sample data");
        Ok(false) // Not a copy, created fresh
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
        
        if self.config.use_production_db || self.is_copy {
            // Production database or copy - expect thousands of stocks
            assert!(stock_count > 1000, "Production database should have > 1000 stocks, found {}", stock_count);
            println!("✅ Database verified: {} stocks (production data)", stock_count);
        } else {
            // Sample data database - expect exactly 10 stocks
            assert_eq!(stock_count, 10, "Sample database should have exactly 10 stocks, found {}", stock_count);
            println!("✅ Database verified: {} stocks (sample data)", stock_count);
        }
        
        // Check S&P 500 symbols count
        let sp500_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM sp500_symbols")
            .fetch_one(&self.pool)
            .await?
            .get("count");
        
        if self.config.use_production_db || self.is_copy {
            // Production database or copy - expect hundreds of S&P 500 symbols
            assert!(sp500_count > 400, "Production database should have > 400 S&P 500 symbols, found {}", sp500_count);
            println!("✅ S&P 500 symbols verified: {} (production data)", sp500_count);
        } else {
            // Sample data database - expect exactly 8 symbols
            assert_eq!(sp500_count, 8, "Sample database should have exactly 8 S&P 500 symbols, found {}", sp500_count);
            println!("✅ S&P 500 symbols verified: {} (sample data)", sp500_count);
        }
        
        // Check daily prices count
        let price_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM daily_prices")
            .fetch_one(&self.pool)
            .await?
            .get("count");
        
        if self.config.use_production_db || self.is_copy {
            // Production database or copy - expect millions of price records
            assert!(price_count > 1000000, "Production database should have > 1M price records, found {}", price_count);
            println!("✅ Price data verified: {} records (production data)", price_count);
        } else {
            // Sample data database - expect at least 10 records
            assert!(price_count >= 10, "Sample database should have at least 10 price records, found {}", price_count);
            println!("✅ Price data verified: {} records (sample data)", price_count);
        }
        
        // Check income statements count - this might not exist in production
        let income_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM income_statements")
            .fetch_one(&self.pool)
            .await?
            .get("count");
        
        if self.config.use_production_db || self.is_copy {
            // Production database - expect thousands of income statements
            assert!(income_count > 1000, "Production database should have > 1000 income statements, found {}", income_count);
            println!("✅ Income statements verified: {} records (production data)", income_count);
        } else {
            // Sample data database - expect exactly 4 records
            assert_eq!(income_count, 4, "Sample database should have exactly 4 income statements, found {}", income_count);
            println!("✅ Income statements verified: {} records (sample data)", income_count);
        }
        
        // Check valuation ratios count
        let ratios_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM daily_valuation_ratios")
            .fetch_one(&self.pool)
            .await?
            .get("count");
        
        if self.config.use_production_db || self.is_copy {
            // Production database - expect thousands of ratio records
            assert!(ratios_count > 1000, "Production database should have > 1000 ratio records, found {}", ratios_count);
            println!("✅ Valuation ratios verified: {} records (production data)", ratios_count);
        } else {
            // Sample data database - expect at least 8 records
            assert!(ratios_count >= 8, "Sample database should have at least 8 ratio records, found {}", ratios_count);
            println!("✅ Valuation ratios verified: {} records (sample data)", ratios_count);
        }
        
        let data_type = if self.config.use_production_db || self.is_copy { "production" } else { "sample" };
        println!("✅ Test database verified: {} stocks, {} S&P 500, {} prices, {} income, {} ratios ({})", 
                stock_count, sp500_count, price_count, income_count, ratios_count, data_type);
        
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

/// Mock database connection helper for testing
/// This replaces the real get_database_connection() function during tests
pub async fn get_test_database_connection() -> Result<SqlitePool, String> {
    // This should be set by the test setup
    // For now, we'll use a simple approach where tests set up their own database
    Err("Use TestDatabase::new() instead".to_string())
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