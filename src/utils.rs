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
    /// Calculate trading week batches between two dates
    pub fn calculate_batches(from_date: NaiveDate, to_date: NaiveDate) -> Vec<TradingWeekBatch> {
        let mut batches = Vec::new();
        let mut current_date = from_date;
        let mut batch_number = 1;

        while current_date <= to_date {
            let _week_start = MarketCalendar::get_week_start(current_date);
            let week_end = MarketCalendar::get_week_end(current_date);
            
            // Ensure we don't go beyond the to_date
            let batch_end = if week_end > to_date { to_date } else { week_end };
            
            batches.push(TradingWeekBatch {
                batch_number,
                start_date: current_date,
                end_date: batch_end,
            });
            
            // Move to next week
            current_date = week_end + chrono::Duration::days(1);
            batch_number += 1;
        }

        batches
    }
}

/// Represents a trading week batch for data collection
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TradingWeekBatch {
    pub batch_number: usize,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}
