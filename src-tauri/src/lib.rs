// Stock Analysis System - Tauri Backend
pub mod commands;
pub mod models;
pub mod api;
pub mod database;
pub mod database_sqlx;
pub mod tools;
pub mod analysis;

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
            commands::analysis::get_price_history,
            commands::analysis::get_valuation_ratios,
            commands::analysis::get_ps_evs_history,
            commands::analysis::get_undervalued_stocks_by_ps,
            
            // Recommendation commands
            recommendations::get_value_recommendations_with_stats,
            recommendations::get_value_recommendations,
            recommendations::analyze_sp500_pe_values,
            recommendations::get_recommendation_stats,
            recommendations::analyze_stock_pe_history,
            
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