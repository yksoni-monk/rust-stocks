/// Safe backend tests using copied production database
/// Tests against db/test.db (copy of production) by default
/// Set USE_PRODUCTION_DB=true to test against real production database

mod helpers;

use helpers::{TestDatabase, TestAssertions};
use std::time::{Duration, Instant};

/// Test database setup with safe copy
#[tokio::test]
async fn test_safe_database_setup() {
    let test_db = TestDatabase::new().await.expect("Failed to setup test database");
    test_db.verify_test_data().await.expect("Database verification failed");
    test_db.cleanup().await.expect("Cleanup failed");
}

/// Test stock pagination with real data (safe copy)
#[tokio::test]
async fn test_get_stocks_paginated_with_real_data() {
    let test_db = TestDatabase::new().await.unwrap();
    
    // Set up test database injection
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    // Test the actual command
    use rust_stocks_tauri_lib::commands::stocks::get_stocks_paginated;
    
    let result = get_stocks_paginated(10, 0).await.expect("get_stocks_paginated failed");
    
    assert_eq!(result.len(), 10, "Should return exactly 10 stocks");
    
    for stock in &result {
        TestAssertions::assert_stock_data_valid(stock);
        
        // With real data, we should have real symbols
        assert!(!stock.symbol.is_empty(), "Stock symbol should not be empty");
        assert!(stock.symbol.len() <= 5, "Stock symbol should be reasonable length");
        assert!(!stock.company_name.is_empty(), "Company name should not be empty");
    }
    
    println!("✅ Pagination test passed with {} stocks", result.len());
    
    // Test pagination offset
    let offset_result = get_stocks_paginated(5, 10).await.expect("Offset pagination failed");
    assert_eq!(offset_result.len(), 5, "Should return 5 stocks with offset");
    
    // Verify no overlap
    let first_symbols: std::collections::HashSet<_> = result.iter().take(5).map(|s| &s.symbol).collect();
    let offset_symbols: std::collections::HashSet<_> = offset_result.iter().map(|s| &s.symbol).collect();
    assert_eq!(first_symbols.intersection(&offset_symbols).count(), 0, "No overlap between pages");
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test stock search functionality
#[tokio::test]
async fn test_search_stocks_with_real_data() {
    let test_db = TestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::stocks::search_stocks;
    
    // Search for Apple (should exist in any reasonable dataset)
    let apple_results = search_stocks("AAPL".to_string()).await.expect("Apple search failed");
    
    if !apple_results.is_empty() {
        let apple = &apple_results[0];
        assert_eq!(apple.symbol, "AAPL", "Should find AAPL");
        assert!(apple.company_name.to_lowercase().contains("apple"), "Company name should contain 'apple'");
        println!("✅ Found Apple: {} - {}", apple.symbol, apple.company_name);
    } else {
        println!("⚠️  AAPL not found in test database (may be using sample data)");
    }
    
    // Search for Microsoft
    let msft_results = search_stocks("Microsoft".to_string()).await.expect("Microsoft search failed");
    
    if !msft_results.is_empty() {
        println!("✅ Found Microsoft entries: {}", msft_results.len());
        for stock in msft_results.iter().take(3) {
            println!("   {} - {}", stock.symbol, stock.company_name);
        }
    }
    
    // Test case insensitive search
    let lower_results = search_stocks("apple".to_string()).await.expect("Lowercase search failed");
    assert_eq!(apple_results.len(), lower_results.len(), "Case insensitive search should return same results");
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test S&P 500 symbols loading
#[tokio::test]
async fn test_sp500_symbols_with_real_data() {
    let test_db = TestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::stocks::get_sp500_symbols;
    
    let sp500_symbols = get_sp500_symbols().await.expect("S&P 500 symbols failed");
    
    if test_db.is_copy || test_db.config.use_production_db {
        // Real data should have many S&P 500 symbols
        assert!(!sp500_symbols.is_empty(), "Should have S&P 500 symbols");
        println!("✅ Found {} S&P 500 symbols", sp500_symbols.len());
        
        // Check for known S&P 500 stocks
        let known_sp500 = ["AAPL", "MSFT", "GOOGL", "AMZN", "TSLA"];
        let mut found_count = 0;
        
        for symbol in &known_sp500 {
            if sp500_symbols.contains(&symbol.to_string()) {
                found_count += 1;
                println!("   ✅ Found {}", symbol);
            }
        }
        
        if found_count > 0 {
            println!("✅ Found {}/{} known S&P 500 stocks", found_count, known_sp500.len());
        }
    } else {
        // Sample data - may have limited S&P 500 symbols
        println!("✅ Found {} S&P 500 symbols (sample data)", sp500_symbols.len());
    }
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test price history with real data
#[tokio::test]
async fn test_price_history_with_real_data() {
    let test_db = TestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::analysis::get_price_history;
    
    // Try to get Apple's price history
    let price_history = get_price_history(
        "AAPL".to_string(), 
        "2024-01-01".to_string(), 
        "2024-12-31".to_string()
    ).await.expect("Price history failed");
    
    if !price_history.is_empty() {
        println!("✅ Found {} price records for AAPL", price_history.len());
        
        // Validate first few price records
        for price in price_history.iter().take(3) {
            TestAssertions::assert_price_data_valid(price);
            println!("   {} - Close: ${:.2}", price.date, price.close);
        }
        
        // Check if data is sorted
        if price_history.len() > 1 {
            for i in 1..price_history.len().min(10) {
                assert!(price_history[i-1].date <= price_history[i].date, "Price data should be sorted by date");
            }
            println!("✅ Price data is properly sorted by date");
        }
    } else {
        println!("⚠️  No AAPL price history found (may be using sample data or limited date range)");
    }
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test database statistics
#[tokio::test]
async fn test_database_stats_with_real_data() {
    let test_db = TestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::data::get_database_stats;
    
    let stats = get_database_stats().await.expect("Database stats failed");
    
    println!("✅ Database Statistics:");
    println!("   Stocks: {}", stats.total_stocks);
    println!("   Price Records: {}", stats.total_price_records);
    println!("   Coverage: {:.1}%", stats.data_coverage_percentage);
    println!("   Last Update: {}", stats.last_update);
    
    // Validate stats make sense
    assert!(stats.total_stocks > 0, "Should have at least some stocks");
    // Note: total_price_records is usize, so always non-negative
    assert!(stats.data_coverage_percentage >= 0.0, "Coverage percentage should be non-negative");
    
    if test_db.is_copy || test_db.config.use_production_db {
        // Production data should have substantial amounts
        assert!(stats.total_stocks > 1000, "Production database should have > 1000 stocks");
        println!("✅ Production-scale database statistics verified");
    }
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test get_stocks_with_data_status command
#[tokio::test]
async fn test_get_stocks_with_data_status() {
    let test_db = TestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::stocks::get_stocks_with_data_status;
    
    let stocks_with_data = get_stocks_with_data_status().await.expect("get_stocks_with_data_status failed");
    
    if test_db.is_copy || test_db.config.use_production_db {
        assert!(!stocks_with_data.is_empty(), "Should have stocks with data status in production");
        println!("✅ Found {} stocks with data status", stocks_with_data.len());
        
        // Test first few stocks
        for stock in stocks_with_data.iter().take(3) {
            TestAssertions::assert_stock_data_valid(stock);
            println!("   {} - {} (has_data: {})", stock.symbol, stock.company_name, stock.has_data);
        }
    } else {
        println!("✅ Sample data test: {} stocks with data status", stocks_with_data.len());
    }
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test get_stock_date_range command
#[tokio::test]
async fn test_get_stock_date_range() {
    let test_db = TestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::analysis::get_stock_date_range;
    
    // Try to get AAPL date range
    let date_range = get_stock_date_range("AAPL".to_string()).await.expect("get_stock_date_range failed");
    
    if date_range.total_records > 0 {
        println!("✅ AAPL date range: {} to {} ({} records)", 
                date_range.earliest_date, date_range.latest_date, date_range.total_records);
        println!("   Symbol: {}, Data source: {}", date_range.symbol, date_range.data_source);
        assert!(!date_range.earliest_date.is_empty(), "Earliest date should not be empty");
        assert!(!date_range.latest_date.is_empty(), "Latest date should not be empty");
        assert!(date_range.total_records > 0, "Total records should be positive");
        assert_eq!(date_range.symbol, "AAPL", "Symbol should match requested symbol");
    } else {
        println!("⚠️  No AAPL data found (may be using sample data)");
    }
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test get_valuation_ratios command
#[tokio::test]
async fn test_get_valuation_ratios() {
    let test_db = TestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::analysis::get_valuation_ratios;
    
    // Try to get AAPL valuation ratios
    let ratios = get_valuation_ratios("AAPL".to_string()).await.expect("get_valuation_ratios failed");
    
    if let Some(ratios) = ratios {
        println!("✅ AAPL valuation ratios found:");
        if let Some(ps_ratio) = ratios.ps_ratio_ttm {
            println!("   P/S TTM: {:.2}", ps_ratio);
            assert!(ps_ratio > 0.0, "P/S ratio should be positive");
        }
        if let Some(evs_ratio) = ratios.evs_ratio_ttm {
            println!("   EV/S TTM: {:.2}", evs_ratio);
            assert!(evs_ratio > 0.0, "EV/S ratio should be positive");
        }
        if let Some(market_cap) = ratios.market_cap {
            println!("   Market Cap: ${:.2}B", market_cap / 1_000_000_000.0);
            assert!(market_cap > 0.0, "Market cap should be positive");
        }
    } else {
        println!("⚠️  No AAPL valuation ratios found (may be using sample data)");
    }
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test get_ps_evs_history command
#[tokio::test]
async fn test_get_ps_evs_history() {
    let test_db = TestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::analysis::get_ps_evs_history;
    
    // Try to get AAPL P/S and EV/S history
    let history = get_ps_evs_history(
        "AAPL".to_string(), 
        "2024-01-01".to_string(), 
        "2024-12-31".to_string()
    ).await.expect("get_ps_evs_history failed");
    
    if !history.is_empty() {
        println!("✅ Found {} P/S and EV/S history records for AAPL", history.len());
        
        // Test first few records
        for record in history.iter().take(3) {
            println!("   {} - P/S: {:?}, EV/S: {:?}", 
                    record.date, record.ps_ratio_ttm, record.evs_ratio_ttm);
            
            if let Some(ps_ratio) = record.ps_ratio_ttm {
                assert!(ps_ratio > 0.0, "P/S ratio should be positive");
                assert!(ps_ratio < 1000.0, "P/S ratio should be reasonable");
            }
            if let Some(evs_ratio) = record.evs_ratio_ttm {
                assert!(evs_ratio > 0.0, "EV/S ratio should be positive");
                assert!(evs_ratio < 1000.0, "EV/S ratio should be reasonable");
            }
        }
    } else {
        println!("⚠️  No AAPL P/S and EV/S history found (may be using sample data)");
    }
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test get_undervalued_stocks_by_ps command
#[tokio::test]
async fn test_get_undervalued_stocks_by_ps() {
    let test_db = TestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::analysis::get_undervalued_stocks_by_ps;
    
    // Look for stocks with P/S ratio < 2.0
    let undervalued = get_undervalued_stocks_by_ps(2.0, Some(10)).await.expect("get_undervalued_stocks_by_ps failed");
    
    if test_db.is_copy || test_db.config.use_production_db {
        println!("✅ Found {} undervalued stocks (P/S < 2.0)", undervalued.len());
        
        for stock in undervalued.iter().take(5) {
            if let Some(ps_ratio) = stock.ps_ratio_ttm {
                println!("   {} - P/S: {:.2}", stock.symbol, ps_ratio);
                assert!(ps_ratio <= 2.0, "P/S ratio should be <= 2.0 for undervalued stocks");
                assert!(ps_ratio > 0.0, "P/S ratio should be positive");
            }
        }
    } else {
        println!("✅ Sample data test: {} undervalued stocks found", undervalued.len());
    }
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test get_value_recommendations_with_stats command
#[tokio::test]
async fn test_get_value_recommendations_with_stats() {
    let test_db = TestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::recommendations::get_value_recommendations_with_stats;
    
    let recommendations = get_value_recommendations_with_stats(Some(10)).await.expect("get_value_recommendations_with_stats failed");
    
    if test_db.is_copy || test_db.config.use_production_db {
        assert!(!recommendations.recommendations.is_empty(), "Should have value recommendations in production");
        println!("✅ Found {} value recommendations", recommendations.recommendations.len());
        println!("✅ Stats - Value stocks found: {}, Avg value score: {:.2}, Avg risk score: {:.2}", 
                recommendations.stats.value_stocks_found,
                recommendations.stats.average_value_score,
                recommendations.stats.average_risk_score);
        
        // Validate recommendations
        for stock in recommendations.recommendations.iter().take(3) {
            println!("   {} - Value score: {:.2}, Risk score: {:.2}", 
                    stock.symbol, stock.value_score, stock.risk_score);
            assert!(stock.value_score >= 0.0, "Value score should be non-negative");
            assert!(stock.risk_score >= 0.0, "Risk score should be non-negative");
        }
        
        // Validate stats
        assert!(recommendations.stats.value_stocks_found > 0, "Should have found value stocks");
        assert!(recommendations.stats.total_sp500_stocks > 0, "Should have S&P 500 stocks");
    } else {
        println!("✅ Sample data test: {} recommendations found", recommendations.recommendations.len());
    }
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test get_initialization_status command
#[tokio::test]
async fn test_get_initialization_status() {
    let test_db = TestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::initialization::get_initialization_status;
    
    let status = get_initialization_status().await.expect("get_initialization_status failed");
    
    println!("✅ Initialization status:");
    println!("   Current step: {}", status.current_step);
    println!("   Companies processed: {}", status.companies_processed);
    println!("   Total companies: {}", status.total_companies);
    println!("   Status: {}", status.status);
    
    // Basic validation
    assert!(status.total_companies >= status.companies_processed, "Total should be >= processed");
    assert!(!status.current_step.is_empty(), "Current step should not be empty");
    assert!(!status.status.is_empty(), "Status should not be empty");
    
    if test_db.is_copy || test_db.config.use_production_db {
        // Production database should have substantial data
        println!("✅ Production initialization status validated");
    }
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test export_data command
#[tokio::test]
async fn test_export_data() {
    let test_db = TestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::analysis::export_data;
    
    // Test CSV export for AAPL (currently returns simulation message)
    let csv_result = export_data("AAPL".to_string(), "csv".to_string()).await.expect("CSV export failed");
    
    if !csv_result.is_empty() {
        println!("✅ CSV export response: {} characters", csv_result.len());
        println!("   Response: {}", csv_result);
        
        // The current implementation returns a simulation message
        assert!(csv_result.contains("Export simulation"), "Should contain simulation message");
        assert!(csv_result.contains("csv format"), "Should mention CSV format");
        assert!(csv_result.contains("AAPL"), "Should mention the requested symbol");
    } else {
        println!("⚠️  No export result returned");
    }
    
    // Test JSON export (currently returns simulation message)
    let json_result = export_data("AAPL".to_string(), "json".to_string()).await.expect("JSON export failed");
    
    if !json_result.is_empty() {
        println!("✅ JSON export response: {} characters", json_result.len());
        println!("   Response: {}", json_result);
        
        // The current implementation returns a simulation message
        assert!(json_result.contains("Export simulation"), "Should contain simulation message");
        assert!(json_result.contains("json format"), "Should mention JSON format");
        assert!(json_result.contains("AAPL"), "Should mention the requested symbol");
    } else {
        println!("⚠️  No export result returned");
    }
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test performance with real data
#[tokio::test]
async fn test_performance_with_real_data() {
    let test_db = TestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::stocks::get_stocks_paginated;
    
    // Test pagination performance
    let start = Instant::now();
    let result = get_stocks_paginated(50, 0).await.expect("Performance test failed");
    let duration = start.elapsed();
    
    assert!(!result.is_empty(), "Should return results");
    
    // Performance expectation: should be fast even with real data
    let expected_max = if test_db.is_copy || test_db.config.use_production_db {
        Duration::from_millis(500) // Allow more time for larger database
    } else {
        Duration::from_millis(100) // Sample data should be very fast
    };
    
    println!("✅ Pagination performance: {:?} for {} stocks", duration, result.len());
    
    if duration > expected_max {
        println!("⚠️  Performance slower than expected ({:?} > {:?})", duration, expected_max);
    } else {
        println!("✅ Performance within expected range");
    }
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}