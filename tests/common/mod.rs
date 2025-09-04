//! Common test utilities and helpers

pub mod database;
pub mod api_mock;
pub mod fixtures;

use chrono::{NaiveDate, Utc};

// Re-export main database functions for convenience
pub use database::{
    init_fresh_test_database,
    init_fresh_test_database_with_cleanup,
    init_test_database,
    get_test_database,
    insert_sample_stocks,
    cleanup_test_database,
    cleanup_all_test_databases,
    cleanup_tmp_directory,
    TestDatabase,
};

/// Test data utilities
pub mod test_data {
    use super::*;
    use rust_stocks::models::{Stock, DailyPrice};

    /// Create a test stock
    pub fn create_test_stock(symbol: &str, company_name: &str) -> Stock {
        Stock {
            id: None,
            symbol: symbol.to_string(),
            company_name: company_name.to_string(),
            sector: Some("Technology".to_string()),
            industry: Some("Software".to_string()),
            market_cap: Some(1_000_000_000.0),
            status: rust_stocks::models::StockStatus::Active,
            first_trading_date: Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            last_updated: Some(Utc::now()),
        }
    }

    /// Create test daily price data
    pub fn create_test_daily_price(stock_id: i64, date: NaiveDate) -> DailyPrice {
        DailyPrice {
            id: None,
            stock_id,
            date,
            open_price: 100.0,
            high_price: 105.0,
            low_price: 95.0,
            close_price: 102.0,
            volume: Some(1_000_000),
            pe_ratio: Some(25.0),
            market_cap: Some(1_000_000_000.0),
            dividend_yield: Some(0.02),
        }
    }

    /// Create a range of test dates
    pub fn create_test_date_range(start_date: NaiveDate, days: i64) -> Vec<NaiveDate> {
        (0..days)
            .map(|i| start_date + chrono::Duration::days(i))
            .collect()
    }
}

/// Logging utilities for tests
pub mod logging {
    use tracing::{info, debug};
    use std::sync::Once;

    static INIT: Once = Once::new();

    /// Initialize test logging
    pub fn init_test_logging() {
        INIT.call_once(|| {
            // Only initialize if not already initialized
            if tracing::subscriber::set_global_default(
                tracing_subscriber::fmt()
                    .with_env_filter("rust_stocks=debug,test=debug")
                    .with_test_writer()
                    .finish()
            ).is_err() {
                // If already initialized, just continue
            }
        });
    }

    /// Log test step
    pub fn log_test_step(step: &str) {
        info!("🧪 Test Step: {}", step);
    }

    // Log assertion function removed to reduce warnings

    /// Log test data
    pub fn log_test_data<T: std::fmt::Debug>(label: &str, data: &T) {
        debug!("📊 {}: {:?}", label, data);
    }
}
