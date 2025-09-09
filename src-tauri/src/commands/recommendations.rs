use sqlx::{SqlitePool, Row};
use crate::analysis::recommendation_engine::{RecommendationEngine, StockRecommendation, RecommendationStats, RecommendationResponse};
use crate::analysis::pe_statistics::PEAnalysis;

async fn get_database_connection() -> Result<SqlitePool, String> {
    let database_url = "sqlite:db/stocks.db";
    SqlitePool::connect(database_url).await
        .map_err(|e| format!("Database connection failed: {}", e))
}

#[tauri::command]
pub async fn get_value_recommendations_with_stats(
    limit: Option<usize>,
) -> Result<RecommendationResponse, String> {
    let pool = get_database_connection().await?;
    let engine = RecommendationEngine::new(pool);
    
    engine
        .get_value_recommendations_with_stats(limit)
        .await
        .map_err(|e| format!("Failed to get value recommendations with stats: {}", e))
}

#[tauri::command]
pub async fn get_value_recommendations(
    limit: Option<usize>,
) -> Result<Vec<StockRecommendation>, String> {
    let pool = get_database_connection().await?;
    let engine = RecommendationEngine::new(pool);
    
    engine
        .get_value_recommendations(limit)
        .await
        .map_err(|e| format!("Failed to get value recommendations: {}", e))
}

#[tauri::command]
pub async fn analyze_sp500_pe_values() -> Result<Vec<PEAnalysis>, String> {
    let pool = get_database_connection().await?;
    let engine = RecommendationEngine::new(pool);
    
    engine
        .analyze_sp500_pe_values()
        .await
        .map_err(|e| format!("Failed to analyze S&P 500 P/E values: {}", e))
}

#[tauri::command]
pub async fn get_recommendation_stats() -> Result<RecommendationStats, String> {
    let pool = get_database_connection().await?;
    let engine = RecommendationEngine::new(pool);
    
    engine
        .get_recommendation_stats()
        .await
        .map_err(|e| format!("Failed to get recommendation stats: {}", e))
}

#[tauri::command]
pub async fn analyze_stock_pe_history(
    symbol: String,
) -> Result<Option<PEAnalysis>, String> {
    let pool = get_database_connection().await?;
    let engine = RecommendationEngine::new(pool.clone());
    
    // Get stock details first
    let stock_query = "SELECT id, symbol, company_name FROM stocks WHERE symbol = ?";
    let row = sqlx::query(stock_query)
        .bind(&symbol)
        .fetch_optional(&pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    
    if let Some(row) = row {
        let stock_id: i64 = row.get("id");
        let symbol: String = row.get("symbol");
        let company_name: String = row.get("company_name");
        
        engine
            .analyze_stock_pe_history(stock_id, &symbol, &company_name)
            .await
            .map(Some)
            .map_err(|e| format!("Failed to analyze stock P/E history: {}", e))
    } else {
        Ok(None)
    }
}