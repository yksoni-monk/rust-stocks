use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
use crate::database::helpers::get_database_connection;

#[derive(Debug, Serialize, Deserialize)]
pub struct PiotoskiFScoreResult {
    pub stock_id: i64,
    pub symbol: String,
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub current_net_income: Option<f64>,
    pub f_score_complete: i32,
    pub data_completeness_score: i32,
    
    // Complete 9 criteria breakdown
    pub criterion_positive_net_income: i32,
    pub criterion_positive_operating_cash_flow: i32,
    pub criterion_improving_roa: i32,
    pub criterion_cash_flow_quality: i32,
    pub criterion_decreasing_debt_ratio: i32,
    pub criterion_improving_current_ratio: i32,
    pub criterion_no_dilution: i32,
    pub criterion_improving_gross_margin: i32,
    pub criterion_improving_asset_turnover: i32,
    
    // Financial metrics
    pub current_roa: Option<f64>,
    pub current_debt_ratio: Option<f64>,
    pub current_current_ratio: Option<f64>,
    pub current_gross_margin: Option<f64>,
    pub current_asset_turnover: Option<f64>,
    pub current_operating_cash_flow: Option<f64>,
    pub pb_ratio: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PiotroskilScreeningCriteria {
    pub min_f_score: Option<i32>,
    pub min_data_completeness: Option<i32>,
    pub sectors: Option<Vec<String>>,
    pub min_market_cap: Option<f64>,
    pub passes_screening_only: Option<bool>,
}

impl Default for PiotroskilScreeningCriteria {
    fn default() -> Self {
        Self {
            min_f_score: Some(3),
            min_data_completeness: Some(50),
            sectors: None,
            min_market_cap: None,
            passes_screening_only: Some(false),
        }
    }
}

#[tauri::command]
pub async fn get_piotroski_screening_results(
    stock_tickers: Vec<String>,
    criteria: Option<PiotroskilScreeningCriteria>,
    limit: Option<i32>,
) -> Result<Vec<PiotoskiFScoreResult>, String> {
    let pool = get_database_connection().await?;

    get_piotroski_screening_results_internal(&pool, stock_tickers, criteria, limit).await
}

async fn get_piotroski_screening_results_internal(
    pool: &SqlitePool,
    stock_tickers: Vec<String>,
    criteria: Option<PiotroskilScreeningCriteria>,
    limit: Option<i32>,
) -> Result<Vec<PiotoskiFScoreResult>, String> {
    let criteria = criteria.unwrap_or_default();

    let mut query = String::from(
        "SELECT
            stock_id,
            symbol,
            sector,
            industry,
            current_net_income,
            f_score_complete,
            data_completeness_score,
            criterion_positive_net_income,
            criterion_positive_operating_cash_flow,
            criterion_improving_roa,
            criterion_cash_flow_quality,
            criterion_decreasing_debt_ratio,
            criterion_improving_current_ratio,
            criterion_no_dilution,
            criterion_improving_gross_margin,
            criterion_improving_asset_turnover,
            current_roa,
            current_debt_ratio,
            current_current_ratio,
            current_gross_margin,
            current_asset_turnover,
            current_operating_cash_flow,
            pb_ratio
        FROM piotroski_f_score_complete
        WHERE 1=1"
    );

    let mut params = Vec::new();

    // Apply filters
    if let Some(min_f_score) = criteria.min_f_score {
        query.push_str(" AND f_score_complete >= ?");
        params.push(min_f_score.to_string());
    }

    if let Some(min_completeness) = criteria.min_data_completeness {
        query.push_str(" AND data_completeness_score >= ?");
        params.push(min_completeness.to_string());
    }

    if criteria.passes_screening_only.unwrap_or(false) {
        query.push_str(" AND passes_screening = 1");
    }

    if let Some(sectors) = &criteria.sectors {
        if !sectors.is_empty() {
            let placeholders = sectors.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            query.push_str(&format!(" AND sector IN ({})", placeholders));
            for sector in sectors {
                params.push(sector.clone());
            }
        }
    }

    if !stock_tickers.is_empty() {
        let placeholders = stock_tickers.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        query.push_str(&format!(" AND symbol IN ({})", placeholders));
        for ticker in &stock_tickers {
            params.push(ticker.clone());
        }
    }

    query.push_str(" ORDER BY f_score_complete DESC, data_completeness_score DESC");

    if let Some(limit_val) = limit {
        query.push_str(&format!(" LIMIT {}", limit_val));
    }

    // Build the query with parameters
    let mut sqlx_query = sqlx::query_as::<_, PiotoskiFScoreResult>(&query);
    for param in params {
        sqlx_query = sqlx_query.bind(param);
    }

    let results = sqlx_query
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Database query failed: {}", e))?;

    Ok(results)
}

// For sqlx FromRow trait
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for PiotoskiFScoreResult {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(PiotoskiFScoreResult {
            stock_id: row.try_get("stock_id")?,
            symbol: row.try_get("symbol")?,
            sector: row.try_get("sector")?,
            industry: row.try_get("industry")?,
            current_net_income: row.try_get("current_net_income")?,
            f_score_complete: row.try_get("f_score_complete")?,
            data_completeness_score: row.try_get("data_completeness_score")?,
            criterion_positive_net_income: row.try_get("criterion_positive_net_income")?,
            criterion_positive_operating_cash_flow: row.try_get("criterion_positive_operating_cash_flow")?,
            criterion_improving_roa: row.try_get("criterion_improving_roa")?,
            criterion_cash_flow_quality: row.try_get("criterion_cash_flow_quality")?,
            criterion_decreasing_debt_ratio: row.try_get("criterion_decreasing_debt_ratio")?,
            criterion_improving_current_ratio: row.try_get("criterion_improving_current_ratio")?,
            criterion_no_dilution: row.try_get("criterion_no_dilution")?,
            criterion_improving_gross_margin: row.try_get("criterion_improving_gross_margin")?,
            criterion_improving_asset_turnover: row.try_get("criterion_improving_asset_turnover")?,
            current_roa: row.try_get("current_roa")?,
            current_debt_ratio: row.try_get("current_debt_ratio")?,
            current_current_ratio: row.try_get("current_current_ratio")?,
            current_gross_margin: row.try_get("current_gross_margin")?,
            current_asset_turnover: row.try_get("current_asset_turnover")?,
            current_operating_cash_flow: row.try_get("current_operating_cash_flow")?,
            pb_ratio: row.try_get("pb_ratio")?,
        })
    }
}

#[tauri::command]
pub async fn get_piotroski_statistics() -> Result<serde_json::Value, String> {
    let pool = get_database_connection().await?;

    let stats = sqlx::query(
        "SELECT
            COUNT(*) as total_stocks,
            AVG(f_score_complete) as avg_f_score,
            AVG(data_completeness_score) as avg_completeness,
            COUNT(CASE WHEN f_score_complete >= 6 THEN 1 END) as high_quality_stocks,
            COUNT(CASE WHEN f_score_complete >= 7 THEN 1 END) as excellent_stocks,
            COUNT(CASE WHEN passes_screening = 1 THEN 1 END) as passing_stocks
        FROM piotroski_screening_results"
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| format!("Failed to get Piotroski statistics: {}", e))?;

    let result = serde_json::json!({
        "total_stocks": stats.get::<i64, _>("total_stocks"),
        "avg_f_score": stats.get::<f64, _>("avg_f_score"),
        "avg_completeness": stats.get::<f64, _>("avg_completeness"),
        "high_quality_stocks": stats.get::<i64, _>("high_quality_stocks"),
        "excellent_stocks": stats.get::<i64, _>("excellent_stocks"),
        "passing_stocks": stats.get::<i64, _>("passing_stocks"),
    });

    Ok(result)
}