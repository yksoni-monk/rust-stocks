/// Populate CIK Mappings for S&P 500 Companies
/// 
/// Reads SEC company_tickers_exchange.json and populates the cik_mappings_sp500 table
/// with mappings for S&P 500 companies only. This creates a single source of truth
/// for EDGAR data extraction.

use anyhow::{Result, anyhow};
use serde::Deserialize;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::Path;
use std::fs;
use tracing::info;

#[derive(Debug, Deserialize)]
struct SecCompanyTickers {
    _fields: Vec<String>,
    data: Vec<Vec<serde_json::Value>>,
}

#[derive(Debug)]
struct CikMapping {
    cik: String,
    stock_id: i64,
    symbol: String,
    company_name: String,
    edgar_file_path: String,
    file_exists: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("🔧 Populating CIK Mappings for S&P 500 Companies");
    info!("================================================");
    
    // Connect to database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:./db/stocks.db".to_string());
    let db_pool = SqlitePool::connect(&database_url).await?;
    
    // Table should already exist from migration
    
    // Populate CIK mappings
    populate_cik_mappings(&db_pool).await?;
    
    info!("✅ CIK mappings population completed successfully!");
    
    Ok(())
}

async fn populate_cik_mappings(db_pool: &SqlitePool) -> Result<()> {
    info!("📋 Loading S&P 500 stocks from database...");
    
    // Load all S&P 500 stocks
    let sp500_stocks = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, symbol, company_name FROM stocks WHERE is_sp500 = 1 ORDER BY symbol"
    )
    .fetch_all(db_pool)
    .await?;
    
    info!("Found {} S&P 500 stocks in database", sp500_stocks.len());
    
    // Create symbol lookup map
    let mut symbol_to_stock: HashMap<String, (i64, String)> = HashMap::new();
    for (id, symbol, name) in sp500_stocks {
        symbol_to_stock.insert(symbol.clone(), (id, name));
    }
    
    info!("📁 Loading SEC company tickers file...");
    
    // Load SEC company tickers
    let sec_tickers = load_sec_company_tickers().await?;
    
    info!("Found {} companies in SEC file", sec_tickers.data.len());
    
    info!("🔗 Building CIK mappings for S&P 500 companies...");
    
    // Build CIK mappings (deduplicate by CIK)
    let mut cik_mappings = Vec::new();
    let mut seen_ciks = HashMap::new();
    let mut matched_count = 0;
    let mut file_exists_count = 0;
    
    for entry in &sec_tickers.data {
        if entry.len() >= 3 {
            if let (Some(cik_val), Some(ticker_val)) = (entry.get(0), entry.get(2)) {
                if let (Some(cik_num), Some(ticker_str)) = (cik_val.as_i64(), ticker_val.as_str()) {
                    let cik = cik_num.to_string();
                    let ticker = ticker_str.to_string();
                    
                    // Skip if we've already seen this CIK
                    if seen_ciks.contains_key(&cik) {
                        continue;
                    }
                    
                    // Check if this ticker is in our S&P 500 list
                    if let Some((stock_id, company_name)) = symbol_to_stock.get(&ticker) {
                        matched_count += 1;
                        seen_ciks.insert(cik.clone(), ticker.clone());
                        
                        // Build EDGAR file path
                        let edgar_file_path = format!(
                            "/Users/yksoni/code/misc/rust-stocks/edgar_data/companyfacts/CIK{:010}.json", 
                            cik_num
                        );
                        
                        // Check if file exists
                        let file_exists = Path::new(&edgar_file_path).exists();
                        if file_exists {
                            file_exists_count += 1;
                        }
                        
                        cik_mappings.push(CikMapping {
                            cik,
                            stock_id: *stock_id,
                            symbol: ticker,
                            company_name: company_name.clone(),
                            edgar_file_path,
                            file_exists,
                        });
                    }
                }
            }
        }
    }
    
    info!("📊 CIK Mapping Results:");
    info!("   S&P 500 companies: {}", symbol_to_stock.len());
    info!("   SEC file matches: {}", matched_count);
    info!("   EDGAR files exist: {}", file_exists_count);
    info!("   Coverage: {:.1}%", (file_exists_count as f64 / symbol_to_stock.len() as f64) * 100.0);
    
    info!("💾 Inserting CIK mappings into database...");
    
    // Clear existing mappings
    let deleted_count = sqlx::query("DELETE FROM cik_mappings_sp500")
        .execute(db_pool)
        .await?
        .rows_affected();
    
    info!("Cleared {} existing CIK mappings", deleted_count);
    
    // Insert new mappings in batches
    let mut tx = db_pool.begin().await?;
    let mut inserted_count = 0;
    
    for mapping in &cik_mappings {
        sqlx::query(
            r#"
            INSERT INTO cik_mappings_sp500 (cik, stock_id, symbol, company_name, edgar_file_path, file_exists)
            VALUES (?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&mapping.cik)
        .bind(mapping.stock_id)
        .bind(&mapping.symbol)
        .bind(&mapping.company_name)
        .bind(&mapping.edgar_file_path)
        .bind(mapping.file_exists)
        .execute(&mut *tx)
        .await?;
        
        inserted_count += 1;
    }
    
    tx.commit().await?;
    
    info!("✅ Inserted {} CIK mappings successfully", inserted_count);
    
    // Show some sample mappings
    info!("📋 Sample CIK mappings:");
    for (i, mapping) in cik_mappings.iter().take(10).enumerate() {
        let status = if mapping.file_exists { "✅" } else { "❌" };
        info!("   {}. {} -> {} {} (CIK: {})", 
              i + 1, mapping.symbol, mapping.company_name, status, mapping.cik);
    }
    
    // Show statistics
    let stats = sqlx::query_as::<_, (i64, i64)>(
        "SELECT COUNT(*) as total, SUM(CASE WHEN file_exists = 1 THEN 1 ELSE 0 END) as with_files 
         FROM cik_mappings_sp500"
    )
    .fetch_one(db_pool)
    .await?;
    
    info!("📈 Final Statistics:");
    info!("   Total mappings: {}", stats.0);
    info!("   With EDGAR files: {}", stats.1);
    info!("   Ready for extraction: {:.1}%", (stats.1 as f64 / stats.0 as f64) * 100.0);
    
    Ok(())
}

async fn load_sec_company_tickers() -> Result<SecCompanyTickers> {
    let tickers_file_path = "/Users/yksoni/code/misc/rust-stocks/edgar_data/company_tickers_exchange.json";
    
    if !Path::new(tickers_file_path).exists() {
        return Err(anyhow!("SEC company tickers file not found at {}", tickers_file_path));
    }
    
    let content = fs::read_to_string(tickers_file_path)?;
    let sec_tickers: SecCompanyTickers = serde_json::from_str(&content)?;
    
    Ok(sec_tickers)
}