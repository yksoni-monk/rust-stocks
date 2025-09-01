use anyhow::Result;
use rust_stocks::database::DatabaseManager;
use rust_stocks::models::Config;

fn main() -> Result<()> {
    let config = Config::from_env()?;
    let database = DatabaseManager::new(&config.database_path)?;
    
    println!("=== Debug Stock Loading ===");
    
    // Test get_active_stocks
    match database.get_active_stocks() {
        Ok(stocks) => {
            println!("✅ Found {} active stocks", stocks.len());
            
            // Test get_stock_data_stats for first few stocks
            for (i, stock) in stocks.iter().take(5).enumerate() {
                if let Some(stock_id) = stock.id {
                    match database.get_stock_data_stats(stock_id) {
                        Ok(stats) => {
                            println!("✅ Stock {} ({}): {} data points, date range: {:?} to {:?}", 
                                   i + 1, stock.symbol, stats.data_points, stats.earliest_date, stats.latest_date);
                        }
                        Err(e) => {
                            println!("❌ Stock {} ({}): Error getting stats: {}", i + 1, stock.symbol, e);
                        }
                    }
                } else {
                    println!("❌ Stock {} ({}): No ID", i + 1, stock.symbol);
                }
            }
            
            // Count stocks with data
            let mut stocks_with_data = 0;
            for stock in &stocks {
                if let Some(stock_id) = stock.id {
                    if let Ok(stats) = database.get_stock_data_stats(stock_id) {
                        if stats.data_points > 0 {
                            stocks_with_data += 1;
                        }
                    }
                }
            }
            println!("📊 Total stocks with data: {}/{}", stocks_with_data, stocks.len());
        }
        Err(e) => {
            println!("❌ Error loading stocks: {}", e);
        }
    }
    
    Ok(())
}
