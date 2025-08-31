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



/// Data collection action definition
#[derive(Debug, Clone)]
pub struct DataCollectionAction {
    pub id: String,
    pub title: String,
    pub description: String,
    pub action_type: ActionType,
    pub requires_confirmation: bool,
}

/// Types of data collection actions
#[derive(Debug, Clone)]
pub enum ActionType {
    CollectHistoricalData { start_date: NaiveDate, end_date: NaiveDate },
    SingleStockCollection { symbol: String, start_date: NaiveDate, end_date: NaiveDate },
    SelectStockAndDates,
}

/// Active operation status
#[derive(Debug, Clone)]
pub struct ActiveOperation {
    pub action_id: String,
    pub start_time: DateTime<Utc>,
    pub progress: f64,
    pub current_message: String,
    pub logs: Vec<String>,
}

/// Log message with timestamp and level
#[derive(Debug, Clone)]
pub struct LogMessage {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
}

/// Log levels
#[derive(Debug, Clone)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// Data collection view state
pub struct DataCollectionView {
    pub selected_action: usize,
    pub actions: Vec<DataCollectionAction>,
    pub is_executing: bool,
    pub current_operation: Option<ActiveOperation>,
    pub log_messages: Vec<LogMessage>,
    pub confirmation_state: Option<ConfirmationState>,
    pub stock_selection_state: Option<StockSelectionState>,
    pub date_selection_state: Option<DateSelectionState>,
    pub pending_log_message: Option<String>,
    pub pending_log_level: Option<LogLevel>,
}

/// Confirmation dialog state
#[derive(Debug, Clone)]
pub struct ConfirmationState {
    pub action_title: String,
    pub action_type: ActionType,
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
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub selected_field: DateField, // start_date or end_date
    pub start_date_input: String,
    pub end_date_input: String,
    pub cursor_position: usize, // cursor position within the current field
}

/// Date field being edited
#[derive(Debug, Clone)]
pub enum DateField {
    StartDate,
    EndDate,
}

impl DataCollectionView {
    pub fn new() -> Self {
        let actions = vec![
            DataCollectionAction {
                id: "single_stock".to_string(),
                title: "📈 Fetch Single Stock Data".to_string(),
                description: "Fetch data for a specific stock and date range".to_string(),
                requires_confirmation: false,
                action_type: ActionType::SelectStockAndDates,
            },
            DataCollectionAction {
                id: "all_stocks".to_string(),
                title: "📊 Fetch All Stocks Data".to_string(),
                description: "Fetch data for all stocks in a given date range".to_string(),
                requires_confirmation: true,
                action_type: ActionType::CollectHistoricalData {
                    start_date: chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
                    end_date: chrono::Utc::now().date_naive(),
                },
            },
        ];

        Self {
            actions,
            selected_action: 0,
            is_executing: false,
            current_operation: None,
            log_messages: Vec::new(),
            confirmation_state: None,
            stock_selection_state: None,
            date_selection_state: None,
            pending_log_message: None,
            pending_log_level: None,
        }
    }

