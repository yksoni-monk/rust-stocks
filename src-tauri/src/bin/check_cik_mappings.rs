use anyhow::Result;
use rust_stocks_tauri_lib::database::helpers::get_database_connection;
use serde_json::Value;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Checking CIK mappings and correcting format");
    println!("══════════════════════════════════════");

    // Initialize database connection
    let pool = get_database_connection().await.map_err(|e| anyhow::anyhow!("Database connection failed: {}", e))?;
    
    // Check current CIK mappings
    println!("📊 Current CIK mappings in cik_mappings_sp500:");
    let mappings = sqlx::query!(
        "SELECT symbol, cik FROM cik_mappings_sp500 WHERE symbol IN ('AAPL', 'MSFT', 'GOOGL', 'AMZN', 'TSLA', 'NVDA', 'META', 'BRK.A', 'UNH', 'JNJ') ORDER BY symbol"
    )
    .fetch_all(&pool)
    .await?;
    
    for mapping in mappings {
        let cik = mapping.cik.unwrap_or("".to_string());
        println!("  {}: {}", mapping.symbol, cik);
    }
    
    // Check current CIKs in stocks table
    println!("\n📊 Current CIKs in stocks table:");
    let stocks = sqlx::query!(
        "SELECT symbol, cik FROM stocks WHERE symbol IN ('AAPL', 'MSFT', 'GOOGL', 'AMZN', 'TSLA', 'NVDA', 'META', 'BRK.A', 'UNH', 'JNJ') AND is_sp500 = 1 ORDER BY symbol"
    )
    .fetch_all(&pool)
    .await?;
    
    for stock in stocks {
        let cik = stock.cik.unwrap_or("".to_string());
        println!("  {}: {}", stock.symbol, cik);
    }
    
    // Download correct CIKs from SEC
    println!("\n🌐 Downloading correct CIKs from SEC...");
    let client = reqwest::Client::builder()
        .user_agent("rust-stocks-edgar-client/1.0 (contact@example.com)")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    
    let response = client.get("https://www.sec.gov/files/company_tickers.json").send().await?;
    let json: Value = response.json().await?;
    
    let mut sec_ciks = HashMap::new();
    if let Some(obj) = json.as_object() {
        for (_, company) in obj {
            if let (Some(ticker), Some(cik)) = (company.get("ticker").and_then(|v| v.as_str()), company.get("cik_str").and_then(|v| v.as_str())) {
                sec_ciks.insert(ticker.to_string(), cik.to_string());
            }
        }
    }
    
    println!("✅ Downloaded {} CIKs from SEC", sec_ciks.len());
    
    // Check SEC CIKs for our test symbols
    println!("\n📊 SEC CIKs for test symbols:");
    let test_symbols = ["AAPL", "MSFT", "GOOGL", "AMZN", "TSLA", "NVDA", "META", "BRK.A", "UNH", "JNJ"];
    for symbol in &test_symbols {
        if let Some(sec_cik) = sec_ciks.get(*symbol) {
            println!("  {}: {}", symbol, sec_cik);
        } else {
            println!("  {}: NOT FOUND", symbol);
        }
    }
    
    // Compare with our database CIKs
    println!("\n🔍 Comparison:");
    for stock in &stocks {
        let symbol = &stock.symbol;
        let our_cik = stock.cik.as_ref().unwrap_or(&"".to_string());
        
        if let Some(sec_cik) = sec_ciks.get(symbol) {
            let correct_cik = format!("{:010}", sec_cik.parse::<u64>().unwrap_or(0));
            if our_cik == &correct_cik {
                println!("  ✅ {}: {} (CORRECT)", symbol, our_cik);
            } else {
                println!("  ❌ {}: {} (OURS) vs {} (SEC)", symbol, our_cik, correct_cik);
            }
        } else {
            println!("  ⚠️  {}: {} (NOT FOUND IN SEC)", symbol, our_cik);
        }
    }
    
    Ok(())
}
