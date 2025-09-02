//! Trading week batch calculation tests

use test_log::test;
use pretty_assertions::assert_eq;
use chrono::{NaiveDate, Weekday};
use rust_stocks::ui::data_collection::TradingWeekBatchCalculator;

#[test]
fn test_trading_week_batch_calculation_basic() {
    // Test case: Aug 6 (Wed) to Aug 19 (Tue), 2025
    let start_date = NaiveDate::from_ymd_opt(2025, 8, 6).unwrap(); // Wednesday
    let end_date = NaiveDate::from_ymd_opt(2025, 8, 19).unwrap();   // Tuesday
    
    let batches = TradingWeekBatchCalculator::calculate_batches(start_date, end_date);
    
    // Should create 3 batches:
    // Week 1: Aug 4 (Mon) to Aug 8 (Fri)
    // Week 2: Aug 11 (Mon) to Aug 15 (Fri)  
    // Week 3: Aug 18 (Mon) to Aug 22 (Fri)
    
    assert_eq!(batches.len(), 3);
    

    
    // Week 1: Aug 6-8 (respects user's start date)
    assert_eq!(batches[0].batch_number, 1);
    assert_eq!(batches[0].start_date, NaiveDate::from_ymd_opt(2025, 8, 6).unwrap());
    assert_eq!(batches[0].end_date, NaiveDate::from_ymd_opt(2025, 8, 8).unwrap());
    assert_eq!(batches[0].description, "Week 1: 2025-08-06 to 2025-08-08");
    
    // Week 2: Aug 9-15 (next trading week)
    assert_eq!(batches[1].batch_number, 2);
    assert_eq!(batches[1].start_date, NaiveDate::from_ymd_opt(2025, 8, 9).unwrap());
    assert_eq!(batches[1].end_date, NaiveDate::from_ymd_opt(2025, 8, 15).unwrap());
    assert_eq!(batches[1].description, "Week 2: 2025-08-09 to 2025-08-15");
    
    // Week 3: Aug 16-19 (next trading week)
    assert_eq!(batches[2].batch_number, 3);
    assert_eq!(batches[2].start_date, NaiveDate::from_ymd_opt(2025, 8, 16).unwrap());
    assert_eq!(batches[2].end_date, NaiveDate::from_ymd_opt(2025, 8, 19).unwrap()); // Respects user's end date
    assert_eq!(batches[2].description, "Week 3: 2025-08-16 to 2025-08-19");
}

#[test]
fn test_trading_week_batch_calculation_single_week() {
    // Test case: Aug 6 (Wed) to Aug 8 (Fri), 2025 - single week
    let start_date = NaiveDate::from_ymd_opt(2025, 8, 6).unwrap(); // Wednesday
    let end_date = NaiveDate::from_ymd_opt(2025, 8, 8).unwrap();   // Friday
    
    let batches = TradingWeekBatchCalculator::calculate_batches(start_date, end_date);
    
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].batch_number, 1);
    assert_eq!(batches[0].start_date, NaiveDate::from_ymd_opt(2025, 8, 6).unwrap()); // Respects user's start date
    assert_eq!(batches[0].end_date, NaiveDate::from_ymd_opt(2025, 8, 8).unwrap());   // Friday
}

#[test]
fn test_trading_week_batch_calculation_weekend_start() {
    // Test case: Aug 9 (Sat) to Aug 15 (Fri), 2025 - starts on weekend
    let start_date = NaiveDate::from_ymd_opt(2025, 8, 9).unwrap(); // Saturday
    let end_date = NaiveDate::from_ymd_opt(2025, 8, 15).unwrap();   // Friday
    
    let batches = TradingWeekBatchCalculator::calculate_batches(start_date, end_date);
    

    
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].batch_number, 1);
    assert_eq!(batches[0].start_date, NaiveDate::from_ymd_opt(2025, 8, 9).unwrap()); // Respects user's start date
    assert_eq!(batches[0].end_date, NaiveDate::from_ymd_opt(2025, 8, 15).unwrap());   // Friday
}

