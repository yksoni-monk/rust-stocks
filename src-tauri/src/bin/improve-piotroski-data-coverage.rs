use std::fs;
use serde_json::Value;
use sqlx::{SqlitePool, Row};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Improving Piotroski Data Coverage for S&P 500");

    // Load environment variables
    if let Err(e) = dotenvy::dotenv() {
        if !e.to_string().contains("No such file or directory") {
            eprintln!("Warning: Failed to load .env file: {}", e);
        }
    }

    // Connect to database using environment variable
    let database_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_PATH").map(|path| format!("sqlite:{}", path)))
        .unwrap_or_else(|_| {
            eprintln!("⚠️  Using fallback database path. Set DATABASE_URL in .env file.");
            "sqlite:src-tauri/db/stocks.db".to_string()
        });

    println!("📊 Connecting to: {}", database_url);
    let pool = SqlitePool::connect(&database_url).await?;

    // Find S&P 500 stocks with missing data
    let missing_data_stocks = find_missing_data_stocks(&pool).await?;
    println!("📊 Found {} S&P 500 stocks with missing critical data", missing_data_stocks.len());

    // Process each stock
    let mut improved_count = 0;
    for stock in missing_data_stocks {
        if let Some(improvements) = process_stock_edgar_data(&stock).await {
            if update_stock_data(&pool, &stock, &improvements).await.is_ok() {
                improved_count += 1;
                println!("✅ Improved data for {}", stock.symbol);
            }
        }
    }

    println!("🎉 Successfully improved data for {} stocks", improved_count);
    Ok(())
}

#[derive(Debug)]
struct MissingDataStock {
    stock_id: i64,
    symbol: String,
    company_name: String,
    cik: Option<String>,
    completeness_score: i32,
    missing_debt: bool,
    missing_current_ratio: bool,
    missing_cash_flow: bool,
}

#[derive(Debug)]
struct DataImprovements {
    current_assets: Option<f64>,
    current_liabilities: Option<f64>,
    total_debt: Option<f64>,
    total_assets: Option<f64>,
    operating_cash_flow: Option<f64>,
    shares_outstanding: Option<f64>,
}

async fn find_missing_data_stocks(pool: &SqlitePool) -> Result<Vec<MissingDataStock>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            s.id as stock_id,
            sp.symbol,
            s.company_name,
            cm.cik,
            COALESCE(pf.data_completeness_score, 0) as completeness_score,
            CASE WHEN pf.current_debt_ratio IS NULL THEN 1 ELSE 0 END as missing_debt,
            CASE WHEN pf.current_current_ratio IS NULL THEN 1 ELSE 0 END as missing_current_ratio,
            CASE WHEN pf.current_operating_cash_flow IS NULL THEN 1 ELSE 0 END as missing_cash_flow
        FROM sp500_symbols sp
        INNER JOIN stocks s ON sp.symbol = s.symbol
        LEFT JOIN piotroski_f_score_complete pf ON s.id = pf.stock_id
        LEFT JOIN cik_mappings_sp500 cm ON s.id = cm.stock_id AND cm.file_exists = 1
        WHERE cm.cik IS NOT NULL
          AND (pf.data_completeness_score < 90
               OR pf.current_debt_ratio IS NULL
               OR pf.current_current_ratio IS NULL
               OR pf.current_operating_cash_flow IS NULL
               OR pf.stock_id IS NULL)
        ORDER BY pf.data_completeness_score ASC
        LIMIT 50
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| MissingDataStock {
            stock_id: row.get("stock_id"),
            symbol: row.get("symbol"),
            company_name: row.get("company_name"),
            cik: row.get("cik"),
            completeness_score: row.get("completeness_score"),
            missing_debt: row.get::<i32, _>("missing_debt") != 0,
            missing_current_ratio: row.get::<i32, _>("missing_current_ratio") != 0,
            missing_cash_flow: row.get::<i32, _>("missing_cash_flow") != 0,
        })
        .collect())
}

async fn process_stock_edgar_data(stock: &MissingDataStock) -> Option<DataImprovements> {
    if let Some(cik) = &stock.cik {
        let edgar_path = format!("edgar_data/companyfacts/CIK{}.json", cik);

        if Path::new(&edgar_path).exists() {
            if let Ok(edgar_data) = fs::read_to_string(&edgar_path) {
                if let Ok(json_data) = serde_json::from_str::<Value>(&edgar_data) {
                    return extract_missing_financial_data(&json_data, stock);
                }
            }
        }
    }
    None
}

