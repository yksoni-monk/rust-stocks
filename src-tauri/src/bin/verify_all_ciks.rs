use anyhow::Result;
use rust_stocks_tauri_lib::database::helpers::get_database_connection;
use serde_json::Value;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Verifying CIK values across all database tables");
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
    
    let mut sec_ciks = HashMap::new();
    if let Some(obj) = json.as_object() {
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
                sec_ciks.insert(ticker.to_string(), cik_10_digit);
            }
        }
    }
    
    println!("✅ Downloaded {} CIKs from SEC", sec_ciks.len());
    
    // Check each table with CIK values
    let tables = vec![
        ("stocks", "SELECT symbol, cik FROM stocks WHERE is_sp500 = 1 AND cik IS NOT NULL ORDER BY symbol"),
    ];
    
    for (table_name, query) in tables {
        println!("\n📊 Checking table: {}", table_name);
        println!("{}", "─".repeat(50));
        
        let rows = sqlx::query_as::<_, (String, String)>(query)
            .fetch_all(&pool)
            .await?;
        
        let mut correct_count = 0;
        let mut incorrect_count = 0;
        let mut not_found_count = 0;
        let mut corrections = Vec::new();
        
        for (symbol, our_cik) in rows {
            if let Some(sec_cik) = sec_ciks.get(&symbol) {
                if our_cik == *sec_cik {
                    correct_count += 1;
                    println!("✅ {}: {} (CORRECT)", symbol, our_cik);
                } else {
                    incorrect_count += 1;
                    println!("❌ {}: {} (OURS) vs {} (SEC)", symbol, our_cik, sec_cik);
                    corrections.push((symbol, sec_cik.clone()));
                }
            } else {
                not_found_count += 1;
                println!("⚠️  {}: {} (NOT FOUND IN SEC)", symbol, our_cik);
            }
        }
        
        println!("\n📈 Summary for {}:", table_name);
        println!("✅ Correct: {}", correct_count);
        println!("❌ Incorrect: {}", incorrect_count);
        println!("⚠️  Not found: {}", not_found_count);
        
        if !corrections.is_empty() {
            println!("🔧 {} corrections needed", corrections.len());
        }
    }
    
    Ok(())
}
