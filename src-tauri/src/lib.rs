// Stock Analysis System - Tauri Backend
pub mod commands;
pub mod models;
pub mod api;
pub mod database;
pub mod tools;

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
            stocks::get_stocks_paginated,
            stocks::get_sp500_symbols,
            
            // Data collection commands
            data::get_database_stats,
            
            // Analysis commands
            analysis::get_price_history,
            analysis::export_data,
            analysis::get_stock_date_range,
            
            // Initialization commands
            initialization::get_initialization_status,
            initialization::check_database_schema
        ])
        .setup(|_app| {
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}