//! Database operation tests

use test_log::test;
use pretty_assertions::assert_eq;
use chrono::NaiveDate;
// Stock model is used indirectly through test_data
use crate::common::{test_data, logging, database};

#[test]
fn test_stock_crud_operations() {
    logging::init_test_logging();
    logging::log_test_step("Testing stock CRUD operations");
    
    let db_manager = database::init_test_database().expect("Failed to create test database");
    
    // Test stock insertion
    let stock = test_data::create_test_stock("AAPL", "Apple Inc.");
    let stock_id = db_manager.upsert_stock(&stock).expect("Failed to insert stock");
    logging::log_test_data("Inserted stock", &(stock.symbol.clone(), stock_id));
    
    assert!(stock_id > 0, "Stock ID should be positive");
    
    // Test stock retrieval
    let retrieved_stock = db_manager.get_stock_by_symbol("AAPL").expect("Failed to get stock");
    assert!(retrieved_stock.is_some(), "Stock should exist");
    
    let retrieved_stock = retrieved_stock.unwrap();
    assert_eq!(retrieved_stock.symbol, "AAPL");
    assert_eq!(retrieved_stock.company_name, "Apple Inc.");
    assert_eq!(retrieved_stock.id, Some(stock_id));
    
    // Test stock update
    let mut updated_stock = stock.clone();
    updated_stock.company_name = "Apple Inc. (Updated)".to_string();
    updated_stock.market_cap = Some(2_000_000_000.0);
    
    db_manager.upsert_stock(&updated_stock).expect("Failed to update stock");
    
    // Get the updated stock to verify the ID
    let retrieved_stock = db_manager.get_stock_by_symbol("AAPL").expect("Failed to get updated stock").unwrap();
    let updated_id = retrieved_stock.id.unwrap();
    assert_eq!(updated_id, stock_id, "Update should maintain same ID");
    
    // Verify update
    let retrieved_stock = db_manager.get_stock_by_symbol("AAPL").expect("Failed to get updated stock").unwrap();
    assert_eq!(retrieved_stock.company_name, "Apple Inc. (Updated)");
    assert_eq!(retrieved_stock.market_cap, Some(2_000_000_000.0));
    
    logging::log_test_step("Stock CRUD operations completed successfully");
}

#[test]
fn test_daily_price_operations() {
    logging::init_test_logging();
    logging::log_test_step("Testing daily price operations");
    
    let db_manager = database::init_fresh_test_database().expect("Failed to create test database");
    
    // Insert a stock first
    let stock = test_data::create_test_stock("MSFT", "Microsoft Corporation");
    let stock_id = db_manager.upsert_stock(&stock).expect("Failed to insert stock");
    
    // Test price insertion
    let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
    let price = test_data::create_test_daily_price(stock_id, date);
    
    db_manager.insert_daily_price(&price).expect("Failed to insert price");
    logging::log_test_data("Inserted price", &(stock.symbol.clone(), date));
    
    // Test price retrieval
    let retrieved_price = db_manager.get_price_on_date(stock_id, date).expect("Failed to get price");
    assert!(retrieved_price.is_some(), "Price should exist");
    
    let retrieved_price = retrieved_price.unwrap();
    assert_eq!(retrieved_price.stock_id, stock_id);
    assert_eq!(retrieved_price.date, date);
    assert_eq!(retrieved_price.close_price, 102.0);
    assert_eq!(retrieved_price.volume, Some(1_000_000));
    
    // Test duplicate insertion (should succeed and replace due to INSERT OR REPLACE)
    let duplicate_price = test_data::create_test_daily_price(stock_id, date);
    let result = db_manager.insert_daily_price(&duplicate_price);
    assert!(result.is_ok(), "Duplicate insertion should succeed and replace existing record");
    
    // Test price for non-existent date
    let non_existent_date = NaiveDate::from_ymd_opt(2024, 1, 16).unwrap();
    let retrieved_price = db_manager.get_price_on_date(stock_id, non_existent_date).expect("Failed to get price");
    assert!(retrieved_price.is_none(), "Price should not exist for non-existent date");
    
    logging::log_test_step("Daily price operations completed successfully");
}

