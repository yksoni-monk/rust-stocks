// Stock Analysis System - Tauri Backend
pub mod commands;
pub mod models;
pub mod api;

use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            // Stock commands
            stocks::get_all_stocks,
            stocks::search_stocks,
            stocks::get_stocks_with_data_status,
            
            // Data collection commands
            data::get_database_stats,
            data::fetch_stock_data,
            
            // Analysis commands
            analysis::get_price_history,
            analysis::export_data,
            
            // Data fetching commands
            fetching::get_available_stock_symbols,
            fetching::fetch_single_stock_data,
            fetching::fetch_all_stocks_concurrent,
            fetching::get_fetch_progress,
            
            // Initialization commands
            initialization::initialize_sp500_stocks,
            initialization::get_initialization_status,
            initialization::check_database_schema,
            
            // Earnings commands
            earnings::test_alpha_vantage_earnings,
            earnings::test_alpha_vantage_daily,
            earnings::calculate_daily_pe_ratio
        ])
        .setup(|_app| {
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}