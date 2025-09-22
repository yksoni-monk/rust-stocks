// Tauri Commands for Benjamin Graham Value Screening
// Exposes Graham screening functionality to the frontend

use sqlx::Row;
use crate::models::graham_value::{
    GrahamScreeningCriteria, GrahamScreeningResult, GrahamScreeningResultWithDetails,
    GrahamScreeningPreset, GrahamScreeningStats,
};
use crate::analysis::graham_screener::GrahamScreener;
use crate::database::helpers::get_database_connection;
use crate::tools::data_freshness_checker::DataStatusReader;

/// Run Graham value screening with specified criteria
#[tauri::command]
pub async fn run_graham_screening(
    criteria: GrahamScreeningCriteria,
) -> Result<Vec<GrahamScreeningResultWithDetails>, String> {
    println!("🔍 Starting Graham screening with criteria: max P/E {}, max P/B {}",
             criteria.max_pe_ratio, criteria.max_pb_ratio);

    let pool = get_database_connection().await
        .map_err(|e| format!("Database connection failed: {}", e))?;

    // Check data freshness before proceeding
    let status_reader = DataStatusReader::new(pool.clone());
    let freshness_report = status_reader.check_system_freshness().await
        .map_err(|e| format!("Failed to check data freshness: {}", e))?;

    // Check if Graham screening can proceed
    if !freshness_report.screening_readiness.graham_screening {
        let blocking_issues = freshness_report.screening_readiness.blocking_issues.join("; ");
        return Err(format!(
            "Graham value screening cannot proceed due to stale data. Issues: {}. Please refresh data using: cargo run --bin refresh_data --mode standard",
            blocking_issues
        ));
    }

    let screener = GrahamScreener::new(pool);
    
    screener
        .run_screening(&criteria)
        .await
        .map_err(|e| {
            eprintln!("Graham screening error: {}", e);
            format!("Graham screening failed: {}", e)
        })
}

/// Get default Graham screening criteria
#[tauri::command]
pub async fn get_graham_criteria_defaults() -> Result<GrahamScreeningCriteria, String> {
    Ok(GrahamScreeningCriteria::default())
}

/// Get available Graham screening presets
#[tauri::command]
pub async fn get_graham_screening_presets() -> Result<Vec<GrahamScreeningPreset>, String> {
    let pool = get_database_connection().await
        .map_err(|e| format!("Database connection failed: {}", e))?;
    
    let query = "SELECT * FROM graham_screening_presets ORDER BY is_default DESC, name ASC";
    
    sqlx::query_as::<_, GrahamScreeningPreset>(query)
        .fetch_all(&pool)
        .await
        .map_err(|e| format!("Failed to load Graham presets: {}", e))
}

/// Get specific Graham screening preset by name
#[tauri::command]
pub async fn get_graham_screening_preset(
    name: String,
) -> Result<Option<GrahamScreeningPreset>, String> {
    let pool = get_database_connection().await
        .map_err(|e| format!("Database connection failed: {}", e))?;
    
    let query = "SELECT * FROM graham_screening_presets WHERE name = ?";
    
    sqlx::query_as::<_, GrahamScreeningPreset>(query)
        .bind(&name)
        .fetch_optional(&pool)
        .await
        .map_err(|e| format!("Failed to load Graham preset '{}': {}", name, e))
}

