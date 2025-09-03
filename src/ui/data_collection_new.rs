use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use std::process::Command;
use std::io::Write;
use std::sync::Arc;
use std::fs::OpenOptions;
use tokio::sync::broadcast;

use crate::ui::{
    View, ViewLayout,
    state::{AsyncStateManager, LogLevel, StateUpdate},
};
use crate::database_sqlx::DatabaseManagerSqlx;

/// Debug logging function
fn debug_log(message: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("debug_tui.log") 
    {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(file, "[{}] {}", timestamp, message);
    }
}

/// Data collection action definition
#[derive(Debug, Clone)]
pub struct DataCollectionAction {
    pub id: String,
    pub title: String,
    pub description: String,
    pub requires_confirmation: bool,
}

/// Confirmation dialog state
#[derive(Debug, Clone)]
pub struct ConfirmationState {
    pub action_title: String,
    pub selected_option: bool, // true = yes, false = no
}

/// Stock selection state
#[derive(Debug, Clone)]
pub struct StockSelectionState {
    pub available_stocks: Vec<String>,
    pub selected_index: usize,
    pub search_query: String,
    pub is_searching: bool,
}

/// Date selection state
#[derive(Debug, Clone)]
pub struct DateSelectionState {
    pub selected_stock: String,
    pub start_date_input: String,
    pub end_date_input: String,
    pub selected_field: DateField, // start_date or end_date
    pub cursor_position: usize, // cursor position within the current field
}

/// Date field being edited
#[derive(Debug, Clone)]
pub enum DateField {
    StartDate,
    EndDate,
}

/// Refactored DataCollectionView implementing the View trait
pub struct DataCollectionView {
    // View state
    pub selected_action: usize,
    pub actions: Vec<DataCollectionAction>,
    pub status: String,
    
    // Interactive states
    pub confirmation_state: Option<ConfirmationState>,
    pub stock_selection_state: Option<StockSelectionState>,
    pub date_selection_state: Option<DateSelectionState>,
    
    // Async state management
    pub state_manager: AsyncStateManager,
    
    // Database reference
    pub database: Option<Arc<DatabaseManagerSqlx>>,
    
    // Global broadcast sender for state updates
    pub global_broadcast_sender: Option<broadcast::Sender<StateUpdate>>,
    
    // Pending operations
    pub pending_log_message: Option<String>,
    pub pending_log_level: Option<LogLevel>,
}

impl DataCollectionView {
    pub fn new() -> Self {
        let actions = vec![
            DataCollectionAction {
                id: "single_stock".to_string(),
                title: "📈 Fetch Single Stock Data".to_string(),
                description: "Fetch data for a specific stock and date range".to_string(),
                requires_confirmation: false,
            },
            DataCollectionAction {
                id: "all_stocks".to_string(),
                title: "📊 Fetch All Stocks Data".to_string(),
                description: "Fetch data for all stocks in a given date range".to_string(),
                requires_confirmation: true,
            },
        ];

        Self {
            actions,
            selected_action: 0,
            status: "Ready".to_string(),
            confirmation_state: None,
            stock_selection_state: None,
            date_selection_state: None,
            state_manager: AsyncStateManager::new(), // This will be replaced by global one
            database: None,
            global_broadcast_sender: None,
            pending_log_message: None,
            pending_log_level: None,
        }
    }

    /// Set the global state manager
    pub fn set_state_manager(&mut self, state_manager: AsyncStateManager) {
        self.state_manager = state_manager;
    }

    /// Set the global broadcast sender
    pub fn set_global_broadcast_sender(&mut self, sender: broadcast::Sender<StateUpdate>) {
        self.global_broadcast_sender = Some(sender);
    }

    /// Set database reference
    pub fn set_database(&mut self, database: Arc<DatabaseManagerSqlx>) {
        self.database = Some(database);
    }

    /// Add a log message
    fn add_log_message(&mut self, level: LogLevel, message: &str) {
        self.state_manager.add_log_message(level, message);
    }

    /// Process pending log messages
    fn process_pending_logs(&mut self) {
        if let (Some(message), Some(level)) = (self.pending_log_message.take(), self.pending_log_level.take()) {
            self.add_log_message(level, &message);
        }
    }

    /// Get available stocks from database
    async fn get_available_stocks(&self) -> Vec<String> {
        // This method is deprecated - stocks are now loaded via async state updates
        vec![] // Empty list - will be populated from database
    }

    /// Start date selection for a selected stock
    fn start_date_selection(&mut self, stock: String) {
        let today = chrono::Utc::now().date_naive();
        let default_start = today - chrono::Duration::days(30);
        
        self.date_selection_state = Some(DateSelectionState {
            selected_stock: stock,
            start_date_input: default_start.format("%Y-%m-%d").to_string(),
            end_date_input: today.format("%Y-%m-%d").to_string(),
            selected_field: DateField::StartDate,
            cursor_position: 0,
        });
        
        self.stock_selection_state = None;
        self.add_log_message(LogLevel::Info, "Date selection started. Use ↑/↓ to navigate, ←/→ to edit, Enter to confirm");
    }

