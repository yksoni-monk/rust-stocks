use sqlx::{SqlitePool, pool::PoolOptions};
use std::path::Path;
use std::time::Duration;
use anyhow::Result;

/// Simple test database setup
pub struct SimpleTestDatabase {
    pub pool: SqlitePool,
    pub is_copy: bool,
}

impl SimpleTestDatabase {
    /// Create a new test database connection
    /// Automatically syncs with production database if needed
    pub async fn new() -> Result<Self> {
        println!("🔄 Setting up test database...");
        
        // Get the current working directory and resolve paths
        let current_dir = std::env::current_dir()?;
        let production_db = current_dir.join("db/stocks.db");
        let test_db_path = current_dir.join("db/test.db");
        
        // Check if production database exists
        if !production_db.exists() {
            return Err(anyhow::anyhow!("Production database not found at {}", production_db.display()));
        }
        
        // Ensure test db directory exists
        if let Some(parent) = test_db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Simple sync: Copy production DB to test DB
        let is_copy = Self::sync_test_database(&production_db.to_string_lossy(), &test_db_path.to_string_lossy()).await?;
        
        // Connect to test database with WAL mode for concurrency
        let pool = Self::connect_to_test_database(&test_db_path.to_string_lossy()).await?;
        
        println!("✅ Test database ready (copy: {})", is_copy);
        
        Ok(SimpleTestDatabase {
            pool,
            is_copy,
        })
    }
    
    /// Create a new test database connection without syncing (for concurrent tests)
    pub async fn new_no_sync() -> Result<Self> {
        let current_dir = std::env::current_dir()?;
        let test_db_path = current_dir.join("db/test.db");
        
        // Just connect to existing test database
        let pool = Self::connect_to_test_database(&test_db_path.to_string_lossy()).await?;
        
        Ok(SimpleTestDatabase {
            pool,
            is_copy: false,
        })
    }
    
    /// Simple database sync using file copy (reliable approach)
    async fn sync_test_database(production_db: &str, test_db_path: &str) -> Result<bool> {
        let start_time = std::time::Instant::now();
        
        // Check if test database exists and is newer than production
        if Path::new(test_db_path).exists() {
            let prod_modified = std::fs::metadata(production_db)?.modified()?;
            let test_modified = std::fs::metadata(test_db_path)?.modified()?;
            
            if test_modified >= prod_modified {
                println!("✅ Test database is up to date (no sync needed)");
                return Ok(false);
            }
        }
        
        // Remove existing test database and WAL files to ensure clean copy
        let _ = std::fs::remove_file(test_db_path);
        let _ = std::fs::remove_file(format!("{}-shm", test_db_path));
        let _ = std::fs::remove_file(format!("{}-wal", test_db_path));
        
        println!("📋 Copying production database to test database...");
        
        // Use std::fs::copy for synchronous, reliable copy
        std::fs::copy(production_db, test_db_path)?;
        
        let duration_ms = start_time.elapsed().as_millis();
        println!("✅ Database sync completed in {}ms", duration_ms);
        
        Ok(true)
    }
    
    /// Connect to test database with WAL mode for concurrency
    async fn connect_to_test_database(db_path: &str) -> Result<SqlitePool> {
        let database_url = format!("sqlite:{}", db_path);
        
        let pool = PoolOptions::new()
            .max_connections(10)
            .min_connections(2)
            .acquire_timeout(Duration::from_secs(10))
            .idle_timeout(Some(Duration::from_secs(600)))
            .connect(&database_url).await?;
        
        Ok(pool)
    }
    
    /// Get the database pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
    
    /// Verify test data exists
    pub async fn verify_data(&self) -> Result<()> {
        use sqlx::Row;
        
        // First check if the stocks table exists
        let table_exists: i64 = sqlx::query("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='stocks'")
            .fetch_one(&self.pool)
            .await?
            .get(0);
        
        if table_exists == 0 {
            return Err(anyhow::anyhow!("Stocks table not found in test database"));
        }
        
        // Check if we have stocks
        let stock_count: i64 = sqlx::query("SELECT COUNT(*) FROM stocks")
            .fetch_one(&self.pool)
            .await?
            .get(0);
        
        if stock_count == 0 {
            return Err(anyhow::anyhow!("No stocks found in test database"));
        }
        
        // Check if we have price data
        let price_count: i64 = sqlx::query("SELECT COUNT(*) FROM daily_prices")
            .fetch_one(&self.pool)
            .await?
            .get(0);
        
        if price_count == 0 {
            return Err(anyhow::anyhow!("No price data found in test database"));
        }
        
        println!("✅ Test database verified: {} stocks, {} price records", stock_count, price_count);
        Ok(())
    }
    
    /// Cleanup test database connection
    pub async fn cleanup(&self) -> Result<()> {
        self.pool.close().await;
        Ok(())
    }
}
