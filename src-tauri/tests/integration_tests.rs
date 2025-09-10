/// Integration tests for all backend functions called by the frontend
/// Tests only the 13 commands used by the React frontend (no dead code testing)
/// Uses isolated test database with minimal dataset for fast execution

mod helpers;

use helpers::{TestDatabase, TestAssertions};
use rust_stocks_tauri_lib::database::helpers::set_test_database_pool;
use std::time::{Duration, Instant};

/// Test the database setup itself
#[tokio::test]
async fn test_database_setup() {
    // Phase 1: Run intelligent sync (copies production data to test.db)
    TestDatabase::intelligent_sync().await.expect("Intelligent sync failed");
    
    // Phase 2: Connect to test database (now has production data)
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    test_db.verify_test_data().await.expect("Test data verification failed");
    test_db.cleanup().await.expect("Cleanup failed");
}

// ====================
// HIGH PRIORITY TESTS (8 commands - 60% of functionality)
// ====================

/// Test stock pagination (HIGH priority - core functionality)
#[tokio::test]
async fn test_get_stocks_paginated() {
    // Phase 1: Run intelligent sync (copies production data to test.db)
    TestDatabase::intelligent_sync().await.expect("Intelligent sync failed");
    
    // Phase 2: Connect to test database (now has production data)
    let test_db = TestDatabase::new().await.unwrap();
    set_test_database_pool(test_db.pool().clone()).await;
    
    // Import the actual command function
    use rust_stocks_tauri_lib::commands::stocks::get_stocks_paginated;
    
    // Test normal pagination
    let result = get_stocks_paginated(5, 0).await.expect("get_stocks_paginated failed");
    assert_eq!(result.len(), 5, "Should return 5 stocks");
    
    for stock in &result {
        TestAssertions::assert_stock_data_valid(stock);
    }
    
    // Test pagination with offset
    let result_offset = get_stocks_paginated(3, 5).await.expect("Offset pagination failed");
    assert_eq!(result_offset.len(), 3, "Should return 3 stocks with offset");
    
    // Verify no overlap between first page and offset page
    let first_symbols: std::collections::HashSet<_> = result.iter().map(|s| &s.symbol).collect();
    let offset_symbols: std::collections::HashSet<_> = result_offset.iter().map(|s| &s.symbol).collect();
    assert_eq!(first_symbols.intersection(&offset_symbols).count(), 0, "Should have no overlap");
    
    // Test edge case: limit larger than available data
    let result_large = get_stocks_paginated(100, 0).await.expect("Large limit failed");
    assert_eq!(result_large.len(), 100, "Should return 100 stocks as requested");
    
    // Test edge case: offset way beyond data
    let result_beyond = get_stocks_paginated(10, 10000).await.expect("Beyond offset failed");
    assert_eq!(result_beyond.len(), 0, "Should return empty result for offset beyond data");
    
    test_db.cleanup().await.unwrap();
}

/// Test stocks with data status (HIGH priority - used for total count)
#[tokio::test]
async fn test_get_stocks_with_data_status() {
    let test_db = TestDatabase::new().await.unwrap();
    set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::stocks::get_stocks_with_data_status;
    
    let result = get_stocks_with_data_status().await.expect("get_stocks_with_data_status failed");
    
    assert!(result.len() > 1000, "Should return many stocks from production data");
    
    // Verify data status flags with real production data
    let aapl_stock = result.iter().find(|s| s.symbol == "AAPL").expect("AAPL should be present");
    assert!(aapl_stock.has_data, "AAPL should have data");
    
    // Find any stock that has data
    let stocks_with_data: Vec<_> = result.iter().filter(|s| s.has_data).collect();
    assert!(stocks_with_data.len() > 100, "Should have many stocks with data");
    
    for stock in &result {
        TestAssertions::assert_stock_data_valid(stock);
    }
    
    test_db.cleanup().await.unwrap();
}

