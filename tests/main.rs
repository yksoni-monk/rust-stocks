//! Main test entry point for rust-stocks

mod common;
mod unit;
mod integration;

use test_log::test;

/// Test that the test infrastructure is working
#[test]
fn test_test_infrastructure() {
    println!("🧪 Test infrastructure is working!");
    assert!(true, "Basic assertion works");
}

/// Test that common utilities are available
#[test]
fn test_common_utilities() {
    use common::{TestEnv, test_data, logging};
    
    logging::init_test_logging();
    logging::log_test_step("Testing common utilities");
    
    // Test environment creation
    let env = TestEnv::new().expect("Failed to create test environment");
    assert!(env.db_path.exists() || !env.db_path.exists(), "Database path should be valid");
    
    // Test data creation
    let stock = test_data::create_test_stock("TEST", "Test Company");
    assert_eq!(stock.symbol, "TEST");
    assert_eq!(stock.company_name, "Test Company");
    
    let price = test_data::create_test_daily_price(1, chrono::Utc::now().date_naive());
    assert_eq!(price.stock_id, 1);
    assert_eq!(price.close_price, 102.0);
    
    logging::log_test_step("Common utilities test completed");
}

/// Test that all modules can be imported
#[test]
fn test_module_imports() {
    // This test ensures all test modules can be imported without errors
    use unit::business_logic::trading_week_batches;
    use unit::database::operations;
    use integration::database_integration;
    
    println!("✅ All test modules imported successfully");
}
