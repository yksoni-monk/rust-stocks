//! Database test helpers

use anyhow::Result;
use rust_stocks::database::DatabaseManager;
use rust_stocks::models::{Stock, DailyPrice};
use super::{TestEnv, test_data};
use chrono::{NaiveDate, Utc};

/// Database test helper
pub struct DatabaseTestHelper {
    pub db: DatabaseManager,
    pub env: TestEnv,
}

impl DatabaseTestHelper {
    /// Create a new database test helper
    pub fn new() -> Result<Self> {
        let env = TestEnv::new()?;
        let db = DatabaseManager::new(env.db_path_str())?;
        
        Ok(Self { db, env })
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

        // Insert stocks
        for stock in &stocks {
            let stock_id = self.db.upsert_stock(stock)?;
            self.log_test_data("Inserted stock", &(stock.symbol.clone(), stock_id));
        }

        // Create some test price data
        let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let dates = test_data::create_test_date_range(start_date, 30);

        for (i, stock) in stocks.iter().enumerate() {
            let stock_id = (i + 1) as i64; // Assuming IDs are sequential starting from 1
            
            for (j, date) in dates.iter().enumerate() {
                let mut price = test_data::create_test_daily_price(stock_id, *date);
                price.close_price += (j as f64) * 0.1; // Vary the price slightly
                
                self.db.insert_daily_price(&price)?;
            }
            
            self.log_test_data("Inserted price data", &(stock.symbol.clone(), dates.len()));
        }

        Ok(stocks)
    }

    /// Clean up test database
    pub fn cleanup(&self) -> Result<()> {
        self.log_test_step("Cleaning up test database");
        
        // Clear all data
        self.db.clear_stocks()?;
        
        Ok(())
    }

    /// Get database statistics
    pub fn get_stats(&self) -> Result<(usize, usize, Option<NaiveDate>)> {
        self.db.get_stats()
    }

    /// Verify stock exists
    pub fn verify_stock_exists(&self, symbol: &str) -> Result<bool> {
        let stock = self.db.get_stock_by_symbol(symbol)?;
        Ok(stock.is_some())
    }

    /// Verify price data exists for date range
    pub fn verify_price_data_exists(&self, stock_id: i64, start_date: NaiveDate, end_date: NaiveDate) -> Result<usize> {
        self.db.count_existing_records(stock_id, start_date, end_date)
    }

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
    use super::*;

    /// Create a minimal test database
    pub fn create_minimal_test_db() -> Result<DatabaseTestHelper> {
        let helper = DatabaseTestHelper::new()?;
        helper.setup_with_sample_data()?;
        Ok(helper)
    }

    /// Create a test database with specific stocks
    pub fn create_test_db_with_stocks(symbols: &[&str]) -> Result<DatabaseTestHelper> {
        let helper = DatabaseTestHelper::new()?;
        
        for symbol in symbols {
            let stock = test_data::create_test_stock(symbol, &format!("Test Company {}", symbol));
            helper.db.upsert_stock(&stock)?;
        }
        
        Ok(helper)
    }

    /// Assert database state
    pub fn assert_db_state(helper: &DatabaseTestHelper, expected_stocks: usize, expected_prices: usize) -> Result<()> {
        let (stock_count, price_count, _) = helper.get_stats()?;
        
        crate::common::logging::log_assertion(
            &format!("Database has {} stocks (expected {})", stock_count, expected_stocks),
            stock_count == expected_stocks
        );
        
        crate::common::logging::log_assertion(
            &format!("Database has {} prices (expected {})", price_count, expected_prices),
            price_count == expected_prices
        );
        
        assert_eq!(stock_count, expected_stocks);
        assert_eq!(price_count, expected_prices);
        
        Ok(())
    }
}