    /// Parse date input in YYYY-MM-DD format
    fn parse_date_input(&self, input: &str) -> Result<NaiveDate> {
        // Try YYYY-MM-DD format first
        if input.len() == 10 && input.contains('-') {
            let parts: Vec<&str> = input.split('-').collect();
            if parts.len() == 3 {
                let year: i32 = parts[0].parse()?;
                let month: u32 = parts[1].parse()?;
                let day: u32 = parts[2].parse()?;
                
                return NaiveDate::from_ymd_opt(year, month, day)
                    .ok_or_else(|| anyhow::anyhow!("Invalid date"));
            }
        }
        
        // Fallback to YYYYMMDD format
        if input.len() != 8 {
            return Err(anyhow::anyhow!("Date must be YYYY-MM-DD or YYYYMMDD format"));
        }
        
        let year: i32 = input[0..4].parse()?;
        let month: u32 = input[4..6].parse()?;
        let day: u32 = input[6..8].parse()?;
        
        NaiveDate::from_ymd_opt(year, month, day)
            .ok_or_else(|| anyhow::anyhow!("Invalid date"))
    }

    /// Execute the currently selected action
    fn execute_selected_action(&mut self) {
        if self.selected_action >= self.actions.len() {
            return;
        }

        let action_id = self.actions[self.selected_action].id.clone();
        
        // Check if confirmation is required
        if self.actions[self.selected_action].requires_confirmation {
            self.confirmation_state = Some(ConfirmationState {
                action_title: self.actions[self.selected_action].title.clone(),
                selected_option: true, // Default to "Yes"
            });
            self.add_log_message(LogLevel::Info, &format!("Confirmation required for: {}", self.actions[self.selected_action].title));
            return;
        }

        // Execute the action
        self.start_operation_by_type(&action_id);
    }

    /// Start an operation by action type
    fn start_operation_by_type(&mut self, action_id: &str) {
        match action_id {
            "single_stock" => {
                self.add_log_message(LogLevel::Info, "Starting stock and date selection...");
                self.start_stock_and_date_selection();
            }
            "all_stocks" => {
                self.add_log_message(LogLevel::Info, "Starting historical data collection...");
                self.run_historical_collection();
            }
            _ => {
                self.add_log_message(LogLevel::Error, &format!("Unknown action: {}", action_id));
            }
        }
    }

    /// Start stock and date selection process
    fn start_stock_and_date_selection(&mut self) {
        // Clone database reference to avoid borrow checker issues
        let database = if let Some(db) = &self.database {
            db.clone()
        } else {
            self.add_log_message(LogLevel::Error, "Database not available");
            return;
        };
        
        // Start with a loading state
        self.stock_selection_state = Some(StockSelectionState {
            available_stocks: vec!["Loading S&P500 stocks...".to_string()],
            selected_index: 0,
            search_query: String::new(),
            is_searching: false,
        });
        self.add_log_message(LogLevel::Info, "Loading S&P500 stocks from database...");
        
        // Start async operation to load S&P500 stocks
        let operation_id = "load_sp500_stocks".to_string();
        let _ = self.state_manager.start_operation(operation_id.clone(), "Loading S&P500 stocks".to_string(), true);
        
        // Spawn the actual work
        let mut state_manager = self.state_manager.clone();
        let operation_id_clone = operation_id.clone();
        let global_broadcast_sender = self.global_broadcast_sender.clone().expect("Global broadcast sender not set");
        tokio::spawn(async move {
            debug_log("Async task started - loading S&P500 stocks");
            match database.get_active_stocks().await {
                Ok(stocks) => {
                    let symbols: Vec<String> = stocks.into_iter().map(|s| s.symbol).collect();
                    debug_log(&format!("Loaded {} stocks from database: {:?}", symbols.len(), &symbols[0..5]));
                    let _ = state_manager.complete_operation(&operation_id_clone, Ok(format!("Loaded {} S&P500 stocks", symbols.len())));
                    let _ = state_manager.add_log_message(LogLevel::Success, &format!("Loaded {} S&P500 stocks", symbols.len()));
                    
                    // Send stock list update to UI via global broadcast channel
                    debug_log("Sending StockListUpdated state update");
                    let _ = global_broadcast_sender.send(StateUpdate::StockListUpdated { stocks: symbols });
                    debug_log("StockListUpdated state update sent");
                }
                Err(e) => {
                    debug_log(&format!("Failed to load stocks: {}", e));
                    let _ = state_manager.complete_operation(&operation_id_clone, Err(format!("Failed to load S&P500 stocks: {}", e)));
                }
            }
        });
    }

