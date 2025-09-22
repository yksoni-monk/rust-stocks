use crate::models::garp_pe::{GarpPeScreeningResult, GarpPeScreeningCriteria};
use crate::database::helpers::get_database_connection;
use crate::tools::data_freshness_checker::DataStatusReader;

#[tauri::command]
pub async fn get_garp_pe_screening_results(
    stock_tickers: Vec<String>,
    criteria: Option<GarpPeScreeningCriteria>,
    limit: Option<i32>
) -> Result<Vec<GarpPeScreeningResult>, String> {
    let pool = get_database_connection().await?;
    let criteria = criteria.unwrap_or_default();
    let limit_value = limit.unwrap_or(50);

    if stock_tickers.is_empty() {
        return Ok(vec![]);
    }

    // Check data freshness before proceeding
    let status_reader = DataStatusReader::new(pool.clone());
    let freshness_report = status_reader.check_system_freshness().await
        .map_err(|e| format!("Failed to check data freshness: {}", e))?;

    // Check if GARP screening can proceed
    if !freshness_report.screening_readiness.garp_screening {
        let blocking_issues = freshness_report.screening_readiness.blocking_issues.join("; ");
        return Err(format!(
            "GARP P/E screening cannot proceed due to stale data. Issues: {}. Please refresh data using: cargo run --bin refresh_data --mode quick",
            blocking_issues
        ));
    }
    
    // Create placeholders for the IN clause
    let placeholders = stock_tickers.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    
    let query = format!("
        SELECT 
            pra.stock_id,
            pra.symbol,
            pra.sector,
            pra.current_pe_ratio,
            pra.peg_ratio,
            pra.current_price,
            pra.passes_positive_earnings,
            pra.passes_peg_filter,
            pra.current_eps_ttm,
            pra.current_eps_annual,
            pra.eps_growth_rate_ttm,
            pra.eps_growth_rate_annual,
            pra.current_ttm_revenue,
            pra.ttm_growth_rate,
            pra.current_annual_revenue,
            pra.annual_growth_rate,
            pra.passes_revenue_growth_filter,
            pra.current_ttm_net_income,
            pra.net_profit_margin,
            pra.passes_profitability_filter,
            pra.total_debt,
            pra.total_equity,
            pra.debt_to_equity_ratio,
            pra.passes_debt_filter,
            
            -- GARP Score: Revenue Growth % / PEG Ratio
            CASE 
                WHEN pra.peg_ratio > 0 AND pra.ttm_growth_rate IS NOT NULL THEN 
                    CAST(pra.ttm_growth_rate AS REAL) / CAST(pra.peg_ratio AS REAL)
                WHEN pra.peg_ratio > 0 AND pra.annual_growth_rate IS NOT NULL THEN 
                    CAST(pra.annual_growth_rate AS REAL) / CAST(pra.peg_ratio AS REAL)
                ELSE 0.0
            END as garp_score,
            
            -- Quality score (0-100)
            CASE 
                WHEN pra.eps_growth_rate_ttm IS NOT NULL 
                     AND pra.ttm_growth_rate IS NOT NULL 
                     AND pra.net_profit_margin IS NOT NULL 
                     AND pra.debt_to_equity_ratio IS NOT NULL THEN 100
                WHEN pra.eps_growth_rate_ttm IS NOT NULL 
                     AND pra.ttm_growth_rate IS NOT NULL 
                     AND pra.net_profit_margin IS NOT NULL THEN 75
                WHEN pra.eps_growth_rate_ttm IS NOT NULL 
                     AND (pra.ttm_growth_rate IS NOT NULL OR pra.net_profit_margin IS NOT NULL) THEN 50
                ELSE 25
            END as quality_score,
            
            -- Final GARP screening result
            CASE 
                WHEN pra.passes_positive_earnings 
                     AND pra.passes_peg_filter 
                     AND pra.passes_revenue_growth_filter 
                     AND pra.passes_profitability_filter 
                     AND pra.passes_debt_filter
                     AND (CASE 
                        WHEN pra.eps_growth_rate_ttm IS NOT NULL 
                             AND pra.ttm_growth_rate IS NOT NULL 
                             AND pra.net_profit_margin IS NOT NULL 
                             AND pra.debt_to_equity_ratio IS NOT NULL THEN 100
                        WHEN pra.eps_growth_rate_ttm IS NOT NULL 
                             AND pra.ttm_growth_rate IS NOT NULL 
                             AND pra.net_profit_margin IS NOT NULL THEN 75
                        WHEN pra.eps_growth_rate_ttm IS NOT NULL 
                             AND (pra.ttm_growth_rate IS NOT NULL OR pra.net_profit_margin IS NOT NULL) THEN 50
                        ELSE 25
                     END) >= ?
                THEN true
                ELSE false
            END as passes_garp_screening,
            
            pra.market_cap,
            pra.data_completeness_score
            
        FROM peg_ratio_analysis pra
        WHERE pra.symbol IN ({})
          AND (pra.market_cap > ? OR pra.market_cap IS NULL)
        ORDER BY 
            passes_garp_screening DESC,
            garp_score DESC,
            quality_score DESC,
            pra.peg_ratio ASC
        LIMIT ?
    ", placeholders);
    
    let mut query_builder = sqlx::query_as::<_, GarpPeScreeningResult>(&query);
    
    // Bind parameters
    query_builder = query_builder.bind(criteria.min_quality_score);
    
    // Bind stock tickers
    for ticker in &stock_tickers {
        query_builder = query_builder.bind(ticker);
    }
    
    // Bind remaining parameters
    query_builder = query_builder.bind(criteria.min_market_cap);
    query_builder = query_builder.bind(limit_value);
    
    let results = query_builder.fetch_all(&pool).await
        .map_err(|e| format!("GARP P/E screening query failed: {}", e))?;

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{SqlitePool, pool::PoolOptions};
    use std::time::Duration;
    use anyhow::Result;

    /// Simple test database setup for GARP PE module tests
    struct TestDatabase {
        pool: SqlitePool,
    }

    impl TestDatabase {
        async fn new() -> Result<Self> {
            let current_dir = std::env::current_dir()?;
            let test_db_path = current_dir.join("db/test.db");

            let database_url = format!("sqlite:{}", test_db_path.to_string_lossy());

            let pool = PoolOptions::new()
                .max_connections(10)
                .min_connections(2)
                .acquire_timeout(Duration::from_secs(10))
                .idle_timeout(Some(Duration::from_secs(600)))
                .connect(&database_url).await?;

            Ok(TestDatabase { pool })
        }
    }

    #[tokio::test]
    async fn test_garp_pe_screening() {
        let _test_db = TestDatabase::new().await.unwrap();

        // Test with S&P 500 tickers - using common ones that should exist
        let stock_tickers = vec![
            "AAPL".to_string(),
            "MSFT".to_string(),
            "GOOGL".to_string(),
            "AMZN".to_string(),
            "TSLA".to_string(),
        ];

        let criteria = Some(GarpPeScreeningCriteria {
            max_peg_ratio: 2.0,
            min_revenue_growth: 5.0,
            min_profit_margin: 3.0,
            max_debt_to_equity: 3.0,
            min_market_cap: 100_000_000.0,
            min_quality_score: 25,
            require_positive_earnings: true,
        });

        let result = super::get_garp_pe_screening_results(
            stock_tickers,
            criteria,
            Some(10),
        ).await;

        // Note: This test may fail if data is stale, which is expected behavior
        // The test validates that the function executes without panicking
        match result {
            Ok(results) => {
                println!("✅ GARP P/E screening test passed with {} results", results.len());
                // If we get results, validate the structure
                if !results.is_empty() {
                    assert!(!results[0].symbol.is_empty(), "Symbol should not be empty");
                }
            }
            Err(err) => {
                // Check if it's a data freshness error (expected in some cases)
                if err.contains("cannot proceed due to stale data") {
                    println!("⚠️ GARP P/E screening test: Data is stale (expected): {}", err);
                } else {
                    panic!("Unexpected GARP P/E screening error: {}", err);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_garp_pe_screening_empty_tickers() {
        let _test_db = TestDatabase::new().await.unwrap();

        let result = super::get_garp_pe_screening_results(
            vec![], // Empty tickers
            None,
            Some(10),
        ).await;

        assert!(result.is_ok(), "Empty tickers should return Ok with empty results");
        let results = result.unwrap();
        assert!(results.is_empty(), "Should return empty results for empty tickers");

        println!("✅ GARP P/E screening empty tickers test passed");
    }
}
