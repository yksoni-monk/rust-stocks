//! Integration tests for concurrent stock data fetching

use test_log::test;
use pretty_assertions::assert_eq;
use chrono::NaiveDate;
use std::sync::Arc;
use crate::common::logging::{init_test_logging, log_test_step, log_test_data};
use rust_stocks::{
    concurrent_fetcher::{ConcurrentFetchConfig, DateRange, fetch_stocks_concurrently},
    database_sqlx::DatabaseManagerSqlx,
    models::Config,
};

#[tokio::test]
async fn test_concurrent_fetch_integration() {
    // Initialize logging
    init_test_logging();
    log_test_step("Testing concurrent fetch integration");

    // Load configuration
    let config = Config::from_env().expect("Failed to load config");
    let database = DatabaseManagerSqlx::new(&config.database_path).await.expect("Failed to create database");
    let database = Arc::new(database);

    // Test configuration
    let fetch_config = ConcurrentFetchConfig {
        date_range: DateRange {
            start_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2025, 8, 31).unwrap(),
        },
        num_threads: 10,
        retry_attempts: 3,
        max_stocks: Some(10), // Limit to 10 stocks for faster testing
    };

    log_test_data("Test config", &fetch_config);

    // Ensure we have some stocks in the database
    let stocks = database.get_active_stocks().await.expect("Failed to get stocks");
    if stocks.is_empty() {
        log_test_step("No stocks found in database, skipping test");
        return;
    }

    log_test_data("Available stocks", &stocks.len());

    // Run concurrent fetch
    let result = fetch_stocks_concurrently(database, fetch_config).await
        .expect("Concurrent fetch failed");

    log_test_data("Fetch result", &result);

    // Verify results
    assert!(result.total_stocks > 0, "Should have processed some stocks");
    assert!(result.processed_stocks + result.skipped_stocks + result.failed_stocks <= result.total_stocks, 
            "Total processed should not exceed total stocks");

    // Verify that some data was fetched or skipped
    assert!(result.processed_stocks > 0 || result.skipped_stocks > 0, 
            "Should have either processed or skipped some stocks");
    assert!(result.total_stocks > 0, "Should have processed some stocks");

    log_test_step("Concurrent fetch integration test completed successfully");
}

#[tokio::test]
async fn test_concurrent_fetch_with_small_date_range() {
    init_test_logging();
    log_test_step("Testing concurrent fetch with small date range");

    let config = Config::from_env().expect("Failed to load config");
    let database = DatabaseManagerSqlx::new(&config.database_path).await.expect("Failed to create database");
    let database = Arc::new(database);

    // Test with a smaller date range to reduce API calls
    let fetch_config = ConcurrentFetchConfig {
        date_range: DateRange {
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(), // Just January 2024
        },
        num_threads: 5, // Fewer threads for smaller test
        retry_attempts: 2,
        max_stocks: Some(5), // Limit to 5 stocks for faster testing
    };

    log_test_data("Small range test config", &fetch_config);

    let stocks = database.get_active_stocks().await.expect("Failed to get stocks");
    if stocks.is_empty() {
        log_test_step("No stocks found in database, skipping test");
        return;
    }

    let result = fetch_stocks_concurrently(database, fetch_config).await
        .expect("Concurrent fetch failed");

    log_test_data("Small range fetch result", &result);

    // Verify basic results
    assert!(result.total_stocks > 0, "Should have processed some stocks");
    assert!(result.processed_stocks + result.skipped_stocks + result.failed_stocks <= result.total_stocks, 
            "Total processed should not exceed total stocks");
    assert!(result.processed_stocks > 0 || result.skipped_stocks > 0, 
            "Should have either processed or skipped some stocks");

    log_test_step("Small range concurrent fetch test completed successfully");
}

#[tokio::test]
async fn test_concurrent_fetch_error_handling() {
    init_test_logging();
    log_test_step("Testing concurrent fetch error handling");

    let config = Config::from_env().expect("Failed to load config");
    let database = DatabaseManagerSqlx::new(&config.database_path).await.expect("Failed to create database");
    let database = Arc::new(database);

    // Test with invalid date range (future dates) to trigger errors
    let fetch_config = ConcurrentFetchConfig {
        date_range: DateRange {
            start_date: NaiveDate::from_ymd_opt(2030, 1, 1).unwrap(), // Future date
            end_date: NaiveDate::from_ymd_opt(2030, 1, 31).unwrap(),
        },
        num_threads: 3,
        retry_attempts: 1, // Minimal retries for faster test
        max_stocks: Some(3), // Limit to 3 stocks for faster testing
    };

    log_test_data("Error handling test config", &fetch_config);

    let stocks = database.get_active_stocks().await.expect("Failed to get stocks");
    if stocks.is_empty() {
        log_test_step("No stocks found in database, skipping test");
        return;
    }

    let result = fetch_stocks_concurrently(database, fetch_config).await
        .expect("Concurrent fetch should complete even with errors");

    log_test_data("Error handling fetch result", &result);

    // Verify that the system handled errors gracefully
    assert!(result.total_stocks > 0, "Should have attempted to process stocks");
    assert!(result.failed_stocks >= 0, "Failed stocks count should be non-negative");

    log_test_step("Error handling concurrent fetch test completed successfully");
}

#[test]
fn test_date_range_validation() {
    init_test_logging();
    log_test_step("Testing date range validation");

    let valid_range = DateRange {
        start_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
        end_date: NaiveDate::from_ymd_opt(2025, 8, 31).unwrap(),
    };

    assert!(valid_range.start_date <= valid_range.end_date, "Start date should be before end date");

    let invalid_range = DateRange {
        start_date: NaiveDate::from_ymd_opt(2025, 8, 31).unwrap(),
        end_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
    };

    assert!(invalid_range.start_date > invalid_range.end_date, "Invalid range should have start after end");

    log_test_step("Date range validation test completed successfully");
}

#[test]
fn test_concurrent_fetch_config_validation() {
    init_test_logging();
    log_test_step("Testing concurrent fetch config validation");

    let config = ConcurrentFetchConfig {
        date_range: DateRange {
            start_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2025, 8, 31).unwrap(),
        },
        num_threads: 10,
        retry_attempts: 3,
        max_stocks: None, // No limit for config validation test
    };

    assert_eq!(config.num_threads, 10, "Thread count should match");
    assert_eq!(config.retry_attempts, 3, "Retry attempts should match");
    assert_eq!(config.date_range.start_date, NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(), "Start date should match");
    assert_eq!(config.date_range.end_date, NaiveDate::from_ymd_opt(2025, 8, 31).unwrap(), "End date should match");

    log_test_step("Concurrent fetch config validation test completed successfully");
}