    /// Run historical data collection
    fn run_historical_collection(&mut self) {
        let start_date = chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let end_date = chrono::Utc::now().date_naive();
        
        self.add_log_message(LogLevel::Info, &format!("Starting historical data collection from {} to {}", start_date, end_date));
        
        // Start async operation
        let operation_id = "historical_collection".to_string();
        let _ = self.state_manager.start_operation(operation_id.clone(), "Historical Data Collection".to_string(), true);
        
        // Spawn the actual work
        let mut state_manager = self.state_manager.clone();
        tokio::spawn(async move {
            let start_str = start_date.format("%Y%m%d").to_string();
            let end_str = end_date.format("%Y%m%d").to_string();
            
            let output = Command::new("cargo")
                .args([
                    "run", "--bin", "collect_with_detailed_logs", "--", 
                    "-s", &start_str, "-e", &end_str
                ])
                .output();

            match output {
                Ok(output) => {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let _ = state_manager.complete_operation(&operation_id, Ok("Historical data collection completed successfully".to_string()));
                        let _ = state_manager.add_log_message(LogLevel::Success, &format!("Output: {}", stdout.trim()));
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let _ = state_manager.complete_operation(&operation_id, Err("Failed to collect historical data".to_string()));
                        let _ = state_manager.add_log_message(LogLevel::Error, &format!("Error: {}", stderr.trim()));
                    }
                }
                Err(e) => {
                    let _ = state_manager.complete_operation(&operation_id, Err(format!("Failed to execute command: {}", e)));
                }
            }
        });
    }

    /// Run single stock collection
    fn run_single_stock_collection(&mut self, symbol: String, start_date: NaiveDate, end_date: NaiveDate) {
        self.add_log_message(LogLevel::Info, &format!("Starting single stock collection for {} from {} to {}", symbol, start_date, end_date));

        // Start async operation
        let operation_id = format!("single_stock_{}", symbol);
        let _ = self.state_manager.start_operation(operation_id.clone(), format!("Single Stock Collection: {}", symbol), true);
        
        // Spawn the actual work
        let mut state_manager = self.state_manager.clone();
        let symbol_clone = symbol.clone();
        
        tokio::spawn(async move {
            // Create log file for debugging in archive folder
            let archive_dir = "archive/debug_logs";
            std::fs::create_dir_all(archive_dir).unwrap_or_else(|_| ());
            let log_file_path = format!("{}/debug_collection_{}_{}_{}.log", archive_dir, symbol_clone, start_date, end_date);
            
            // Create log file inside the async task
            let log_file = std::fs::File::create(&log_file_path).unwrap_or_else(|_| {
                // Fallback to main directory if archive creation fails
                std::fs::File::create("debug_collection.log").unwrap()
            });
            let mut log_writer = std::io::BufWriter::new(log_file);

            let log_message = format!("🔄 Preparing to fetch {} from {} to {}", symbol_clone, start_date, end_date);
            let _ = state_manager.add_log_message(LogLevel::Info, &log_message);
            let _ = writeln!(log_writer, "[{}] {}", Utc::now().format("%H:%M:%S"), log_message);

            // Load config, DB and client
            let config = match crate::models::Config::from_env() {
                Ok(c) => {
                    let _ = writeln!(log_writer, "[{}] ✅ Config loaded successfully", Utc::now().format("%H:%M:%S"));
                    c
                },
                Err(e) => { 
                    let error_msg = format!("❌ Config error: {}", e);
                    let _ = state_manager.add_log_message(LogLevel::Error, &error_msg);
                    let _ = writeln!(log_writer, "[{}] {}", Utc::now().format("%H:%M:%S"), error_msg);
                    let _ = state_manager.complete_operation(&operation_id, Err(error_msg));
                    return;
                }
            };
            
            let database = match crate::database_sqlx::DatabaseManagerSqlx::new(&config.database_path).await {
                Ok(db) => {
                    let _ = writeln!(log_writer, "[{}] ✅ Database initialized successfully", Utc::now().format("%H:%M:%S"));
                    db
                },
                Err(e) => { 
                    let error_msg = format!("❌ DB init error: {}", e);
                    let _ = state_manager.add_log_message(LogLevel::Error, &error_msg);
                    let _ = writeln!(log_writer, "[{}] {}", Utc::now().format("%H:%M:%S"), error_msg);
                    let _ = state_manager.complete_operation(&operation_id, Err(error_msg));
                    return;
                }
            };
            
            let client = match crate::api::SchwabClient::new(&config) {
                Ok(c) => {
                    let _ = writeln!(log_writer, "[{}] ✅ Schwab client initialized successfully", Utc::now().format("%H:%M:%S"));
                    c
                },
                Err(e) => { 
                    let error_msg = format!("❌ Client init error: {}", e);
                    let _ = state_manager.add_log_message(LogLevel::Error, &error_msg);
                    let _ = writeln!(log_writer, "[{}] {}", Utc::now().format("%H:%M:%S"), error_msg);
                    let _ = state_manager.complete_operation(&operation_id, Err(error_msg));
                    return;
                }
            };

            // Find stock by symbol
            let stock = match database.get_stock_by_symbol(&symbol_clone).await {
                Ok(Some(s)) => {
                    let _ = writeln!(log_writer, "[{}] ✅ Found stock: {} ({})", Utc::now().format("%H:%M:%S"), s.symbol, s.company_name);
                    s
                },
                Ok(None) => { 
                    let error_msg = format!("❌ Unknown symbol {} in DB", symbol_clone);
                    let _ = state_manager.add_log_message(LogLevel::Error, &error_msg);
                    let _ = writeln!(log_writer, "[{}] {}", Utc::now().format("%H:%M:%S"), error_msg);
                    let _ = state_manager.complete_operation(&operation_id, Err(error_msg));
                    return;
                }
                Err(e) => { 
                    let error_msg = format!("❌ DB query error: {}", e);
                    let _ = state_manager.add_log_message(LogLevel::Error, &error_msg);
                    let _ = writeln!(log_writer, "[{}] {}", Utc::now().format("%H:%M:%S"), error_msg);
                    let _ = state_manager.complete_operation(&operation_id, Err(error_msg));
                    return;
                }
            };

            let client_arc = Arc::new(client);
            let db_arc = Arc::new(database);

            let log_message = format!("📡 Fetching {} ({} → {})", symbol_clone, start_date, end_date);
            let _ = state_manager.add_log_message(LogLevel::Info, &log_message);
            let _ = writeln!(log_writer, "[{}] {}", Utc::now().format("%H:%M:%S"), log_message);

            // Calculate trading week batches
            let batches = crate::ui::data_collection::TradingWeekBatchCalculator::calculate_batches(start_date, end_date);
            let log_message = format!("📊 Created {} trading week batches", batches.len());
            let _ = state_manager.add_log_message(LogLevel::Info, &log_message);
            let _ = writeln!(log_writer, "[{}] {}", Utc::now().format("%H:%M:%S"), log_message);

            // Log batch plan
            for batch in &batches {
                let log_message = format!("📅 {}", batch.description);
                let _ = state_manager.add_log_message(LogLevel::Info, &log_message);
                let _ = writeln!(log_writer, "[{}] {}", Utc::now().format("%H:%M:%S"), log_message);
            }

            let mut total_inserted = 0;

            // Process each trading week batch
            for batch in batches {
                let log_message = format!("🔄 Processing {}", batch.description);
                let _ = state_manager.add_log_message(LogLevel::Info, &log_message);
                let _ = writeln!(log_writer, "[{}] {}", Utc::now().format("%H:%M:%S"), log_message);

                // Check existing records for this batch
                let _ = writeln!(log_writer, "[{}] 🔍 Checking existing records for batch {}", Utc::now().format("%H:%M:%S"), batch.batch_number);
                let existing_count = {
                    let db_arc = db_arc.clone();
                    let stock_id = stock.id.unwrap();
                    let start_date = batch.start_date;
                    let end_date = batch.end_date;
                    db_arc.count_existing_records(stock_id, start_date, end_date).await.unwrap_or(0)
                };
                let _ = writeln!(log_writer, "[{}] 📊 Found {} existing records for batch {}", Utc::now().format("%H:%M:%S"), existing_count, batch.batch_number);

                if existing_count > 0 {
                    let log_message = format!("ℹ️ Batch {}: Found {} existing records, skipping", batch.batch_number, existing_count);
                    let _ = state_manager.add_log_message(LogLevel::Info, &log_message);
                    let _ = writeln!(log_writer, "[{}] {}", Utc::now().format("%H:%M:%S"), log_message);
                    continue;
                }

                let _ = writeln!(log_writer, "[{}] 🚀 Starting fetch_stock_history for batch {}", Utc::now().format("%H:%M:%S"), batch.batch_number);
                match crate::data_collector::DataCollector::fetch_stock_history(
                    client_arc.clone(),
                    db_arc.clone(),
                    stock.clone(),
                    batch.start_date,
                    batch.end_date,
                ).await {
                    Ok(inserted) => {
                        total_inserted += inserted;
                        let _ = writeln!(log_writer, "[{}] ✅ fetch_stock_history completed for batch {}: {} records", Utc::now().format("%H:%M:%S"), batch.batch_number, inserted);
                        if inserted > 0 {
                            let log_message = format!("✅ Batch {}: Inserted {} records (Total: {})", batch.batch_number, inserted, total_inserted);
                            let _ = state_manager.add_log_message(LogLevel::Success, &log_message);
                            let _ = writeln!(log_writer, "[{}] {}", Utc::now().format("%H:%M:%S"), log_message);
                        } else {
                            let log_message = format!("ℹ️ Batch {}: No new records (data already exists) (Total: {})", batch.batch_number, total_inserted);
                            let _ = state_manager.add_log_message(LogLevel::Info, &log_message);
                            let _ = writeln!(log_writer, "[{}] {}", Utc::now().format("%H:%M:%S"), log_message);
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("❌ Batch {}: Failed - {}", batch.batch_number, e);
                        let _ = state_manager.add_log_message(LogLevel::Error, &error_msg);
                        let _ = writeln!(log_writer, "[{}] {}", Utc::now().format("%H:%M:%S"), error_msg);
                    }
                }

                let _ = writeln!(log_writer, "[{}] ⏱️ Waiting 500ms before next batch", Utc::now().format("%H:%M:%S"));
                // Small delay between batches to avoid rate limiting
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }

            let _ = writeln!(log_writer, "[{}] 🏁 All batches processed. Total inserted: {}", Utc::now().format("%H:%M:%S"), total_inserted);
            if total_inserted > 0 {
                let log_message = format!("✅ Successfully completed: {} new records inserted", total_inserted);
                let _ = state_manager.complete_operation(&operation_id, Ok(log_message.clone()));
                let _ = writeln!(log_writer, "[{}] {}", Utc::now().format("%H:%M:%S"), log_message);
            } else {
                let log_message = format!("ℹ️ Completed: No new records inserted (all data already exists)");
                let _ = state_manager.complete_operation(&operation_id, Ok(log_message.clone()));
                let _ = writeln!(log_writer, "[{}] {}", Utc::now().format("%H:%M:%S"), log_message);
            }
        });
    }

    /// Cancel current operation
    fn cancel_operation(&mut self) {
        self.add_log_message(LogLevel::Warning, "Operation cancelled by user");
        // Cancel all active operations
        let active_operations: Vec<String> = self.state_manager.get_active_operations()
            .iter()
            .map(|op| op.id.clone())
            .collect();
        
        for op_id in active_operations {
            let _ = self.state_manager.cancel_operation(&op_id);
        }
    }

    /// Complete current operation
    fn complete_operation(&mut self) {
        // This is now handled by the AsyncStateManager
    }

    /// Render the main view using centralized layout
    fn render_main_view(&self, f: &mut Frame, view_layout: ViewLayout) {
        // Title
        let title = Paragraph::new("🚀 Data Collection")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        f.render_widget(title, view_layout.title);

        // Split main content into actions and logs
        let main_chunks = view_layout.split_main_content_vertical(&[
            Constraint::Length(8), // Actions list (reduced from 12)
            Constraint::Min(0),    // Logs (increased space)
        ]);

        // Actions list
        self.render_actions_list(f, main_chunks[0]);

        // Logs
        self.render_logs(f, main_chunks[1]);

        // Status
        self.render_status(f, view_layout.status);
    }

    /// Render the actions list
    fn render_actions_list(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.actions
            .iter()
            .enumerate()
            .map(|(i, action)| {
                let style = if i == self.selected_action && !self.state_manager.has_active_operations() {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let content = vec![
                    Line::from(vec![
                        Span::styled(&action.title, style),
                    ]),
                    Line::from(vec![
                        Span::styled(&action.description, Style::default().fg(Color::Gray)),
                    ]),
                ];
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Available Actions"))
            .highlight_style(Style::default().bg(Color::LightBlue).fg(Color::Black))
            .highlight_symbol("→ ");

        f.render_stateful_widget(list, area, &mut ratatui::widgets::ListState::default().with_selected(if self.state_manager.has_active_operations() { None } else { Some(self.selected_action) }));
    }

    /// Render the logs
    fn render_logs(&self, f: &mut Frame, area: Rect) {
        let recent_logs = self.state_manager.get_recent_logs(50); // Increased from 20 to 50
        let log_items: Vec<ListItem> = recent_logs
            .iter()
            .map(|log| {
                let timestamp = log.timestamp.format("%H:%M:%S").to_string();
                let level_style = match log.level {
                    LogLevel::Info => Style::default().fg(Color::White),
                    LogLevel::Success => Style::default().fg(Color::Green),
                    LogLevel::Warning => Style::default().fg(Color::Yellow),
                    LogLevel::Error => Style::default().fg(Color::Red),
                };
                
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(format!("[{}] ", timestamp), Style::default().fg(Color::Gray)),
                        Span::styled(&log.message, level_style),
                    ])
                ])
            })
            .collect();

        let logs = List::new(log_items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Logs"))
            .style(Style::default().fg(Color::White));

        // Auto-scroll to the bottom (latest logs)
        let mut list_state = ratatui::widgets::ListState::default();
        if !recent_logs.is_empty() {
            list_state.select(Some(recent_logs.len() - 1));
        }

        f.render_stateful_widget(logs, area, &mut list_state);
    }

    /// Render the status
    fn render_status(&self, f: &mut Frame, area: Rect) {
        let status_text = if self.state_manager.has_active_operations() {
            vec![
                Line::from(vec![
                    Span::styled("🔄 Executing operation... ", Style::default().fg(Color::Yellow)),
                    Span::styled("Press Q to cancel", Style::default().fg(Color::Gray)),
                ]),
            ]
        } else {
            vec![
                Line::from(vec![
                    Span::styled("↑/↓: Navigate • ", Style::default().fg(Color::Gray)),
                    Span::styled("Enter", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::styled(": Execute • ", Style::default().fg(Color::Gray)),
                    Span::styled("Q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                    Span::styled(": Quit", Style::default().fg(Color::Gray)),
                ]),
            ]
        };

        let status = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White));

        f.render_widget(status, area);
    }

    /// Render confirmation dialog
    fn render_confirmation_dialog(&self, f: &mut Frame, area: Rect, confirmation: &ConfirmationState) {
        // Create a centered dialog box
        let dialog_width = 60;
        let dialog_height = 8;
        let dialog_x = (area.width.saturating_sub(dialog_width)) / 2;
        let dialog_y = (area.height.saturating_sub(dialog_height)) / 2;
        
        let dialog_area = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);
        
        // Render semi-transparent background
        let background = Paragraph::new("")
            .block(Block::default().style(Style::default().bg(Color::Black)));
        f.render_widget(background, area);
        
        // Render dialog box
        let dialog_content = vec![
            Line::from(vec![
                Span::styled("⚠️  Confirmation Required", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("", Style::default()),
            ]),
            Line::from(vec![
                Span::styled("Are you sure you want to execute:", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled(&confirmation.action_title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("", Style::default()),
            ]),
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(if confirmation.selected_option { "▶ " } else { "   " }, Style::default().fg(Color::Yellow)),
                Span::styled("Yes", if confirmation.selected_option { Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::White) }),
                Span::styled("    ", Style::default()),
                Span::styled(if !confirmation.selected_option { "▶ " } else { "   " }, Style::default().fg(Color::Yellow)),
                Span::styled("No", if !confirmation.selected_option { Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::White) }),
            ]),
            Line::from(vec![
                Span::styled("", Style::default()),
            ]),
            Line::from(vec![
                Span::styled("←/→: Select • Enter: Confirm • Esc: Cancel", Style::default().fg(Color::Gray)),
            ]),
        ];
        
        let dialog = Paragraph::new(dialog_content)
            .block(Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Yellow)));
        
        f.render_widget(dialog, dialog_area);
    }

    /// Render stock selection
    fn render_stock_selection(&self, f: &mut Frame, area: Rect, stock_state: &StockSelectionState) {
        let view_layout = ViewLayout::new(area);
        
        // Title
        let title = Paragraph::new("📈 Select Stock")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        f.render_widget(title, view_layout.title);

        // Split main content
        let main_chunks = view_layout.split_main_content_vertical(&[
            Constraint::Length(3), // Search
            Constraint::Min(0),    // Stock list
        ]);

        // Search input
        let search_text = if stock_state.is_searching {
            format!("Search: {}", stock_state.search_query)
        } else {
            "Type to search stocks...".to_string()
        };
        let search = Paragraph::new(search_text)
            .block(Block::default().borders(Borders::ALL).title("Search"))
            .style(Style::default().fg(if stock_state.is_searching { Color::Cyan } else { Color::Gray }));
        f.render_widget(search, main_chunks[0]);

        // Stock list
        let items: Vec<ListItem> = stock_state.available_stocks
            .iter()
            .enumerate()
            .map(|(i, stock)| {
                let style = if i == stock_state.selected_index {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(vec![Line::from(vec![Span::styled(stock, style)])])
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Available Stocks"))
            .highlight_style(Style::default().bg(Color::LightBlue).fg(Color::Black))
            .highlight_symbol("→ ");

        f.render_stateful_widget(list, main_chunks[1], &mut ratatui::widgets::ListState::default().with_selected(Some(stock_state.selected_index)));

        // Status
        let status = Paragraph::new("↑/↓: Navigate • Type: Search • Enter: Select • Esc: Cancel")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(status, view_layout.status);
    }

    /// Render date selection
    fn render_date_selection(&self, f: &mut Frame, area: Rect, date_state: &DateSelectionState) {
        let view_layout = ViewLayout::new(area);
        
        // Title
        let title = Paragraph::new(format!("📅 Select Date Range for {}", date_state.selected_stock))
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        f.render_widget(title, view_layout.title);

        // Date inputs with cursor
        let start_date_with_cursor = self.render_input_field_with_cursor(&date_state.start_date_input, date_state.cursor_position, true);
        let end_date_with_cursor = self.render_input_field_with_cursor(&date_state.end_date_input, 0, false);
        
        let date_content = vec![
            Line::from(vec![
                Span::styled("📅 Default Date Range", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![Span::styled("", Style::default())]),
            Line::from(vec![
                Span::styled("Start Date: ", Style::default().fg(Color::White)),
                Span::styled(start_date_with_cursor, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![Span::styled("", Style::default())]),
            Line::from(vec![
                Span::styled("End Date: ", Style::default().fg(Color::White)),
                Span::styled(end_date_with_cursor, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![Span::styled("", Style::default())]),
            Line::from(vec![
                Span::styled("Format: YYYY-MM-DD (e.g., 2024-01-01)", Style::default().fg(Color::Gray)),
            ]),
            Line::from(vec![Span::styled("", Style::default())]),
            Line::from(vec![
                Span::styled("↑/↓: Navigate fields • ←/→: Move cursor • Enter: Start collection • Esc: Cancel", Style::default().fg(Color::Gray)),
            ]),
        ];

        let date_inputs = Paragraph::new(date_content)
            .block(Block::default().borders(Borders::ALL).title("Date Range Selection"))
            .style(Style::default().fg(Color::White));
        f.render_widget(date_inputs, view_layout.main_content);

        // Status
        let status = Paragraph::new("Editing date range • Use arrow keys to navigate • Enter to start collection")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(status, view_layout.status);
    }

    /// Render input field with cursor
    fn render_input_field_with_cursor(&self, input: &str, cursor_pos: usize, is_active: bool) -> String {
        if !is_active {
            return input.to_string();
        }
        
        if cursor_pos > input.len() {
            return format!("{}{}", input, "█");
        }
        
        let mut result = String::new();
        for (i, c) in input.chars().enumerate() {
            if i == cursor_pos {
                result.push('█');
            }
            result.push(c);
        }
        
        if cursor_pos == input.len() {
            result.push('█');
        }
        
        result
    }
}

impl View for DataCollectionView {
    fn render(&self, f: &mut Frame, area: Rect) {
        // If confirmation dialog is active, render it as an overlay
        if let Some(ref confirmation) = self.confirmation_state {
            self.render_confirmation_dialog(f, area, confirmation);
            return;
        }

        // If stock selection is active, render stock selection
        if let Some(ref stock_state) = self.stock_selection_state {
            self.render_stock_selection(f, area, stock_state);
            return;
        }

        // If date selection is active, render date selection
        if let Some(ref date_state) = self.date_selection_state {
            self.render_date_selection(f, area, date_state);
            return;
        }

        // Main data collection view using centralized layout
        let view_layout = ViewLayout::for_data_collection(area);
        self.render_main_view(f, view_layout);
    }

    fn get_title(&self) -> String {
        "Data Collection".to_string()
    }

    fn get_status(&self) -> String {
        if self.state_manager.has_active_operations() {
            self.state_manager.get_status_text()
        } else {
            self.status.clone()
        }
    }

    fn handle_key(&mut self, key: crossterm::event::KeyCode) -> Result<bool> {
        // Debug log key events
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("debug_tui.log") 
        {
            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let _ = writeln!(file, "[{}] Data collection view received key: {:?}, has_active_ops: {}, confirmation_state: {:?}, stock_selection_state: {:?}, selected_action: {}", 
                          timestamp, key, 
                          self.state_manager.has_active_operations(),
                          self.confirmation_state.is_some(),
                          self.stock_selection_state.is_some(),
                          self.selected_action);
        }
        
        // Process any pending log messages
        self.process_pending_logs();
        
        // If we have active operations, only allow quit (unless we're in interactive selection states)
        if self.state_manager.has_active_operations() && 
           self.confirmation_state.is_none() && 
           self.stock_selection_state.is_none() && 
           self.date_selection_state.is_none() {
            if key == crossterm::event::KeyCode::Char('q') || key == crossterm::event::KeyCode::Esc {
                // Cancel all active operations
                let active_operations: Vec<String> = self.state_manager.get_active_operations()
                    .iter()
                    .map(|op| op.id.clone())
                    .collect();
                
                for op_id in active_operations {
                    self.state_manager.cancel_operation(&op_id)?;
                }
                return Ok(true);
            }
            return Ok(false);
        }

        // Handle confirmation dialog
        if let Some(ref mut confirmation) = self.confirmation_state {
            match key {
                crossterm::event::KeyCode::Left | crossterm::event::KeyCode::Right => {
                    confirmation.selected_option = !confirmation.selected_option;
                }
                crossterm::event::KeyCode::Enter => {
                    if confirmation.selected_option {
                        // User confirmed - execute the action
                        self.confirmation_state = None;
                        self.add_log_message(LogLevel::Info, "Executing confirmed action...");
                        self.execute_selected_action();
                    } else {
                        // User cancelled
                        self.add_log_message(LogLevel::Info, "Operation cancelled by user");
                        self.confirmation_state = None;
                    }
                }
                crossterm::event::KeyCode::Esc => {
                    // Cancel confirmation
                    self.add_log_message(LogLevel::Info, "Operation cancelled by user");
                    self.confirmation_state = None;
                }
                _ => {}
            }
            return Ok(true);
        }

        // Handle stock selection
        if let Some(ref mut stock_state) = self.stock_selection_state {
            match key {
                crossterm::event::KeyCode::Up => {
                    let old_index = stock_state.selected_index;
                    if stock_state.selected_index > 0 {
                        stock_state.selected_index -= 1;
                    }
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("debug_tui.log") 
                    {
                        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                        let _ = writeln!(file, "[{}] Stock selection Up navigation: {} -> {}", timestamp, old_index, stock_state.selected_index);
                    }
                }
                crossterm::event::KeyCode::Down => {
                    let old_index = stock_state.selected_index;
                    if stock_state.selected_index < stock_state.available_stocks.len().saturating_sub(1) {
                        stock_state.selected_index += 1;
                    }
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("debug_tui.log") 
                    {
                        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                        let _ = writeln!(file, "[{}] Stock selection Down navigation: {} -> {}", timestamp, old_index, stock_state.selected_index);
                    }
                }
                crossterm::event::KeyCode::Char(c) => {
                    if c.is_alphanumeric() || c == '.' {
                        stock_state.search_query.push(c);
                        stock_state.is_searching = true;
                        // Simple filter
                        let query = stock_state.search_query.to_uppercase();
                        let filtered: Vec<String> = stock_state.available_stocks
                            .iter()
                            .filter(|stock| stock.to_uppercase().contains(&query))
                            .cloned()
                            .collect();
                        stock_state.available_stocks = filtered;
                        stock_state.selected_index = 0;
                    }
                }
                crossterm::event::KeyCode::Backspace => {
                    stock_state.search_query.pop();
                    stock_state.is_searching = !stock_state.search_query.is_empty();
                    // Reset to full list if search is empty
                    if stock_state.search_query.is_empty() {
                        // This would need to be async, so we'll just keep the current filtered list
                    }
                }
                crossterm::event::KeyCode::Enter => {
                    if !stock_state.available_stocks.is_empty() {
                        let selected_stock = stock_state.available_stocks[stock_state.selected_index].clone();
                        self.start_date_selection(selected_stock);
                    }
                }
                crossterm::event::KeyCode::Esc => {
                    self.add_log_message(LogLevel::Info, "Stock selection cancelled");
                    self.stock_selection_state = None;
                }
                _ => {}
            }
            return Ok(true);
        }

        // Handle date selection
        if let Some(ref mut date_state) = self.date_selection_state {
            match key {
                crossterm::event::KeyCode::Up => {
                    // Navigate to previous field
                    date_state.cursor_position = 0;
                }
                crossterm::event::KeyCode::Down => {
                    // Navigate to next field
                    date_state.cursor_position = 0;
                }
                crossterm::event::KeyCode::Left => {
                    // Move cursor left within current field
                    if date_state.cursor_position > 0 {
                        date_state.cursor_position -= 1;
                    }
                }
                crossterm::event::KeyCode::Right => {
                    // Move cursor right within current field
                    let current_input = &date_state.start_date_input;
                    if date_state.cursor_position < current_input.len() {
                        date_state.cursor_position += 1;
                    }
                }
                crossterm::event::KeyCode::Char(c) => {
                    // Only allow digits and hyphens
                    if c.is_numeric() || c == '-' {
                        let current_input = &mut date_state.start_date_input;
                        if date_state.cursor_position <= current_input.len() {
                            current_input.insert(date_state.cursor_position, c);
                            date_state.cursor_position += 1;
                        }
                    }
                }
                crossterm::event::KeyCode::Backspace => {
                    let current_input = &mut date_state.start_date_input;
                    if date_state.cursor_position > 0 {
                        current_input.remove(date_state.cursor_position - 1);
                        date_state.cursor_position -= 1;
                    }
                }
                crossterm::event::KeyCode::Enter => {
                    // Parse dates and execute single stock collection
                    if let Some(date_state) = &self.date_selection_state {
                        match (self.parse_date_input(&date_state.start_date_input), self.parse_date_input(&date_state.end_date_input)) {
                            (Ok(start_date), Ok(end_date)) => {
                                if start_date <= end_date {
                                    let stock = date_state.selected_stock.clone();
                                    self.add_log_message(LogLevel::Info, &format!("Starting collection for {} from {} to {}", stock, start_date, end_date));
                                    self.date_selection_state = None;
                                    self.run_single_stock_collection(stock, start_date, end_date);
                                } else {
                                    self.add_log_message(LogLevel::Error, "Start date must be before or equal to end date");
                                }
                            }
                            _ => {
                                self.add_log_message(LogLevel::Error, "Invalid date format. Use YYYY-MM-DD");
                            }
                        }
                    }
                }
                crossterm::event::KeyCode::Esc => {
                    self.add_log_message(LogLevel::Info, "Date selection cancelled");
                    self.date_selection_state = None;
                }
                _ => {}
            }
            return Ok(true);
        }

        // Handle main view navigation
        match key {
            crossterm::event::KeyCode::Up => {
                let old_action = self.selected_action;
                self.selected_action = if self.selected_action == 0 {
                    self.actions.len() - 1
                } else {
                    self.selected_action - 1
                };
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("debug_tui.log") 
                {
                    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                    let _ = writeln!(file, "[{}] Data collection Up navigation: {} -> {}", timestamp, old_action, self.selected_action);
                }
                Ok(true)
            }
            crossterm::event::KeyCode::Down => {
                let old_action = self.selected_action;
                self.selected_action = (self.selected_action + 1) % self.actions.len();
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("debug_tui.log") 
                {
                    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                    let _ = writeln!(file, "[{}] Data collection Down navigation: {} -> {}", timestamp, old_action, self.selected_action);
                }
                Ok(true)
            }
            crossterm::event::KeyCode::Enter => {
                // Execute the selected action
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("debug_tui.log") 
                {
                    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                    let _ = writeln!(file, "[{}] Data collection Enter pressed, executing action {}", timestamp, self.selected_action);
                }
                self.execute_selected_action();
                Ok(true)
            }
            _ => {
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("debug_tui.log") 
                {
                    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                    let _ = writeln!(file, "[{}] Data collection unhandled key: {:?}", timestamp, key);
                }
                Ok(false)
            }
        }
    }

    fn handle_state_update(&mut self, update: &crate::ui::state::StateUpdate) -> Result<bool> {
        debug_log(&format!("DataCollectionView received state update: {:?}", update));
        // Process state updates from async operations
        match update {
            crate::ui::state::StateUpdate::LogMessage { level, message } => {
                self.add_log_message(level.clone(), message);
                Ok(true)
            }
            crate::ui::state::StateUpdate::OperationCompleted { id, result } => {
                match result {
                    Ok(msg) => {
                        self.add_log_message(LogLevel::Success, &format!("Operation {} completed: {}", id, msg));
                    }
                    Err(err) => {
                        self.add_log_message(LogLevel::Error, &format!("Operation {} failed: {}", id, err));
                    }
                }
                Ok(true)
            }
            crate::ui::state::StateUpdate::StockListUpdated { stocks } => {
                debug_log(&format!("Received StockListUpdated state update with {} stocks", stocks.len()));
                // Update the stock selection state with real S&P500 stocks
                if let Some(stock_state) = &mut self.stock_selection_state {
                    stock_state.available_stocks = stocks.clone();
                    stock_state.selected_index = 0;
                    self.add_log_message(LogLevel::Success, &format!("Stock list updated with {} S&P500 stocks", stocks.len()));
                    debug_log(&format!("Stock list updated with {} stocks: {:?}", stocks.len(), &stocks[0..5]));
                } else {
                    debug_log("ERROR: No stock_selection_state available to update");
                }
                Ok(true)
            }
            _ => Ok(false)
        }
    }

    fn update(&mut self) -> Result<()> {
        // Process any pending log messages
        self.process_pending_logs();
        
        // Process state updates from async operations
        self.state_manager.process_updates();
        
        // Update status based on current state
        if self.state_manager.has_active_operations() {
            self.status = self.state_manager.get_status_text();
        } else {
            self.status = "Ready".to_string();
        }
        
        Ok(())
    }
}