fn extract_missing_financial_data(edgar_data: &Value, stock: &MissingDataStock) -> Option<DataImprovements> {
    let facts = edgar_data.get("facts")?.get("us-gaap")?;

    let mut improvements = DataImprovements {
        current_assets: None,
        current_liabilities: None,
        total_debt: None,
        total_assets: None,
        operating_cash_flow: None,
        shares_outstanding: None,
    };

    // Extract current assets if missing current ratio data
    if stock.missing_current_ratio {
        improvements.current_assets = extract_latest_annual_value(facts, "AssetsCurrent");
        improvements.current_liabilities = extract_latest_annual_value(facts, "LiabilitiesCurrent");
    }

    // Extract debt data if missing debt ratio
    if stock.missing_debt {
        improvements.total_debt = extract_latest_annual_value(facts, "LiabilitiesAndStockholdersEquity")
            .or_else(|| extract_latest_annual_value(facts, "Liabilities"))
            .or_else(|| extract_latest_annual_value(facts, "DebtCurrent"))
            .map(|current| current + extract_latest_annual_value(facts, "LongTermDebt").unwrap_or(0.0));

        improvements.total_assets = extract_latest_annual_value(facts, "Assets");
    }

    // Extract cash flow data if missing
    if stock.missing_cash_flow {
        let cash_flow_facts = edgar_data.get("facts")?.get("us-gaap")?;
        improvements.operating_cash_flow = extract_latest_annual_value(cash_flow_facts, "NetCashProvidedByUsedInOperatingActivities");
    }

    // Extract shares outstanding for dilution calculations
    improvements.shares_outstanding = extract_latest_annual_value(facts, "WeightedAverageNumberOfSharesOutstandingBasic")
        .or_else(|| extract_latest_annual_value(facts, "WeightedAverageNumberOfDilutedSharesOutstanding"));

    // Only return if we found some useful data
    if improvements.current_assets.is_some()
        || improvements.total_debt.is_some()
        || improvements.operating_cash_flow.is_some() {
        Some(improvements)
    } else {
        None
    }
}

fn extract_latest_annual_value(facts: &Value, field_name: &str) -> Option<f64> {
    let field_data = facts.get(field_name)?;
    let units = field_data.get("units")?;

    // Try USD first, then other currencies
    let usd_data = units.get("USD").or_else(|| {
        // Try to find any currency unit
        units.as_object()?.values().next()
    })?;

    if let Some(annual_data) = usd_data.as_array() {
        // Find the most recent annual (10-K) filing
        let mut latest_annual = None;
        let mut latest_date = "";

        for entry in annual_data {
            if let (Some(val), Some(end_date), Some(form)) = (
                entry.get("val").and_then(|v| v.as_f64()),
                entry.get("end").and_then(|d| d.as_str()),
                entry.get("form").and_then(|f| f.as_str()),
            ) {
                // Prefer 10-K annual reports
                if form == "10-K" || (form == "10-Q" && latest_annual.is_none()) {
                    if end_date > latest_date {
                        latest_annual = Some(val);
                        latest_date = end_date;
                    }
                }
            }
        }

        return latest_annual;
    }

    None
}

async fn update_stock_data(
    pool: &SqlitePool,
    stock: &MissingDataStock,
    improvements: &DataImprovements,
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;

    // Get the most recent fiscal year for this stock
    let latest_fiscal_year = sqlx::query(
        "SELECT MAX(report_date) as latest_date FROM income_statements WHERE stock_id = ? AND period_type = 'Annual'"
    )
    .bind(stock.stock_id)
    .fetch_optional(&mut *transaction)
    .await?;

    if let Some(row) = latest_fiscal_year {
        if let Some(report_date) = row.get::<Option<String>, _>("latest_date") {
            // Update balance sheet data
            if improvements.current_assets.is_some() || improvements.current_liabilities.is_some() || improvements.total_debt.is_some() {
                sqlx::query(
                    r#"
                    UPDATE balance_sheets
                    SET
                        current_assets = COALESCE(?, current_assets),
                        current_liabilities = COALESCE(?, current_liabilities),
                        total_debt = COALESCE(?, total_debt),
                        total_assets = COALESCE(?, total_assets)
                    WHERE stock_id = ? AND report_date = ? AND period_type = 'Annual'
                    "#
                )
                .bind(improvements.current_assets)
                .bind(improvements.current_liabilities)
                .bind(improvements.total_debt)
                .bind(improvements.total_assets)
                .bind(stock.stock_id)
                .bind(&report_date)
                .execute(&mut *transaction)
                .await?;
            }

            // Update cash flow data
            if let Some(ocf) = improvements.operating_cash_flow {
                sqlx::query(
                    r#"
                    UPDATE cash_flow_statements
                    SET operating_cash_flow = ?
                    WHERE stock_id = ? AND report_date = ? AND period_type = 'Annual'
                    "#
                )
                .bind(ocf)
                .bind(stock.stock_id)
                .bind(&report_date)
                .execute(&mut *transaction)
                .await?;
            }

            // Update shares outstanding in balance sheet
            if let Some(shares) = improvements.shares_outstanding {
                sqlx::query(
                    r#"
                    UPDATE balance_sheets
                    SET shares_outstanding = ?
                    WHERE stock_id = ? AND report_date = ? AND period_type = 'Annual'
                    "#
                )
                .bind(shares)
                .bind(stock.stock_id)
                .bind(&report_date)
                .execute(&mut *transaction)
                .await?;
            }
        }
    }

    transaction.commit().await?;
    Ok(())
}