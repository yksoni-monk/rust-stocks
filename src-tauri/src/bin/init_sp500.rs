/// Initialize S&P 500 stock list
///
/// Downloads the S&P 500 company list from GitHub and populates the stocks table

use anyhow::Result;
use sqlx::sqlite::SqlitePoolOptions;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct StockRecord {
    #[serde(rename = "Symbol")]
    symbol: String,
    #[serde(rename = "Security")]
    company_name: String,
    #[serde(rename = "GICS Sector")]
    sector: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔄 Initializing S&P 500 Stock List");
    println!("══════════════════════════════════");
    println!();

    // Database connection
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:db/stocks.db".to_string());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Step 1: Fetch S&P 500 list from GitHub
    println!("📥 Downloading S&P 500 list from GitHub...");
    let url = "https://raw.githubusercontent.com/datasets/s-and-p-500-companies/main/data/constituents.csv";

    let response = reqwest::get(url).await?;
    let csv_text = response.text().await?;

    // Step 2: Parse CSV
    println!("📄 Parsing CSV data...");
    let mut reader = csv::Reader::from_reader(csv_text.as_bytes());
    let mut companies = Vec::new();

    for result in reader.deserialize() {
        let record: StockRecord = result?;
        companies.push(record);
    }

    println!("   ✅ Found {} companies", companies.len());

    if companies.is_empty() {
        anyhow::bail!("No companies found in S&P 500 data");
    }

    // Step 3: Insert into stocks table
    println!("💾 Inserting stocks into database...");
    let mut inserted = 0;
    let mut updated = 0;

    for company in &companies {
        let sector = if company.sector.is_empty() {
            None
        } else {
            Some(&company.sector)
        };

        // Use INSERT OR REPLACE to handle existing stocks
        let result = sqlx::query(
            "INSERT INTO stocks (symbol, company_name, sector, is_sp500)
             VALUES (?1, ?2, ?3, 1)
             ON CONFLICT(symbol) DO UPDATE SET
                company_name = ?2,
                sector = ?3,
                is_sp500 = 1"
        )
        .bind(&company.symbol)
        .bind(&company.company_name)
        .bind(sector)
        .execute(&pool)
        .await;

        match result {
            Ok(query_result) => {
                if query_result.rows_affected() > 0 {
                    inserted += 1;
                } else {
                    updated += 1;
                }
            }
            Err(e) => eprintln!("   ⚠️  Failed to insert {}: {}", company.symbol, e),
        }
    }

    println!("   ✅ Inserted: {}, Updated: {}", inserted, updated);

    // Step 4: Update metadata
    let current_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    sqlx::query("INSERT OR REPLACE INTO metadata (key, value) VALUES ('sp500_last_updated', ?1)")
        .bind(&current_date)
        .execute(&pool)
        .await?;

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Successfully initialized {} S&P 500 companies!", inserted + updated);
    println!("📅 Last updated: {}", current_date);
    println!();
    println!("💡 Next steps:");
    println!("   1. Run: cargo run --bin fetch_ciks");
    println!("   2. Run: cargo run --bin refresh_data -- all");

    Ok(())
}
