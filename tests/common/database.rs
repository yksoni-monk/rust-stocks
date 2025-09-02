//! Test database utilities using the existing DatabaseManager

use std::path::PathBuf;
use std::sync::Once;
use anyhow::Result;
use rust_stocks::database::DatabaseManager;

// Global database manager for all tests
static mut DB_MANAGER: Option<DatabaseManager> = None;
static INIT: Once = Once::new();

/// Initialize a completely fresh test database (creates new file each time)
pub fn init_fresh_test_database() -> Result<DatabaseManager> {
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

    // Create database manager using existing infrastructure
    let db_manager = DatabaseManager::new(&database_path)?;

    Ok(db_manager)
}

/// Initialize the test database once for all tests
pub fn init_test_database() -> Result<DatabaseManager> {
    unsafe {
        INIT.call_once(|| {
            // This will be set in the function below
        });
    }

    // Create tests/tmp directory if it doesn't exist
    let tests_dir = PathBuf::from("tests");
    let tmp_dir = tests_dir.join("tmp");
    std::fs::create_dir_all(&tmp_dir)?;

    // Create database file path
    let db_path = tmp_dir.join("test.db");
    let database_path = db_path.to_string_lossy().to_string();

    // Create database manager using existing infrastructure
    let db_manager = DatabaseManager::new(&database_path)?;

    // Store the manager globally
    unsafe {
        DB_MANAGER = Some(db_manager.clone());
    }

    Ok(db_manager)
}

/// Get the global database manager
pub fn get_test_database() -> Result<DatabaseManager> {
    unsafe {
        if let Some(manager) = &DB_MANAGER {
            Ok(manager.clone())
        } else {
            init_test_database()
        }
    }
}

/// Insert sample stock data for testing
pub fn insert_sample_stocks(db_manager: &DatabaseManager) -> Result<()> {
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
        db_manager.upsert_stock(&stock)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_creation() -> Result<()> {
        let db_manager = init_test_database()?;
        
        // Test that we can insert and retrieve data
        insert_sample_stocks(&db_manager)?;
        
        let stocks = db_manager.get_active_stocks()?;
        assert_eq!(stocks.len(), 5);
        assert_eq!(stocks[0].symbol, "AAPL");
        
        Ok(())
    }

    #[test]
    fn test_concurrent_access() -> Result<()> {
        let db_manager = get_test_database()?;
        
        // Test concurrent reads
        let handles: Vec<_> = (0..5).map(|_| {
            let db_manager = db_manager.clone();
            std::thread::spawn(move || {
                db_manager.get_active_stocks()
            })
        }).collect();

        for handle in handles {
            let stocks = handle.join().unwrap()?;
            assert_eq!(stocks.len(), 5);
        }
        
        Ok(())
    }
}
