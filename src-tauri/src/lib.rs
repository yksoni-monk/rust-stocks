// Stock Analysis System - Tauri Backend
pub mod commands;
pub mod models;
pub mod api;
pub mod database;
pub mod database_sqlx;
pub mod tools;
pub mod analysis;

use commands::*;
use tauri::WindowEvent;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // .plugin(tauri_plugin_log::Builder::default().build())  // Temporarily disabled due to initialization error
        .on_window_event(|_window, event| match event {
            WindowEvent::CloseRequested { .. } => {
                println!("🔄 Window close requested - cleaning up orphaned processes...");
                
                #[cfg(not(target_os = "windows"))]
                {
                    use std::process::Command;
                    use std::env;
                    
                    // Get the current working directory (project root)
                    if let Ok(current_dir) = env::current_dir() {
                        let project_path = current_dir.to_string_lossy();
                        
                        // Kill orphaned vite processes (not the main one managed by Tauri)
                        // Target processes that are NOT the main dev server
                        let _ = Command::new("pkill")
                            .args(["-f", "vite.*--port.*5174"])
                            .output();
                        
                        // Kill orphaned node processes in our project directory
                        // but exclude the main npm run dev process that Tauri manages
                        let _ = Command::new("pkill")
                            .args(["-f", &format!("{}/src.*node.*vite", project_path)])
                            .output();
                        
                        // Kill any orphaned esbuild processes
                        let _ = Command::new("pkill")
                            .args(["-f", "esbuild"])
                            .output();
                        
                        println!("✅ Orphaned development processes cleaned up");
                    } else {
                        // Fallback: kill orphaned processes only
                        let _ = Command::new("pkill")
                            .args(["-f", "vite.*--port.*5174"])
                            .output();
                        
                        let _ = Command::new("pkill")
                            .args(["-f", "esbuild"])
                            .output();
                        
                        println!("✅ Orphaned development processes cleaned up (fallback)");
                    }
                }
                
                #[cfg(target_os = "windows")]
                {
                    use std::process::Command;
                    
                    // Windows-specific cleanup for orphaned processes only
                    let _ = Command::new("taskkill")
                        .args(["/F", "/IM", "node.exe", "/FI", "WINDOWTITLE ne npm*"])
                        .output();
                    
                    println!("✅ Orphaned development processes cleaned up (Windows)");
                }
            }
            _ => {}
        })
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
            commands::analysis::get_stock_date_range,
            commands::analysis::get_valuation_ratios,
            commands::analysis::get_ps_evs_history,
            commands::analysis::get_undervalued_stocks_by_ps,
            commands::analysis::get_ps_screening_with_revenue_growth,
            commands::analysis::get_valuation_extremes,
            
            // Recommendation commands
            recommendations::get_value_recommendations_with_stats,
            recommendations::get_value_recommendations,
            recommendations::analyze_sp500_pe_values,
            recommendations::get_recommendation_stats,
            recommendations::analyze_stock_pe_history,
            
            // Initialization commands
            initialization::get_initialization_status,
            initialization::check_database_schema,
            
            // GARP P/E screening commands
            garp_pe::get_garp_pe_screening_results,
            
            // Graham value screening commands
            graham_screening::run_graham_screening,
            graham_screening::get_graham_criteria_defaults,
            graham_screening::get_graham_screening_presets,
            graham_screening::get_graham_screening_preset,
            graham_screening::save_graham_screening_preset,
            graham_screening::delete_graham_screening_preset,
            graham_screening::get_graham_screening_stats,
            graham_screening::get_graham_stock_history,
            graham_screening::get_latest_graham_results,
            graham_screening::get_graham_sector_summary,

            // Data refresh commands
            data_refresh::get_data_freshness_status,
            data_refresh::check_screening_readiness,
            data_refresh::start_data_refresh,
            data_refresh::get_refresh_progress,
            data_refresh::get_last_refresh_result,
            data_refresh::cancel_refresh_operation,
            data_refresh::get_refresh_duration_estimates
        ])
        .setup(|_app| {
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}