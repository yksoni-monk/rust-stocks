use chrono::{NaiveDate, Weekday, Datelike};

/// Market calendar utilities for handling trading days
pub struct MarketCalendar;

impl MarketCalendar {
    /// Check if a date is a weekend
    pub fn is_weekend(date: NaiveDate) -> bool {
        matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
    }

    /// Adjust a date for weekends (Saturday/Sunday → Friday)
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
