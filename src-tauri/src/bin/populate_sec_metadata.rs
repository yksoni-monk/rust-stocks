use anyhow::Result;
use chrono::NaiveDate;
use rust_stocks_tauri_lib::database::helpers::get_database_connection;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔄 Starting SEC Filing Metadata Population");
    println!("📅 {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"));
    println!("══════════════════════════════════════");

    // Initialize database connection
    let pool = get_database_connection().await.map_err(|e| anyhow::anyhow!("Database connection failed: {}", e))?;
    
    // Get S&P 500 stocks with CIKs
    let stocks_with_ciks = get_sp500_stocks_with_ciks(&pool).await?;
    println!("✅ Found {} S&P 500 stocks with CIKs", stocks_with_ciks.len());

    // Download and extract bulk submissions if needed
    let submissions_data = download_bulk_submissions_if_needed().await?;
    let extracted_files = extract_submissions_json_files(submissions_data).await?;
    println!("✅ Extracted {} submission JSON files", extracted_files.len());
    
    // Debug: Check if we have some common CIKs
    let test_ciks = ["0000320193", "0000789019", "0001652044", "0001018724", "0001318605"];
    for test_cik in &test_ciks {
        if extracted_files.contains_key(*test_cik) {
            println!("✅ Found CIK {} in submissions", test_cik);
        } else {
            println!("❌ Missing CIK {} in submissions", test_cik);
        }
    }
    
    // Debug: Show first 10 available CIKs
    println!("🔍 First 10 available CIKs in submissions:");
    let mut count = 0;
    for (cik, _) in &extracted_files {
        if count < 10 {
            println!("  - {}", cik);
            count += 1;
        }
    }
    
    // Debug: Check if any of our CIKs have partial matches
    println!("🔍 Checking for partial CIK matches:");
    let our_ciks = ["0000320193", "0000789019", "0001652044", "0001018724", "0001318605"];
    for our_cik in &our_ciks {
        let mut found_partial = false;
        for (sec_cik, _) in &extracted_files {
            if sec_cik.contains(&our_cik[3..]) { // Check last 7 digits
                println!("  - {} matches {} (partial)", our_cik, sec_cik);
                found_partial = true;
                break;
            }
        }
        if !found_partial {
            println!("  - {} has no partial matches", our_cik);
        }
    }

    // Process CIKs concurrently (10 threads)
    let semaphore = Arc::new(Semaphore::new(10));
    let mut handles = vec![];

    for (stock_id, cik, symbol) in stocks_with_ciks {
        let permit = semaphore.clone().acquire_owned().await?;
        let pool_clone = pool.clone();
        let extracted_files_clone = extracted_files.clone();
        
        let handle = task::spawn(async move {
            let _permit = permit;
            process_cik_metadata(&pool_clone, stock_id, &cik, &symbol, &extracted_files_clone).await
        });
        
        handles.push(handle);
    }

    // Wait for all tasks to complete
    let mut successful = 0;
    let mut failed = 0;
    let mut corrupted_ciks = vec![];

    for handle in handles {
        match handle.await? {
            Ok(()) => successful += 1,
            Err(e) => {
                failed += 1;
                corrupted_ciks.push(e.to_string());
            }
        }
    }

    println!("\n📊 Population Results:");
    println!("✅ Successful: {}", successful);
    println!("❌ Failed: {}", failed);
    
    if !corrupted_ciks.is_empty() {
        println!("\n🚨 Corrupted/Missing CIKs:");
        for cik in corrupted_ciks {
            println!("  - {}", cik);
        }
    }

    // Verify results
    let populated_count = verify_populated_metadata(&pool).await?;
    println!("\n✅ Metadata Population Complete!");
    println!("📈 Total records with metadata: {}", populated_count);

    Ok(())
}

async fn get_sp500_stocks_with_ciks(pool: &sqlx::SqlitePool) -> Result<Vec<(i64, String, String)>> {
    let rows = sqlx::query!(
        "SELECT id, cik, symbol FROM stocks WHERE is_sp500 = 1 AND cik IS NOT NULL AND cik != ''"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|row| (row.id, row.cik.unwrap_or_default(), row.symbol)).collect())
}

async fn download_bulk_submissions_if_needed() -> Result<Vec<u8>> {
    let cache_dir = std::env::temp_dir().join("rust-stocks-sec");
    tokio::fs::create_dir_all(&cache_dir).await?;
    
    let zip_cache_path = cache_dir.join("submissions.zip");
    
    // Check if we already have the zip file
    if zip_cache_path.exists() {
        println!("📦 Using existing bulk submissions file");
        return Ok(tokio::fs::read(&zip_cache_path).await?);
    }
    
    println!("🌐 Downloading bulk submissions from SEC...");
    
    let client = reqwest::Client::builder()
        .user_agent("rust-stocks-edgar-client/1.0 (contact@example.com)")
        .timeout(std::time::Duration::from_secs(300))
        .build()?;
    
    let url = "https://www.sec.gov/Archives/edgar/daily-index/bulkdata/submissions.zip";
    let response = client.get(url).send().await?;
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Failed to download submissions.zip: {}", response.status()));
    }
    
    let zip_data = response.bytes().await?.to_vec();
    tokio::fs::write(&zip_cache_path, &zip_data).await?;
    
    println!("✅ Downloaded bulk submissions successfully: {} MB", zip_data.len() / 1024 / 1024);
    Ok(zip_data)
}

