//! Test database utilities using the existing DatabaseManager

use std::path::PathBuf;
use std::sync::Once;
use anyhow::Result;
use rust_stocks::database_sqlx::DatabaseManagerSqlx;

// Global database manager for all tests
static mut DB_MANAGER: Option<DatabaseManagerSqlx> = None;
static INIT: Once = Once::new();

/// Test database info containing both the manager and the file path for cleanup
pub struct TestDatabase {
    pub manager: DatabaseManagerSqlx,
    pub file_path: PathBuf,
}

impl TestDatabase {
    /// Clean up the database file
    pub fn cleanup(&self) -> Result<()> {
        if self.file_path.exists() {
            std::fs::remove_file(&self.file_path)?;
            println!("Cleaned up test database: {}", self.file_path.display());
        }
        Ok(())
    }
}

/// Initialize a completely fresh test database (creates new file each time)
pub async fn init_fresh_test_database() -> Result<DatabaseManagerSqlx> {
    // Create tests/tmp directory if it doesn't exist
    let tests_dir = PathBuf::from("tests");
    let tmp_dir = tests_dir.join("tmp");
    std::fs::create_dir_all(&tmp_dir)?;

    // Create a unique database file path using timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let db_path = tmp_dir.join(format!("test_{}.db", timestamp));
    let database_path = db_path.to_string_lossy().to_string();

    // Convert to absolute path
    let absolute_path = std::fs::canonicalize(&tmp_dir)?
        .join(format!("test_{}.db", timestamp))
        .to_string_lossy()
        .to_string();

    println!("Creating fresh test database at: {}", absolute_path);

    // Create database manager using existing infrastructure
    let db_manager = DatabaseManagerSqlx::new(&database_path).await?;

    Ok(db_manager)
}

/// Initialize a completely fresh test database with cleanup support
/// Returns TestDatabase which includes cleanup functionality
pub async fn init_fresh_test_database_with_cleanup() -> Result<TestDatabase> {
    // Create tests/tmp directory if it doesn't exist
    let tests_dir = PathBuf::from("tests");
    let tmp_dir = tests_dir.join("tmp");
    std::fs::create_dir_all(&tmp_dir)?;

    // Create a unique database file path using timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let db_path = tmp_dir.join(format!("test_{}.db", timestamp));
    let database_path = db_path.to_string_lossy().to_string();

    // Convert to absolute path
    let absolute_path = std::fs::canonicalize(&tmp_dir)?
        .join(format!("test_{}.db", timestamp))
        .to_string_lossy()
        .to_string();

    println!("Creating fresh test database at: {}", absolute_path);

    // Create database manager using existing infrastructure
    let db_manager = DatabaseManagerSqlx::new(&database_path).await?;

    Ok(TestDatabase {
        manager: db_manager,
        file_path: db_path,
    })
}

/// Initialize the test database once for all tests
pub async fn init_test_database() -> Result<DatabaseManagerSqlx> {
    INIT.call_once(|| {
        // This will be set in the function below
    });

    // Create tests/tmp directory if it doesn't exist
    let tests_dir = PathBuf::from("tests");
    let tmp_dir = tests_dir.join("tmp");
    std::fs::create_dir_all(&tmp_dir)?;

    // Create database file path
    let db_path = tmp_dir.join("test.db");
    let database_path = db_path.to_string_lossy().to_string();

    // Create database manager using existing infrastructure
    let db_manager = DatabaseManagerSqlx::new(&database_path).await?;

    // Store the manager globally
    unsafe {
        DB_MANAGER = Some(db_manager.clone());
    }

    Ok(db_manager)
}

/// Get the global database manager
pub async fn get_test_database() -> Result<DatabaseManagerSqlx> {
    unsafe {
        if let Some(manager) = &DB_MANAGER {
            Ok(manager.clone())
        } else {
            init_test_database().await
        }
    }
}

