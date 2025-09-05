use anyhow::Result;
use rust_stocks::database_sqlx::DatabaseManagerSqlx;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize database
    let database = DatabaseManagerSqlx::new("stocks.db").await?;
    
    // Get a few stocks and test their stats
    let stocks = database.get_active_stocks().await?;
    
    println!("Testing first 5 stocks:");
    for stock in stocks.iter().take(5) {
        if let Some(stock_id) = stock.id {
            match database.get_stock_data_stats(stock_id).await {
                Ok(stats) => {
                    println!("  {} (ID: {}): {} data points, {:?} to {:?}", 
                            stock.symbol, stock_id, stats.data_points, 
                            stats.earliest_date, stats.latest_date);
                }
                Err(e) => {
                    println!("  {} (ID: {}): ERROR - {}", stock.symbol, stock_id, e);
                }
            }
        } else {
            println!("  {}: No ID", stock.symbol);
        }
    }
    
    Ok(())
}