#[test]
fn test_trading_week_batch_calculation_weekend_end() {
    // Test case: Aug 11 (Mon) to Aug 16 (Sat), 2025 - ends on weekend
    let start_date = NaiveDate::from_ymd_opt(2025, 8, 11).unwrap(); // Monday
    let end_date = NaiveDate::from_ymd_opt(2025, 8, 16).unwrap();   // Saturday
    
    let batches = TradingWeekBatchCalculator::calculate_batches(start_date, end_date);
    
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0].batch_number, 1);
    assert_eq!(batches[0].start_date, NaiveDate::from_ymd_opt(2025, 8, 11).unwrap()); // Monday
    assert_eq!(batches[0].end_date, NaiveDate::from_ymd_opt(2025, 8, 15).unwrap());   // Friday (last trading day)
}

#[test]
fn test_trading_week_batch_calculation_multiple_months() {
    // Test case: Dec 30 (Mon) to Jan 10 (Fri), 2025 - spans multiple months
    let start_date = NaiveDate::from_ymd_opt(2024, 12, 30).unwrap(); // Monday
    let end_date = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();    // Friday
    
    let batches = TradingWeekBatchCalculator::calculate_batches(start_date, end_date);
    
    assert_eq!(batches.len(), 2);
    

    
    // Week 1: Dec 30 to Jan 3
    assert_eq!(batches[0].batch_number, 1);
    assert_eq!(batches[0].start_date, NaiveDate::from_ymd_opt(2024, 12, 30).unwrap());
    assert_eq!(batches[0].end_date, NaiveDate::from_ymd_opt(2025, 1, 3).unwrap());
    
    // Week 2: Jan 4 to Jan 10 (next trading week)
    assert_eq!(batches[1].batch_number, 2);
    assert_eq!(batches[1].start_date, NaiveDate::from_ymd_opt(2025, 1, 4).unwrap());
    assert_eq!(batches[1].end_date, NaiveDate::from_ymd_opt(2025, 1, 10).unwrap());
}

#[test]
fn test_trading_week_batch_calculation_single_day() {
    // Test case: Aug 6 (Wed) to Aug 6 (Wed), 2025 - single day
    let start_date = NaiveDate::from_ymd_opt(2025, 8, 6).unwrap(); // Wednesday
    let end_date = NaiveDate::from_ymd_opt(2025, 8, 6).unwrap();   // Wednesday
    
    let batches = TradingWeekBatchCalculator::calculate_batches(start_date, end_date);
    
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].batch_number, 1);
    assert_eq!(batches[0].start_date, NaiveDate::from_ymd_opt(2025, 8, 6).unwrap()); // Respects user's start date
    assert_eq!(batches[0].end_date, NaiveDate::from_ymd_opt(2025, 8, 6).unwrap());   // Respects user's end date
}

#[test]
fn test_trading_week_batch_calculation_edge_cases() {
    // Test various edge cases
    
    // Case 1: Monday to Monday (exactly one week)
    let start_date = NaiveDate::from_ymd_opt(2025, 8, 4).unwrap(); // Monday
    let end_date = NaiveDate::from_ymd_opt(2025, 8, 4).unwrap();  // Monday
    let batches = TradingWeekBatchCalculator::calculate_batches(start_date, end_date);
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].start_date, NaiveDate::from_ymd_opt(2025, 8, 4).unwrap());
    assert_eq!(batches[0].end_date, NaiveDate::from_ymd_opt(2025, 8, 4).unwrap());
    
    // Case 2: Friday to Friday (exactly one week)
    let start_date = NaiveDate::from_ymd_opt(2025, 8, 8).unwrap(); // Friday
    let end_date = NaiveDate::from_ymd_opt(2025, 8, 8).unwrap();   // Friday
    let batches = TradingWeekBatchCalculator::calculate_batches(start_date, end_date);
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].start_date, NaiveDate::from_ymd_opt(2025, 8, 8).unwrap());
    assert_eq!(batches[0].end_date, NaiveDate::from_ymd_opt(2025, 8, 8).unwrap());
}