/// Insert sample stock data for testing
pub async fn insert_sample_stocks(db_manager: &DatabaseManagerSqlx) -> Result<()> {
    let sample_stocks = vec![
        ("AAPL", "Apple Inc."),
        ("MSFT", "Microsoft Corporation"),
        ("GOOGL", "Alphabet Inc."),
        ("AMZN", "Amazon.com Inc."),
        ("TSLA", "Tesla Inc."),
    ];

    for (symbol, company_name) in sample_stocks {
        let stock = rust_stocks::models::Stock {
            id: None,
            symbol: symbol.to_string(),
            company_name: company_name.to_string(),
            sector: Some("Technology".to_string()),
            industry: Some("Software".to_string()),
            market_cap: Some(1_000_000_000.0),
            status: rust_stocks::models::StockStatus::Active,
            first_trading_date: Some(chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            last_updated: Some(chrono::Utc::now()),
        };
        db_manager.upsert_stock(&stock).await?;
    }

    println!("Sample stocks inserted successfully");
    Ok(())
}

/// Clean up the test database file
pub fn cleanup_test_database() -> Result<()> {
    let db_path = PathBuf::from("tests/tmp/test.db");
    if db_path.exists() {
        std::fs::remove_file(db_path)?;
        println!("Test database cleaned up");
    }
    Ok(())
}

/// Clean up all test database files in the tmp directory
/// This is useful for cleaning up accumulated test databases
pub fn cleanup_all_test_databases() -> Result<()> {
    let tmp_dir = PathBuf::from("tests/tmp");
    if !tmp_dir.exists() {
        return Ok(());
    }

    let mut cleaned_count = 0;
    for entry in std::fs::read_dir(&tmp_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "db") {
            std::fs::remove_file(&path)?;
            cleaned_count += 1;
        }
    }

    if cleaned_count > 0 {
        println!("Cleaned up {} test database files", cleaned_count);
    }

    Ok(())
}

/// Clean up the entire tmp directory before tests
/// This ensures we start with a clean slate
pub fn cleanup_tmp_directory() -> Result<()> {
    let tmp_dir = PathBuf::from("tests/tmp");
    if tmp_dir.exists() {
        std::fs::remove_dir_all(&tmp_dir)?;
        println!("Cleaned up tmp directory: {}", tmp_dir.display());
    }
    
    // Recreate the directory
    std::fs::create_dir_all(&tmp_dir)?;
    println!("Recreated tmp directory: {}", tmp_dir.display());
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Global test setup - runs once before all tests
    static TEST_SETUP: Once = Once::new();

    fn setup_test_environment() {
        TEST_SETUP.call_once(|| {
            // Clean up tmp directory before any tests run
            if let Err(e) = cleanup_tmp_directory() {
                eprintln!("Warning: Failed to cleanup tmp directory: {}", e);
            }
        });
    }

    #[tokio::test]
    async fn test_database_creation() -> Result<()> {
        setup_test_environment();
        
        let test_db = init_fresh_test_database_with_cleanup().await?;
        let db_manager = &test_db.manager;
        
        // Test that we can insert and retrieve data
        insert_sample_stocks(db_manager).await?;
        
        let stocks = db_manager.get_active_stocks().await?;
        assert_eq!(stocks.len(), 5);
        assert_eq!(stocks[0].symbol, "AAPL");
        
        // Clean up
        test_db.cleanup()?;
        
        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_access() -> Result<()> {
        setup_test_environment();
        
        let db_manager = get_test_database().await?;
        
        // Insert sample data first
        insert_sample_stocks(&db_manager).await?;
        
        // Test concurrent reads
        let handles: Vec<_> = (0..5).map(|_| {
            let db_manager = db_manager.clone();
            tokio::spawn(async move {
                db_manager.get_active_stocks().await
            })
        }).collect();

        for handle in handles {
            let stocks = handle.await.unwrap()?;
            assert_eq!(stocks.len(), 5);
        }
        
        Ok(())
    }
}