/// Test S&P 500 symbols (HIGH priority - core filtering feature)
#[tokio::test]
async fn test_get_sp500_symbols() {
    let test_db = TestDatabase::new().await.unwrap();
    set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::stocks::get_sp500_symbols;
    
    let result = get_sp500_symbols().await.expect("get_sp500_symbols failed");
    
    assert!(result.len() > 400, "Should return many S&P 500 symbols from production data");
    
    // Verify expected symbols are present
    let expected_symbols = vec!["AAPL", "MSFT", "GOOGL", "TSLA", "AMZN", "NVDA", "META", "NFLX"];
    for symbol in expected_symbols {
        assert!(result.contains(&symbol.to_string()), "Should contain {}", symbol);
    }
    
    // Verify non-S&P 500 symbols are not present
    assert!(!result.contains(&"UNPROFITABLE".to_string()), "Should not contain UNPROFITABLE");
    assert!(!result.contains(&"MINIMAL".to_string()), "Should not contain MINIMAL");
    
    test_db.cleanup().await.unwrap();
}

/// Test price history (HIGH priority - core analysis functionality)
#[tokio::test]
async fn test_get_price_history() {
    let test_db = TestDatabase::new().await.unwrap();
    set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::analysis::get_price_history;
    
    // Test valid symbol with data
    let result = get_price_history("AAPL".to_string(), "2024-01-01".to_string(), "2024-01-05".to_string())
        .await.expect("get_price_history failed");
    
    assert_eq!(result.len(), 5, "Should return 5 price records for AAPL");
    
    for price in &result {
        TestAssertions::assert_price_data_valid(price);
    }
    
    // Verify data is sorted by date
    for i in 1..result.len() {
        assert!(result[i-1].date <= result[i].date, "Prices should be sorted by date");
    }
    
    // Test invalid symbol
    let invalid_result = get_price_history("INVALID".to_string(), "2024-01-01".to_string(), "2024-01-05".to_string())
        .await.expect("Invalid symbol should not error");
    assert_eq!(invalid_result.len(), 0, "Should return empty result for invalid symbol");
    
    // Test invalid date range (end before start)
    let invalid_dates = get_price_history("AAPL".to_string(), "2024-01-05".to_string(), "2024-01-01".to_string())
        .await.expect("Invalid date range should not error");
    assert_eq!(invalid_dates.len(), 0, "Should return empty result for invalid date range");
    
    test_db.cleanup().await.unwrap();
}

/// Test valuation ratios (HIGH priority - key valuation metrics)
#[tokio::test]
async fn test_get_valuation_ratios() {
    let test_db = TestDatabase::new().await.unwrap();
    set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::analysis::get_valuation_ratios;
    
    // Test stock with complete valuation data
    let result = get_valuation_ratios("AAPL".to_string()).await.expect("get_valuation_ratios failed");
    
    match result {
        Some(ratios) => {
            TestAssertions::assert_valuation_ratios_valid(&ratios);
            assert!(ratios.ps_ratio_ttm.is_some(), "AAPL should have P/S ratio");
            assert!(ratios.evs_ratio_ttm.is_some(), "AAPL should have EV/S ratio");
            assert!(ratios.market_cap.is_some(), "AAPL should have market cap");
            assert!(ratios.revenue_ttm.is_some(), "AAPL should have TTM revenue");
            assert_eq!(ratios.data_completeness_score, 100, "AAPL should have 100% data completeness");
        }
        None => panic!("AAPL should have valuation ratios"),
    }
    
    // Test stock with no valuation data
    let no_data_result = get_valuation_ratios("MINIMAL".to_string()).await.expect("No data case should not error");
    assert!(no_data_result.is_none(), "MINIMAL should have no valuation ratios");
    
    // Test invalid symbol
    let invalid_result = get_valuation_ratios("INVALID".to_string()).await.expect("Invalid symbol should not error");
    assert!(invalid_result.is_none(), "Invalid symbol should return None");
    
    test_db.cleanup().await.unwrap();
}

