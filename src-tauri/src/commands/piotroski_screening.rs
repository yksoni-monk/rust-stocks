use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
use crate::database::helpers::get_database_connection;
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
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
    pub criterion_improving_net_margin: i32,
    pub criterion_improving_asset_turnover: i32,

    // Financial metrics
    pub current_roa: Option<f64>,
    pub current_debt_ratio: Option<f64>,
    pub current_current_ratio: Option<f64>,
    pub current_net_margin: Option<f64>,
    pub current_asset_turnover: Option<f64>,
    pub current_operating_cash_flow: Option<f64>,
    pub pb_ratio: Option<f64>,

    // NEW: Confidence and weighted scoring
    pub confidence_score: f64,
    pub weighted_score: f64,
    pub quality_tier: String,
    pub criteria_summary: Vec<PiotroskilCriterion>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PiotroskilCriterion {
    pub name: String,
    pub category: String,
    pub weight: f64,
    pub score: i32,
    pub confidence: f64,
    pub data_available: bool,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
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
            criterion_improving_net_margin,
            criterion_improving_asset_turnover,
            current_roa,
            current_debt_ratio,
            current_current_ratio,
            current_net_margin,
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

        let f_score = row.try_get::<i32, _>("f_score_complete")?;
        let completeness = row.try_get::<i32, _>("data_completeness_score")?;

        // Calculate confidence and weighted score
        let (confidence_score, weighted_score, quality_tier) = calculate_confidence_metrics(
            f_score,
            completeness,
            &[
                row.try_get("criterion_positive_net_income").unwrap_or(0),
                row.try_get("criterion_positive_operating_cash_flow").unwrap_or(0),
                row.try_get("criterion_improving_roa").unwrap_or(0),
                row.try_get("criterion_cash_flow_quality").unwrap_or(0),
                row.try_get("criterion_decreasing_debt_ratio").unwrap_or(0),
                row.try_get("criterion_improving_current_ratio").unwrap_or(0),
                row.try_get("criterion_no_dilution").unwrap_or(0),
                row.try_get("criterion_improving_net_margin").unwrap_or(0),
                row.try_get("criterion_improving_asset_turnover").unwrap_or(0),
            ],
            &[
                row.try_get::<Option<f64>, _>("current_net_income")?.is_some(),
                row.try_get::<Option<f64>, _>("current_operating_cash_flow")?.is_some(),
                row.try_get::<Option<f64>, _>("current_roa")?.is_some(),
                row.try_get::<Option<f64>, _>("current_operating_cash_flow")?.is_some(),
                row.try_get::<Option<f64>, _>("current_debt_ratio")?.is_some(),
                row.try_get::<Option<f64>, _>("current_current_ratio")?.is_some(),
                true, // shares data generally available
                row.try_get::<Option<f64>, _>("current_net_margin")?.is_some(),
                row.try_get::<Option<f64>, _>("current_asset_turnover")?.is_some(),
            ]
        );

        let criteria_summary = build_criteria_summary(&row)?;

        Ok(PiotoskiFScoreResult {
            stock_id: row.try_get("stock_id")?,
            symbol: row.try_get("symbol")?,
            sector: row.try_get("sector")?,
            industry: row.try_get("industry")?,
            current_net_income: row.try_get("current_net_income")?,
            f_score_complete: f_score,
            data_completeness_score: completeness,
            criterion_positive_net_income: row.try_get("criterion_positive_net_income")?,
            criterion_positive_operating_cash_flow: row.try_get("criterion_positive_operating_cash_flow")?,
            criterion_improving_roa: row.try_get("criterion_improving_roa")?,
            criterion_cash_flow_quality: row.try_get("criterion_cash_flow_quality")?,
            criterion_decreasing_debt_ratio: row.try_get("criterion_decreasing_debt_ratio")?,
            criterion_improving_current_ratio: row.try_get("criterion_improving_current_ratio")?,
            criterion_no_dilution: row.try_get("criterion_no_dilution")?,
            criterion_improving_net_margin: row.try_get("criterion_improving_net_margin")?,
            criterion_improving_asset_turnover: row.try_get("criterion_improving_asset_turnover")?,
            current_roa: row.try_get("current_roa")?,
            current_debt_ratio: row.try_get("current_debt_ratio")?,
            current_current_ratio: row.try_get("current_current_ratio")?,
            current_net_margin: row.try_get("current_net_margin")?,
            current_asset_turnover: row.try_get("current_asset_turnover")?,
            current_operating_cash_flow: row.try_get("current_operating_cash_flow")?,
            pb_ratio: row.try_get("pb_ratio")?,
            confidence_score,
            weighted_score,
            quality_tier,
            criteria_summary,
        })
    }
}

// Piotroski factor weights based on empirical research and factor importance
const PIOTROSKI_WEIGHTS: [f64; 9] = [
    1.2,  // Positive Net Income - Critical profitability indicator
    1.1,  // Positive Operating Cash Flow - Cash generation ability
    1.0,  // Improving ROA - Asset efficiency trend
    1.2,  // Cash Flow Quality - Earnings quality
    0.9,  // Decreasing Debt Ratio - Leverage improvement
    0.8,  // Improving Current Ratio - Liquidity improvement
    0.8,  // No Share Dilution - Capital discipline
    1.0,  // Improving Net Margin - Profitability trend
    0.9,  // Improving Asset Turnover - Operational efficiency
];

