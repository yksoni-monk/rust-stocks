use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
use crate::database::helpers::get_database_connection;
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OShaughnessyValueResult {
    pub stock_id: i64,
    pub symbol: String,
    pub sector: Option<String>,
    pub current_price: Option<f64>,
    pub market_cap: Option<f64>,
    pub enterprise_value: Option<f64>,

    // All 6 O'Shaughnessy metrics
    pub ps_ratio: Option<f64>,
    pub evs_ratio: Option<f64>,
    pub pe_ratio: Option<f64>,
    pub pb_ratio: Option<f64>,
    pub ev_ebitda_ratio: Option<f64>,
    pub shareholder_yield: Option<f64>,

    // Ranking and scoring
    pub data_completeness_score: i32,
    pub composite_score: f64,
    pub composite_percentile: f64,
    pub overall_rank: i64,
    pub passes_screening: i32,

    // Individual ranks for transparency
    pub ps_rank: Option<i64>,
    pub evs_rank: Option<i64>,
    pub pe_rank: Option<i64>,
    pub pb_rank: Option<i64>,
    pub ebitda_rank: Option<i64>,
    pub yield_rank: Option<i64>,
    pub metrics_available: i32,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OShaughnessyScreeningCriteria {
    pub max_composite_percentile: Option<f64>,
    pub max_ps_ratio: Option<f64>,
    pub max_evs_ratio: Option<f64>,
    pub min_market_cap: Option<f64>,
    pub sectors: Option<Vec<String>>,
    pub passes_screening_only: Option<bool>,
}

impl Default for OShaughnessyScreeningCriteria {
    fn default() -> Self {
        Self {
            max_composite_percentile: Some(20.0), // Top 20%
            max_ps_ratio: Some(2.0),
            max_evs_ratio: Some(2.0),
            min_market_cap: Some(200_000_000.0), // $200M
            sectors: None,
            passes_screening_only: Some(true),
        }
    }
}

#[tauri::command]
pub async fn get_oshaughnessy_screening_results(
    stock_tickers: Vec<String>,
    criteria: Option<OShaughnessyScreeningCriteria>,
    limit: Option<i32>,
) -> Result<Vec<OShaughnessyValueResult>, String> {
    let pool = get_database_connection().await?;

    get_oshaughnessy_screening_results_internal(&pool, stock_tickers, criteria, limit).await
}

async fn get_oshaughnessy_screening_results_internal(
    pool: &SqlitePool,
    stock_tickers: Vec<String>,
    criteria: Option<OShaughnessyScreeningCriteria>,
    limit: Option<i32>,
) -> Result<Vec<OShaughnessyValueResult>, String> {
    let criteria = criteria.unwrap_or_default();
    println!("🔍 Starting O'Shaughnessy screening with criteria: {:?}", criteria);

    let mut query = String::from(
        "SELECT
            stock_id,
            symbol,
            sector,
            current_price,
            market_cap,
            enterprise_value,
            ps_ratio,
            evs_ratio,
            pe_ratio,
            pb_ratio,
            ev_ebitda_ratio,
            shareholder_yield,
            data_completeness_score,
            composite_score,
            composite_percentile,
            overall_rank,
            passes_screening,
            ps_rank,
            evs_rank,
            pe_rank,
            pb_rank,
            ebitda_rank,
            yield_rank,
            metrics_available
        FROM oshaughnessy_ranking
        WHERE 1=1"
    );

    println!("🔍 Query built, applying filters...");
    let mut params = Vec::new();

    // Apply filters
    if let Some(max_percentile) = criteria.max_composite_percentile {
        query.push_str(" AND composite_percentile <= ?");
        params.push(max_percentile.to_string());
    }

    if let Some(max_ps) = criteria.max_ps_ratio {
        query.push_str(" AND ps_ratio <= ?");
        params.push(max_ps.to_string());
    }

    if let Some(max_evs) = criteria.max_evs_ratio {
        query.push_str(" AND evs_ratio <= ?");
        params.push(max_evs.to_string());
    }

    if let Some(min_market_cap) = criteria.min_market_cap {
        query.push_str(" AND market_cap >= ?");
        params.push(min_market_cap.to_string());
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

    query.push_str(" ORDER BY composite_score ASC, overall_rank ASC");

    if let Some(limit_val) = limit {
        query.push_str(&format!(" LIMIT {}", limit_val));
    }

    // Build the query with parameters
    println!("🔍 Final query: {}", query);
    println!("🔍 Executing database query...");
    let mut sqlx_query = sqlx::query_as::<_, OShaughnessyValueResult>(&query);
    for param in params {
        sqlx_query = sqlx_query.bind(param);
    }

    let results = sqlx_query
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Database query failed: {}", e))?;

    println!("🔍 Query executed successfully, got {} results", results.len());
    Ok(results)
}

// For sqlx FromRow trait
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for OShaughnessyValueResult {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(OShaughnessyValueResult {
            stock_id: row.try_get("stock_id")?,
            symbol: row.try_get("symbol")?,
            sector: row.try_get("sector")?,
            current_price: row.try_get("current_price")?,
            market_cap: row.try_get("market_cap")?,
            enterprise_value: row.try_get("enterprise_value")?,

            // All 6 metrics
            ps_ratio: row.try_get("ps_ratio")?,
            evs_ratio: row.try_get("evs_ratio")?,
            pe_ratio: row.try_get("pe_ratio")?,
            pb_ratio: row.try_get("pb_ratio")?,
            ev_ebitda_ratio: row.try_get("ev_ebitda_ratio")?,
            shareholder_yield: row.try_get("shareholder_yield")?,

            // Ranking and scoring
            data_completeness_score: row.try_get("data_completeness_score")?,
            composite_score: row.try_get("composite_score")?,
            composite_percentile: row.try_get("composite_percentile")?,
            overall_rank: row.try_get("overall_rank")?,
            passes_screening: row.try_get("passes_screening")?,

            // Individual ranks
            ps_rank: row.try_get("ps_rank").ok(),
            evs_rank: row.try_get("evs_rank").ok(),
            pe_rank: row.try_get("pe_rank").ok(),
            pb_rank: row.try_get("pb_rank").ok(),
            ebitda_rank: row.try_get("ebitda_rank").ok(),
            yield_rank: row.try_get("yield_rank").ok(),
            metrics_available: row.try_get("metrics_available")?,
        })
    }
}

#[tauri::command]
pub async fn get_oshaughnessy_statistics() -> Result<serde_json::Value, String> {
    let pool = get_database_connection().await?;

    let stats = sqlx::query(
        "SELECT
            COUNT(*) as total_stocks,
            AVG(composite_score) as avg_composite_score,
            AVG(ps_ratio) as avg_ps_ratio,
            AVG(evs_ratio) as avg_evs_ratio,
            COUNT(CASE WHEN composite_percentile <= 10 THEN 1 END) as top_10_percent,
            COUNT(CASE WHEN composite_percentile <= 20 THEN 1 END) as top_20_percent,
            COUNT(CASE WHEN passes_screening = 1 THEN 1 END) as passing_stocks
        FROM oshaughnessy_ranking"
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| format!("Failed to get O'Shaughnessy statistics: {}", e))?;

    let result = serde_json::json!({
        "total_stocks": stats.get::<i64, _>("total_stocks"),
        "avg_composite_score": stats.get::<f64, _>("avg_composite_score"),
        "avg_ps_ratio": stats.get::<f64, _>("avg_ps_ratio"),
        "avg_evs_ratio": stats.get::<f64, _>("avg_evs_ratio"),
        "top_10_percent": stats.get::<i64, _>("top_10_percent"),
        "top_20_percent": stats.get::<i64, _>("top_20_percent"),
        "passing_stocks": stats.get::<i64, _>("passing_stocks"),
    });

    Ok(result)
}