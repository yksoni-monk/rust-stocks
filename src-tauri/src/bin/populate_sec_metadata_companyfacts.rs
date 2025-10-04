/// Populate SEC Filing Metadata using Company Facts API
/// Downloads Company Facts JSON for each S&P 500 CIK with proper rate limiting
/// and concurrent processing to populate filing metadata

use anyhow::Result;
use chrono::NaiveDate;
use governor::{Quota, RateLimiter};
use reqwest::Client;
use rust_stocks_tauri_lib::database::helpers::get_database_connection;
use serde_json::Value;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Semaphore, Mutex};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔄 Starting SEC Filing Metadata Population via Company Facts API");
    println!("📅 {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"));
    println!("══════════════════════════════════════");

    // Initialize database connection
    let pool = get_database_connection().await.map_err(|e| anyhow::anyhow!("Database connection failed: {}", e))?;
    
    // Get all S&P 500 stocks with CIKs
    let stocks_with_ciks = get_sp500_stocks_with_ciks(&pool).await?;
    println!("✅ Found {} S&P 500 stocks with CIKs", stocks_with_ciks.len());

    // Create rate-limited HTTP client
    let (client, limiter) = create_rate_limited_client().await?;
    
    // Process CIKs concurrently with rate limiting
    let results = process_ciks_concurrently(&pool, &client, &limiter, &stocks_with_ciks).await?;
    
    // Report results
    println!("\n📊 Population Results:");
    println!("✅ Successful: {}", results.successful);
    println!("❌ Failed: {}", results.failed);
    
    if !results.errors.is_empty() {
        println!("\n🚨 Errors encountered:");
        for error in &results.errors {
            println!("  - {}", error);
        }
    }

    // Verify results
    let populated_count = verify_populated_metadata(&pool).await?;
    println!("\n✅ Metadata Population Complete!");
    println!("📈 Total records with metadata: {}", populated_count);

    Ok(())
}

async fn create_rate_limited_client() -> Result<(Client, Arc<RateLimiter<governor::state::direct::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>)> {
    // Define rate limit: 10 requests per second (SEC limit)
    let quota = Quota::with_period(Duration::from_millis(100))
        .unwrap()
        .allow_burst(NonZeroU32::new(10).unwrap());
    let limiter = Arc::new(RateLimiter::direct(quota));

    let client = Client::builder()
        .user_agent("rust-stocks-edgar-client/1.0 (contact@example.com)")
        .timeout(Duration::from_secs(30))
        .build()?;

    Ok((client, limiter))
}

async fn get_sp500_stocks_with_ciks(pool: &sqlx::SqlitePool) -> Result<Vec<(i64, String, String)>> {
    let rows = sqlx::query!(
        "SELECT id, cik, symbol FROM stocks WHERE is_sp500 = 1 AND cik IS NOT NULL AND cik != ''"
    )
    .fetch_all(pool)
    .await?;

    let stocks = rows.into_iter()
        .map(|row| (row.id.unwrap(), row.cik.unwrap_or_default(), row.symbol))
        .collect();

    Ok(stocks)
}

#[derive(Debug)]
struct ProcessingResults {
    successful: usize,
    failed: usize,
    errors: Vec<String>,
}

