use anyhow::{Result, anyhow};
use chrono::{DateTime, NaiveDate, Utc};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tracing::{info, warn, error, debug};

use crate::models::{Config, SchwabQuote, SchwabPriceBar};

pub mod schwab_client;
pub use schwab_client::SchwabClient;

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
    async fn get_quotes(&self, symbols: &[String]) -> Result<Vec<SchwabQuote>>;
    async fn get_price_history(
        &self,
        symbol: &str,
        from_date: NaiveDate,
        to_date: NaiveDate,
    ) -> Result<Vec<SchwabPriceBar>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = ApiRateLimiter::new(60); // 60 requests per minute
        
        let start = std::time::Instant::now();
        
        // Should allow first request immediately
        limiter.wait().await;
        assert!(start.elapsed() < Duration::from_millis(100));
        
        // Should rate limit subsequent requests
        limiter.wait().await;
        // With 60 req/min, each request should wait ~1 second
        // But we'll be lenient in the test
        assert!(start.elapsed() >= Duration::from_millis(500));
    }
}