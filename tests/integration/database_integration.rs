//! Database integration tests

use pretty_assertions::assert_eq;
use chrono::NaiveDate;
use crate::common::{test_data, logging, database};

#[tokio::test]
async fn test_full_data_collection_workflow() {
    logging::init_test_logging();
    logging::log_test_step("Testing full data collection workflow");
    
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        async {
            let db_manager = database::init_fresh_test_database().expect("Failed to create test database");
    
    // Step 1: Insert multiple stocks
    let stocks = vec![
        test_data::create_test_stock("AAPL", "Apple Inc."),
        test_data::create_test_stock("MSFT", "Microsoft Corporation"),
        test_data::create_test_stock("GOOGL", "Alphabet Inc."),
        test_data::create_test_stock("AMZN", "Amazon.com Inc."),
        test_data::create_test_stock("TSLA", "Tesla Inc."),
    ];
    
    let mut stock_ids = Vec::new();
    for stock in &stocks {
        let stock_id = db_manager.upsert_stock(stock).expect("Failed to insert stock");
        stock_ids.push(stock_id);
        logging::log_test_data("Inserted stock", &(stock.symbol.clone(), stock_id));
    }
    
    // Step 2: Insert price data for multiple dates
    let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let dates = test_data::create_test_date_range(start_date, 60); // 60 days of data
    
    let mut total_prices = 0;
    for (i, _stock) in stocks.iter().enumerate() {
        let stock_id = stock_ids[i];
        
        for (j, date) in dates.iter().enumerate() {
            let mut price = test_data::create_test_daily_price(stock_id, *date);
            // Vary the price data to make it realistic
            price.open_price += (j as f64) * 0.1;
            price.high_price += (j as f64) * 0.15;
            price.low_price += (j as f64) * 0.05;
            price.close_price += (j as f64) * 0.12;
            price.volume = Some(1_000_000 + (j * 100_000) as i64);
            price.pe_ratio = Some(20.0 + (j as f64) * 0.1);
            price.market_cap = Some(1_000_000_000.0 + (j as f64) * 10_000_000.0);
            
            db_manager.insert_daily_price(&price).expect("Failed to insert price");
            total_prices += 1;
        }
        
        logging::log_test_data("Inserted price data", &(stocks[i].symbol.clone(), dates.len()));
    }
    
    // Step 3: Verify database state
    let (stock_count, price_count, _) = db_manager.get_stats().expect("Failed to get stats");
    assert_eq!(stock_count, 5, "Should have 5 stocks");
    assert!(price_count >= total_prices, "Should have at least the expected number of prices");
    
    // Step 4: Test data retrieval scenarios
    for (i, _stock) in stocks.iter().enumerate() {
        let stock_id = stock_ids[i];
        
        // Test getting latest price
        let latest_price = db_manager.get_latest_price(stock_id).expect("Failed to get latest price");
        assert!(latest_price.is_some(), "Latest price should exist");
        
        let latest_price = latest_price.unwrap();
        assert_eq!(latest_price.stock_id, stock_id);
        assert_eq!(latest_price.date, *dates.last().unwrap());
        
        // Test getting price for specific date
        let mid_date = dates[dates.len() / 2];
        let mid_price = db_manager.get_price_on_date(stock_id, mid_date).expect("Failed to get mid price");
        assert!(mid_price.is_some(), "Mid price should exist");
        
        // Test counting records for date range
        let range_start = dates[10];
        let range_end = dates[20];
        let count = db_manager.count_existing_records(stock_id, range_start, range_end).expect("Failed to count records");
        assert_eq!(count, 11, "Should have 11 records in range");
        
        // Test stock statistics
        let stats = db_manager.get_stock_data_stats(stock_id).expect("Failed to get stock stats");
        assert_eq!(stats.data_points, 60, "Should have 60 data points");
        assert_eq!(stats.latest_date, Some(*dates.last().unwrap()));
        assert_eq!(stats.earliest_date, Some(*dates.first().unwrap()));
        
        logging::log_test_data("Stock verification", &(stocks[i].symbol.clone(), stats.data_points));
    }
    
    // Step 5: Test data analysis scenarios
    let analysis_date = dates[30];
    
    for (i, _stock) in stocks.iter().enumerate() {
        let stock_id = stock_ids[i];
        
        // Test P/E ratio retrieval
        let pe_ratio = db_manager.get_pe_ratio_on_date(stock_id, analysis_date).expect("Failed to get P/E ratio");
        assert!(pe_ratio.is_some(), "P/E ratio should exist");
        assert_eq!(pe_ratio.unwrap(), 20.0 + 30.0 * 0.1);
        
        // Test market cap retrieval
        let market_cap = db_manager.get_market_cap_on_date(stock_id, analysis_date).expect("Failed to get market cap");
        assert!(market_cap.is_some(), "Market cap should exist");
        assert_eq!(market_cap.unwrap(), 1_000_000_000.0 + 30.0 * 10_000_000.0);
    }
    
            logging::log_test_step("Full data collection workflow completed successfully");
        }
    ).await;
    
    match result {
        Ok(_) => logging::log_test_step("Full data collection workflow test completed within timeout"),
        Err(_) => panic!("Full data collection workflow test timed out after 60 seconds"),
    }
}

