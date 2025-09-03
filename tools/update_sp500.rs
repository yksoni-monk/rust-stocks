use anyhow::Result;
use chrono::{NaiveDate, Utc};
use rust_stocks::database_sqlx::DatabaseManagerSqlx;
use rust_stocks::models::{Config, Stock, StockStatus};
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    info!("🔄 S&P 500 List Updater");

    // Load configuration and initialize database
    let config = Config::from_env()?;
    let database = DatabaseManagerSqlx::new(&config.database_path).await?;

    // Check when S&P 500 list was last updated
    let last_update = database.get_metadata("sp500_last_updated").await?;
    let should_update = match last_update {
        Some(date_str) => {
            let last_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?;
            let days_old = (Utc::now().date_naive() - last_date).num_days();
            info!("📅 S&P 500 list last updated: {} ({} days ago)", date_str, days_old);
            
            if days_old > 30 {
                info!("⚠️  List is {} days old, updating...", days_old);
                true
            } else {
                info!("✅ List is recent, no update needed");
                false
            }
        }
        None => {
            info!("📋 No previous S&P 500 update found, fetching initial list...");
            true
        }
    };

    if should_update {
        update_sp500_list(&database).await?;
    }

    // Show current stats
    let stats = database.get_stats().await?;
    let stock_count = stats.get("total_stocks").unwrap_or(&0);
    info!("📊 Database contains {} S&P 500 companies", stock_count);

    Ok(())
}

async fn update_sp500_list(database: &DatabaseManagerSqlx) -> Result<()> {
    info!("🌐 Fetching S&P 500 list from GitHub...");
    
    // Fetch CSV data from GitHub
    let url = "https://raw.githubusercontent.com/datasets/s-and-p-500-companies/main/data/constituents.csv";
    let response = reqwest::get(url).await?;
    let csv_text = response.text().await?;
    
    // Parse CSV
    let mut reader = csv::Reader::from_reader(csv_text.as_bytes());
    let mut companies = Vec::new();
    
    for result in reader.records() {
        let record = result?;
        if record.len() >= 2 {
            let symbol = record[0].trim().to_string();
            let name = record[1].trim().to_string();
            let sector = record.get(2).unwrap_or("").trim().to_string();
            
            companies.push(Stock {
                id: None,
                symbol,
                company_name: name,
                sector: if sector.is_empty() { None } else { Some(sector) },
                industry: None,
                market_cap: None,
                status: StockStatus::Active,
                first_trading_date: None,
                last_updated: Some(Utc::now()),
            });
        }
    }
    
    info!("✅ Parsed {} S&P 500 companies", companies.len());
    
    // Clear existing S&P 500 companies and insert new ones
    database.clear_stocks().await?;
    
    let mut inserted = 0;
    for company in companies {
        database.upsert_stock(&company).await?;
        inserted += 1;
    }
    
    // Update metadata with current date
    let today = Utc::now().date_naive().format("%Y-%m-%d").to_string();
    database.set_metadata("sp500_last_updated", &today).await?;
    
    info!("✅ Updated {} S&P 500 companies in database", inserted);
    info!("📅 Set last update date: {}", today);
    
    Ok(())
}