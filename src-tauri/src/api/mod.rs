use anyhow::Result;
use chrono::NaiveDate;
use std::time::Duration;

use crate::models::{SchwabQuote, SchwabPriceBar};

pub mod schwab_client;
pub mod alpha_vantage_client;
pub use schwab_client::SchwabClient;
pub use alpha_vantage_client::AlphaVantageClient;

/// Simple rate limiter for API requests
pub struct ApiRateLimiter {
    delay_ms: u64,
}

impl ApiRateLimiter {
    pub fn new(requests_per_minute: u32) -> Self {
        let delay_ms = if requests_per_minute > 0 {
            60_000 / requests_per_minute as u64
        } else {
            1000 // Default 1 second delay
        };
        
        Self { delay_ms }
    }

    pub async fn wait(&self) {
        tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
    }
}

/// Common traits for API clients
#[async_trait::async_trait]
pub trait StockDataProvider {
    #[allow(dead_code)]
    async fn get_quotes(&self, symbols: &[String]) -> Result<Vec<SchwabQuote>>;
    async fn get_price_history(
        &self,
        symbol: &str,
        from_date: NaiveDate,
        to_date: NaiveDate,
    ) -> Result<Vec<SchwabPriceBar>>;
}