    /// Handle key events
    pub fn handle_key_event(&mut self, key: crossterm::event::KeyCode) -> Result<()> {
        if self.is_executing {
            // During execution, only allow quit
            if key == crossterm::event::KeyCode::Char('q') || key == crossterm::event::KeyCode::Esc {
                self.cancel_operation();
            }
            return Ok(());
        }

        // Handle confirmation dialog
        if let Some(ref mut confirmation) = self.confirmation_state {
            match key {
                crossterm::event::KeyCode::Left | crossterm::event::KeyCode::Right => {
                    confirmation.selected_option = !confirmation.selected_option;
                }
                crossterm::event::KeyCode::Enter => {
                    let action_type = confirmation.action_type.clone();
                    if confirmation.selected_option {
                        // User confirmed - execute the action
                        self.confirmation_state = None;
                        self.start_operation_by_type(&action_type)?;
                    } else {
                        // User cancelled
                        self.log_info("Operation cancelled by user");
                        self.confirmation_state = None;
                    }
                }
                crossterm::event::KeyCode::Esc => {
                    // Cancel confirmation
                    self.log_info("Operation cancelled by user");
                    self.confirmation_state = None;
                }
                _ => {}
            }
            return Ok(());
        }

        // Handle stock selection
        if let Some(ref mut stock_state) = self.stock_selection_state {
            match key {
                crossterm::event::KeyCode::Up => {
                    if stock_state.selected_index > 0 {
                        stock_state.selected_index -= 1;
                    }
                }
                crossterm::event::KeyCode::Down => {
                    if stock_state.selected_index < stock_state.available_stocks.len().saturating_sub(1) {
                        stock_state.selected_index += 1;
                    }
                }
                crossterm::event::KeyCode::Char(c) => {
                    if c.is_alphanumeric() || c == '.' {
                        stock_state.search_query.push(c);
                        stock_state.is_searching = true;
                        // Use a simple filter without calling self methods
                        let query = stock_state.search_query.to_uppercase();
                        let all_stocks = vec![
                            "AAPL".to_string(), "MSFT".to_string(), "GOOGL".to_string(), "AMZN".to_string(),
                            "TSLA".to_string(), "META".to_string(), "NVDA".to_string(), "NFLX".to_string(),
                            "JPM".to_string(), "JNJ".to_string(), "PG".to_string(), "V".to_string(),
                            "HD".to_string(), "DIS".to_string(), "PYPL".to_string(), "INTC".to_string(),
                            "VZ".to_string(), "ADBE".to_string(), "CRM".to_string(), "NKE".to_string(),
                        ];
                        let filtered: Vec<String> = all_stocks
                            .into_iter()
                            .filter(|stock| stock.to_uppercase().contains(&query))
                            .collect();
                        stock_state.available_stocks = filtered;
                        stock_state.selected_index = 0;
                    }
                }
                crossterm::event::KeyCode::Backspace => {
                    stock_state.search_query.pop();
                    stock_state.is_searching = !stock_state.search_query.is_empty();
                    // Use a simple filter without calling self methods
                    let query = stock_state.search_query.to_uppercase();
                    let all_stocks = vec![
                        "AAPL".to_string(), "MSFT".to_string(), "GOOGL".to_string(), "AMZN".to_string(),
                        "TSLA".to_string(), "META".to_string(), "NVDA".to_string(), "NFLX".to_string(),
                        "JPM".to_string(), "JNJ".to_string(), "PG".to_string(), "V".to_string(),
                        "HD".to_string(), "DIS".to_string(), "PYPL".to_string(), "INTC".to_string(),
                        "VZ".to_string(), "ADBE".to_string(), "CRM".to_string(), "NKE".to_string(),
                    ];
                    let filtered: Vec<String> = all_stocks
                        .into_iter()
                        .filter(|stock| stock.to_uppercase().contains(&query))
                        .collect();
                    stock_state.available_stocks = filtered;
                    stock_state.selected_index = 0;
                }
                crossterm::event::KeyCode::Enter => {
                    if !stock_state.available_stocks.is_empty() {
                        let selected_stock = stock_state.available_stocks[stock_state.selected_index].clone();
                        self.start_date_selection(selected_stock);
                    }
                }
                crossterm::event::KeyCode::Esc => {
                    self.log_info("Stock selection cancelled");
                    self.stock_selection_state = None;
                }
                _ => {}
            }
            return Ok(());
        }

        // Handle date selection
        if let Some(ref mut date_state) = self.date_selection_state {
            match key {
                crossterm::event::KeyCode::Up => {
                    // Navigate to previous field
                    date_state.selected_field = match date_state.selected_field {
                        DateField::StartDate => DateField::EndDate,
                        DateField::EndDate => DateField::StartDate,
                    };
                    date_state.cursor_position = 0;
                }
                crossterm::event::KeyCode::Down => {
                    // Navigate to next field
                    date_state.selected_field = match date_state.selected_field {
                        DateField::StartDate => DateField::EndDate,
                        DateField::EndDate => DateField::StartDate,
                    };
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
                    let current_input = match date_state.selected_field {
                        DateField::StartDate => &date_state.start_date_input,
                        DateField::EndDate => &date_state.end_date_input,
                    };
                    if date_state.cursor_position < current_input.len() {
                        date_state.cursor_position += 1;
                    }
                }
                crossterm::event::KeyCode::Char(c) => {
                    // Only allow digits and hyphens
                    if c.is_numeric() || c == '-' {
                        let current_input = match date_state.selected_field {
                            DateField::StartDate => &mut date_state.start_date_input,
                            DateField::EndDate => &mut date_state.end_date_input,
                        };
                        
                        if date_state.cursor_position <= current_input.len() {
                            current_input.insert(date_state.cursor_position, c);
                            date_state.cursor_position += 1;
                        }
                    }
                }
                crossterm::event::KeyCode::Backspace => {
                    let current_input = match date_state.selected_field {
                        DateField::StartDate => &mut date_state.start_date_input,
                        DateField::EndDate => &mut date_state.end_date_input,
                    };
                    
                    if date_state.cursor_position > 0 {
                        current_input.remove(date_state.cursor_position - 1);
                        date_state.cursor_position -= 1;
                    }
                }
                crossterm::event::KeyCode::Delete => {
                    let current_input = match date_state.selected_field {
                        DateField::StartDate => &mut date_state.start_date_input,
                        DateField::EndDate => &mut date_state.end_date_input,
                    };
                    
                    if date_state.cursor_position < current_input.len() {
                        current_input.remove(date_state.cursor_position);
                    }
                }
                crossterm::event::KeyCode::Enter => {
                    // Extract values before mutable borrow
                    let selected_stock = date_state.selected_stock.clone();
                    let start_input = date_state.start_date_input.clone();
                    let end_input = date_state.end_date_input.clone();
                    
                    // Parse dates outside of mutable borrow
                    let start_result = self.parse_date_input(&start_input);
                    let end_result = self.parse_date_input(&end_input);
                    
                    match (start_result, end_result) {
                        (Ok(start_date), Ok(end_date)) => {
                            if start_date > end_date {
                                self.pending_log_message = Some("Start date cannot be after end date".to_string());
                                self.pending_log_level = Some(LogLevel::Error);
                            } else {
                                // Store log message and action for later
                                self.pending_log_message = Some(format!("Starting collection for {} from {} to {}", 
                                    selected_stock, start_date, end_date));
                                self.pending_log_level = Some(LogLevel::Info);
                                
                                // Clear state and start operation
                                self.date_selection_state = None;
                                self.process_pending_logs();
                                
                                let action_type = ActionType::SingleStockCollection {
                                    symbol: selected_stock,
                                    start_date,
                                    end_date,
                                };
                                self.start_operation_by_type(&action_type)?;
                                return Ok(());
                            }
                        }
                        (Err(_), _) => {
                            self.pending_log_message = Some("Invalid start date format. Use YYYY-MM-DD".to_string());
                            self.pending_log_level = Some(LogLevel::Error);
                        }
                        (_, Err(_)) => {
                            self.pending_log_message = Some("Invalid end date format. Use YYYY-MM-DD".to_string());
                            self.pending_log_level = Some(LogLevel::Error);
                        }
                    }
                }
                crossterm::event::KeyCode::Esc => {
                    self.log_info("Date selection cancelled");
                    self.date_selection_state = None;
                }
                _ => {}
            }
            return Ok(());
        }

        match key {
            crossterm::event::KeyCode::Up => {
                self.selected_action = if self.selected_action == 0 {
                    self.actions.len() - 1
                } else {
                    self.selected_action - 1
                };
            }
            crossterm::event::KeyCode::Down => {
                self.selected_action = (self.selected_action + 1) % self.actions.len();
            }
            crossterm::event::KeyCode::Enter => {
                self.execute_selected_action()?;
            }
            _ => {}
        }
        
        // Process any pending log messages
        self.process_pending_logs();
        Ok(())
    }

