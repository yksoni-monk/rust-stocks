use anyhow::Result;
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyMetadata {
    pub symbol: String,
    pub ipo_date: Option<NaiveDate>,
    pub listing_date: Option<NaiveDate>,
    pub earliest_data_date: Option<NaiveDate>,
    pub latest_data_date: Option<NaiveDate>,
    pub spinoff_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataGap {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub missing_days: i64,
}

pub struct DateRangeCalculator {
    // US Market holidays (simplified set for major holidays)
    market_holidays: HashSet<String>,
}

impl DateRangeCalculator {
    pub fn new() -> Self {
        let mut holidays = HashSet::new();

        // Add some major US market holidays for recent years
        // This is a simplified set - in production, you'd want a more comprehensive list
        let major_holidays = vec![
            // New Year's Day
            "2020-01-01", "2021-01-01", "2022-01-03", "2023-01-02", "2024-01-01", "2025-01-01",
            // Martin Luther King Jr. Day (3rd Monday in January)
            "2020-01-20", "2021-01-18", "2022-01-17", "2023-01-16", "2024-01-15", "2025-01-20",
            // Presidents Day (3rd Monday in February)
            "2020-02-17", "2021-02-15", "2022-02-21", "2023-02-20", "2024-02-19", "2025-02-17",
            // Good Friday
            "2020-04-10", "2021-04-02", "2022-04-15", "2023-04-07", "2024-03-29", "2025-04-18",
            // Memorial Day (last Monday in May)
            "2020-05-25", "2021-05-31", "2022-05-30", "2023-05-29", "2024-05-27", "2025-05-26",
            // Juneteenth (June 19, started 2021)
            "2021-06-19", "2022-06-20", "2023-06-19", "2024-06-19", "2025-06-19",
            // Independence Day
            "2020-07-03", "2021-07-05", "2022-07-04", "2023-07-04", "2024-07-04", "2025-07-04",
            // Labor Day (1st Monday in September)
            "2020-09-07", "2021-09-06", "2022-09-05", "2023-09-04", "2024-09-02", "2025-09-01",
            // Thanksgiving (4th Thursday in November)
            "2020-11-26", "2021-11-25", "2022-11-24", "2023-11-23", "2024-11-28", "2025-11-27",
            // Christmas
            "2020-12-25", "2021-12-24", "2022-12-26", "2023-12-25", "2024-12-25", "2025-12-25",
        ];

        for holiday in major_holidays {
            holidays.insert(holiday.to_string());
        }

        Self {
            market_holidays: holidays,
        }
    }

    /// Calculate optimal date range for a stock based on IPO/listing date and existing data
    pub fn calculate_optimal_range(
        &self,
        _symbol: &str,
        metadata: &CompanyMetadata,
        default_start: NaiveDate,
        end_date: NaiveDate,
    ) -> DateRange {
        // Determine the effective start date
        let mut start_date = default_start;

        // Use listing date if it's later than default start
        if let Some(listing) = metadata.listing_date {
            if listing > start_date {
                start_date = listing;
            }
        }

        // Use IPO date if it's later than current start
        if let Some(ipo) = metadata.ipo_date {
            if ipo > start_date {
                start_date = ipo;
            }
        }

        // For spinoffs, use spinoff date as absolute minimum
        if let Some(spinoff) = metadata.spinoff_date {
            if spinoff > start_date {
                start_date = spinoff;
            }
        }

        DateRange { start_date, end_date }
    }

    /// Calculate missing date ranges based on existing data
    pub fn calculate_missing_ranges(
        &self,
        conn: &Connection,
        stock_id: i64,
        desired_range: &DateRange,
    ) -> Result<Vec<DateRange>> {
        // Get existing data dates
        let mut stmt = conn.prepare(
            "SELECT DISTINCT date FROM daily_prices
             WHERE stock_id = ? AND date >= ? AND date <= ?
             ORDER BY date"
        )?;

        let existing_dates: Vec<NaiveDate> = stmt
            .query_map(params![stock_id, desired_range.start_date, desired_range.end_date], |row| {
                let date_str: String = row.get(0)?;
                NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                    .map_err(|_e| rusqlite::Error::InvalidColumnType(0, "date".to_string(), rusqlite::types::Type::Text))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // Generate expected trading days
        let expected_dates = self.generate_trading_days(desired_range.start_date, desired_range.end_date);

        // Find missing dates
        let existing_set: HashSet<NaiveDate> = existing_dates.into_iter().collect();
        let missing_dates: Vec<NaiveDate> = expected_dates
            .into_iter()
            .filter(|date| !existing_set.contains(date))
            .collect();

        // Group consecutive missing dates into ranges
        let missing_ranges = self.group_consecutive_dates(missing_dates);

        Ok(missing_ranges)
    }

    /// Generate list of expected trading days (excludes weekends and holidays)
    pub fn generate_trading_days(&self, start: NaiveDate, end: NaiveDate) -> Vec<NaiveDate> {
        let mut trading_days = Vec::new();
        let mut current = start;

        while current <= end {
            if self.is_trading_day(current) {
                trading_days.push(current);
            }
            current = current + Duration::days(1);
        }

        trading_days
    }

    /// Check if a given date is a trading day (not weekend or holiday)
    pub fn is_trading_day(&self, date: NaiveDate) -> bool {
        // Skip weekends
        match date.weekday() {
            Weekday::Sat | Weekday::Sun => return false,
            _ => {}
        }

        // Check if it's a market holiday
        let date_str = date.format("%Y-%m-%d").to_string();
        !self.market_holidays.contains(&date_str)
    }

    /// Group consecutive dates into ranges
    fn group_consecutive_dates(&self, dates: Vec<NaiveDate>) -> Vec<DateRange> {
        if dates.is_empty() {
            return Vec::new();
        }

        let mut sorted_dates = dates;
        sorted_dates.sort();

        let mut ranges = Vec::new();
        let mut range_start = sorted_dates[0];
        let mut range_end = sorted_dates[0];

        for (_i, &date) in sorted_dates.iter().enumerate().skip(1) {
            if date == range_end + Duration::days(1) ||
               (date > range_end && self.days_between_trading_days(range_end, date) <= 1) {
                range_end = date;
            } else {
                ranges.push(DateRange {
                    start_date: range_start,
                    end_date: range_end,
                });
                range_start = date;
                range_end = date;
            }
        }

        ranges.push(DateRange {
            start_date: range_start,
            end_date: range_end,
        });

        ranges
    }

    /// Calculate trading days between two dates
    fn days_between_trading_days(&self, start: NaiveDate, end: NaiveDate) -> i64 {
        let trading_days = self.generate_trading_days(start, end);
        trading_days.len() as i64
    }

    /// Fetch company metadata from database using v_price_data_coverage view
    pub fn get_company_metadata(&self, conn: &Connection, symbol: &str) -> Result<Option<CompanyMetadata>> {
        let mut stmt = conn.prepare(
            "SELECT symbol, earliest_data_date, latest_data_date
             FROM v_price_data_coverage
             WHERE symbol = ?"
        )?;

        let metadata = stmt.query_row(params![symbol], |row| {
            let parse_date = |date_str: Option<String>| -> Result<Option<NaiveDate>, rusqlite::Error> {
                match date_str {
                    Some(s) => NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                        .map(Some)
                        .map_err(|_| rusqlite::Error::InvalidColumnType(0, "date".to_string(), rusqlite::types::Type::Text)),
                    None => Ok(None),
                }
            };

            Ok(CompanyMetadata {
                symbol: row.get(0)?,
                ipo_date: None, // No longer available after removing company_metadata table
                listing_date: None, // No longer available after removing company_metadata table
                earliest_data_date: parse_date(row.get(1)?)?,
                latest_data_date: parse_date(row.get(2)?)?,
                spinoff_date: None, // No longer available after removing company_metadata table
            })
        });

        match metadata {
            Ok(meta) => Ok(Some(meta)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Calculate comprehensive update plan for a symbol
    pub fn calculate_update_plan(
        &self,
        conn: &Connection,
        symbol: &str,
        stock_id: i64,
        default_start: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<UpdatePlan> {
        // Get company metadata
        let metadata = self.get_company_metadata(conn, symbol)?
            .unwrap_or_else(|| CompanyMetadata {
                symbol: symbol.to_string(),
                ipo_date: None,
                listing_date: None,
                earliest_data_date: None,
                latest_data_date: None,
                spinoff_date: None,
            });

        // Calculate optimal range
        let optimal_range = self.calculate_optimal_range(symbol, &metadata, default_start, end_date);

        // Calculate missing ranges
        let missing_ranges = self.calculate_missing_ranges(conn, stock_id, &optimal_range)?;

        // Calculate total trading days
        let total_expected_days = self.generate_trading_days(optimal_range.start_date, optimal_range.end_date).len();
        let missing_days: usize = missing_ranges.iter()
            .map(|range| self.generate_trading_days(range.start_date, range.end_date).len())
            .sum();

        Ok(UpdatePlan {
            symbol: symbol.to_string(),
            stock_id,
            metadata,
            optimal_range,
            missing_ranges,
            total_expected_days,
            missing_days,
            coverage_percentage: if total_expected_days > 0 {
                ((total_expected_days - missing_days) as f64 / total_expected_days as f64) * 100.0
            } else {
                0.0
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePlan {
    pub symbol: String,
    pub stock_id: i64,
    pub metadata: CompanyMetadata,
    pub optimal_range: DateRange,
    pub missing_ranges: Vec<DateRange>,
    pub total_expected_days: usize,
    pub missing_days: usize,
    pub coverage_percentage: f64,
}

impl Default for DateRangeCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_trading_day() {
        let calc = DateRangeCalculator::new();

        // Test weekdays (should be trading days if not holidays)
        let monday = NaiveDate::from_ymd_opt(2024, 9, 16).unwrap(); // Monday
        assert!(calc.is_trading_day(monday));

        // Test weekends (should not be trading days)
        let saturday = NaiveDate::from_ymd_opt(2024, 9, 14).unwrap(); // Saturday
        let sunday = NaiveDate::from_ymd_opt(2024, 9, 15).unwrap(); // Sunday
        assert!(!calc.is_trading_day(saturday));
        assert!(!calc.is_trading_day(sunday));

        // Test known holiday
        let new_years = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        assert!(!calc.is_trading_day(new_years));
    }

    #[test]
    fn test_calculate_optimal_range() {
        let calc = DateRangeCalculator::new();
        let default_start = NaiveDate::from_ymd_opt(2015, 1, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();

        // Test with IPO date after default start
        let metadata = CompanyMetadata {
            symbol: "COIN".to_string(),
            ipo_date: Some(NaiveDate::from_ymd_opt(2021, 4, 14).unwrap()),
            listing_date: Some(NaiveDate::from_ymd_opt(2021, 4, 14).unwrap()),
            earliest_data_date: None,
            latest_data_date: None,
            spinoff_date: None,
        };

        let range = calc.calculate_optimal_range("COIN", &metadata, default_start, end_date);
        assert_eq!(range.start_date, NaiveDate::from_ymd_opt(2021, 4, 14).unwrap());
        assert_eq!(range.end_date, end_date);
    }

    #[test]
    fn test_generate_trading_days() {
        let calc = DateRangeCalculator::new();

        // Test a week that includes a weekend
        let start = NaiveDate::from_ymd_opt(2024, 9, 13).unwrap(); // Friday
        let end = NaiveDate::from_ymd_opt(2024, 9, 17).unwrap(); // Tuesday

        let trading_days = calc.generate_trading_days(start, end);

        // Should include Friday, Monday, Tuesday (3 days), excluding Sat/Sun
        assert_eq!(trading_days.len(), 3);
        assert_eq!(trading_days[0], NaiveDate::from_ymd_opt(2024, 9, 13).unwrap()); // Friday
        assert_eq!(trading_days[1], NaiveDate::from_ymd_opt(2024, 9, 16).unwrap()); // Monday
        assert_eq!(trading_days[2], NaiveDate::from_ymd_opt(2024, 9, 17).unwrap()); // Tuesday
    }
}