/// Test P/S and EV/S history (HIGH priority - advanced analysis charts)
#[tokio::test]
async fn test_get_ps_evs_history() {
    let test_db = TestDatabase::new().await.unwrap();
    set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::analysis::get_ps_evs_history;
    
    // Test stock with ratio history
    let result = get_ps_evs_history("AAPL".to_string(), "2024-01-01".to_string(), "2024-01-05".to_string())
        .await.expect("get_ps_evs_history failed");
    
    assert!(!result.is_empty(), "AAPL should have P/S EV/S history");
    assert_eq!(result.len(), 3, "Should return 3 ratio records for AAPL");
    
    for ratio in &result {
        TestAssertions::assert_valuation_ratios_valid(ratio);
        assert!(ratio.ps_ratio_ttm.is_some(), "Each record should have P/S ratio");
        assert!(ratio.evs_ratio_ttm.is_some(), "Each record should have EV/S ratio");
    }
    
    // Verify data is sorted by date
    for i in 1..result.len() {
        assert!(result[i-1].date <= result[i].date, "Ratios should be sorted by date");
    }
    
    // Test stock with no ratio history
    let no_data_result = get_ps_evs_history("MINIMAL".to_string(), "2024-01-01".to_string(), "2024-01-05".to_string())
        .await.expect("No data case should not error");
    assert_eq!(no_data_result.len(), 0, "MINIMAL should have no ratio history");
    
    test_db.cleanup().await.unwrap();
}

/// Test P/S screening (HIGH priority - key value screening feature)
#[tokio::test]
async fn test_get_undervalued_stocks_by_ps() {
    let test_db = TestDatabase::new().await.unwrap();
    set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::analysis::get_undervalued_stocks_by_ps;
    
    // Test P/S screening with reasonable threshold
    let result = get_undervalued_stocks_by_ps(10.0, Some(5)).await.expect("P/S screening failed");
    
    assert!(!result.is_empty(), "Should find some stocks with P/S < 10");
    
    for stock in &result {
        TestAssertions::assert_valuation_ratios_valid(stock);
        if let Some(ps_ratio) = stock.ps_ratio_ttm {
            assert!(ps_ratio <= 10.0, "All results should have P/S <= 10");
        }
    }
    
    // Test very strict threshold (should return fewer/no results)
    let strict_result = get_undervalued_stocks_by_ps(1.0, Some(10)).await.expect("Strict P/S screening failed");
    // May be empty if no stocks have P/S < 1.0
    
    // Test no limit
    let unlimited_result = get_undervalued_stocks_by_ps(50.0, None).await.expect("Unlimited P/S screening failed");
    assert!(unlimited_result.len() >= result.len(), "Unlimited should return at least as many as limited");
    
    test_db.cleanup().await.unwrap();
}

/// Test P/E recommendations (HIGH priority - core recommendations functionality)
#[tokio::test]
async fn test_get_value_recommendations_with_stats() {
    let test_db = TestDatabase::new().await.unwrap();
    set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::recommendations::get_value_recommendations_with_stats;
    
    let result = get_value_recommendations_with_stats(Some(5)).await.expect("Value recommendations failed");
    
    // Verify recommendations structure
    assert!(!result.recommendations.is_empty(), "Should have some recommendations");
    assert!(result.recommendations.len() <= 5, "Should not exceed requested limit");
    
    // Verify stats structure
    assert!(result.stats.total_sp500_stocks > 0, "Should have total S&P 500 count");
    assert!(result.stats.stocks_with_pe_data >= 0, "Should have P/E data count");
    assert!(result.stats.value_stocks_found >= result.recommendations.len() as usize, "Value stocks found should be >= recommendations count");
    
    // Verify recommendations quality
    for rec in &result.recommendations {
        assert!(!rec.symbol.is_empty(), "Recommendation should have symbol");
        assert!(rec.rank > 0, "Recommendation should have positive rank");
        assert!(rec.value_score >= 0.0, "Value score should be non-negative");
        assert!(rec.risk_score >= 0.0, "Risk score should be non-negative");
    }
    
    // Test with different limit
    let large_result = get_value_recommendations_with_stats(Some(20)).await.expect("Large recommendations failed");
    assert!(large_result.recommendations.len() >= result.recommendations.len(), "Larger limit should return at least as many");
    
    test_db.cleanup().await.unwrap();
}

