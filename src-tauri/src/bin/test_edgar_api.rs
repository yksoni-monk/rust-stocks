use std::error::Error;
use reqwest::header;
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🧪 Testing EDGAR API direct access for AAPL");
    println!("===========================================");

    let cik = "0000320193";  // AAPL CIK
    let url = format!("https://data.sec.gov/api/xbrl/companyfacts/CIK{}.json", cik);
    println!("📡 Fetching from: {}", url);

    let mut headers = header::HeaderMap::new();
    headers.insert(header::USER_AGENT, "TestApp test@example.com".parse()?);

    let client = reqwest::Client::new();
    let res = client.get(&url).headers(headers).send().await?;

    println!("📥 Response status: {}", res.status());

    let json: Value = res.json().await?;
    println!("✅ JSON data received");

    // Test GrossProfit extraction
    if let Some(gross_profit) = json.get("facts")
        .and_then(|f| f.get("us-gaap"))
        .and_then(|g| g.get("GrossProfit"))
        .and_then(|gp| gp.get("units"))
        .and_then(|u| u.get("USD"))
        .and_then(|usd| usd.as_array()) {

        println!("✅ Found GrossProfit data!");

        let mut annuals: Vec<(i64, f64)> = gross_profit.iter()
            .filter(|f| f["fp"].as_str() == Some("FY"))
            .filter_map(|f| {
                let fy = f["fy"].as_i64()?;
                let val = f["val"].as_f64()?;
                Some((fy, val))
            })
            .collect();

        annuals.sort_by_key(|&(fy, _)| -fy);
        let last_5 = &annuals[0..5.min(annuals.len())];

        println!("📊 AAPL Gross Profit (Last 5 years):");
        for &(fy, val) in last_5 {
            println!("   Fiscal Year {}: Gross Profit ${:.0}", fy, val);
        }
    } else {
        println!("❌ Gross profit data not found in expected structure");

        // Let's explore what's actually available
        if let Some(us_gaap) = json.get("facts").and_then(|f| f.get("us-gaap")) {
            println!("🔍 Available US-GAAP fields:");
            if let Some(obj) = us_gaap.as_object() {
                let mut fields: Vec<String> = obj.keys().cloned().collect();
                fields.sort();
                for (i, field) in fields.iter().enumerate().take(20) {
                    println!("   {}. {}", i+1, field);
                }
                if fields.len() > 20 {
                    println!("   ... and {} more fields", fields.len() - 20);
                }
            }
        }
    }

    Ok(())
}