#[tokio::test]
async fn test_batch_processing_simulation() {
    logging::init_test_logging();
    logging::log_test_step("Testing batch processing simulation");
    
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        async {
            let db_manager = database::init_fresh_test_database().expect("Failed to create test database");
    
    // Simulate the trading week batch processing
    // Create batches (simulating the trading week logic)
    let batches = vec![
        (NaiveDate::from_ymd_opt(2024, 8, 4).unwrap(), NaiveDate::from_ymd_opt(2024, 8, 8).unwrap()),  // Week 1
        (NaiveDate::from_ymd_opt(2024, 8, 11).unwrap(), NaiveDate::from_ymd_opt(2024, 8, 15).unwrap()), // Week 2
        (NaiveDate::from_ymd_opt(2024, 8, 18).unwrap(), NaiveDate::from_ymd_opt(2024, 8, 22).unwrap()), // Week 3
    ];
    
    // Insert a test stock
    let stock = test_data::create_test_stock("AAPL", "Apple Inc.");
    let stock_id = db_manager.upsert_stock(&stock).expect("Failed to insert stock");
    
    // Simulate processing each batch
    let mut total_inserted = 0;
    
    for (batch_num, (batch_start, batch_end)) in batches.iter().enumerate() {
        logging::log_test_step(&format!("Processing batch {}", batch_num + 1));
        
        // Check existing records for this batch
        let existing_count = db_manager.count_existing_records(stock_id, *batch_start, *batch_end).expect("Failed to count existing records");
        
        if existing_count > 0 {
            logging::log_test_data("Skipping batch", &(batch_num + 1, existing_count));
            continue;
        }
        
        // Simulate inserting data for this batch
        let batch_dates = test_data::create_test_date_range(*batch_start, (*batch_end - *batch_start).num_days() + 1);
        
        for (i, date) in batch_dates.iter().enumerate() {
            let mut price = test_data::create_test_daily_price(stock_id, *date);
            price.close_price += (batch_num as f64) * 10.0 + (i as f64) * 0.5;
            price.volume = Some(1_000_000 + (batch_num * 100_000 + i * 10_000) as i64);
            
            db_manager.insert_daily_price(&price).expect("Failed to insert price");
            total_inserted += 1;
        }
        
        logging::log_test_data("Batch completed", &(batch_num + 1, batch_dates.len()));
    }
    
    // Verify final state
    let (_, price_count, _) = db_manager.get_stats().expect("Failed to get stats");
    assert_eq!(price_count, total_inserted, "Should have correct number of inserted prices");
    
    // Test that we can retrieve data from each batch
    for (batch_num, (batch_start, batch_end)) in batches.iter().enumerate() {
        let count = db_manager.count_existing_records(stock_id, *batch_start, *batch_end).expect("Failed to count records");
        assert!(count > 0, "Batch {} should have data", batch_num + 1);
        
        // Test getting a specific date from this batch
        let mid_date = *batch_start + chrono::Duration::days((*batch_end - *batch_start).num_days() / 2);
        let price = db_manager.get_price_on_date(stock_id, mid_date).expect("Failed to get price");
        assert!(price.is_some(), "Should have price for mid date in batch {}", batch_num + 1);
    }
    
            logging::log_test_step("Batch processing simulation completed successfully");
        }
    ).await;
    
    match result {
        Ok(_) => logging::log_test_step("Batch processing simulation test completed within timeout"),
        Err(_) => panic!("Batch processing simulation test timed out after 60 seconds"),
    }
}

#[tokio::test]
async fn test_error_recovery_scenarios() {
    logging::init_test_logging();
    logging::log_test_step("Testing error recovery scenarios");
    
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        async {
            let db_manager = database::init_fresh_test_database().expect("Failed to create test database");
    
    // Test scenario: Insert data, then simulate partial failure and recovery
    
    // Step 1: Insert initial data
    let stock = test_data::create_test_stock("AAPL", "Apple Inc.");
    let stock_id = db_manager.upsert_stock(&stock).expect("Failed to insert stock");
    
    let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let dates = test_data::create_test_date_range(start_date, 30);
    
    // Insert first 20 days of data
    for i in 0..20 {
        let price = test_data::create_test_daily_price(stock_id, dates[i]);
        db_manager.insert_daily_price(&price).expect("Failed to insert price");
    }
    
    // Verify initial state
    let (_, price_count, _) = db_manager.get_stats().expect("Failed to get stats");
    assert!(price_count >= 20, "Should have at least 20 prices initially");
    
    // Step 2: Simulate failure and partial data loss
    // (In real scenario, this might be due to database corruption, network issues, etc.)
    logging::log_test_step("Simulating partial data loss");
    
    // Step 3: Attempt to recover by re-inserting missing data
    let recovery_start = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(); // Start from middle
    let recovery_end = NaiveDate::from_ymd_opt(2024, 1, 30).unwrap();
    
    let recovery_dates = test_data::create_test_date_range(recovery_start, (recovery_end - recovery_start).num_days() + 1);
    
    let mut recovered_count = 0;
    for date in recovery_dates {
        // Check if data already exists
        let existing_price = db_manager.get_price_on_date(stock_id, date).expect("Failed to check existing price");
        
        if existing_price.is_none() {
            let price = test_data::create_test_daily_price(stock_id, date);
            db_manager.insert_daily_price(&price).expect("Failed to insert recovery price");
            recovered_count += 1;
        }
    }
    
    logging::log_test_data("Recovery completed", &recovered_count);
    
    // Step 4: Verify final state
    let (_, final_price_count, _) = db_manager.get_stats().expect("Failed to get final stats");
    assert_eq!(final_price_count, 30, "Should have 30 prices after recovery");
    
    // Step 5: Test data integrity
    for date in dates {
        let price = db_manager.get_price_on_date(stock_id, date).expect("Failed to get price");
        assert!(price.is_some(), "Should have price for all dates");
    }
    
            logging::log_test_step("Error recovery scenarios completed successfully");
        }
    ).await;
    
    match result {
        Ok(_) => logging::log_test_step("Error recovery scenarios test completed within timeout"),
        Err(_) => panic!("Error recovery scenarios test timed out after 60 seconds"),
    }
}
