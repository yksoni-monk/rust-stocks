use anyhow::Result;
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use crate::api::SchwabClient;

/// Market calendar utilities for handling weekends and holidays
pub struct MarketCalendar {
    client: SchwabClient,
}

impl MarketCalendar {
    pub fn new(client: SchwabClient) -> Self {
        Self { client }
    }

    /// Check if a date is a trading day using the Schwab API
    pub async fn is_trading_day(&self, date: NaiveDate) -> Result<bool> {
        // Check if date is more than 7 days ago (API limitation)
        let today = chrono::Utc::now().date_naive();
        let days_ago = today.signed_duration_since(date).num_days();
        
        if days_ago > 7 {
            // Fallback to weekend check for older dates
            return Ok(!matches!(date.weekday(), Weekday::Sat | Weekday::Sun));
        }

        let date_str = date.format("%Y-%m-%d").to_string();
        match self.client.get_market_hours_for_date("equity", &date_str).await {
            Ok(response) => {
                // Parse the response to check if market is open
                if let Some(equity_root) = response.get("equity") {
                    if let Some(equity_obj) = equity_root.as_object() {
                        for (_, market_data) in equity_obj {
                            if let Some(is_open) = market_data.get("isOpen") {
                                return Ok(is_open.as_bool().unwrap_or(false));
                            }
                        }
                    }
                }
                Ok(false)
            }
            Err(_) => {
                // Fallback to weekend check if API fails
                Ok(!matches!(date.weekday(), Weekday::Sat | Weekday::Sun))
            }
        }
    }

    /// Get the most recent trading day on or before the given date
    pub async fn get_last_trading_day(&self, date: NaiveDate) -> Result<NaiveDate> {
        let mut current_date = date;
        
        // Look back up to 10 days to find a trading day
        for _ in 0..10 {
            if self.is_trading_day(current_date).await? {
                return Ok(current_date);
            }
            current_date = current_date - Duration::days(1);
        }
        
        // If we can't find a trading day in 10 days, something is very wrong
        Err(anyhow::anyhow!("Could not find a trading day within 10 days of {}", date))
    }

    /// Get the next trading day on or after the given date
    pub async fn get_next_trading_day(&self, date: NaiveDate) -> Result<NaiveDate> {
        let mut current_date = date;
        
        // Look ahead up to 10 days to find a trading day
        for _ in 0..10 {
            if self.is_trading_day(current_date).await? {
                return Ok(current_date);
            }
            current_date = current_date + Duration::days(1);
        }
        
        // If we can't find a trading day in 10 days, something is very wrong
        Err(anyhow::anyhow!("Could not find a trading day within 10 days of {}", date))
    }

    /// Adjust date range to trading days only
    pub async fn adjust_date_range(&self, start: NaiveDate, end: NaiveDate) -> Result<(NaiveDate, NaiveDate)> {
        // For single-day requests on weekends/holidays, get the last trading day before the date
        if start == end {
            if self.is_trading_day(start).await? {
                return Ok((start, end));
            } else {
                let trading_day = self.get_last_trading_day(start).await?;
                return Ok((trading_day, trading_day));
            }
        }
        
        // For date ranges, find the appropriate trading days
        let adjusted_start = self.get_next_trading_day(start).await?;
        let adjusted_end = self.get_last_trading_day(end).await?;
        
        if adjusted_start > adjusted_end {
            return Err(anyhow::anyhow!("No trading days found in range {} to {}", start, end));
        }
        
        Ok((adjusted_start, adjusted_end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_weekend_detection_fallback() {
        // Test Saturday
        let saturday = NaiveDate::from_ymd_opt(2025, 8, 30).unwrap(); // Saturday
        assert_eq!(saturday.weekday(), Weekday::Sat);
        
        // Test Sunday
        let sunday = NaiveDate::from_ymd_opt(2025, 8, 31).unwrap(); // Sunday
        assert_eq!(sunday.weekday(), Weekday::Sun);
        
        // Test Friday (trading day)
        let friday = NaiveDate::from_ymd_opt(2025, 8, 29).unwrap(); // Friday
        assert_eq!(friday.weekday(), Weekday::Fri);
    }
}