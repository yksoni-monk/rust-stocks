pub mod api;
pub mod analysis;
pub mod concurrent_fetcher;
pub mod data_collector;
pub mod models;
pub mod database_sqlx;
pub mod ui;
pub mod utils;
// Remaining modules temporarily disabled during SQLX migration
// pub mod database;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_utilities() {
        // Test that we can create a database manager directly
        let db_path = "tests/tmp/test_lib.db";
        let db_manager = database::DatabaseManager::new(db_path)
            .expect("Failed to create database manager");
        
        // Test that we can insert a stock
        let stock = models::Stock {
            id: None,
            symbol: "AAPL".to_string(),
            company_name: "Apple Inc.".to_string(),
            sector: Some("Technology".to_string()),
            industry: Some("Software".to_string()),
            market_cap: Some(1_000_000_000.0),
            status: models::StockStatus::Active,
            first_trading_date: Some(chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            last_updated: Some(chrono::Utc::now()),
        };
        
        let stock_id = db_manager.upsert_stock(&stock).expect("Failed to insert stock");
        assert!(stock_id > 0);
        
        // Test that we can retrieve the stock
        let stocks = db_manager.get_active_stocks().expect("Failed to get stocks");
        assert_eq!(stocks.len(), 1);
        assert_eq!(stocks[0].symbol, "AAPL");
        
        println!("✅ Database manager works correctly!");
        
        // Clean up
        std::fs::remove_file(db_path).ok();
    }
}