async fn extract_submissions_json_files(zip_data: Vec<u8>) -> Result<HashMap<String, PathBuf>> {
    use zip::ZipArchive;
    use std::io::{Cursor, Read};

    println!("📂 Extracting submission JSON files from zip...");
    
    let cache_dir = std::env::temp_dir().join("rust-stocks-sec");
    tokio::fs::create_dir_all(&cache_dir).await?;
    
    let extracted_files = task::spawn_blocking(move || {
        let cursor = Cursor::new(zip_data);
        let mut archive = ZipArchive::new(cursor)?;
        let mut files = HashMap::new();
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let file_path = file.name().to_string();
            
            if file_path.ends_with(".json") && file_path.contains("submissions") {
                let output_path = cache_dir.join(&file_path);
                
                if let Some(parent) = output_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                
                let mut contents = Vec::new();
                file.read_to_end(&mut contents)?;
                std::fs::write(&output_path, contents)?;
                
                // Extract CIK from filename
                let path_buf = std::path::PathBuf::from(&file_path);
                if let Some(filename) = path_buf.file_name().and_then(|n| n.to_str()) {
                    if filename.starts_with("CIK") && filename.ends_with(".json") {
                        if let Some(cik_with_suffix) = filename.strip_prefix("CIK").and_then(|s| s.strip_suffix(".json")) {
                            // Extract just the 10-digit CIK part (before any suffix)
                            let cik = if cik_with_suffix.contains('-') {
                                cik_with_suffix.split('-').next().unwrap_or(cik_with_suffix)
                            } else {
                                cik_with_suffix
                            };
                            files.insert(cik.to_string(), output_path);
                            // Debug: print first few CIKs
                            if files.len() <= 5 {
                                println!("DEBUG: Found CIK file: {} -> {}", filename, cik);
                            }
                        }
                    }
                }
            }
        }
        
        Ok::<HashMap<String, PathBuf>, anyhow::Error>(files)
    }).await??;
    
    Ok(extracted_files)
}

async fn process_cik_metadata(
    pool: &sqlx::SqlitePool,
    stock_id: i64,
    cik: &str,
    symbol: &str,
    extracted_files: &HashMap<String, PathBuf>,
) -> Result<()> {
    // Find the CIK's submission file
    let file_path = extracted_files.get(cik)
        .ok_or_else(|| anyhow::anyhow!("CIK {} not found in submissions", cik))?;
    
    // Read and parse the JSON file
    let content = tokio::fs::read_to_string(file_path).await?;
    let json: Value = serde_json::from_str(&content)?;
    
    // Extract filing data
    let filings = json["filings"]["recent"].as_object()
        .ok_or_else(|| anyhow::anyhow!("Invalid JSON structure for CIK {}", cik))?;
    
    let filing_dates = filings["filingDate"].as_array()
        .ok_or_else(|| anyhow::anyhow!("No filingDate array for CIK {}", cik))?;
    let forms = filings["form"].as_array()
        .ok_or_else(|| anyhow::anyhow!("No form array for CIK {}", cik))?;
    let report_dates = filings["reportDate"].as_array()
        .ok_or_else(|| anyhow::anyhow!("No reportDate array for CIK {}", cik))?;
    let accession_numbers = filings["accessionNumber"].as_array()
        .ok_or_else(|| anyhow::anyhow!("No accessionNumber array for CIK {}", cik))?;
    
    // Get our existing financial records
    let our_records = get_our_financial_records(pool, stock_id).await?;
    
    // Match and update records
    let mut updated_count = 0;
    
    for record in our_records {
        if let Some(metadata) = find_matching_filing(
            &record.report_date,
            filing_dates,
            forms,
            report_dates,
            accession_numbers,
        ) {
            update_record_metadata(pool, &record, &metadata).await?;
            updated_count += 1;
        } else {
            // Remove record with no SEC filing match
            remove_record_without_metadata(pool, &record).await?;
        }
    }
    
    if updated_count > 0 {
        println!("✅ {} ({}) - Updated {} records", symbol, cik, updated_count);
    }
    
    Ok(())
}

#[derive(Debug)]
struct FinancialRecord {
    table_name: String,
    id: i64,
    stock_id: i64,
    report_date: NaiveDate,
    fiscal_year: Option<i32>,
}

#[derive(Debug)]
struct FilingMetadata {
    filed_date: NaiveDate,
    accession_number: String,
    form_type: String,
}

