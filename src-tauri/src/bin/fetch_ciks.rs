/// CIK Fetcher for S&P 500 Stocks
///
/// Fetches Central Index Keys (CIKs) for all S&P 500 stocks from the SEC EDGAR API
/// and populates the stocks table with CIK values for EDGAR data extraction.

use anyhow::Result;
use clap::Parser;
use reqwest;
use serde_json::Value;
use sqlx::sqlite::SqlitePoolOptions;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Parser)]
#[command(
    name = "fetch_ciks",
    about = "🔍 Fetch CIKs for S&P 500 stocks from SEC EDGAR API",
    long_about = "Queries the SEC EDGAR API to get Central Index Keys (CIKs) for all S&P 500 stocks and updates the database."
)]
struct Cli {
    /// Update existing CIKs (overwrite current values)
    #[arg(long)]
    force: bool,
    
    /// Limit number of stocks to process (for testing)
    #[arg(long)]
    limit: Option<usize>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    println!("🔍 CIK Fetcher for S&P 500 Stocks");
    println!("==================================");
    
    // Get database connection
    let database_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_PATH").map(|path| format!("sqlite:{}", path)))
        .unwrap_or_else(|_| "sqlite:db/stocks.db".to_string());
    
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;
    
    // Get S&P 500 stocks without CIKs
    let stocks_query = if cli.force {
        "SELECT id, symbol, company_name FROM stocks WHERE is_sp500 = 1 AND (cik IS NULL OR cik = '') ORDER BY symbol"
    } else {
        "SELECT id, symbol, company_name FROM stocks WHERE is_sp500 = 1 AND (cik IS NULL OR cik = '') ORDER BY symbol"
    };
    
    let stocks: Vec<(i64, String, String)> = sqlx::query_as(stocks_query)
        .fetch_all(&pool)
        .await?;
    
    let stocks_to_process = if let Some(limit) = cli.limit {
        stocks.into_iter().take(limit).collect()
    } else {
        stocks
    };
    
    println!("📊 Found {} S&P 500 stocks to process", stocks_to_process.len());
    
    if stocks_to_process.is_empty() {
        println!("✅ All S&P 500 stocks already have CIKs!");
        return Ok(());
    }
    
    // Create HTTP client with rate limiting
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("rust-stocks-cik-fetcher/1.0")
        .build()?;
    
    let mut success_count = 0;
    let mut error_count = 0;
    
    for (i, (stock_id, symbol, company_name)) in stocks_to_process.iter().enumerate() {
        println!("[{}/{}] Processing {} ({})", i + 1, stocks_to_process.len(), symbol, company_name);
        
        match fetch_cik_for_stock(&client, symbol, &company_name).await {
            Ok(Some(cik)) => {
                // Update database with CIK
                match sqlx::query("UPDATE stocks SET cik = ? WHERE id = ?")
                    .bind(&cik)
                    .bind(stock_id)
                    .execute(&pool)
                    .await
                {
                    Ok(_) => {
                        println!("  ✅ {} -> CIK: {}", symbol, cik);
                        success_count += 1;
                    }
                    Err(e) => {
                        println!("  ❌ {} -> Database update failed: {}", symbol, e);
                        error_count += 1;
                    }
                }
            }
            Ok(None) => {
                println!("  ⚠️  {} -> No CIK found", symbol);
                error_count += 1;
            }
            Err(e) => {
                println!("  ❌ {} -> API error: {}", symbol, e);
                error_count += 1;
            }
        }
        
        // Rate limiting: SEC allows 10 requests per second
        sleep(Duration::from_millis(100)).await;
    }
    
    println!("\n📊 CIK Fetch Summary:");
    println!("  ✅ Successfully updated: {}", success_count);
    println!("  ❌ Failed/Not found: {}", error_count);
    println!("  📈 Success rate: {:.1}%", 
        if stocks_to_process.len() > 0 { 
            (success_count as f64 / stocks_to_process.len() as f64) * 100.0 
        } else { 
            0.0 
        });
    
    Ok(())
}

/// Fetch CIK for a specific stock symbol from SEC EDGAR API
async fn fetch_cik_for_stock(client: &reqwest::Client, symbol: &str, company_name: &str) -> Result<Option<String>> {
    // Try multiple search strategies
    let search_terms = vec![
        symbol.to_string(),
        company_name.to_string(),
        // Try without common suffixes
        company_name.replace(" Inc.", "").replace(" Corp.", "").replace(" Company", "").replace(" Ltd.", ""),
    ];
    
    for search_term in search_terms {
        if let Some(cik) = search_cik_by_term(client, &search_term).await? {
            return Ok(Some(cik));
        }
    }
    
    Ok(None)
}

/// Search for CIK using a specific search term
async fn search_cik_by_term(client: &reqwest::Client, search_term: &str) -> Result<Option<String>> {
    let url = format!(
        "https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&CIK={}&type=10-K&dateb=&owner=include&count=1&output=json",
        search_term
    );
    
    let response = client.get(&url).send().await?;
    
    if !response.status().is_success() {
        return Ok(None);
    }
    
    let text = response.text().await?;
    
    // Try to parse as JSON
    if let Ok(json) = serde_json::from_str::<Value>(&text) {
        if let Some(companies) = json.get("companies") {
            if let Some(companies_array) = companies.as_array() {
                if let Some(first_company) = companies_array.first() {
                    if let Some(cik) = first_company.get("CIK") {
                        if let Some(cik_str) = cik.as_str() {
                            // CIK should be 10 digits, pad with leading zeros if needed
                            let padded_cik = format!("{:0>10}", cik_str);
                            return Ok(Some(padded_cik));
                        }
                    }
                }
            }
        }
    }
    
    // Fallback: try to extract CIK from HTML response
    if let Some(cik) = extract_cik_from_html(&text) {
        return Ok(Some(cik));
    }
    
    Ok(None)
}

/// Extract CIK from HTML response (fallback method)
fn extract_cik_from_html(html: &str) -> Option<String> {
    // Look for CIK pattern in HTML
    if let Some(start) = html.find("CIK=") {
        let after_cik = &html[start + 4..];
        if let Some(end) = after_cik.find('&') {
            let cik_part = &after_cik[..end];
            // Extract just the numeric part
            let cik_digits: String = cik_part.chars().filter(|c| c.is_ascii_digit()).collect();
            if cik_digits.len() >= 4 && cik_digits.len() <= 10 {
                return Some(format!("{:0>10}", cik_digits));
            }
        }
    }
    
    None
}