#[test]
fn test_week_start_calculation() {
    // Test the get_week_start helper function
    // Monday should return same day
    let monday = NaiveDate::from_ymd_opt(2025, 8, 4).unwrap();
    assert_eq!(TradingWeekBatchCalculator::get_week_start(monday), monday);
    
    // Tuesday should return previous Monday
    let tuesday = NaiveDate::from_ymd_opt(2025, 8, 5).unwrap();
    assert_eq!(TradingWeekBatchCalculator::get_week_start(tuesday), monday);
    
    // Wednesday should return previous Monday
    let wednesday = NaiveDate::from_ymd_opt(2025, 8, 6).unwrap();
    assert_eq!(TradingWeekBatchCalculator::get_week_start(wednesday), monday);
    
    // Thursday should return previous Monday
    let thursday = NaiveDate::from_ymd_opt(2025, 8, 7).unwrap();
    assert_eq!(TradingWeekBatchCalculator::get_week_start(thursday), monday);
    
    // Friday should return previous Monday
    let friday = NaiveDate::from_ymd_opt(2025, 8, 8).unwrap();
    assert_eq!(TradingWeekBatchCalculator::get_week_start(friday), monday);
    
    // Saturday should return previous Monday
    let saturday = NaiveDate::from_ymd_opt(2025, 8, 9).unwrap();
    assert_eq!(TradingWeekBatchCalculator::get_week_start(saturday), monday);
    
    // Sunday should return previous Monday
    let sunday = NaiveDate::from_ymd_opt(2025, 8, 10).unwrap();
    assert_eq!(TradingWeekBatchCalculator::get_week_start(sunday), monday);
}

#[test]
fn test_week_end_calculation() {
    // Test the get_week_end helper function
    let friday = NaiveDate::from_ymd_opt(2025, 8, 8).unwrap();
    
    // Monday should return Friday
    let monday = NaiveDate::from_ymd_opt(2025, 8, 4).unwrap();
    assert_eq!(TradingWeekBatchCalculator::get_week_end(monday), friday);
    
    // Tuesday should return Friday
    let tuesday = NaiveDate::from_ymd_opt(2025, 8, 5).unwrap();
    assert_eq!(TradingWeekBatchCalculator::get_week_end(tuesday), friday);
    
    // Wednesday should return Friday
    let wednesday = NaiveDate::from_ymd_opt(2025, 8, 6).unwrap();
    assert_eq!(TradingWeekBatchCalculator::get_week_end(wednesday), friday);
    
    // Thursday should return Friday
    let thursday = NaiveDate::from_ymd_opt(2025, 8, 7).unwrap();
    assert_eq!(TradingWeekBatchCalculator::get_week_end(thursday), friday);
    
    // Friday should return same day
    assert_eq!(TradingWeekBatchCalculator::get_week_end(friday), friday);
    
    // Saturday should return next Friday
    let saturday = NaiveDate::from_ymd_opt(2025, 8, 9).unwrap();
    let next_friday = NaiveDate::from_ymd_opt(2025, 8, 15).unwrap();
    assert_eq!(TradingWeekBatchCalculator::get_week_end(saturday), next_friday);
    
    // Sunday should return next Friday
    let sunday = NaiveDate::from_ymd_opt(2025, 8, 10).unwrap();
    assert_eq!(TradingWeekBatchCalculator::get_week_end(sunday), next_friday);
}

#[test]
fn test_batch_ordering() {
    // Test that batches are properly ordered by date
    let start_date = NaiveDate::from_ymd_opt(2025, 8, 6).unwrap();
    let end_date = NaiveDate::from_ymd_opt(2025, 8, 19).unwrap();
    
    let batches = TradingWeekBatchCalculator::calculate_batches(start_date, end_date);
    
    // Verify batches are in chronological order
    for i in 1..batches.len() {
        assert!(batches[i-1].start_date <= batches[i].start_date);
        assert!(batches[i-1].end_date <= batches[i].end_date);
        assert_eq!(batches[i-1].batch_number + 1, batches[i].batch_number);
    }
}

#[test]
fn test_batch_descriptions() {
    // Test that batch descriptions are properly formatted
    let start_date = NaiveDate::from_ymd_opt(2025, 8, 6).unwrap();
    let end_date = NaiveDate::from_ymd_opt(2025, 8, 8).unwrap();
    
    let batches = TradingWeekBatchCalculator::calculate_batches(start_date, end_date);
    
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].description, "Week 1: 2025-08-06 to 2025-08-08");
    
    // Test multiple batches
    let start_date = NaiveDate::from_ymd_opt(2025, 8, 6).unwrap();
    let end_date = NaiveDate::from_ymd_opt(2025, 8, 19).unwrap();
    
    let batches = TradingWeekBatchCalculator::calculate_batches(start_date, end_date);
    
    assert_eq!(batches[0].description, "Week 1: 2025-08-06 to 2025-08-08");
    assert_eq!(batches[1].description, "Week 2: 2025-08-09 to 2025-08-15");
    assert_eq!(batches[2].description, "Week 3: 2025-08-16 to 2025-08-19");
}
