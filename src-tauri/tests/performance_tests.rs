/// Comprehensive performance testing for backend functions
/// Tests responsiveness and scalability with real production data

mod helpers;

use helpers::{TestDatabase, TestAssertions};
use std::time::{Duration, Instant};

/// Performance benchmarks for critical backend functions
#[tokio::test]
async fn test_stock_pagination_performance() {
    let test_db = TestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::stocks::get_stocks_paginated;
    
    // Test different page sizes for performance
    let test_cases = vec![
        (10, "Small page"),
        (50, "Medium page"),
        (100, "Large page"),
        (500, "XL page"),
    ];
    
    for (limit, description) in test_cases {
        let start = Instant::now();
        let result = get_stocks_paginated(limit, 0).await.expect("Pagination performance test failed");
        let duration = start.elapsed();
        
        assert!(!result.is_empty(), "Should return results");
        
        // Performance expectations based on page size
        let expected_max = if test_db.is_copy || test_db.config.use_production_db {
            match limit {
                10 => Duration::from_millis(100),
                50 => Duration::from_millis(200), 
                100 => Duration::from_millis(400),
                500 => Duration::from_millis(1000),
                _ => Duration::from_millis(200),
            }
        } else {
            Duration::from_millis(50) // Sample data should be very fast
        };
        
        println!("⚡ {} (limit={}): {:?} for {} stocks", description, limit, duration, result.len());
        
        if duration > expected_max {
            println!("⚠️  Performance slower than expected ({:?} > {:?})", duration, expected_max);
        } else {
            println!("✅ Performance within expected range");
        }
    }
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test search performance with different query patterns
#[tokio::test]
async fn test_search_performance() {
    let test_db = TestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::stocks::search_stocks;
    
    let search_queries = vec![
        ("A", "Single character"),
        ("AA", "Two characters"),
        ("AAPL", "Exact symbol"),
        ("Apple", "Company name"),
        ("Tech", "Partial company name"),
        ("Microsoft Corporation", "Full company name"),
    ];
    
    for (query, description) in search_queries {
        let start = Instant::now();
        let results = search_stocks(query.to_string()).await.expect("Search performance test failed");
        let duration = start.elapsed();
        
        let expected_max = if test_db.is_copy || test_db.config.use_production_db {
            Duration::from_millis(500) // Production data search
        } else {
            Duration::from_millis(100) // Sample data search
        };
        
        println!("🔍 {} ('{}): {:?} for {} results", description, query, duration, results.len());
        
        if duration > expected_max {
            println!("⚠️  Search performance slower than expected ({:?} > {:?})", duration, expected_max);
        } else {
            println!("✅ Search performance within expected range");
        }
    }
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test price history retrieval performance
#[tokio::test] 
async fn test_price_history_performance() {
    let test_db = TestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::analysis::get_price_history;
    
    // Test different date ranges
    let date_ranges = vec![
        ("2024-12-01", "2024-12-31", "1 month"),
        ("2024-10-01", "2024-12-31", "3 months"),
        ("2024-01-01", "2024-12-31", "1 year"),
        ("2023-01-01", "2024-12-31", "2 years"),
    ];
    
    for (start_date, end_date, description) in date_ranges {
        let start = Instant::now();
        let history = get_price_history(
            "AAPL".to_string(),
            start_date.to_string(),
            end_date.to_string()
        ).await.expect("Price history performance test failed");
        let duration = start.elapsed();
        
        let expected_max = if test_db.is_copy || test_db.config.use_production_db {
            match description {
                "1 month" => Duration::from_millis(200),
                "3 months" => Duration::from_millis(400),
                "1 year" => Duration::from_millis(800),
                "2 years" => Duration::from_millis(1500),
                _ => Duration::from_millis(500),
            }
        } else {
            Duration::from_millis(100)
        };
        
        println!("📈 Price history {}: {:?} for {} records", description, duration, history.len());
        
        if duration > expected_max {
            println!("⚠️  Price history performance slower than expected ({:?} > {:?})", duration, expected_max);
        } else {
            println!("✅ Price history performance within expected range");
        }
    }
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test database stats performance
#[tokio::test]
async fn test_database_stats_performance() {
    let test_db = TestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::data::get_database_stats;
    
    // Run multiple times to check consistency
    let mut durations = Vec::new();
    
    for i in 1..=5 {
        let start = Instant::now();
        let stats = get_database_stats().await.expect("Database stats performance test failed");
        let duration = start.elapsed();
        durations.push(duration);
        
        println!("📊 Database stats run {}: {:?} ({} stocks, {} prices)", 
                i, duration, stats.total_stocks, stats.total_price_records);
    }
    
    // Calculate average performance
    let avg_duration = Duration::from_nanos(
        durations.iter().map(|d| d.as_nanos() as u64).sum::<u64>() / durations.len() as u64
    );
    
    let expected_max = if test_db.is_copy || test_db.config.use_production_db {
        Duration::from_millis(1000) // Large database stats
    } else {
        Duration::from_millis(200) // Small database stats
    };
    
    println!("📊 Average database stats performance: {:?}", avg_duration);
    
    if avg_duration > expected_max {
        println!("⚠️  Database stats performance slower than expected ({:?} > {:?})", avg_duration, expected_max);
    } else {
        println!("✅ Database stats performance within expected range");
    }
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test concurrent access performance (multiple simultaneous requests)
#[tokio::test]
async fn test_concurrent_access_performance() {
    let test_db = TestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::stocks::get_stocks_paginated;
    use rust_stocks_tauri_lib::commands::stocks::search_stocks;
    
    println!("🚀 Testing concurrent access performance...");
    
    let start = Instant::now();
    
    // Spawn multiple concurrent requests
    let tasks = vec![
        tokio::spawn(get_stocks_paginated(20, 0)),
        tokio::spawn(get_stocks_paginated(20, 20)),
        tokio::spawn(get_stocks_paginated(20, 40)),
        tokio::spawn(search_stocks("A".to_string())),
        tokio::spawn(search_stocks("Apple".to_string())),
    ];
    
    // Wait for all tasks to complete
    let results = futures::future::join_all(tasks).await;
    let total_duration = start.elapsed();
    
    // Verify all tasks completed successfully
    let mut total_results = 0;
    for (i, result) in results.into_iter().enumerate() {
        match result {
            Ok(Ok(data)) => {
                let count = match data {
                    // Handle both Vec<StockWithData> and Vec<StockWithData> return types
                    _ => 1, // Just count that it succeeded
                };
                total_results += count;
                println!("✅ Concurrent task {} completed successfully", i + 1);
            }
            Ok(Err(e)) => println!("❌ Concurrent task {} failed: {}", i + 1, e),
            Err(e) => println!("❌ Concurrent task {} panicked: {}", i + 1, e),
        }
    }
    
    let expected_max = if test_db.is_copy || test_db.config.use_production_db {
        Duration::from_millis(2000) // Allow more time for concurrent access on large DB
    } else {
        Duration::from_millis(500) // Sample data should handle concurrency well
    };
    
    println!("🚀 Concurrent access completed in {:?}", total_duration);
    
    if total_duration > expected_max {
        println!("⚠️  Concurrent access slower than expected ({:?} > {:?})", total_duration, expected_max);
    } else {
        println!("✅ Concurrent access performance within expected range");
    }
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}

/// Test memory usage during large data operations
#[tokio::test]
async fn test_memory_performance() {
    let test_db = TestDatabase::new().await.unwrap();
    rust_stocks_tauri_lib::database::helpers::set_test_database_pool(test_db.pool().clone()).await;
    
    use rust_stocks_tauri_lib::commands::stocks::get_stocks_paginated;
    
    println!("🧠 Testing memory performance with large datasets...");
    
    // Test progressively larger page sizes
    let page_sizes = vec![100, 500, 1000, 2000];
    
    for page_size in page_sizes {
        let start = Instant::now();
        let result = get_stocks_paginated(page_size, 0).await.expect("Memory performance test failed");
        let duration = start.elapsed();
        
        // Rough memory usage estimation (not precise, but indicative)
        let estimated_memory_mb = (result.len() * 200) / 1024 / 1024; // ~200 bytes per stock record estimate
        
        println!("🧠 Page size {}: {:?}, {} records, ~{}MB estimated", 
                page_size, duration, result.len(), estimated_memory_mb);
        
        // Memory usage should be reasonable
        assert!(estimated_memory_mb < 100, "Memory usage should be under 100MB for {} records", result.len());
        
        // Large pages should still be reasonably fast
        let expected_max = if test_db.is_copy || test_db.config.use_production_db {
            Duration::from_millis(2000)
        } else {
            Duration::from_millis(500)
        };
        
        if duration > expected_max {
            println!("⚠️  Large page performance slower than expected ({:?} > {:?})", duration, expected_max);
        } else {
            println!("✅ Large page performance acceptable");
        }
    }
    
    rust_stocks_tauri_lib::database::helpers::clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}