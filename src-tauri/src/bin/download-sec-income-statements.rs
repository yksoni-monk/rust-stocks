use anyhow::Result;
use rust_stocks_tauri_lib::database::helpers::get_database_connection;
use rust_stocks_tauri_lib::tools::sec_edgar_client::SecEdgarClient;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 SEC EDGAR Income Statement Data Downloader");
    println!("=============================================");
    
    // Get database connection using environment variables
    let pool = get_database_connection().await
        .map_err(|e| anyhow::anyhow!("Database connection failed: {}", e))?;
    
    println!("✅ Connected to database");
    
    // Create SEC EDGAR client
    let mut client = SecEdgarClient::new(pool.clone());
    
    // Download income statement data for all S&P 500 companies
    client.download_all_sp500_income_statements().await?;
    
    println!("✅ Income statement data download completed successfully!");
    
    Ok(())
}