async fn get_our_financial_records(pool: &sqlx::SqlitePool, stock_id: i64) -> Result<Vec<FinancialRecord>> {
    let mut records = Vec::new();
    
    // Get income statements
    let income_rows = sqlx::query!(
        "SELECT id, report_date, fiscal_year FROM income_statements WHERE stock_id = ?",
        stock_id
    )
    .fetch_all(pool)
    .await?;
    
    for row in income_rows {
        records.push(FinancialRecord {
            table_name: "income_statements".to_string(),
            id: row.id.unwrap_or(0),
            stock_id,
            report_date: row.report_date,
            fiscal_year: row.fiscal_year.map(|y| y as i32),
        });
    }
    
    // Get balance sheets
    let balance_rows = sqlx::query!(
        "SELECT id, report_date, fiscal_year FROM balance_sheets WHERE stock_id = ?",
        stock_id
    )
    .fetch_all(pool)
    .await?;
    
    for row in balance_rows {
        records.push(FinancialRecord {
            table_name: "balance_sheets".to_string(),
            id: row.id.unwrap_or(0),
            stock_id,
            report_date: row.report_date,
            fiscal_year: row.fiscal_year.map(|y| y as i32),
        });
    }
    
    // Get cash flow statements
    let cashflow_rows = sqlx::query!(
        "SELECT id, report_date, fiscal_year FROM cash_flow_statements WHERE stock_id = ?",
        stock_id
    )
    .fetch_all(pool)
    .await?;
    
    for row in cashflow_rows {
        records.push(FinancialRecord {
            table_name: "cash_flow_statements".to_string(),
            id: row.id.unwrap_or(0),
            stock_id,
            report_date: row.report_date,
            fiscal_year: row.fiscal_year.map(|y| y as i32),
        });
    }
    
    Ok(records)
}

fn find_matching_filing(
    report_date: &NaiveDate,
    filing_dates: &[Value],
    forms: &[Value],
    report_dates: &[Value],
    accession_numbers: &[Value],
) -> Option<FilingMetadata> {
    // Look for 10-K filings first, then 10-Q
    for form_type in ["10-K", "10-Q"] {
        for (i, (form, report_date_val)) in forms.iter().zip(report_dates.iter()).enumerate() {
            if let (Some(form_str), Some(report_date_str)) = (form.as_str(), report_date_val.as_str()) {
                if form_str == form_type && report_date_str == &report_date.to_string() {
                    if let (Some(filed_date_str), Some(accession_str)) = (
                        filing_dates.get(i).and_then(|v| v.as_str()),
                        accession_numbers.get(i).and_then(|v| v.as_str())
                    ) {
                        if let Ok(filed_date) = NaiveDate::parse_from_str(filed_date_str, "%Y-%m-%d") {
                            return Some(FilingMetadata {
                                filed_date,
                                accession_number: accession_str.to_string(),
                                form_type: form_type.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
    None
}

async fn update_record_metadata(
    pool: &sqlx::SqlitePool,
    record: &FinancialRecord,
    metadata: &FilingMetadata,
) -> Result<()> {
    let accession_number = metadata.accession_number.clone();
    let form_type = metadata.form_type.clone();
    
    match record.table_name.as_str() {
        "income_statements" => {
            sqlx::query!(
                "UPDATE income_statements SET filed_date = ?, accession_number = ?, form_type = ? WHERE id = ?",
                metadata.filed_date,
                accession_number,
                form_type,
                record.id
            )
            .execute(pool)
            .await?;
        }
        "balance_sheets" => {
            sqlx::query!(
                "UPDATE balance_sheets SET filed_date = ?, accession_number = ?, form_type = ? WHERE id = ?",
                metadata.filed_date,
                accession_number,
                form_type,
                record.id
            )
            .execute(pool)
            .await?;
        }
        "cash_flow_statements" => {
            sqlx::query!(
                "UPDATE cash_flow_statements SET filed_date = ?, accession_number = ?, form_type = ? WHERE id = ?",
                metadata.filed_date,
                accession_number,
                form_type,
                record.id
            )
            .execute(pool)
            .await?;
        }
        _ => return Err(anyhow::anyhow!("Unknown table: {}", record.table_name)),
    }
    
    Ok(())
}

async fn remove_record_without_metadata(
    pool: &sqlx::SqlitePool,
    record: &FinancialRecord,
) -> Result<()> {
    match record.table_name.as_str() {
        "income_statements" => {
            sqlx::query!(
                "DELETE FROM income_statements WHERE id = ?",
                record.id
            )
            .execute(pool)
            .await?;
        }
        "balance_sheets" => {
            sqlx::query!(
                "DELETE FROM balance_sheets WHERE id = ?",
                record.id
            )
            .execute(pool)
            .await?;
        }
        "cash_flow_statements" => {
            sqlx::query!(
                "DELETE FROM cash_flow_statements WHERE id = ?",
                record.id
            )
            .execute(pool)
            .await?;
        }
        _ => return Err(anyhow::anyhow!("Unknown table: {}", record.table_name)),
    }
    
    Ok(())
}

async fn verify_populated_metadata(pool: &sqlx::SqlitePool) -> Result<i64> {
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM (
            SELECT id FROM income_statements WHERE filed_date IS NOT NULL
            UNION ALL
            SELECT id FROM balance_sheets WHERE filed_date IS NOT NULL
            UNION ALL
            SELECT id FROM cash_flow_statements WHERE filed_date IS NOT NULL
        )"
    )
    .fetch_one(pool)
    .await?;
    
    Ok(count)
}
