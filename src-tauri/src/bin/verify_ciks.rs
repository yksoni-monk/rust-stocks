use anyhow::Result;
use rust_stocks_tauri_lib::database::helpers::get_database_connection;
use serde_json::Value;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Verifying CIK values against SEC official data");
    println!("══════════════════════════════════════");

    // Initialize database connection
    let pool = get_database_connection().await.map_err(|e| anyhow::anyhow!("Database connection failed: {}", e))?;
    
    // Download correct CIKs from SEC
    println!("🌐 Downloading CIKs from SEC...");
    let client = reqwest::Client::builder()
        .user_agent("rust-stocks-edgar-client/1.0 (contact@example.com)")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    
    let response = client.get("https://www.sec.gov/files/company_tickers.json").send().await?;
    let json: Value = response.json().await?;
    
    println!("🔍 Debug: JSON structure - {:?}", json);
    
    let mut sec_ciks = HashMap::new();
    if let Some(obj) = json.as_object() {
        println!("🔍 Debug: Found {} companies in JSON", obj.len());
        for (_key, company) in obj {
            if let Some(ticker) = company.get("ticker").and_then(|v| v.as_str()) {
                // Handle both string and number CIK formats
                let cik_value = if let Some(cik_str) = company.get("cik_str").and_then(|v| v.as_str()) {
                    cik_str.parse::<u64>().unwrap_or(0)
                } else if let Some(cik_num) = company.get("cik_str").and_then(|v| v.as_u64()) {
                    cik_num
                } else {
                    continue;
                };
                
                // Convert to 10-digit format
                let cik_10_digit = format!("{:010}", cik_value);
                if sec_ciks.len() <= 5 {
                    println!("🔍 Debug: {} -> {} (CIK: {})", ticker, cik_10_digit, cik_value);
                }
                sec_ciks.insert(ticker.to_string(), cik_10_digit);
            }
        }
    } else {
        println!("❌ Debug: JSON is not an object");
    }
    
    println!("✅ Downloaded {} CIKs from SEC", sec_ciks.len());
    
    // Get all S&P 500 stocks from our database
    let stocks = sqlx::query!(
        "SELECT id, symbol, cik FROM stocks WHERE is_sp500 = 1 AND cik IS NOT NULL ORDER BY symbol"
    )
    .fetch_all(&pool)
    .await?;
    
    println!("📊 Checking {} S&P 500 stocks...", stocks.len());
    
    let mut correct_count = 0;
    let mut incorrect_count = 0;
    let mut not_found_count = 0;
    let mut corrections = Vec::new();
    
    for stock in stocks {
        let our_cik = stock.cik.unwrap_or_default();
        let symbol = &stock.symbol;
        
        if let Some(sec_cik) = sec_ciks.get(symbol) {
            if our_cik == *sec_cik {
                correct_count += 1;
                println!("✅ {}: {} (CORRECT)", symbol, our_cik);
            } else {
                incorrect_count += 1;
                println!("❌ {}: {} (OURS) vs {} (SEC)", symbol, our_cik, sec_cik);
                corrections.push((stock.id, sec_cik.clone()));
            }
        } else {
            not_found_count += 1;
            println!("⚠️  {}: {} (NOT FOUND IN SEC)", symbol, our_cik);
        }
    }
    
    println!("\n📈 Summary:");
    println!("✅ Correct: {}", correct_count);
    println!("❌ Incorrect: {}", incorrect_count);
    println!("⚠️  Not found: {}", not_found_count);
    
    if !corrections.is_empty() {
        println!("\n🔧 Applying corrections...");
        let correction_count = corrections.len();
        for (stock_id, correct_cik) in corrections {
            sqlx::query!(
                "UPDATE stocks SET cik = ? WHERE id = ?",
                correct_cik,
                stock_id
            )
            .execute(&pool)
            .await?;
        }
        println!("✅ Applied {} corrections", correction_count);
    }
    
    Ok(())
}
