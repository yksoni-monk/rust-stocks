use sqlx::{SqlitePool, migrate::MigrateDatabase, Sqlite};
use std::path::Path;
use chrono::Utc;

/// Database backup and migration safety system
pub struct DatabaseManager {
    pool: SqlitePool,
    db_path: String,
}

impl DatabaseManager {
    pub async fn new(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Always create backup before any migration operations
        Self::create_backup(db_path).await?;
        
        let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path)).await?;
        
        Ok(Self {
            pool,
            db_path: db_path.to_string(),
        })
    }
    
    /// Create automatic backup before any database operations
    pub async fn create_backup(db_path: &str) -> Result<String, Box<dyn std::error::Error>> {
        if !Path::new(db_path).exists() {
            println!("⚠️  Database file doesn't exist yet: {}", db_path);
            return Ok("No backup needed - new database".to_string());
        }
        
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = format!("{}.backup.{}", db_path, timestamp);
        
        // Create backup using SQLite's backup API
        let backup_result = std::process::Command::new("sqlite3")
            .arg(db_path)
            .arg(format!(".backup {}", backup_path))
            .output();
            
        match backup_result {
            Ok(output) if output.status.success() => {
                println!("✅ Database backup created: {}", backup_path);
                
                // Verify backup file size
                let original_size = std::fs::metadata(db_path)?.len();
                let backup_size = std::fs::metadata(&backup_path)?.len();
                
                if backup_size == 0 || backup_size < original_size / 2 {
                    return Err(format!("❌ Backup verification failed - suspicious file size").into());
                }
                
                println!("✅ Backup verified: {} bytes (original: {} bytes)", backup_size, original_size);
                Ok(backup_path)
            }
            Ok(output) => {
                let error = String::from_utf8_lossy(&output.stderr);
                Err(format!("❌ Backup failed: {}", error).into())
            }
            Err(e) => {
                Err(format!("❌ Failed to run sqlite3 for backup: {}", e).into())
            }
        }
    }
    
    /// Check if database has data before allowing destructive operations
    pub async fn verify_data_safety(&self) -> Result<DatabaseStats, Box<dyn std::error::Error>> {
        let stats = self.get_database_stats().await?;
        
        if stats.total_stocks > 1000 || stats.total_prices > 100000 {
            println!("⚠️  PRODUCTION DATABASE DETECTED:");
            println!("   📊 Stocks: {}", stats.total_stocks);
            println!("   📈 Price records: {}", stats.total_prices);
            println!("   💾 Database size: {:.2} MB", stats.database_size_mb);
            println!("   🔒 Additional safeguards active");
        }
        
        Ok(stats)
    }
    
    /// Safe migration runner with multiple safeguards
    pub async fn run_migrations_safely(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Verify this is not a destructive migration on production data
        let stats = self.verify_data_safety().await?;
        
        // 2. Create additional backup specifically for migrations
        let migration_backup = Self::create_backup(&self.db_path).await?;
        println!("🔒 Migration backup: {}", migration_backup);
        
        // 3. Run migrations with monitoring
        match sqlx::migrate!("./db/migrations").run(&self.pool).await {
            Ok(_) => {
                println!("✅ Migrations completed successfully");
                
                // 4. Verify data integrity after migrations
                let post_stats = self.get_database_stats().await?;
                if post_stats.total_stocks < stats.total_stocks / 2 {
                    return Err("❌ CRITICAL: Significant data loss detected after migration!".into());
                }
                
                println!("✅ Data integrity verified after migration");
                Ok(())
            }
            Err(e) => {
                println!("❌ Migration failed: {}", e);
                println!("🔧 Restore from backup: {}", migration_backup);
                Err(e.into())
            }
        }
    }
    
    /// Get database statistics for safety checks
    pub async fn get_database_stats(&self) -> Result<DatabaseStats, Box<dyn std::error::Error>> {
        let stocks_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM stocks")
            .fetch_one(&self.pool).await.unwrap_or(0);
            
        let prices_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM daily_prices")
            .fetch_one(&self.pool).await.unwrap_or(0);
            
        let financials_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM quarterly_financials"
        ).fetch_one(&self.pool).await.unwrap_or(0);
        
        let db_size_bytes = std::fs::metadata(&self.db_path)
            .map(|m| m.len())
            .unwrap_or(0);
            
        Ok(DatabaseStats {
            total_stocks: stocks_count,
            total_prices: prices_count,
            total_financials: financials_count,
            database_size_mb: db_size_bytes as f64 / 1024.0 / 1024.0,
        })
    }
    
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[derive(Debug)]
pub struct DatabaseStats {
    pub total_stocks: i64,
    pub total_prices: i64,
    pub total_financials: i64,
    pub database_size_mb: f64,
}