async fn process_ciks_concurrently(
    pool: &sqlx::SqlitePool,
    client: &Client,
    limiter: &Arc<RateLimiter<governor::state::direct::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>,
    stocks: &[(i64, String, String)],
) -> Result<ProcessingResults> {
    let semaphore = Arc::new(Semaphore::new(10)); // 10 concurrent workers
    let results = Arc::new(Mutex::new(ProcessingResults {
        successful: 0,
        failed: 0,
        errors: Vec::new(),
    }));

    let mut handles = Vec::new();

    for (stock_id, cik, symbol) in stocks {
        let permit = semaphore.clone().acquire_owned().await?;
        let client = client.clone();
        let pool = pool.clone();
        let results = results.clone();
        let limiter = limiter.clone();
        let cik = cik.clone();
        let symbol = symbol.clone();
        let stock_id = *stock_id;

        let handle = tokio::spawn(async move {
            let _permit = permit; // Hold the permit for the duration of the task
            
            match process_single_cik(&pool, &client, &limiter, stock_id, &cik, &symbol).await {
                Ok(_) => {
                    let mut res = results.lock().await;
                    res.successful += 1;
                    println!("✅ Processed {} (CIK: {})", symbol, cik);
                }
                Err(e) => {
                    let mut res = results.lock().await;
                    res.failed += 1;
                    res.errors.push(format!("{} (CIK: {}): {}", symbol, cik, e));
                    println!("❌ Failed {} (CIK: {}): {}", symbol, cik, e);
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await?;
    }

    let results = Arc::try_unwrap(results).unwrap().into_inner();
    Ok(results)
}

async fn process_single_cik(
    pool: &sqlx::SqlitePool,
    client: &Client,
    limiter: &Arc<RateLimiter<governor::state::direct::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>,
    stock_id: i64,
    cik: &str,
    symbol: &str,
) -> Result<()> {
    // Download Company Facts JSON with proper rate limiting using governor
    let url = format!("https://data.sec.gov/api/xbrl/companyfacts/CIK{}.json", cik);
    
    // Use governor rate limiter - this will automatically handle the timing
    limiter.until_ready().await;
    
    let response = client.get(&url).send().await?;
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("HTTP {} for CIK {}", response.status(), cik));
    }

    let json: Value = response.json().await?;
    
    // Extract filing data from Company Facts JSON
    let filing_data = extract_filing_data_from_company_facts(&json, cik)?;
    
    // Get our existing financial records
    let our_records = get_our_financial_records(pool, stock_id).await?;
    
    // Match and update records
    let mut updated_count = 0;
    
    for record in our_records {
        if let Some(metadata) = find_matching_filing_in_company_facts(
            &record.report_date,
            &filing_data,
        ) {
            update_financial_record_metadata(
                pool,
                record.id,
                &metadata.filed_date,
                &metadata.accession_number,
                &metadata.form_type,
            ).await?;
            updated_count += 1;
        }
    }
    
    if updated_count > 0 {
        println!("  📝 Updated {} records for {}", updated_count, symbol);
    }

    Ok(())
}

#[derive(Debug)]
struct FilingMetadata {
    filed_date: String,
    accession_number: String,
    form_type: String,
}

#[derive(Debug)]
struct FilingData {
    filing_dates: Vec<String>,
    forms: Vec<String>,
    report_dates: Vec<String>,
    accession_numbers: Vec<String>,
}

fn extract_filing_data_from_company_facts(json: &Value, cik: &str) -> Result<FilingData> {
    let mut filing_dates = Vec::new();
    let mut forms = Vec::new();
    let mut report_dates = Vec::new();
    let mut accession_numbers = Vec::new();
    
    // Navigate to the facts section in Company Facts JSON
    if let Some(facts) = json.get("facts") {
        // Iterate through all fact categories (us-gaap, dei, etc.)
        for (_category, category_data) in facts.as_object().unwrap_or(&serde_json::Map::new()) {
            if let Some(category_obj) = category_data.as_object() {
                // Iterate through all metrics in this category
                for (_metric, metric_data) in category_obj {
                    if let Some(units) = metric_data.get("units") {
                        if let Some(units_obj) = units.as_object() {
                            // Iterate through all unit types (USD, shares, etc.)
                            for (_unit_type, unit_data) in units_obj {
                                if let Some(data_array) = unit_data.as_array() {
                                    // Extract filing metadata from each data point
                                    for data_point in data_array {
                                        if let Some(obj) = data_point.as_object() {
                                            // Extract filing metadata
                                            if let Some(filed) = obj.get("filed").and_then(|v| v.as_str()) {
                                                filing_dates.push(filed.to_string());
                                            }
                                            if let Some(form) = obj.get("form").and_then(|v| v.as_str()) {
                                                forms.push(form.to_string());
                                            }
                                            if let Some(end) = obj.get("end").and_then(|v| v.as_str()) {
                                                report_dates.push(end.to_string());
                                            }
                                            if let Some(accn) = obj.get("accn").and_then(|v| v.as_str()) {
                                                accession_numbers.push(accn.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Remove duplicates and sort
    filing_dates.sort();
    filing_dates.dedup();
    forms.sort();
    forms.dedup();
    report_dates.sort();
    report_dates.dedup();
    accession_numbers.sort();
    accession_numbers.dedup();
    
    println!("  📊 Extracted {} filing dates, {} forms, {} report dates, {} accession numbers for CIK {}", 
             filing_dates.len(), forms.len(), report_dates.len(), accession_numbers.len(), cik);
    
    Ok(FilingData {
        filing_dates,
        forms,
        report_dates,
        accession_numbers,
    })
}

fn find_matching_filing_in_company_facts(
    report_date: &NaiveDate,
    filing_data: &FilingData,
) -> Option<FilingMetadata> {
    // Convert our report_date to string format for comparison
    let report_date_str = report_date.format("%Y-%m-%d").to_string();
    
    // Find matching filing based on report date
    // We need to find the index where report_dates matches our report_date
    if let Some(index) = filing_data.report_dates.iter().position(|date| date == &report_date_str) {
        // Get the corresponding filing metadata
        let filed_date = filing_data.filing_dates.get(index).unwrap_or(&String::new()).clone();
        let form_type = filing_data.forms.get(index).unwrap_or(&String::new()).clone();
        let accession_number = filing_data.accession_numbers.get(index).unwrap_or(&String::new()).clone();
        
        // Only return metadata for 10-K and 10-Q forms
        if form_type == "10-K" || form_type == "10-Q" {
            return Some(FilingMetadata {
                filed_date,
                accession_number,
                form_type,
            });
        }
    }
    
    None
}

#[derive(Debug)]
struct FinancialRecord {
    id: i64,
    report_date: NaiveDate,
    fiscal_year: Option<i32>,
}

async fn get_our_financial_records(pool: &sqlx::SqlitePool, stock_id: i64) -> Result<Vec<FinancialRecord>> {
    // Get records from all three tables
    let mut all_records = Vec::new();
    
    // Income statements
    let income_records = sqlx::query!(
        "SELECT id, report_date as \"report_date: chrono::NaiveDate\", fiscal_year FROM income_statements WHERE stock_id = ?",
        stock_id
    )
    .fetch_all(pool)
    .await?;
    
    for record in income_records {
        all_records.push(FinancialRecord {
            id: record.id.unwrap_or(0),
            report_date: record.report_date,
            fiscal_year: record.fiscal_year.map(|y| y as i32),
        });
    }
    
    // Balance sheets
    let balance_records = sqlx::query!(
        "SELECT id, report_date as \"report_date: chrono::NaiveDate\", fiscal_year FROM balance_sheets WHERE stock_id = ?",
        stock_id
    )
    .fetch_all(pool)
    .await?;
    
    for record in balance_records {
        all_records.push(FinancialRecord {
            id: record.id.unwrap_or(0),
            report_date: record.report_date,
            fiscal_year: record.fiscal_year.map(|y| y as i32),
        });
    }
    
    // Cash flow statements
    let cash_flow_records = sqlx::query!(
        "SELECT id, report_date as \"report_date: chrono::NaiveDate\", fiscal_year FROM cash_flow_statements WHERE stock_id = ?",
        stock_id
    )
    .fetch_all(pool)
    .await?;
    
    for record in cash_flow_records {
        all_records.push(FinancialRecord {
            id: record.id.unwrap_or(0),
            report_date: record.report_date,
            fiscal_year: record.fiscal_year.map(|y| y as i32),
        });
    }
    
    Ok(all_records)
}

async fn update_financial_record_metadata(
    pool: &sqlx::SqlitePool,
    record_id: i64,
    filed_date: &str,
    accession_number: &str,
    form_type: &str,
) -> Result<()> {
    // Update the record with metadata
    // We need to determine which table this record belongs to
    // For now, try all three tables
    
    // Try income_statements first
    let result = sqlx::query!(
        "UPDATE income_statements SET filed_date = ?, accession_number = ?, form_type = ? WHERE id = ?",
        filed_date, accession_number, form_type, record_id
    )
    .execute(pool)
    .await?;
    
    if result.rows_affected() > 0 {
        return Ok(());
    }
    
    // Try balance_sheets
    let result = sqlx::query!(
        "UPDATE balance_sheets SET filed_date = ?, accession_number = ?, form_type = ? WHERE id = ?",
        filed_date, accession_number, form_type, record_id
    )
    .execute(pool)
    .await?;
    
    if result.rows_affected() > 0 {
        return Ok(());
    }
    
    // Try cash_flow_statements
    sqlx::query!(
        "UPDATE cash_flow_statements SET filed_date = ?, accession_number = ?, form_type = ? WHERE id = ?",
        filed_date, accession_number, form_type, record_id
    )
    .execute(pool)
    .await?;
    
    Ok(())
}

async fn verify_populated_metadata(pool: &sqlx::SqlitePool) -> Result<i64> {
    let result = sqlx::query!(
        "SELECT COUNT(*) as count FROM (
            SELECT id FROM income_statements WHERE filed_date IS NOT NULL AND accession_number IS NOT NULL AND form_type IS NOT NULL
            UNION ALL
            SELECT id FROM balance_sheets WHERE filed_date IS NOT NULL AND accession_number IS NOT NULL AND form_type IS NOT NULL
            UNION ALL
            SELECT id FROM cash_flow_statements WHERE filed_date IS NOT NULL AND accession_number IS NOT NULL AND form_type IS NOT NULL
        )"
    )
    .fetch_one(pool)
    .await?;
    
    Ok(result.count)
}
