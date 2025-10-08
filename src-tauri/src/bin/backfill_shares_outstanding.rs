/// One-time script to backfill shares_outstanding data from SEC EDGAR DEI taxonomy
///
/// This script extracts shares_outstanding from the Company Facts API and updates
/// existing balance_sheets records. This is needed because shares_outstanding was
/// added after the initial data import.

use anyhow::Result;
use sqlx::sqlite::SqlitePoolOptions;
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔄 Backfilling shares_outstanding from SEC EDGAR DEI taxonomy");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Connect to database
    let database_url = "sqlite:db/stocks.db?mode=rwc";
    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    // Get all S&P 500 stocks
    let stocks: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT id, symbol, cik FROM stocks WHERE is_sp500 = 1 ORDER BY symbol"
    )
    .fetch_all(&pool)
    .await?;

    println!("📊 Found {} S&P 500 stocks to process\n", stocks.len());

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; StockAnalyzer/1.0)")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let mut success_count = 0;
    let mut skip_count = 0;
    let mut error_count = 0;

    for (i, (stock_id, symbol, cik)) in stocks.iter().enumerate() {
        println!("[{}/{}] Processing {}...", i + 1, stocks.len(), symbol);

        // Fetch company facts
        let cik_padded = format!("{:010}", cik.parse::<u64>().unwrap_or(0));
        let url = format!("https://data.sec.gov/api/xbrl/companyfacts/CIK{}.json", cik_padded);

        let response = match client.get(&url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                println!("  ❌ Failed to fetch: {}", e);
                error_count += 1;
                continue;
            }
        };

        if !response.status().is_success() {
            println!("  ❌ API error: {}", response.status());
            error_count += 1;
            continue;
        }

        let company_facts: Value = match response.json().await {
            Ok(json) => json,
            Err(e) => {
                println!("  ❌ Failed to parse JSON: {}", e);
                error_count += 1;
                continue;
            }
        };

        // Extract shares_outstanding from us-gaap (primary) or dei/weighted avg (fallbacks)
        let mut updates_made = 0;
        let mut shares_by_year = std::collections::HashMap::new();

        // Primary: Try us-gaap CommonStockSharesOutstanding
        if let Some(us_gaap_facts) = company_facts.get("facts").and_then(|f| f.get("us-gaap")) {
            if let Some(shares_field) = us_gaap_facts.get("CommonStockSharesOutstanding") {
                if let Some(units) = shares_field.get("units") {
                    if let Some(shares_data) = units.get("shares") {
                        if let Some(values) = shares_data.as_array() {
                            for value in values {
                                if let (Some(fy), Some(val)) = (
                                    value.get("fy").and_then(|v| v.as_i64()),
                                    value.get("val").and_then(|v| v.as_f64())
                                ) {
                                    if val > 0.0 {
                                        // Keep the latest value for each fiscal year
                                        shares_by_year.insert(fy as i32, val);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fallback #1: Try dei EntityCommonStockSharesOutstanding if us-gaap didn't have data
        if shares_by_year.is_empty() {
            if let Some(dei_facts) = company_facts.get("facts").and_then(|f| f.get("dei")) {
                if let Some(shares_field) = dei_facts.get("EntityCommonStockSharesOutstanding") {
                    if let Some(units) = shares_field.get("units") {
                        if let Some(shares_data) = units.get("shares") {
                            if let Some(values) = shares_data.as_array() {
                                for value in values {
                                    if let (Some(fy), Some(val)) = (
                                        value.get("fy").and_then(|v| v.as_i64()),
                                        value.get("val").and_then(|v| v.as_f64())
                                    ) {
                                        if val > 0.0 {
                                            shares_by_year.insert(fy as i32, val);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fallback #2: Try WeightedAverageNumberOfSharesOutstandingBasic as last resort
        if shares_by_year.is_empty() {
            if let Some(us_gaap_facts) = company_facts.get("facts").and_then(|f| f.get("us-gaap")) {
                if let Some(shares_field) = us_gaap_facts.get("WeightedAverageNumberOfSharesOutstandingBasic") {
                    if let Some(units) = shares_field.get("units") {
                        if let Some(shares_data) = units.get("shares") {
                            if let Some(values) = shares_data.as_array() {
                                for value in values {
                                    if let (Some(fy), Some(val)) = (
                                        value.get("fy").and_then(|v| v.as_i64()),
                                        value.get("val").and_then(|v| v.as_f64())
                                    ) {
                                        if val > 0.0 {
                                            shares_by_year.insert(fy as i32, val);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Update balance_sheets for each fiscal year
        for (fiscal_year, shares) in shares_by_year {
            let result = sqlx::query(
                "UPDATE balance_sheets SET shares_outstanding = ? WHERE stock_id = ? AND fiscal_year = ? AND shares_outstanding IS NULL"
            )
            .bind(shares)
            .bind(stock_id)
            .bind(fiscal_year)
            .execute(&pool)
            .await?;

            if result.rows_affected() > 0 {
                updates_made += result.rows_affected();
            }
        }

        if updates_made > 0 {
            println!("  ✅ Updated {} balance sheet records", updates_made);
            success_count += 1;
        } else {
            println!("  ⚠️  No shares_outstanding data found in DEI taxonomy");
            error_count += 1;
        }

        // Rate limiting: 10 requests per second max
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎉 Backfill complete!");
    println!("   ✅ Success: {} stocks", success_count);
    println!("   ⏭️  Skipped: {} stocks (already had data)", skip_count);
    println!("   ❌ Errors: {} stocks", error_count);

    Ok(())
}
