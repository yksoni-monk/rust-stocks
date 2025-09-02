//! Database test helpers

use anyhow::Result;
use rust_stocks::database::DatabaseManager;
use rust_stocks::models::Stock;
use super::{TestEnv, test_data};
use chrono::NaiveDate;

/// Database test helper
pub struct DatabaseTestHelper {
    pub db: DatabaseManager,
    _env: TestEnv, // Keep for cleanup
}

impl DatabaseTestHelper {
    /// Create a new database test helper
    pub fn new() -> Result<Self> {
        let env = TestEnv::new()?;
        let db = DatabaseManager::new(env.db_path_str())?;
        
        Ok(Self { db, _env: env })
    }

    /// Set up test database with sample data
    pub fn setup_with_sample_data(&self) -> Result<Vec<Stock>> {
        self.log_test_step("Setting up database with sample data");

        // Create test stocks
        let stocks = vec![
            test_data::create_test_stock("AAPL", "Apple Inc."),
            test_data::create_test_stock("MSFT", "Microsoft Corporation"),
            test_data::create_test_stock("GOOGL", "Alphabet Inc."),
        ];

        // Insert stocks and update their IDs
        let mut stocks_with_ids = Vec::new();
        for stock in stocks {
            let stock_id = self.db.upsert_stock(&stock)?;
            self.log_test_data("Inserted stock", &(stock.symbol.clone(), stock_id));
            
            // Create a new stock with the correct ID
            let mut stock_with_id = stock;
            stock_with_id.id = Some(stock_id);
            stocks_with_ids.push(stock_with_id);
        }

        // Create some test price data
        let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let dates = test_data::create_test_date_range(start_date, 30);

        for (_i, stock) in stocks_with_ids.iter().enumerate() {
            let stock_id = stock.id.unwrap(); // Use the actual ID from the database
            
            for (j, date) in dates.iter().enumerate() {
                let mut price = test_data::create_test_daily_price(stock_id, *date);
                price.close_price += (j as f64) * 0.1; // Vary the price slightly
                
                self.db.insert_daily_price(&price)?;
            }
            
            self.log_test_data("Inserted price data", &(stock.symbol.clone(), dates.len()));
        }

        Ok(stocks_with_ids)
    }

    /// Clean up test database
    pub fn cleanup(&self) -> Result<()> {
        self.log_test_step("Cleaning up test database");
        
        // Clear all data
        self.db.clear_stocks()?;
        
        Ok(())
    }

    // Helper methods removed to reduce warnings

    /// Log test step
    fn log_test_step(&self, step: &str) {
        super::logging::log_test_step(step);
    }

    /// Log test data
    fn log_test_data<T: std::fmt::Debug>(&self, label: &str, data: &T) {
        super::logging::log_test_data(label, data);
    }
}

impl Drop for DatabaseTestHelper {
    fn drop(&mut self) {
        // Cleanup happens automatically when temp_dir is dropped
        self.log_test_step("DatabaseTestHelper dropped - cleanup complete");
    }
}

/// Database test utilities
pub mod utils {
    // Placeholder for future utilities

    // Utility functions removed to reduce warnings
}