    /// Execute the currently selected action
    pub fn execute_selected_action(&mut self) -> Result<()> {
        if self.selected_action >= self.actions.len() {
            return Ok(());
        }

        let action_title = self.actions[self.selected_action].title.clone();
        let requires_confirmation = self.actions[self.selected_action].requires_confirmation;
        let action_type = self.actions[self.selected_action].action_type.clone();
        
        // Check if confirmation is required
        if requires_confirmation {
            self.confirmation_state = Some(ConfirmationState {
                action_title: action_title.clone(),
                action_type: action_type.clone(),
                selected_option: true, // Default to "Yes"
            });
            self.log_info(&format!("Confirmation required for: {}", action_title));
            return Ok(());
        }

        self.start_operation_by_type(&action_type)?;
        Ok(())
    }

    /// Start an operation by action type
    pub fn start_operation_by_type(&mut self, action_type: &ActionType) -> Result<()> {
        // Execute the action based on type
        match action_type {
            ActionType::CollectHistoricalData { start_date, end_date } => {
                self.is_executing = true;
                self.current_operation = Some(ActiveOperation {
                    action_id: "operation".to_string(),
                    start_time: Utc::now(),
                    progress: 0.0,
                    current_message: "Starting operation...".to_string(),
                    logs: Vec::new(),
                });
                self.log_info(&format!("Starting historical data collection from {} to {}", start_date, end_date));
                self.run_historical_collection(*start_date, *end_date)?;
            }
            ActionType::SingleStockCollection { symbol, start_date, end_date } => {
                self.is_executing = true;
                self.current_operation = Some(ActiveOperation {
                    action_id: "operation".to_string(),
                    start_time: Utc::now(),
                    progress: 0.0,
                    current_message: "Starting operation...".to_string(),
                    logs: Vec::new(),
                });
                self.log_info(&format!("Starting single stock collection for {} from {} to {}", symbol, start_date, end_date));
                self.run_single_stock_collection(symbol.clone(), *start_date, *end_date)?;
            }
            ActionType::SelectStockAndDates => {
                // Don't set executing state for interactive selection
                self.log_info("Starting stock and date selection...");
                self.start_stock_and_date_selection();
            }
        }

        Ok(())
    }