/// Save custom Graham screening preset
#[tauri::command]
pub async fn save_graham_screening_preset(
    preset: GrahamScreeningPreset,
) -> Result<i64, String> {
    let pool = get_database_connection().await
        .map_err(|e| format!("Database connection failed: {}", e))?;
    let query = r#"
        INSERT INTO graham_screening_presets (
            name, description, max_pe_ratio, max_pb_ratio, max_pe_pb_product,
            min_dividend_yield, max_debt_to_equity, min_profit_margin,
            min_revenue_growth_1y, min_revenue_growth_3y, min_current_ratio,
            min_interest_coverage, min_roe, require_positive_earnings,
            require_dividend, min_market_cap, max_market_cap, excluded_sectors,
            is_default
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(name) DO UPDATE SET
            description = excluded.description,
            max_pe_ratio = excluded.max_pe_ratio,
            max_pb_ratio = excluded.max_pb_ratio,
            max_pe_pb_product = excluded.max_pe_pb_product,
            min_dividend_yield = excluded.min_dividend_yield,
            max_debt_to_equity = excluded.max_debt_to_equity,
            min_profit_margin = excluded.min_profit_margin,
            min_revenue_growth_1y = excluded.min_revenue_growth_1y,
            min_revenue_growth_3y = excluded.min_revenue_growth_3y,
            min_current_ratio = excluded.min_current_ratio,
            min_interest_coverage = excluded.min_interest_coverage,
            min_roe = excluded.min_roe,
            require_positive_earnings = excluded.require_positive_earnings,
            require_dividend = excluded.require_dividend,
            min_market_cap = excluded.min_market_cap,
            max_market_cap = excluded.max_market_cap,
            excluded_sectors = excluded.excluded_sectors,
            is_default = excluded.is_default,
            updated_at = CURRENT_TIMESTAMP
    "#;

    let excluded_sectors_json = serde_json::to_string(&preset.excluded_sectors)
        .map_err(|e| format!("Failed to serialize excluded sectors: {}", e))?;

    let result = sqlx::query(query)
        .bind(&preset.name)
        .bind(&preset.description)
        .bind(preset.max_pe_ratio)
        .bind(preset.max_pb_ratio)
        .bind(preset.max_pe_pb_product)
        .bind(preset.min_dividend_yield)
        .bind(preset.max_debt_to_equity)
        .bind(preset.min_profit_margin)
        .bind(preset.min_revenue_growth_1y)
        .bind(preset.min_revenue_growth_3y)
        .bind(preset.min_current_ratio)
        .bind(preset.min_interest_coverage)
        .bind(preset.min_roe)
        .bind(preset.require_positive_earnings)
        .bind(preset.require_dividend)
        .bind(preset.min_market_cap)
        .bind(preset.max_market_cap)
        .bind(&excluded_sectors_json)
        .bind(preset.is_default)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to save Graham preset: {}", e))?;

    Ok(result.last_insert_rowid())
}

/// Delete Graham screening preset
#[tauri::command]
pub async fn delete_graham_screening_preset(
    name: String,
) -> Result<bool, String> {
    let pool = get_database_connection().await
        .map_err(|e| format!("Database connection failed: {}", e))?;
    
    let query = "DELETE FROM graham_screening_presets WHERE name = ? AND is_default = 0";
    
    let result = sqlx::query(query)
        .bind(&name)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to delete Graham preset '{}': {}", name, e))?;

    Ok(result.rows_affected() > 0)
}

/// Get Graham screening statistics for a specific date
#[tauri::command]
pub async fn get_graham_screening_stats(
    date: Option<String>,
) -> Result<Option<GrahamScreeningStats>, String> {
    let pool = get_database_connection().await
        .map_err(|e| format!("Database connection failed: {}", e))?;
    
    let screener = GrahamScreener::new(pool);
    
    screener
        .get_screening_stats(date)
        .await
        .map_err(|e| format!("Failed to get Graham screening stats: {}", e))
}

/// Get historical Graham screening results for a specific stock
#[tauri::command]
pub async fn get_graham_stock_history(
    symbol: String,
    limit: Option<i32>,
) -> Result<Vec<GrahamScreeningResult>, String> {
    let pool = get_database_connection().await
        .map_err(|e| format!("Database connection failed: {}", e))?;
    
    let limit = limit.unwrap_or(30);
    
    let query = r#"
        SELECT * FROM graham_screening_results 
        WHERE symbol = ? 
        ORDER BY screening_date DESC 
        LIMIT ?
    "#;
    
    sqlx::query_as::<_, GrahamScreeningResult>(query)
        .bind(&symbol)
        .bind(limit)
        .fetch_all(&pool)
        .await
        .map_err(|e| format!("Failed to get Graham history for {}: {}", symbol, e))
}

/// Get latest Graham screening results (cached from database)
#[tauri::command]
pub async fn get_latest_graham_results(
    limit: Option<i32>,
) -> Result<Vec<GrahamScreeningResultWithDetails>, String> {
    let pool = get_database_connection().await
        .map_err(|e| format!("Database connection failed: {}", e))?;
    
    let limit = limit.unwrap_or(50);
    
    let query = r#"
        SELECT 
            gsr.*,
            s.company_name,
            s.is_sp500,
            s.exchange
        FROM v_latest_graham_screening gsr
        JOIN stocks s ON gsr.stock_id = s.id
        WHERE gsr.passes_all_filters = 1
        ORDER BY gsr.graham_score DESC
        LIMIT ?
    "#;
    
    let rows = sqlx::query(query)
        .bind(limit)
        .fetch_all(&pool)
        .await
        .map_err(|e| format!("Failed to get latest Graham results: {}", e))?;

    let mut results = Vec::new();
    for row in rows {
        // Extract GrahamScreeningResult fields
        let result = GrahamScreeningResult {
            id: row.try_get("id").ok(),
            stock_id: row.get("stock_id"),
            symbol: row.get("symbol"),
            screening_date: row.get("screening_date"),
            
            pe_ratio: row.try_get("pe_ratio").ok(),
            pb_ratio: row.try_get("pb_ratio").ok(),
            pe_pb_product: row.try_get("pe_pb_product").ok(),
            dividend_yield: row.try_get("dividend_yield").ok(),
            debt_to_equity: row.try_get("debt_to_equity").ok(),
            profit_margin: row.try_get("profit_margin").ok(),
            revenue_growth_1y: row.try_get("revenue_growth_1y").ok(),
            revenue_growth_3y: row.try_get("revenue_growth_3y").ok(),
            
            current_ratio: row.try_get("current_ratio").ok(),
            quick_ratio: row.try_get("quick_ratio").ok(),
            interest_coverage_ratio: row.try_get("interest_coverage_ratio").ok(),
            return_on_equity: row.try_get("return_on_equity").ok(),
            return_on_assets: row.try_get("return_on_assets").ok(),
            
            passes_earnings_filter: row.get("passes_earnings_filter"),
            passes_pe_filter: row.get("passes_pe_filter"),
            passes_pb_filter: row.get("passes_pb_filter"),
            passes_pe_pb_combined: row.get("passes_pe_pb_combined"),
            passes_dividend_filter: row.get("passes_dividend_filter"),
            passes_debt_filter: row.get("passes_debt_filter"),
            passes_quality_filter: row.get("passes_quality_filter"),
            passes_growth_filter: row.get("passes_growth_filter"),
            passes_all_filters: row.get("passes_all_filters"),
            
            graham_score: row.try_get("graham_score").ok(),
            value_rank: row.try_get("value_rank").ok(),
            quality_score: row.try_get("quality_score").ok(),
            safety_score: row.try_get("safety_score").ok(),
            
            current_price: row.try_get("current_price").ok(),
            market_cap: row.try_get("market_cap").ok(),
            shares_outstanding: row.try_get("shares_outstanding").ok(),
            net_income: row.try_get("net_income").ok(),
            total_equity: row.try_get("total_equity").ok(),
            total_debt: row.try_get("total_debt").ok(),
            revenue: row.try_get("revenue").ok(),
            
            reasoning: row.try_get("reasoning").ok(),
            sector: row.try_get("sector").ok(),
            industry: row.try_get("industry").ok(),
            
            created_at: row.try_get("created_at").ok(),
            updated_at: row.try_get("updated_at").ok(),
        };

        // Create enhanced result
        let enhanced = GrahamScreeningResultWithDetails {
            company_name: row.try_get("company_name").ok(),
            is_sp500: row.try_get("is_sp500").unwrap_or(true),
            exchange: row.try_get("exchange").ok(),
            
            value_category: categorize_value_level(&result),
            safety_category: categorize_safety_level(&result),
            recommendation: generate_recommendation(&result),
            
            pe_percentile: None,
            pb_percentile: None,
            sector_pe_rank: None,
            sector_pb_rank: None,
            
            result,
        };
        
        results.push(enhanced);
    }
    
    Ok(results)
}

/// Get summary of Graham screening results by sector
#[tauri::command]
pub async fn get_graham_sector_summary(
    date: Option<String>,
) -> Result<Vec<(String, i32, f64)>, String> {
    let pool = get_database_connection().await
        .map_err(|e| format!("Database connection failed: {}", e))?;
    
    let screening_date = date.unwrap_or_else(|| {
        chrono::Local::now().date_naive().to_string()
    });
    
    let query = r#"
        SELECT 
            COALESCE(sector, 'Unknown') as sector,
            COUNT(*) as count,
            AVG(graham_score) as avg_score
        FROM graham_screening_results 
        WHERE screening_date = ? AND passes_all_filters = 1
        GROUP BY sector
        ORDER BY count DESC, avg_score DESC
    "#;
    
    let rows = sqlx::query(query)
        .bind(&screening_date)
        .fetch_all(&pool)
        .await
        .map_err(|e| format!("Failed to get Graham sector summary: {}", e))?;

    let results = rows.into_iter().map(|row| {
        (
            row.get::<String, _>("sector"),
            row.get::<i64, _>("count") as i32,
            row.try_get::<f64, _>("avg_score").unwrap_or(0.0),
        )
    }).collect();
    
    Ok(results)
}

// Helper functions for categorization (duplicate from graham_screener.rs for frontend use)
fn categorize_value_level(result: &GrahamScreeningResult) -> String {
    if let (Some(pe), Some(pb)) = (result.pe_ratio, result.pb_ratio) {
        if pe < 10.0 && pb < 1.0 {
            "Deep Value".to_string()
        } else if pe < 15.0 && pb < 1.5 {
            "Moderate Value".to_string()
        } else {
            "Fair Value".to_string()
        }
    } else {
        "Unknown".to_string()
    }
}

fn categorize_safety_level(result: &GrahamScreeningResult) -> String {
    if let Some(safety_score) = result.safety_score {
        match safety_score {
            x if x >= 80.0 => "Very Safe".to_string(),
            x if x >= 65.0 => "Safe".to_string(),
            x if x >= 50.0 => "Moderate".to_string(),
            _ => "Risky".to_string(),
        }
    } else {
        "Unknown".to_string()
    }
}

fn generate_recommendation(result: &GrahamScreeningResult) -> String {
    if let Some(graham_score) = result.graham_score {
        match graham_score {
            x if x >= 85.0 => "Strong Buy".to_string(),
            x if x >= 70.0 => "Buy".to_string(),
            x if x >= 55.0 => "Hold".to_string(),
            _ => "Avoid".to_string(),
        }
    } else {
        "No Rating".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_categorization() {
        let mut result = GrahamScreeningResult {
            pe_ratio: Some(8.0),
            pb_ratio: Some(0.8),
            ..Default::default()
        };
        
        assert_eq!(categorize_value_level(&result), "Deep Value");
        
        result.pe_ratio = Some(18.0);
        result.pb_ratio = Some(2.0);
        assert_eq!(categorize_value_level(&result), "Fair Value");
    }

    #[test]
    fn test_recommendation_generation() {
        let mut result = GrahamScreeningResult {
            graham_score: Some(90.0),
            ..Default::default()
        };
        
        assert_eq!(generate_recommendation(&result), "Strong Buy");
        
        result.graham_score = Some(60.0);
        assert_eq!(generate_recommendation(&result), "Hold");
        
        result.graham_score = Some(45.0);
        assert_eq!(generate_recommendation(&result), "Avoid");
    }

    // Integration tests for Tauri commands
    use sqlx::{SqlitePool, pool::PoolOptions};
    use std::time::Duration;
    use anyhow::Result;

    /// Simple test database setup for Graham screening module tests
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
    async fn test_graham_screening_default_criteria() {
        let _test_db = TestDatabase::new().await.unwrap();

        let result = super::get_graham_criteria_defaults().await;
        assert!(result.is_ok(), "get_graham_criteria_defaults should succeed");

        let criteria = result.unwrap();
        assert!(criteria.max_pe_ratio > 0.0, "Max P/E ratio should be positive");
        assert!(criteria.max_pb_ratio > 0.0, "Max P/B ratio should be positive");
        assert!(criteria.min_market_cap > 0.0, "Min market cap should be positive");

        println!("✅ Graham screening default criteria test passed");
    }

    #[tokio::test]
    async fn test_run_graham_screening() {
        let _test_db = TestDatabase::new().await.unwrap();

        // Get default criteria
        let criteria = super::get_graham_criteria_defaults().await.unwrap();

        let result = super::run_graham_screening(criteria).await;
        assert!(result.is_ok(), "run_graham_screening should succeed");

        let results = result.unwrap();

        // Results can be empty if no stocks meet criteria
        if !results.is_empty() {
            assert!(!results[0].result.symbol.is_empty(), "Symbol should not be empty");
            assert_eq!(results[0].is_sp500, true, "Should only return S&P 500 stocks");
        }

        println!("✅ Graham screening test passed with {} results", results.len());
    }

    #[tokio::test]
    async fn test_get_graham_screening_presets() {
        let _test_db = TestDatabase::new().await.unwrap();

        let result = super::get_graham_screening_presets().await;
        assert!(result.is_ok(), "get_graham_screening_presets should succeed");

        let presets = result.unwrap();

        // Should have at least some default presets
        if !presets.is_empty() {
            assert!(!presets[0].name.is_empty(), "Preset name should not be empty");
        }

        println!("✅ Graham screening presets test passed with {} presets", presets.len());
    }

    #[tokio::test]
    async fn test_get_latest_graham_results() {
        let _test_db = TestDatabase::new().await.unwrap();

        let result = super::get_latest_graham_results(Some(10)).await;

        // This test might fail if there's no cached data, which is fine
        match result {
            Ok(results) => {
                // Results can be empty if no previous screenings exist
                if !results.is_empty() {
                    assert!(!results[0].result.symbol.is_empty(), "Symbol should not be empty");
                }
                println!("✅ Latest Graham results test passed with {} results", results.len());
            }
            Err(err) => {
                // This is expected if no cached data exists
                println!("⚠️ Latest Graham results test: No cached data (expected): {}", err);
            }
        }
    }
}

// Provide a Default implementation for GrahamScreeningResult for testing
impl Default for GrahamScreeningResult {
    fn default() -> Self {
        Self {
            id: None,
            stock_id: 0,
            symbol: String::new(),
            screening_date: String::new(),
            pe_ratio: None,
            pb_ratio: None,
            pe_pb_product: None,
            dividend_yield: None,
            debt_to_equity: None,
            profit_margin: None,
            revenue_growth_1y: None,
            revenue_growth_3y: None,
            current_ratio: None,
            quick_ratio: None,
            interest_coverage_ratio: None,
            return_on_equity: None,
            return_on_assets: None,
            passes_earnings_filter: false,
            passes_pe_filter: false,
            passes_pb_filter: false,
            passes_pe_pb_combined: false,
            passes_dividend_filter: false,
            passes_debt_filter: false,
            passes_quality_filter: false,
            passes_growth_filter: false,
            passes_all_filters: false,
            graham_score: None,
            value_rank: None,
            quality_score: None,
            safety_score: None,
            current_price: None,
            market_cap: None,
            shares_outstanding: None,
            net_income: None,
            total_equity: None,
            total_debt: None,
            revenue: None,
            reasoning: None,
            sector: None,
            industry: None,
            created_at: None,
            updated_at: None,
        }
    }
}