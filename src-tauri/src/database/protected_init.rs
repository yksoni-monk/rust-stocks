use sqlx::{SqlitePool, Row};
use std::path::Path;
use crate::database::migrations::DatabaseManager;

/// Protected database initialization with safeguards
pub async fn initialize_database_safely(db_path: &str) -> Result<SqlitePool, Box<dyn std::error::Error>> {
    println!("🔒 Initializing database with safety checks: {}", db_path);
    
    // Check if database exists and has data
    if Path::new(db_path).exists() {
        let file_size = std::fs::metadata(db_path)?.len();
        let size_mb = file_size as f64 / 1024.0 / 1024.0;
        
        if size_mb > 10.0 {
            println!("⚠️  EXISTING DATABASE DETECTED: {:.2} MB", size_mb);
            println!("🔒 Running in PRODUCTION SAFETY MODE");
            
            // Quick data check
            let pool = SqlitePool::connect(&format!("sqlite:{}?mode=ro", db_path)).await?;
            let stock_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM stocks")
                .fetch_one(&pool).await.unwrap_or(0);
                
            if stock_count > 100 {
                println!("🚨 CRITICAL: Production database with {} stocks detected!", stock_count);
                println!("🔒 Automatic migrations DISABLED for safety");
                println!("💡 Use manual backup and migration commands only");
                
                pool.close().await;
                return Ok(SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path)).await?);
            }
            pool.close().await;
        }
    }
    
    // Safe to proceed with initialization for small/empty databases
    let db_manager = DatabaseManager::new(db_path).await?;
    
    // Only run migrations on small databases or new databases
    let stats = db_manager.verify_data_safety().await?;
    if stats.total_stocks < 100 && stats.database_size_mb < 50.0 {
        println!("✅ Safe to run migrations on small database");
        db_manager.run_migrations_safely().await?;
    } else {
        println!("⚠️  Skipping automatic migrations - use manual backup and migrate");
    }
    
    Ok(db_manager.pool().clone())
}

/// Manual migration runner with explicit confirmation
pub async fn run_manual_migration(db_path: &str, confirm: bool) -> Result<(), Box<dyn std::error::Error>> {
    if !confirm {
        return Err("Manual migration requires explicit confirmation flag".into());
    }
    
    println!("🔧 Running MANUAL migration with EXPLICIT confirmation");
    let db_manager = DatabaseManager::new(db_path).await?;
    
    // Always create backup for manual migrations
    DatabaseManager::create_backup(db_path).await?;
    
    db_manager.run_migrations_safely().await?;
    
    Ok(())
}