    /// Run historical data collection
    pub fn run_historical_collection(&mut self, start_date: NaiveDate, end_date: NaiveDate) -> Result<()> {
        let start_str = start_date.format("%Y%m%d").to_string();
        let end_str = end_date.format("%Y%m%d").to_string();
        
        self.log_info(&format!("Executing: cargo run --bin collect_with_detailed_logs -- -s {} -e {}", start_str, end_str));
        
        let output = Command::new("cargo")
            .args([
                "run", "--bin", "collect_with_detailed_logs", "--", 
                "-s", &start_str, "-e", &end_str
            ])
            .output()?;

        if output.status.success() {
            self.log_success("Historical data collection completed successfully");
            let stdout = String::from_utf8_lossy(&output.stdout);
            self.log_info(&format!("Output: {}", stdout.trim()));
        } else {
            self.log_error("Failed to collect historical data");
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.log_error(&format!("Error: {}", stderr.trim()));
        }

        self.complete_operation();
        Ok(())
    }



    /// Start stock and date selection process
    pub fn start_stock_and_date_selection(&mut self) {
        // Get available stocks from database
        let stocks = self.get_available_stocks();
        self.stock_selection_state = Some(StockSelectionState {
            available_stocks: stocks,
            selected_index: 0,
            search_query: String::new(),
            is_searching: false,
        });
        self.log_info("Stock selection started. Type to search, ↑/↓ to navigate, Enter to select");
    }

    /// Get available stocks from database
    fn get_available_stocks(&self) -> Vec<String> {
        // Query the database for all active stocks
        match crate::database::DatabaseManager::new("stocks.db") {
            Ok(database) => {
                match database.get_active_stocks() {
                    Ok(stocks) => {
                        stocks.into_iter()
                            .map(|stock| stock.symbol)
                            .collect()
                    }
                    Err(_) => {
                        // Fallback to sample list if database query fails
                        vec![
                            "AAPL".to_string(), "MSFT".to_string(), "GOOGL".to_string(), "AMZN".to_string(),
                            "TSLA".to_string(), "META".to_string(), "NVDA".to_string(), "NFLX".to_string(),
                            "JPM".to_string(), "JNJ".to_string(), "PG".to_string(), "V".to_string(),
                            "HD".to_string(), "DIS".to_string(), "PYPL".to_string(), "INTC".to_string(),
                            "VZ".to_string(), "ADBE".to_string(), "CRM".to_string(), "NKE".to_string(),
                        ]
                    }
                }
            }
            Err(_) => {
                // Fallback to sample list if database connection fails
                vec![
                    "AAPL".to_string(), "MSFT".to_string(), "GOOGL".to_string(), "AMZN".to_string(),
                    "TSLA".to_string(), "META".to_string(), "NVDA".to_string(), "NFLX".to_string(),
                    "JPM".to_string(), "JNJ".to_string(), "PG".to_string(), "V".to_string(),
                    "HD".to_string(), "DIS".to_string(), "PYPL".to_string(), "INTC".to_string(),
                    "VZ".to_string(), "ADBE".to_string(), "CRM".to_string(), "NKE".to_string(),
                ]
            }
        }
    }

    /// Filter stocks based on search query
    fn filter_stocks(&self, stock_state: &mut StockSelectionState) {
        if stock_state.search_query.is_empty() {
            return;
        }
        
        let query = stock_state.search_query.to_uppercase();
        let filtered: Vec<String> = self.get_available_stocks()
            .into_iter()
            .filter(|stock| stock.to_uppercase().contains(&query))
            .collect();
        
        stock_state.available_stocks = filtered;
        stock_state.selected_index = 0;
    }