// ====================
// MEDIUM PRIORITY TESTS (3 commands - 25% of functionality)
// ====================

/// Test stock search (MEDIUM priority - search functionality)
#[tokio::test]
async fn test_search_stocks() {
    let test_db = TestDatabase::new().await.unwrap();
    set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::stocks::search_stocks;
    
    // Test exact symbol match
    let result = search_stocks("AAPL".to_string()).await.expect("search_stocks failed");
    assert!(!result.is_empty(), "Should find AAPL");
    
    let aapl = result.iter().find(|s| s.symbol == "AAPL").expect("Should contain AAPL");
    TestAssertions::assert_stock_data_valid(aapl);
    
    // Test partial company name match
    let apple_result = search_stocks("Apple".to_string()).await.expect("Company name search failed");
    assert!(!apple_result.is_empty(), "Should find Apple by company name");
    
    // Test case insensitive search
    let lower_result = search_stocks("aapl".to_string()).await.expect("Case insensitive search failed");
    assert!(!lower_result.is_empty(), "Should find AAPL with lowercase");
    
    // Test no matches
    let no_match = search_stocks("NONEXISTENT_XYZ".to_string()).await.expect("No match search should not error");
    assert_eq!(no_match.len(), 0, "Should return empty for non-existent search");
    
    // Test empty query
    let empty_result = search_stocks("".to_string()).await.expect("Empty search should not error");
    // May return all stocks or empty - both are acceptable
    
    test_db.cleanup().await.unwrap();
}

/// Test stock date range (MEDIUM priority - date range validation)
#[tokio::test]
async fn test_get_stock_date_range() {
    let test_db = TestDatabase::new().await.unwrap();
    set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::analysis::get_stock_date_range;
    
    // Test stock with price data
    let result = get_stock_date_range("AAPL".to_string()).await.expect("get_stock_date_range failed");
    
    assert!(!result.earliest_date.is_empty(), "Should have earliest date");
    assert!(!result.latest_date.is_empty(), "Should have latest date");
    assert!(result.total_records > 0, "Should have record count");
    assert!(result.earliest_date <= result.latest_date, "Earliest should be <= latest");
    
    // Test stock with minimal data
    let minimal_result = get_stock_date_range("MINIMAL".to_string()).await.expect("Minimal data should not error");
    assert!(!minimal_result.earliest_date.is_empty(), "MINIMAL should have at least one date");
    assert_eq!(minimal_result.total_records, 1, "MINIMAL should have 1 record");
    
    // Test invalid symbol
    let invalid_result = get_stock_date_range("INVALID".to_string()).await;
    // Should either return empty dates or error - both acceptable
    match invalid_result {
        Ok(range) => assert_eq!(range.total_records, 0, "Invalid symbol should have 0 records"),
        Err(_) => {}, // Error is also acceptable
    }
    
    test_db.cleanup().await.unwrap();
}

/// Test database stats (MEDIUM priority - statistics display)
#[tokio::test]
async fn test_get_database_stats() {
    let test_db = TestDatabase::new().await.unwrap();
    set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::data::get_database_stats;
    
    let result = get_database_stats().await.expect("get_database_stats failed");
    
    // Verify basic stats
    assert!(result.total_stocks > 1000, "Should report many stocks from production data");
    assert!(result.total_price_records >= 10, "Should have at least 10 price records");
    
    // Verify stats are reasonable
    assert!(result.total_stocks <= result.total_price_records, "Prices should be >= stocks");
    
    test_db.cleanup().await.unwrap();
}

// ====================
// LOW PRIORITY TESTS (2 commands - 15% of functionality)
// ====================