#[tokio::test]
async fn test_database_statistics() {
    logging::init_test_logging();
    logging::log_test_step("Testing database statistics");
    
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        async {
            let db_manager = database::init_fresh_test_database().expect("Failed to create test database");
            
            // Don't insert sample data - we want to test with empty database
    
    // Test statistics
    let (stock_count, price_count, last_update) = db_manager.get_stats().expect("Failed to get stats");
    
    logging::log_test_data("Database stats", &(stock_count, price_count, last_update));
    
    assert_eq!(stock_count, 0, "Should have 0 stocks initially");
    assert_eq!(price_count, 0, "Should have 0 price records initially");
    // Note: last_update might be None if no metadata was set, which is fine for this test
    
    // Test individual stock statistics (skip if no stocks)
    if stock_count > 0 {
        let stocks = db_manager.get_active_stocks().expect("Failed to get stocks");
        for stock in &stocks {
            if let Some(stock_id) = stock.id {
                let stats = db_manager.get_stock_data_stats(stock_id).expect("Failed to get stock stats");
                
                logging::log_test_data("Stock stats", &(stock.symbol.clone(), stats.data_points));
                
                assert_eq!(stats.data_points, 0, "Each stock should have 0 data points initially");
                assert!(stats.latest_date.is_none(), "Should not have latest date initially");
                assert!(stats.earliest_date.is_none(), "Should not have earliest date initially");
            }
        }
    }
    
            logging::log_test_step("Database statistics completed successfully");
        }
    ).await;
    
    match result {
        Ok(_) => logging::log_test_step("Database statistics test completed within timeout"),
        Err(_) => panic!("Database statistics test timed out after 30 seconds"),
    }
}

#[tokio::test]
async fn test_existing_records_counting() {
    logging::init_test_logging();
    logging::log_test_step("Testing existing records counting");
    
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        async {
            let db_manager = database::init_test_database().expect("Failed to create test database");
            
            // Set up test data
            database::insert_sample_stocks(&db_manager).expect("Failed to setup test data");
    let stocks = db_manager.get_active_stocks().expect("Failed to get stocks");
    let stock_id = stocks[0].id.unwrap();
    
    // Test counting existing records
    let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let end_date = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap();
    
    let count = db_manager.count_existing_records(stock_id, start_date, end_date).expect("Failed to count records");
    logging::log_test_data("Existing records count", &(stock_id, start_date, end_date, count));
    
    assert_eq!(count, 10, "Should have 10 records in date range");
    
    // Test counting for non-existent date range
    let future_start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let future_end = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
    
    let count = db_manager.count_existing_records(stock_id, future_start, future_end).expect("Failed to count records");
    assert_eq!(count, 0, "Should have 0 records in future date range");
    
    // Test counting for partial date range
    let partial_start = NaiveDate::from_ymd_opt(2024, 1, 5).unwrap();
    let partial_end = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
    
    let count = db_manager.count_existing_records(stock_id, partial_start, partial_end).expect("Failed to count records");
    assert_eq!(count, 11, "Should have 11 records in partial date range");
    
            logging::log_test_step("Existing records counting completed successfully");
        }
    ).await;
    
    match result {
        Ok(_) => logging::log_test_step("Existing records counting test completed within timeout"),
        Err(_) => panic!("Existing records counting test timed out after 30 seconds"),
    }
}

#[test]
fn test_metadata_operations() {
    logging::init_test_logging();
    logging::log_test_step("Testing metadata operations");
    
    let db_manager = database::init_test_database().expect("Failed to create test database");
    
    // Test metadata setting and getting
    let key = "test_key";
    let value = "test_value";
    
    db_manager.set_metadata(key, value).expect("Failed to set metadata");
    
    let retrieved_value = db_manager.get_metadata(key).expect("Failed to get metadata");
    assert!(retrieved_value.is_some(), "Metadata should exist");
    assert_eq!(retrieved_value.unwrap(), value);
    
    // Test metadata update
    let new_value = "updated_value";
    db_manager.set_metadata(key, new_value).expect("Failed to update metadata");
    
    let retrieved_value = db_manager.get_metadata(key).expect("Failed to get updated metadata").unwrap();
    assert_eq!(retrieved_value, new_value);
    
    // Test non-existent metadata
    let non_existent_value = db_manager.get_metadata("non_existent_key").expect("Failed to get non-existent metadata");
    assert!(non_existent_value.is_none(), "Non-existent metadata should return None");
    
    // Test last update date
    let test_date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
    db_manager.set_last_update_date(test_date).expect("Failed to set last update date");
    
    let retrieved_date = db_manager.get_last_update_date().expect("Failed to get last update date");
    assert!(retrieved_date.is_some(), "Last update date should exist");
    assert_eq!(retrieved_date.unwrap(), test_date);
    
    logging::log_test_step("Metadata operations completed successfully");
}

