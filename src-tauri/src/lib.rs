// Stock Analysis System - Tauri Backend
pub mod commands;

use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            // Stock commands
            stocks::get_all_stocks,
            stocks::search_stocks,
            
            // Data collection commands
            data::get_database_stats,
            data::fetch_stock_data,
            
            // Analysis commands
            analysis::get_price_history,
            analysis::export_data
        ])
        .setup(|_app| {
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}