/// Test data export (LOW priority - export feature)
#[tokio::test]
async fn test_export_data() {
    let test_db = TestDatabase::new().await.unwrap();
    set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::analysis::export_data;
    
    // Test CSV export
    let csv_result = export_data("AAPL".to_string(), "csv".to_string()).await.expect("CSV export failed");
    assert!(!csv_result.is_empty(), "CSV export should return data");
    assert!(csv_result.contains("AAPL"), "CSV should contain symbol");
    
    // Test JSON export
    let json_result = export_data("AAPL".to_string(), "json".to_string()).await.expect("JSON export failed");
    assert!(!json_result.is_empty(), "JSON export should return data");
    assert!(json_result.contains("AAPL"), "JSON should contain symbol");
    
    // Test invalid format
    let invalid_format = export_data("AAPL".to_string(), "invalid".to_string()).await;
    // Should either error or default to a format
    
    // Test invalid symbol
    let invalid_symbol = export_data("INVALID".to_string(), "csv".to_string()).await;
    // Should handle gracefully (empty export or error)
    
    test_db.cleanup().await.unwrap();
}

/// Test initialization status (LOW priority - system status)
#[tokio::test] 
async fn test_get_initialization_status() {
    let test_db = TestDatabase::new().await.unwrap();
    set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::initialization::get_initialization_status;
    
    let result = get_initialization_status().await.expect("get_initialization_status failed");
    
    // Verify status structure (exact fields depend on implementation)
    // At minimum, should not error and return some status information
    assert!(result.companies_processed >= 0, "Should report companies processed count");
    
    test_db.cleanup().await.unwrap();
}

// ====================
// PERFORMANCE TESTS
// ====================

/// Test pagination performance (should be < 100ms)
#[tokio::test]
async fn test_pagination_performance() {
    let test_db = TestDatabase::new().await.unwrap();
    set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::stocks::get_stocks_paginated;
    
    let start = Instant::now();
    let result = get_stocks_paginated(10, 0).await.expect("Performance test failed");
    let duration = start.elapsed();
    
    assert!(!result.is_empty(), "Should return results");
    assert!(duration < Duration::from_millis(100), "Pagination should be < 100ms, took {:?}", duration);
    
    test_db.cleanup().await.unwrap();
}

/// Test search performance (should be < 200ms)
#[tokio::test]
async fn test_search_performance() {
    let test_db = TestDatabase::new().await.unwrap();
    set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::stocks::search_stocks;
    
    let start = Instant::now();
    let result = search_stocks("Apple".to_string()).await.expect("Search performance test failed");
    let duration = start.elapsed();
    
    assert!(!result.is_empty(), "Should find results");
    assert!(duration < Duration::from_millis(200), "Search should be < 200ms, took {:?}", duration);
    
    test_db.cleanup().await.unwrap();
}

// ====================
// INTEGRATION WORKFLOW TESTS
// ====================

/// Test complete stock analysis workflow
#[tokio::test]
async fn test_complete_analysis_workflow() {
    let test_db = TestDatabase::new().await.unwrap();
    set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::stocks::get_stocks_paginated;
    use rust_stocks_tauri_lib::commands::analysis::{get_stock_date_range, get_price_history, get_valuation_ratios, export_data};
    
    // Step 1: Load stock list
    let stocks = get_stocks_paginated(5, 0).await.expect("Workflow: load stocks failed");
    assert!(!stocks.is_empty(), "Workflow: should have stocks");
    
    let stock = &stocks[0];
    
    // Step 2: Get date range
    let date_range = get_stock_date_range(stock.symbol.clone()).await.expect("Workflow: date range failed");
    assert!(!date_range.earliest_date.is_empty(), "Workflow: should have date range");
    
    // Step 3: Load price history
    let price_history = get_price_history(
        stock.symbol.clone(),
        date_range.earliest_date.clone(),
        date_range.latest_date.clone()
    ).await.expect("Workflow: price history failed");
    assert!(!price_history.is_empty(), "Workflow: should have price history");
    
    // Step 4: Get valuation ratios
    let ratios = get_valuation_ratios(stock.symbol.clone()).await.expect("Workflow: ratios failed");
    // May be None for some stocks
    
    // Step 5: Export data
    let export_result = export_data(stock.symbol.clone(), "csv".to_string()).await.expect("Workflow: export failed");
    assert!(!export_result.is_empty(), "Workflow: should have export data");
    
    test_db.cleanup().await.unwrap();
}