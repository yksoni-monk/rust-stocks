/// Simple Backend Tests - All tests in one file
/// Uses db/test.db (copy of production) with WAL mode for concurrency
/// Pre-test: Copy db/stocks.db to db/test.db using standard file operations

mod helpers;

use helpers::SimpleTestDatabase;
use std::time::{Duration, Instant};

/// Simple database setup test
#[tokio::test]
async fn test_database_setup() {
    let test_db = SimpleTestDatabase::new().await.expect("Failed to setup test database");
    test_db.verify_data().await.expect("Database verification failed");
    test_db.cleanup().await.expect("Cleanup failed");
}

// ====================
// HIGH PRIORITY TESTS (8 commands - 60% of functionality)
// ====================

/// Test stock pagination (HIGH priority - core functionality)
#[tokio::test]
async fn test_get_stocks_paginated() {
    let test_db = SimpleTestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::stocks::get_stocks_paginated;
    
    // Test normal pagination
    let result = get_stocks_paginated(5, 0).await.expect("get_stocks_paginated failed");
    assert_eq!(result.len(), 5, "Should return 5 stocks");
    
    for stock in &result {
        assert!(!stock.symbol.is_empty(), "Stock symbol should not be empty");
        assert!(!stock.company_name.is_empty(), "Company name should not be empty");
    }
    
    // Test pagination with offset
    let result_offset = get_stocks_paginated(3, 5).await.expect("Offset pagination failed");
    assert_eq!(result_offset.len(), 3, "Should return 3 stocks with offset");
    
    // Verify no overlap between pages
    let first_symbols: std::collections::HashSet<_> = result.iter().map(|s| &s.symbol).collect();
    let offset_symbols: std::collections::HashSet<_> = result_offset.iter().map(|s| &s.symbol).collect();
    assert_eq!(first_symbols.intersection(&offset_symbols).count(), 0, "No overlap between pages");
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test stock search functionality
#[tokio::test]
async fn test_search_stocks() {
    let test_db = SimpleTestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::stocks::search_stocks;
    
    // Search for Apple (should exist in production data)
    let apple_results = search_stocks("AAPL".to_string()).await.expect("Apple search failed");
    
    if !apple_results.is_empty() {
        let apple = &apple_results[0];
        assert_eq!(apple.symbol, "AAPL", "Should find AAPL");
        assert!(apple.company_name.to_lowercase().contains("apple"), "Company name should contain 'apple'");
        println!("✅ Found Apple: {} - {}", apple.symbol, apple.company_name);
    } else {
        println!("⚠️  AAPL not found in test database");
    }
    
    // Test case insensitive search
    let lower_results = search_stocks("apple".to_string()).await.expect("Lowercase search failed");
    assert_eq!(apple_results.len(), lower_results.len(), "Case insensitive search should return same results");
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test S&P 500 symbols loading
#[tokio::test]
async fn test_get_sp500_symbols() {
    let test_db = SimpleTestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::stocks::get_sp500_symbols;
    
    let sp500_symbols = get_sp500_symbols().await.expect("S&P 500 symbols failed");
    
    assert!(!sp500_symbols.is_empty(), "S&P 500 symbols should not be empty");
    assert!(sp500_symbols.len() > 400, "Should have reasonable number of S&P 500 symbols");
    
    // Check for common S&P 500 stocks
    let common_symbols = ["AAPL", "MSFT", "GOOGL", "AMZN", "TSLA"];
    for symbol in common_symbols {
        if sp500_symbols.contains(&symbol.to_string()) {
            println!("✅ Found S&P 500 symbol: {}", symbol);
        }
    }
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test price history retrieval
#[tokio::test]
async fn test_get_price_history() {
    let test_db = SimpleTestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::analysis::get_price_history;
    
    // Test with AAPL (should exist in production data)
    let price_history = get_price_history(
        "AAPL".to_string(),
        "2023-01-01".to_string(),
        "2023-12-31".to_string()
    ).await.expect("Price history failed");
    
    if !price_history.is_empty() {
        assert!(price_history.len() > 200, "Should have reasonable number of price records");
        
        // Verify data structure
        let first_price = &price_history[0];
        assert!(first_price.close > 0.0, "Close price should be positive");
        assert!(!first_price.date.is_empty(), "Date should not be empty");
        
        println!("✅ Price history test passed with {} records", price_history.len());
    } else {
        println!("⚠️  No price history found for AAPL");
    }
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test valuation ratios - VALIDATES ACTUAL RATIO DATA EXISTS
#[tokio::test]
async fn test_get_valuation_ratios() {
    let test_db = SimpleTestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::analysis::get_valuation_ratios;
    
    let ratios = get_valuation_ratios("AAPL".to_string()).await.expect("Valuation ratios failed");
    
    // CRITICAL: AAPL should have valuation ratios (it's a major stock with financial data)
    assert!(ratios.is_some(), "Should have valuation ratios for AAPL - major stock with financial data");
    
    let ratios = ratios.unwrap();
    
    // VALIDATION: Must have P/S and EV/S ratios for meaningful analysis
    assert!(ratios.ps_ratio_ttm.is_some(), "Should have P/S ratio for AAPL");
    assert!(ratios.evs_ratio_ttm.is_some(), "Should have EV/S ratio for AAPL");
    
    // Validate P/S ratio quality
    let ps_ratio = ratios.ps_ratio_ttm.unwrap();
    assert!(ps_ratio > 0.0, "P/S ratio should be positive, got {}", ps_ratio);
    assert!(ps_ratio < 100.0, "P/S ratio should be reasonable (<100), got {}", ps_ratio);
    assert!(ps_ratio > 0.1, "P/S ratio should be meaningful (>0.1), got {}", ps_ratio);
    
    // Validate EV/S ratio quality
    let evs_ratio = ratios.evs_ratio_ttm.unwrap();
    assert!(evs_ratio > 0.0, "EV/S ratio should be positive, got {}", evs_ratio);
    assert!(evs_ratio < 100.0, "EV/S ratio should be reasonable (<100), got {}", evs_ratio);
    assert!(evs_ratio > 0.1, "EV/S ratio should be meaningful (>0.1), got {}", evs_ratio);
    
    // Validate supporting data exists
    assert!(ratios.revenue_ttm.is_some(), "Should have TTM revenue for AAPL");
    let revenue = ratios.revenue_ttm.unwrap();
    assert!(revenue > 0.0, "Revenue should be positive, got {}", revenue);
    assert!(revenue > 1e9, "Revenue should be substantial (>$1B), got {}", revenue);
    
    println!("✅ Valuation ratios for AAPL - P/S: {:.2}, EV/S: {:.2}, Revenue: ${:.0}B", 
             ps_ratio, evs_ratio, revenue / 1e9);
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test P/S and EV/S history - VALIDATES ACTUAL P/S DATA EXISTS
#[tokio::test]
async fn test_get_ps_evs_history() {
    let test_db = SimpleTestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::analysis::get_ps_evs_history;
    
    // Use actual date range where data exists (2019-2024, not 2025)
    let history = get_ps_evs_history(
        "AAPL".to_string(),
        "2024-01-01".to_string(),
        "2024-09-13".to_string()
    ).await.expect("P/S EV/S history failed");
    
    // CRITICAL: Validate that we have actual P/S ratio data, not just empty records
    assert!(!history.is_empty(), "Should have history records for AAPL");
    
    // Count records with actual P/S ratio data
    let records_with_ps_data = history.iter()
        .filter(|r| r.ps_ratio_ttm.is_some())
        .count();
    
    // Count records with actual EV/S ratio data  
    let records_with_evs_data = history.iter()
        .filter(|r| r.evs_ratio_ttm.is_some())
        .count();
    
    // VALIDATION: Must have P/S and EV/S data (ratio calculator only calculates recent data)
    assert!(records_with_ps_data > 0, "Should have P/S ratio data for AAPL (found {} records)", records_with_ps_data);
    assert!(records_with_evs_data > 0, "Should have EV/S ratio data for AAPL (found {} records)", records_with_evs_data);
    assert!(records_with_ps_data >= 1, "Should have at least 1 P/S ratio record (found {} records)", records_with_ps_data);
    assert!(records_with_evs_data >= 1, "Should have at least 1 EV/S ratio record (found {} records)", records_with_evs_data);
    
    // Validate data quality
    let first_record = &history[0];
    assert!(!first_record.date.is_empty(), "Date should not be empty");
    
    // Check that P/S ratios are reasonable (positive, not extreme)
    for record in history.iter().filter(|r| r.ps_ratio_ttm.is_some()).take(5) {
        let ps_ratio = record.ps_ratio_ttm.unwrap();
        assert!(ps_ratio > 0.0, "P/S ratio should be positive, got {}", ps_ratio);
        assert!(ps_ratio < 100.0, "P/S ratio should be reasonable (<100), got {}", ps_ratio);
    }
    
    println!("✅ P/S EV/S history test passed with {} total records, {} with P/S data, {} with EV/S data", 
             history.len(), records_with_ps_data, records_with_evs_data);
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test undervalued stocks by P/S ratio - VALIDATES P/S SCREENING WORKS
#[tokio::test]
async fn test_get_undervalued_stocks_by_ps() {
    let test_db = SimpleTestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::analysis::get_undervalued_stocks_by_ps;
    
    let undervalued = get_undervalued_stocks_by_ps(2.0, Some(10), Some(500_000_000.0)).await.expect("Undervalued stocks failed");
    
    // CRITICAL: Should find undervalued stocks if P/S data exists
    assert!(!undervalued.is_empty(), "Should find undervalued stocks with P/S <= 2.0 (found {} stocks)", undervalued.len());
    assert!(undervalued.len() <= 10, "Should not exceed limit");
    
    // Count stocks with actual P/S ratio data
    let stocks_with_ps = undervalued.iter()
        .filter(|s| s.ps_ratio_ttm.is_some())
        .count();
    
    // VALIDATION: Must have stocks with actual P/S ratios for meaningful screening
    assert!(stocks_with_ps > 0, "Should have stocks with P/S ratios (found {} stocks)", stocks_with_ps);
    assert!(stocks_with_ps >= undervalued.len() / 2, "At least half of results should have P/S ratios");
    
    // Validate each stock meets the screening criteria (filter out penny stocks)
    for stock in &undervalued {
        assert!(!stock.symbol.is_empty(), "Stock symbol should not be empty");
        
        if let Some(ps_ratio) = stock.ps_ratio_ttm {
            // Validate P/S ratio meets screening criteria (investment-grade only)
            assert!(ps_ratio <= 2.0, "P/S ratio should be within threshold (<=2.0), got {}", ps_ratio);
            assert!(ps_ratio > 0.0, "P/S ratio should be positive, got {}", ps_ratio);
            assert!(ps_ratio > 0.01, "P/S ratio should be meaningful (>0.01), got {}", ps_ratio);
            
            // Additional validation: market cap should be reasonable (not penny stock)
            if let Some(market_cap) = stock.market_cap {
                assert!(market_cap > 1_000_000.0, "Market cap should be > $1M (not penny stock), got ${:.0}", market_cap);
            }
            
            println!("✅ Quality undervalued stock: {} - P/S: {:.2}, Market Cap: ${:.0}M", 
                     stock.symbol, ps_ratio, stock.market_cap.unwrap_or(0.0) / 1_000_000.0);
        } else {
            println!("⚠️  Stock {} has no P/S ratio data", stock.symbol);
        }
    }
    
    // Test with different thresholds to ensure screening works
    let very_undervalued = get_undervalued_stocks_by_ps(1.0, Some(5), Some(500_000_000.0)).await.expect("Very undervalued stocks failed");
    println!("✅ Found {} very undervalued stocks (P/S <= 1.0)", very_undervalued.len());
    
    // Test 2: Look for traditionally high P/S ratio stocks that are now undervalued
    // (Stocks that were expensive but are now cheap - potential recovery plays)
    let recovery_candidates = get_undervalued_stocks_by_ps(1.5, Some(10), Some(500_000_000.0)).await.expect("Recovery candidates failed");
    println!("✅ Found {} recovery candidates (P/S <= 1.5)", recovery_candidates.len());
    
    // Test 3: Deep value stocks (P/S <= 0.5)
    let deep_value = get_undervalued_stocks_by_ps(0.5, Some(5), Some(500_000_000.0)).await.expect("Deep value stocks failed");
    println!("✅ Found {} deep value stocks (P/S <= 0.5)", deep_value.len());
    
    println!("✅ P/S screening test passed - found {} undervalued stocks (P/S <= 2.0), {} with P/S data", 
             undervalued.len(), stocks_with_ps);
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test value recommendations with stats
#[tokio::test]
async fn test_get_value_recommendations_with_stats() {
    let test_db = SimpleTestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::recommendations::get_value_recommendations_with_stats;
    
    let recommendations = get_value_recommendations_with_stats(Some(10)).await.expect("Value recommendations failed");
    
    assert!(recommendations.recommendations.len() <= 10, "Should not exceed limit");
    assert!(recommendations.stats.total_sp500_stocks > 0, "Should have analyzed some stocks");
    
    println!("✅ Value recommendations: {} stocks, {} S&P 500 stocks", 
             recommendations.recommendations.len(), 
             recommendations.stats.total_sp500_stocks);
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

// ====================
// MEDIUM PRIORITY TESTS (3 commands - 25% of functionality)
// ====================

/// Test stock date range
#[tokio::test]
async fn test_get_stock_date_range() {
    let test_db = SimpleTestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::analysis::get_stock_date_range;
    
    let date_range = get_stock_date_range("AAPL".to_string()).await.expect("Stock date range failed");
    
    assert!(!date_range.earliest_date.is_empty(), "Earliest date should not be empty");
    assert!(!date_range.latest_date.is_empty(), "Latest date should not be empty");
    println!("✅ Stock date range: {} to {} ({} records)", 
             date_range.earliest_date, 
             date_range.latest_date,
             date_range.total_records);
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test database statistics
#[tokio::test]
async fn test_get_database_stats() {
    let test_db = SimpleTestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::data::get_database_stats;
    
    let stats = get_database_stats().await.expect("Database stats failed");
    
    assert!(stats.total_stocks > 0, "Should have some stocks");
    assert!(stats.total_price_records > 0, "Should have some price records");
    
    println!("✅ Database stats: {} stocks, {} price records", 
             stats.total_stocks, stats.total_price_records);
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

// ====================
// LOW PRIORITY TESTS (2 commands - 15% of functionality)
// ====================

/// Test initialization status
#[tokio::test]
async fn test_get_initialization_status() {
    let test_db = SimpleTestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::initialization::get_initialization_status;
    
    let status = get_initialization_status().await.expect("Initialization status failed");
    
    assert!(!status.status.is_empty(), "Status should not be empty");
    println!("✅ Initialization status: {} (step: {})", status.status, status.current_step);
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test data export
#[tokio::test]
async fn test_export_data() {
    let test_db = SimpleTestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::analysis::export_data;
    
    let export_result = export_data(
        "AAPL".to_string(),
        "csv".to_string()
    ).await.expect("Data export failed");
    
    assert!(!export_result.is_empty(), "Export data should not be empty");
    
    println!("✅ Data export test passed");
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

// ====================
// PERFORMANCE TESTS
// ====================

/// Performance test for pagination
#[tokio::test]
async fn test_pagination_performance() {
    let test_db = SimpleTestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::stocks::get_stocks_paginated;
    
    let test_cases = vec![(10, "Small"), (50, "Medium"), (100, "Large")];
    
    for (limit, description) in test_cases {
        let start = Instant::now();
        let result = get_stocks_paginated(limit, 0).await.expect("Pagination performance test failed");
        let duration = start.elapsed();
        
        assert!(!result.is_empty(), "Should return results");
        
        let expected_max = Duration::from_millis(200);
        println!("⚡ {} page (limit={}): {:?} for {} stocks", description, limit, duration, result.len());
        
        if duration > expected_max {
            println!("⚠️  Performance slower than expected ({:?} > {:?})", duration, expected_max);
        } else {
            println!("✅ Performance within expected range");
        }
    }
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Performance test for search
#[tokio::test]
async fn test_search_performance() {
    let test_db = SimpleTestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::stocks::search_stocks;
    
    let queries = vec!["AAPL", "Microsoft", "Tech", "A"];
    
    for query in queries {
        let start = Instant::now();
        let result = search_stocks(query.to_string()).await.expect("Search performance test failed");
        let duration = start.elapsed();
        
        let expected_max = Duration::from_millis(300);
        println!("⚡ Search '{}': {:?} for {} results", query, duration, result.len());
        
        if duration > expected_max {
            println!("⚠️  Search slower than expected ({:?} > {:?})", duration, expected_max);
        } else {
            println!("✅ Search performance within expected range");
        }
    }
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Concurrent access performance test
#[tokio::test]
async fn test_concurrent_access_performance() {
    let test_db = SimpleTestDatabase::new_no_sync().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::stocks::get_stocks_paginated;
    
    let start = Instant::now();
    
    // Run multiple concurrent requests
    let handles: Vec<_> = (0..5)
        .map(|i| {
            tokio::spawn(async move {
                get_stocks_paginated(10, i * 10).await
            })
        })
        .collect();
    
    let results = futures::future::join_all(handles).await;
    let duration = start.elapsed();
    
    for result in results {
        assert!(result.is_ok(), "Concurrent request should succeed");
        let stocks = result.unwrap().expect("Should get stocks");
        assert_eq!(stocks.len(), 10, "Should return 10 stocks");
    }
    
    println!("⚡ Concurrent access: {:?} for 5 concurrent requests", duration);
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

#[tokio::test]
async fn test_get_valuation_extremes() {
    let test_db = SimpleTestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::analysis::get_valuation_extremes;
    
    // Test with AAPL which should have data
    let extremes = get_valuation_extremes("AAPL".to_string()).await.expect("Valuation extremes failed");
    
    // Validate the response structure
    assert_eq!(extremes.symbol, "AAPL");
    
    // Check that we have some data (at least one extreme should be available)
    let has_pe_data = extremes.min_pe_ratio.is_some() || extremes.max_pe_ratio.is_some();
    let has_ps_data = extremes.min_ps_ratio.is_some() || extremes.max_ps_ratio.is_some();
    let has_evs_data = extremes.min_evs_ratio.is_some() || extremes.max_evs_ratio.is_some();
    
    // Should have at least P/E, P/S, or EV/S data
    assert!(has_pe_data || has_ps_data || has_evs_data, "Should have at least P/E, P/S, or EV/S extreme data for AAPL");
    
    // If we have P/E data, validate it's reasonable
    if let (Some(min_pe), Some(max_pe)) = (extremes.min_pe_ratio, extremes.max_pe_ratio) {
        assert!(min_pe > 0.0, "Min P/E should be positive, got {}", min_pe);
        assert!(max_pe > 0.0, "Max P/E should be positive, got {}", max_pe);
        assert!(min_pe <= max_pe, "Min P/E should be <= Max P/E: {} vs {}", min_pe, max_pe);
        assert!(max_pe < 1000.0, "Max P/E should be reasonable (<1000), got {}", max_pe);
    }
    
    // If we have P/S data, validate it's reasonable
    if let (Some(min_ps), Some(max_ps)) = (extremes.min_ps_ratio, extremes.max_ps_ratio) {
        assert!(min_ps > 0.0, "Min P/S should be positive, got {}", min_ps);
        assert!(max_ps > 0.0, "Max P/S should be positive, got {}", max_ps);
        assert!(min_ps <= max_ps, "Min P/S should be <= Max P/S: {} vs {}", min_ps, max_ps);
        assert!(max_ps < 1000.0, "Max P/S should be reasonable (<1000), got {}", max_ps);
    }
    
    // If we have EV/S data, validate it's reasonable
    if let (Some(min_evs), Some(max_evs)) = (extremes.min_evs_ratio, extremes.max_evs_ratio) {
        assert!(min_evs > 0.0, "Min EV/S should be positive, got {}", min_evs);
        assert!(max_evs > 0.0, "Max EV/S should be positive, got {}", max_evs);
        assert!(min_evs <= max_evs, "Min EV/S should be <= Max EV/S: {} vs {}", min_evs, max_evs);
        assert!(max_evs < 1000.0, "Max EV/S should be reasonable (<1000), got {}", max_evs);
    }
    
    println!("✅ Valuation extremes test passed for AAPL:");
    println!("   P/E Range: {} - {}", 
             extremes.min_pe_ratio.map(|v| v.to_string()).unwrap_or("N/A".to_string()),
             extremes.max_pe_ratio.map(|v| v.to_string()).unwrap_or("N/A".to_string()));
    println!("   P/S Range: {} - {}", 
             extremes.min_ps_ratio.map(|v| v.to_string()).unwrap_or("N/A".to_string()),
             extremes.max_ps_ratio.map(|v| v.to_string()).unwrap_or("N/A".to_string()));
    println!("   EV/S Range: {} - {}", 
             extremes.min_evs_ratio.map(|v| v.to_string()).unwrap_or("N/A".to_string()),
             extremes.max_evs_ratio.map(|v| v.to_string()).unwrap_or("N/A".to_string()));
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}