const PIOTROSKI_NAMES: [&str; 9] = [
    "Positive Net Income",
    "Positive Operating Cash Flow",
    "Improving ROA",
    "Cash Flow Quality",
    "Decreasing Debt Ratio",
    "Improving Current Ratio",
    "No Share Dilution",
    "Improving Net Margin",
    "Improving Asset Turnover",
];

const PIOTROSKI_CATEGORIES: [&str; 9] = [
    "Profitability", "Profitability", "Profitability", "Profitability",
    "Leverage", "Leverage", "Leverage",
    "Operating Efficiency", "Operating Efficiency",
];

const PIOTROSKI_DESCRIPTIONS: [&str; 9] = [
    "Company reported positive net income in the most recent fiscal year",
    "Company generated positive operating cash flow in the most recent fiscal year",
    "Return on Assets improved compared to the prior year",
    "Operating cash flow exceeds net income, indicating good earnings quality",
    "Debt-to-assets ratio decreased compared to the prior year",
    "Current ratio improved compared to the prior year",
    "Company did not issue additional shares (no dilution)",
    "Net profit margin improved compared to the prior year",
    "Asset turnover improved compared to the prior year",
];

fn calculate_confidence_metrics(
    f_score: i32,
    completeness: i32,
    criteria_scores: &[i32; 9],
    data_availability: &[bool; 9],
) -> (f64, f64, String) {
    // Calculate weighted score
    let weighted_score: f64 = criteria_scores
        .iter()
        .zip(PIOTROSKI_WEIGHTS.iter())
        .map(|(&score, &weight)| score as f64 * weight)
        .sum();

    let max_weighted_score: f64 = PIOTROSKI_WEIGHTS.iter().sum();
    let normalized_weighted_score = (weighted_score / max_weighted_score) * 9.0;

    // Calculate confidence based on data availability and consistency
    let data_confidence = data_availability.iter().map(|&available| if available { 1.0 } else { 0.0 }).sum::<f64>() / 9.0;
    let completeness_confidence = (completeness as f64) / 100.0;

    // Penalize for inconsistent patterns (e.g., high score with low completeness)
    let consistency_factor = if completeness < 70 && f_score > 6 {
        0.8 // Reduce confidence for potentially incomplete high scores
    } else if completeness > 90 {
        1.1 // Boost confidence for very complete data
    } else {
        1.0
    };

    let confidence_score = (data_confidence * 0.6 + completeness_confidence * 0.4) * consistency_factor * 100.0;
    let confidence_score = confidence_score.min(100.0).max(0.0);

    // Determine quality tier
    let quality_tier = match (f_score, confidence_score as i32) {
        (8..=9, 85..) => "Elite",
        (7..=9, 70..) => "High Quality",
        (5..=6, 80..) => "Good",
        (3..=6, 60..) => "Average",
        (0..=4, 70..) => "Weak",
        _ => "Insufficient Data",
    }.to_string();

    (confidence_score, normalized_weighted_score, quality_tier)
}

fn build_criteria_summary(row: &sqlx::sqlite::SqliteRow) -> Result<Vec<PiotroskilCriterion>, sqlx::Error> {
    use sqlx::Row;

    let criteria_scores = [
        row.try_get("criterion_positive_net_income").unwrap_or(0),
        row.try_get("criterion_positive_operating_cash_flow").unwrap_or(0),
        row.try_get("criterion_improving_roa").unwrap_or(0),
        row.try_get("criterion_cash_flow_quality").unwrap_or(0),
        row.try_get("criterion_decreasing_debt_ratio").unwrap_or(0),
        row.try_get("criterion_improving_current_ratio").unwrap_or(0),
        row.try_get("criterion_no_dilution").unwrap_or(0),
        row.try_get("criterion_improving_net_margin").unwrap_or(0),
        row.try_get("criterion_improving_asset_turnover").unwrap_or(0),
    ];

    let data_availability = [
        row.try_get::<Option<f64>, _>("current_net_income")?.is_some(),
        row.try_get::<Option<f64>, _>("current_operating_cash_flow")?.is_some(),
        row.try_get::<Option<f64>, _>("current_roa")?.is_some(),
        row.try_get::<Option<f64>, _>("current_operating_cash_flow")?.is_some(),
        row.try_get::<Option<f64>, _>("current_debt_ratio")?.is_some(),
        row.try_get::<Option<f64>, _>("current_current_ratio")?.is_some(),
        true, // shares data generally available
        row.try_get::<Option<f64>, _>("current_net_margin")?.is_some(),
        row.try_get::<Option<f64>, _>("current_asset_turnover")?.is_some(),
    ];

    let mut criteria = Vec::new();
    for i in 0..9 {
        let confidence = if data_availability[i] {
            if PIOTROSKI_WEIGHTS[i] > 1.0 { 95.0 } else { 85.0 }
        } else {
            30.0
        };

        criteria.push(PiotroskilCriterion {
            name: PIOTROSKI_NAMES[i].to_string(),
            category: PIOTROSKI_CATEGORIES[i].to_string(),
            weight: PIOTROSKI_WEIGHTS[i],
            score: criteria_scores[i],
            confidence,
            data_available: data_availability[i],
            description: PIOTROSKI_DESCRIPTIONS[i].to_string(),
        });
    }

    Ok(criteria)
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