    /// Start date selection for a selected stock
    fn start_date_selection(&mut self, stock: String) {
        let today = chrono::Utc::now().date_naive();
        let default_start = today - chrono::Duration::days(30);
        
        self.date_selection_state = Some(DateSelectionState {
            selected_stock: stock,
            start_date: default_start,
            end_date: today,
            selected_field: DateField::StartDate,
            start_date_input: default_start.format("%Y-%m-%d").to_string(),
            end_date_input: today.format("%Y-%m-%d").to_string(),
            cursor_position: 0,
        });
        
        self.stock_selection_state = None;
        self.log_info("Date selection started. Use ↑/↓ to navigate, ←/→ to edit, Enter to confirm");
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

    /// Run single stock collection using batched approach
    pub fn run_single_stock_collection(&mut self, symbol: String, start_date: NaiveDate, end_date: NaiveDate) -> Result<()> {
        self.log_info(&format!("Starting batched single stock collection for {} from {} to {}", symbol, start_date, end_date));
        
        // Create a progress callback that logs to the TUI
        let _progress_callback = Box::new(|_message: String| {
            // This will be called from the async function to update the UI
            // For now, we'll use a simple approach
        });
        
        // For now, let's use the working batched test approach
        let start_str = start_date.format("%Y%m%d").to_string();
        let end_str = end_date.format("%Y%m%d").to_string();
        
        self.log_info(&format!("Executing: cargo run --bin test_batched_stock -- {} {} {}", symbol, start_str, end_str));
        
        let output = Command::new("cargo")
            .args(["run", "--bin", "test_batched_stock", "--", &symbol, &start_str, &end_str])
            .output()?;

        if output.status.success() {
            self.log_success(&format!("Batched single stock collection completed for {}", symbol));
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            // Parse the output and log the progress
            for line in stdout.lines() {
                if line.contains("🔄 Batch") || line.contains("✅ Batch") || line.contains("❌ Batch") {
                    self.log_info(line);
                } else if line.contains("🎉") {
                    self.log_success(line);
                }
            }
        } else {
            self.log_error(&format!("Failed to collect data for {}", symbol));
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.log_error(&format!("Error: {}", stderr.trim()));
        }

        self.complete_operation();
        Ok(())
    }

    /// Cancel current operation
    pub fn cancel_operation(&mut self) {
        self.log_warning("Operation cancelled by user");
        self.complete_operation();
    }

    /// Complete current operation
    pub fn complete_operation(&mut self) {
        self.is_executing = false;
        self.current_operation = None;
    }

    /// Add log message
    pub fn log_info(&mut self, message: &str) {
        self.add_log_message(LogLevel::Info, message);
    }

    /// Add success log message
    pub fn log_success(&mut self, message: &str) {
        self.add_log_message(LogLevel::Success, message);
    }

    /// Add warning log message
    pub fn log_warning(&mut self, message: &str) {
        self.add_log_message(LogLevel::Warning, message);
    }

    /// Add error log message
    pub fn log_error(&mut self, message: &str) {
        self.add_log_message(LogLevel::Error, message);
    }

    /// Add log message with timestamp
    fn add_log_message(&mut self, level: LogLevel, message: &str) {
        let log_message = LogMessage {
            timestamp: Utc::now(),
            level,
            message: message.to_string(),
        };
        
        self.log_messages.push(log_message);
        
        // Keep only last 50 log messages
        if self.log_messages.len() > 50 {
            self.log_messages.remove(0);
        }
    }

    /// Process pending log messages
    fn process_pending_logs(&mut self) {
        if let (Some(message), Some(level)) = (self.pending_log_message.take(), self.pending_log_level.take()) {
            self.add_log_message(level, &message);
        }
    }

    /// Render the data collection view
    pub fn render(&self, f: &mut Frame, area: Rect) {
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

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(12), // Actions list
                Constraint::Min(0),    // Logs
                Constraint::Length(3), // Status
            ])
            .split(area);

        // Title
        let title = Paragraph::new("🚀 Data Collection")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        f.render_widget(title, chunks[0]);

        // Actions list
        self.render_actions_list(f, chunks[1]);

        // Logs
        self.render_logs(f, chunks[2]);

