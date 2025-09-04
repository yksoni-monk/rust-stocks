use chrono::{NaiveDate, Weekday, Datelike};

/// Market calendar utilities for handling trading days
#[allow(dead_code)]
pub struct MarketCalendar;

impl MarketCalendar {
    /// Check if a date is a weekend
    #[allow(dead_code)]
    pub fn is_weekend(date: NaiveDate) -> bool {
        matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
    }

    /// Adjust a date for weekends (Saturday/Sunday → Friday)
    #[allow(dead_code)]
    pub fn adjust_for_weekend(date: NaiveDate) -> NaiveDate {
        match date.weekday() {
            Weekday::Sat => date - chrono::Duration::days(1), // Saturday -> Friday
            Weekday::Sun => date - chrono::Duration::days(2), // Sunday -> Friday
            _ => date, // Weekdays stay the same
        }
    }

    /// Get the start of the trading week (Monday) for a given date
    pub fn get_week_start(date: NaiveDate) -> NaiveDate {
        let weekday = date.weekday();
        let days_to_monday = match weekday {
            Weekday::Mon => 0,
            Weekday::Tue => 1,
            Weekday::Wed => 2,
            Weekday::Thu => 3,
            Weekday::Fri => 4,
            Weekday::Sat => 5,
            Weekday::Sun => 6,
        };
        date - chrono::Duration::days(days_to_monday as i64)
    }

    /// Get the end of the trading week (Friday) for a given date
    pub fn get_week_end(date: NaiveDate) -> NaiveDate {
        let weekday = date.weekday();
        let days_to_friday = match weekday {
            Weekday::Mon => 4,
            Weekday::Tue => 3,
            Weekday::Wed => 2,
            Weekday::Thu => 1,
            Weekday::Fri => 0,
            Weekday::Sat => 6,
            Weekday::Sun => 5,
        };
        date + chrono::Duration::days(days_to_friday as i64)
    }
}

/// Trading week batch calculator for data collection
#[allow(dead_code)]
pub struct TradingWeekBatchCalculator;

impl TradingWeekBatchCalculator {
    /// Calculate trading week batches for a given date range
    pub fn calculate_batches(start_date: NaiveDate, end_date: NaiveDate) -> Vec<TradingWeekBatch> {
        let mut batches = Vec::new();
        let mut batch_number = 1;
        
        // Start with the first trading week that contains the start date
        let mut current_week_start = Self::get_week_start(start_date);
        
        while current_week_start <= end_date {
            // Find the end of the current trading week (Friday)
            let current_week_end = Self::get_week_end(current_week_start);
            
            // Adjust to user's requested range
            let batch_start = std::cmp::max(current_week_start, start_date);
            let batch_end = std::cmp::min(current_week_end, end_date);
            
            // Skip if batch is empty
            if batch_start > batch_end {
                // Move to next week
                current_week_start = current_week_end + chrono::Duration::days(1);
                continue;
            }

            let description = format!("Week {}: {} to {}", 
                batch_number, 
                batch_start.format("%Y-%m-%d"), 
                batch_end.format("%Y-%m-%d")
            );

            batches.push(TradingWeekBatch {
                batch_number,
                start_date: batch_start,
                end_date: batch_end,
                description,
            });

            // Move to next week (Monday of next week)
            current_week_start = current_week_end + chrono::Duration::days(1);
            batch_number += 1;
        }

        batches
    }

    /// Get the start of the trading week (Monday) for a given date
    pub fn get_week_start(date: NaiveDate) -> NaiveDate {
        let weekday = date.weekday();
        let days_to_monday = match weekday {
            chrono::Weekday::Mon => 0,
            chrono::Weekday::Tue => 1,
            chrono::Weekday::Wed => 2,
            chrono::Weekday::Thu => 3,
            chrono::Weekday::Fri => 4,
            chrono::Weekday::Sat => 5,
            chrono::Weekday::Sun => 6,
        };
        date - chrono::Duration::days(days_to_monday as i64)
    }

    /// Get the end of the trading week (Friday) for a given date
    pub fn get_week_end(date: NaiveDate) -> NaiveDate {
        let weekday = date.weekday();
        let days_to_friday = match weekday {
            chrono::Weekday::Mon => 4,
            chrono::Weekday::Tue => 3,
            chrono::Weekday::Wed => 2,
            chrono::Weekday::Thu => 1,
            chrono::Weekday::Fri => 0,
            chrono::Weekday::Sat => 6,
            chrono::Weekday::Sun => 5,
        };
        date + chrono::Duration::days(days_to_friday as i64)
    }
}

/// Represents a trading week batch for data collection
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TradingWeekBatch {
    pub batch_number: usize,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub description: String,
}