#[test]
fn test_error_handling() {
    logging::init_test_logging();
    logging::log_test_step("Testing error handling");
    
    let db_manager = database::init_test_database().expect("Failed to create test database");
    
    // Test getting non-existent stock
    let non_existent_stock = db_manager.get_stock_by_symbol("NONEXISTENT");
    assert!(non_existent_stock.is_ok(), "Getting non-existent stock should not panic");
    assert!(non_existent_stock.unwrap().is_none(), "Non-existent stock should return None");
    
    // Test getting price for non-existent stock
    let non_existent_price = db_manager.get_price_on_date(999, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
    assert!(non_existent_price.is_ok(), "Getting price for non-existent stock should not panic");
    assert!(non_existent_price.unwrap().is_none(), "Non-existent price should return None");
    
    // Test counting records for non-existent stock
    let non_existent_count = db_manager.count_existing_records(999, 
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), 
        NaiveDate::from_ymd_opt(2024, 1, 10).unwrap()
    );
    assert!(non_existent_count.is_ok(), "Counting records for non-existent stock should not panic");
    assert_eq!(non_existent_count.unwrap(), 0, "Non-existent stock should have 0 records");
    
    logging::log_test_step("Error handling completed successfully");
}

#[tokio::test]
async fn test_database_cleanup() {
    logging::init_test_logging();
    logging::log_test_step("Testing database cleanup");
    
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        async {
            let db_manager = database::init_fresh_test_database().expect("Failed to create test database");
            
            // Set up test data
            database::insert_sample_stocks(&db_manager).expect("Failed to setup test data");
    
    // Verify data exists
    let (stock_count, price_count, _) = db_manager.get_stats().expect("Failed to get stats");
    assert_eq!(stock_count, 5, "Should have 5 stocks before cleanup");
    assert_eq!(price_count, 0, "Should have 0 prices before cleanup");
    
    // Test cleanup
    db_manager.clear_stocks().expect("Failed to cleanup stocks");
    
    // Verify data is gone
    let (stock_count, price_count, _) = db_manager.get_stats().expect("Failed to get stats after cleanup");
    assert_eq!(stock_count, 0, "Should have 0 stocks after cleanup");
    assert_eq!(price_count, 0, "Should have 0 prices after cleanup");
    
            logging::log_test_step("Database cleanup completed successfully");
        }
    ).await;
    
    match result {
        Ok(_) => logging::log_test_step("Database cleanup test completed within timeout"),
        Err(_) => panic!("Database cleanup test timed out after 30 seconds"),
    }
}

#[test]
fn test_pe_ratio_and_market_cap_operations() {
    logging::init_test_logging();
    logging::log_test_step("Testing P/E ratio and market cap operations");
    
    let db_manager = database::init_fresh_test_database().expect("Failed to create test database");
    
    // Insert a stock
    let stock = test_data::create_test_stock("TSLA", "Tesla Inc.");
    let stock_id = db_manager.upsert_stock(&stock).expect("Failed to insert stock");
    
    // Insert price with P/E ratio and market cap
    let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
    let mut price = test_data::create_test_daily_price(stock_id, date);
    price.pe_ratio = Some(150.0);
    price.market_cap = Some(500_000_000_000.0);
    
    db_manager.insert_daily_price(&price).expect("Failed to insert price");
    
    // Test P/E ratio retrieval
    let pe_ratio = db_manager.get_pe_ratio_on_date(stock_id, date).expect("Failed to get P/E ratio");
    assert!(pe_ratio.is_some(), "P/E ratio should exist");
    assert_eq!(pe_ratio.unwrap(), 150.0);
    
    // Test market cap retrieval
    let market_cap = db_manager.get_market_cap_on_date(stock_id, date).expect("Failed to get market cap");
    assert!(market_cap.is_some(), "Market cap should exist");
    assert_eq!(market_cap.unwrap(), 500_000_000_000.0);
    
    // Test for non-existent date
    let non_existent_date = NaiveDate::from_ymd_opt(2024, 1, 16).unwrap();
    let pe_ratio = db_manager.get_pe_ratio_on_date(stock_id, non_existent_date).expect("Failed to get P/E ratio");
    assert!(pe_ratio.is_none(), "P/E ratio should not exist for non-existent date");
    
    let market_cap = db_manager.get_market_cap_on_date(stock_id, non_existent_date).expect("Failed to get market cap");
    assert!(market_cap.is_none(), "Market cap should not exist for non-existent date");
    
    logging::log_test_step("P/E ratio and market cap operations completed successfully");
}