        // Status
        self.render_status(f, chunks[3]);
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
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(3), // Search
                Constraint::Min(0),    // Stock list
                Constraint::Length(3), // Status
            ])
            .split(area);

        // Title
        let title = Paragraph::new("📈 Select Stock")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        f.render_widget(title, chunks[0]);

        // Search input
        let search_text = if stock_state.is_searching {
            format!("Search: {}", stock_state.search_query)
        } else {
            "Type to search stocks...".to_string()
        };
        let search = Paragraph::new(search_text)
            .block(Block::default().borders(Borders::ALL).title("Search"))
            .style(Style::default().fg(if stock_state.is_searching { Color::Cyan } else { Color::Gray }));
        f.render_widget(search, chunks[1]);

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

        f.render_stateful_widget(list, chunks[2], &mut ratatui::widgets::ListState::default().with_selected(Some(stock_state.selected_index)));

        // Status
        let status = Paragraph::new("↑/↓: Navigate • Type: Search • Enter: Select • Esc: Cancel")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(status, chunks[3]);
    }

    /// Render date selection
    fn render_date_selection(&self, f: &mut Frame, area: Rect, date_state: &DateSelectionState) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(8), // Date inputs
                Constraint::Length(3), // Status
            ])
            .split(area);

        // Title
        let title = Paragraph::new(format!("📅 Select Date Range for {}", date_state.selected_stock))
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        f.render_widget(title, chunks[0]);

        // Date inputs
        let start_selected = matches!(date_state.selected_field, DateField::StartDate);
        let end_selected = matches!(date_state.selected_field, DateField::EndDate);
        
        let start_style = if start_selected {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let end_style = if end_selected {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        
        // Create input field strings with cursor
        let start_input_with_cursor = self.render_input_field(&date_state.start_date_input, date_state.cursor_position, start_selected);
        let end_input_with_cursor = self.render_input_field(&date_state.end_date_input, date_state.cursor_position, end_selected);
        
        let date_content = vec![
            Line::from(vec![
                Span::styled("📅 Default Date Range", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![Span::styled("", Style::default())]),
            Line::from(vec![
                Span::styled("Start Date: ", start_style),
                Span::styled(&start_input_with_cursor, start_style),
            ]),
            Line::from(vec![Span::styled("", Style::default())]),
            Line::from(vec![
                Span::styled("End Date: ", end_style),
                Span::styled(&end_input_with_cursor, end_style),
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
        f.render_widget(date_inputs, chunks[1]);

        // Status
        let status_text = match date_state.selected_field {
            DateField::StartDate => "Editing start date • Use arrow keys to navigate • Enter to start collection",
            DateField::EndDate => "Editing end date • Use arrow keys to navigate • Enter to start collection",
        };
        let status = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(status, chunks[2]);
    }

    /// Render an input field with cursor
    fn render_input_field(&self, input: &str, cursor_pos: usize, is_selected: bool) -> String {
        if !is_selected {
            return input.to_string();
        }
        
        let mut result = input.to_string();
        if cursor_pos <= result.len() {
            result.insert(cursor_pos, '|');
        } else {
            result.push('|');
        }
        result
    }

    /// Render the actions list
    fn render_actions_list(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.actions
            .iter()
            .enumerate()
            .map(|(i, action)| {
                let style = if i == self.selected_action && !self.is_executing {
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

        f.render_stateful_widget(list, area, &mut ratatui::widgets::ListState::default().with_selected(if self.is_executing { None } else { Some(self.selected_action) }));
    }

    /// Render the logs
    fn render_logs(&self, f: &mut Frame, area: Rect) {
        let log_lines: Vec<Line> = self.log_messages
            .iter()
            .rev() // Show newest first
            .take(20) // Limit to 20 lines
            .map(|log| {
                let timestamp = log.timestamp.format("%H:%M:%S").to_string();
                let level_style = match log.level {
                    LogLevel::Info => Style::default().fg(Color::White),
                    LogLevel::Success => Style::default().fg(Color::Green),
                    LogLevel::Warning => Style::default().fg(Color::Yellow),
                    LogLevel::Error => Style::default().fg(Color::Red),
                };
                
                Line::from(vec![
                    Span::styled(format!("[{}] ", timestamp), Style::default().fg(Color::Gray)),
                    Span::styled(&log.message, level_style),
                ])
            })
            .collect();

        let logs = Paragraph::new(log_lines)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Logs"))
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(logs, area);
    }

    /// Render the status
    fn render_status(&self, f: &mut Frame, area: Rect) {
        let status_text = if self.is_executing {
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
}
