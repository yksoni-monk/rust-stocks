use anyhow::Result;
use chrono::NaiveDate;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::ui::{
    View, ViewLayout,
    state::{AsyncStateManager, LogLevel, StateUpdate},
};
use crate::database_sqlx::DatabaseManagerSqlx;

/// Stock information with data availability
#[derive(Debug, Clone)]
pub struct StockInfo {
    pub symbol: String,
    pub company_name: String,
    pub data_points: usize,
    pub latest_date: Option<NaiveDate>,
    pub earliest_date: Option<NaiveDate>,
}

/// Stock data for a specific date
#[derive(Debug, Clone)]
pub struct StockData {
    pub symbol: String,
    pub company_name: String,
    pub date: NaiveDate,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: i64,
    pub pe_ratio: Option<f64>,
    pub market_cap: Option<f64>,
}

/// Refactored DataAnalysisView implementing the View trait
pub struct DataAnalysisView {
    // View state
    pub selected_stock_index: usize,
    pub available_stocks: Vec<StockInfo>,
    pub selected_stock: Option<StockInfo>,
    pub date_input: String,
    pub cursor_position: usize,
    pub stock_data: Option<StockData>,
    
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

impl DataAnalysisView {
    pub fn new() -> Self {
        Self {
            selected_stock_index: 0,
            available_stocks: Vec::new(),
            selected_stock: None,
            date_input: String::new(),
            cursor_position: 0,
            stock_data: None,
            state_manager: AsyncStateManager::new(),
            database: None,
            global_broadcast_sender: None,
            pending_log_message: None,
            pending_log_level: None,
        }
    }

    /// Set database reference
    pub fn set_database(&mut self, database: Arc<DatabaseManagerSqlx>) {
        self.database = Some(database);
    }

    /// Set the global state manager
    pub fn set_state_manager(&mut self, state_manager: AsyncStateManager) {
        self.state_manager = state_manager;
    }

    /// Set the global broadcast sender
    pub fn set_global_broadcast_sender(&mut self, sender: broadcast::Sender<StateUpdate>) {
        self.global_broadcast_sender = Some(sender);
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

    /// Load available stocks from database
    async fn load_available_stocks(&mut self) -> Result<()> {
        // Clone database reference to avoid borrow checker issues
        let database = if let Some(db) = &self.database {
            db.clone()
        } else {
            return Ok(());
        };
        
        self.add_log_message(LogLevel::Info, "Loading available stocks...");
        
        let operation_id = "load_stocks".to_string();
        let _ = self.state_manager.start_operation(operation_id.clone(), "Loading Stocks".to_string(), false);
        
        let mut state_manager = self.state_manager.clone();
        
        tokio::spawn(async move {
            match database.get_active_stocks().await {
                Ok(stocks) => {
                    let mut stock_infos = Vec::new();
                    
                    for stock in stocks {
                        // Get data statistics for this stock
                        if let Some(stock_id) = stock.id {
                            match database.get_stock_data_stats(stock_id).await {
                                Ok(stats) => {
                                    if stats.data_points > 0 {
                                        stock_infos.push(StockInfo {
                                            symbol: stock.symbol.clone(),
                                            company_name: stock.company_name.clone(),
                                            data_points: stats.data_points,
                                            latest_date: stats.latest_date,
                                            earliest_date: stats.earliest_date,
                                        });
                                    }
                                }
                                Err(_) => {
                                    // Ignore errors for individual stocks
                                }
                            }
                        }
                    }
                    
                    // Sort by symbol for easier browsing
                    stock_infos.sort_by(|a, b| a.symbol.cmp(&b.symbol));
                    
                    let _ = state_manager.add_log_message(LogLevel::Success, &format!("Loaded {} stocks with data", stock_infos.len()));
                    let _ = state_manager.complete_operation(&operation_id, Ok(format!("Loaded {} stocks", stock_infos.len())));
                }
                Err(e) => {
                    let error_msg = format!("Failed to load stocks: {}", e);
                    let _ = state_manager.add_log_message(LogLevel::Error, &error_msg);
                    let _ = state_manager.complete_operation(&operation_id, Err(error_msg));
                }
            }
        });
        
        Ok(())
    }

    /// Fetch stock data for a specific date
    async fn fetch_stock_data(&mut self, symbol: &str, date: NaiveDate) -> Result<()> {
        // Clone database reference to avoid borrow checker issues
        let database = if let Some(db) = &self.database {
            db.clone()
        } else {
            return Ok(());
        };
        
        self.add_log_message(LogLevel::Info, &format!("Fetching data for {} on {}", symbol, date));
        
        let operation_id = format!("fetch_data_{}_{}", symbol, date);
        let _ = self.state_manager.start_operation(operation_id.clone(), format!("Fetching {} Data", symbol), false);
        
        let mut state_manager = self.state_manager.clone();
        let symbol = symbol.to_string();
        
        tokio::spawn(async move {
            // Get stock ID
            match database.get_stock_by_symbol(&symbol).await {
                Ok(Some(db_stock)) => {
                    if let Some(stock_id) = db_stock.id {
                        // Get price data
                        match database.get_price_on_date(stock_id, date).await {
                            Ok(Some(price_data)) => {
                                // Get fundamentals data (P/E ratio, market cap) - TODO: Implement in SQLX
                                let pe_ratio = None; // database.get_pe_ratio_on_date(stock_id, date).ok().flatten();
                                let market_cap = None; // database.get_market_cap_on_date(stock_id, date).ok().flatten();
                                
                                let _stock_data = StockData {
                                    symbol: symbol.clone(),
                                    company_name: db_stock.company_name.clone(),
                                    date,
                                    open: price_data.open_price,
                                    high: price_data.high_price,
                                    low: price_data.low_price,
                                    close: price_data.close_price,
                                    volume: price_data.volume.unwrap_or(0),
                                    pe_ratio,
                                    market_cap,
                                };
                                
                                let _ = state_manager.add_log_message(LogLevel::Success, &format!("Successfully fetched data for {} on {}", symbol, date));
                                let _ = state_manager.complete_operation(&operation_id, Ok(format!("Fetched {} data", symbol)));
                            }
                            Ok(None) => {
                                let error_msg = format!("No price data available for {} on {}", symbol, date);
                                let _ = state_manager.add_log_message(LogLevel::Error, &error_msg);
                                let _ = state_manager.complete_operation(&operation_id, Err(error_msg));
                            }
                            Err(e) => {
                                let error_msg = format!("Failed to fetch price data for {} on {}: {}", symbol, date, e);
                                let _ = state_manager.add_log_message(LogLevel::Error, &error_msg);
                                let _ = state_manager.complete_operation(&operation_id, Err(error_msg));
                            }
                        }
                    } else {
                        let error_msg = format!("Stock {} has no ID in database", symbol);
                        let _ = state_manager.add_log_message(LogLevel::Error, &error_msg);
                        let _ = state_manager.complete_operation(&operation_id, Err(error_msg));
                    }
                }
                Ok(None) => {
                    let error_msg = format!("Stock {} not found in database", symbol);
                    let _ = state_manager.add_log_message(LogLevel::Error, &error_msg);
                    let _ = state_manager.complete_operation(&operation_id, Err(error_msg));
                }
                Err(e) => {
                    let error_msg = format!("Failed to find stock {}: {}", symbol, e);
                    let _ = state_manager.add_log_message(LogLevel::Error, &error_msg);
                    let _ = state_manager.complete_operation(&operation_id, Err(error_msg));
                }
            }
        });
        
        Ok(())
    }

    /// Parse date input in YYYY-MM-DD format
    fn parse_date_input(&self, input: &str) -> Result<NaiveDate> {
        NaiveDate::parse_from_str(input, "%Y-%m-%d")
            .map_err(|_| anyhow::anyhow!("Invalid date format. Use YYYY-MM-DD"))
    }

    /// Render the stock list view
    fn render_stock_list_view(&self, f: &mut Frame, area: Rect) {
        let view_layout = ViewLayout::new(area);
        
        // Title
        let title = Paragraph::new("📊 Data Analysis - Available Stocks")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        f.render_widget(title, view_layout.title);

        // Stock list
        if self.state_manager.has_active_operations() {
            let loading = Paragraph::new("Loading available stocks...")
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(loading, view_layout.main_content);
        } else if self.available_stocks.is_empty() {
            let empty = Paragraph::new("No stocks with data found in database.\nUse Data Collection to fetch stock data first.")
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Gray));
            f.render_widget(empty, view_layout.main_content);
        } else {
            // Calculate how many rows fit; each item uses 2 lines + 1 for borders/title
            let list_height = view_layout.main_content.height as usize;
            let visible_rows = list_height.saturating_sub(2) / 2; // approximate visible items
            let visible_rows = visible_rows.max(1);

            // Compute start index so selected item stays in view
            let selected = self.selected_stock_index.min(self.available_stocks.len().saturating_sub(1));
            let start_index = if selected >= visible_rows { selected + 1 - visible_rows } else { 0 };

            let items: Vec<ListItem> = self.available_stocks
                .iter()
                .enumerate()
                .skip(start_index)
                .take(visible_rows)
                .map(|(i, stock)| {
                    let is_selected = i == self.selected_stock_index;
                    let style = if is_selected {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    
                    let date_range = if let (Some(earliest), Some(latest)) = (stock.earliest_date, stock.latest_date) {
                        format!("{} to {}", earliest.format("%Y-%m-%d"), latest.format("%Y-%m-%d"))
                    } else {
                        "No date range".to_string()
                    };
                    
                    ListItem::new(vec![
                        Line::from(vec![
                            Span::styled(if is_selected { "▶ " } else { "   " }, style),
                            Span::styled(&stock.symbol, style),
                            Span::styled(" - ", Style::default().fg(Color::Gray)),
                            Span::styled(&stock.company_name, style),
                        ]),
                        Line::from(vec![
                            Span::styled("   ", Style::default()),
                            Span::styled(format!("{} data points", stock.data_points), Style::default().fg(Color::Cyan)),
                            Span::styled(" | ", Style::default().fg(Color::Gray)),
                            Span::styled(date_range, Style::default().fg(Color::Green)),
                        ]),
                    ])
                })
                .collect();
            
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Stocks with Data"))
                .style(Style::default().fg(Color::White));
            f.render_widget(list, view_layout.main_content);
        }

        // Status
        let status = Paragraph::new("↑/↓: Navigate • Enter: Select Stock • R: Refresh • Q: Quit")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(status, view_layout.status);
    }

    /// Render the stock detail view
    fn render_stock_detail_view(&self, f: &mut Frame, area: Rect) {
        let view_layout = ViewLayout::new(area);
        
        // Header
        if let Some(stock) = &self.selected_stock {
            let header = Paragraph::new(format!("📊 {} - {}", stock.symbol, stock.company_name))
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
            f.render_widget(header, view_layout.title);
        }

        // Split main content
        let main_chunks = view_layout.split_main_content_vertical(&[
            Constraint::Length(5), // Date input
            Constraint::Min(0),   // Data display
        ]);

        // Date input
        let date_input_with_cursor = self.render_date_input_with_cursor();
        let date_input = Paragraph::new(format!("Enter date (YYYY-MM-DD): {}", date_input_with_cursor))
            .block(Block::default().borders(Borders::ALL).title("Date Selection"))
            .style(Style::default().fg(Color::White));
        f.render_widget(date_input, main_chunks[0]);

        // Data display
        if self.state_manager.has_active_operations() {
            let loading = Paragraph::new("Loading stock data...")
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(loading, main_chunks[1]);
        } else if let Some(data) = &self.stock_data {
            let data_content = vec![
                Line::from(vec![
                    Span::styled("Date: ", Style::default().fg(Color::Yellow)),
                    Span::styled(data.date.format("%Y-%m-%d").to_string(), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("Open: ", Style::default().fg(Color::Green)),
                    Span::styled(format!("${:.2}", data.open), Style::default().fg(Color::White)),
                    Span::styled("  High: ", Style::default().fg(Color::Green)),
                    Span::styled(format!("${:.2}", data.high), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("Low: ", Style::default().fg(Color::Red)),
                    Span::styled(format!("${:.2}", data.low), Style::default().fg(Color::White)),
                    Span::styled("  Close: ", Style::default().fg(Color::Red)),
                    Span::styled(format!("${:.2}", data.close), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("Volume: ", Style::default().fg(Color::Cyan)),
                    Span::styled(format!("{}", data.volume), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("P/E Ratio: ", Style::default().fg(Color::Magenta)),
                    Span::styled(
                        data.pe_ratio.map_or("N/A".to_string(), |pe| format!("{:.2}", pe)),
                        Style::default().fg(Color::White)
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Market Cap: ", Style::default().fg(Color::Blue)),
                    Span::styled(
                        data.market_cap.map_or("N/A".to_string(), |cap| format!("${}", cap)),
                        Style::default().fg(Color::White)
                    ),
                ]),
            ];
            
            let data_widget = Paragraph::new(data_content)
                .block(Block::default().borders(Borders::ALL).title("Stock Data"))
                .style(Style::default().fg(Color::White));
            f.render_widget(data_widget, main_chunks[1]);
        } else {
            let empty = Paragraph::new("Enter a date and press Enter to view stock data")
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Gray));
            f.render_widget(empty, main_chunks[1]);
        }

        // Status
        let status = Paragraph::new("←/→: Navigate • Type: Enter Date • Enter: Fetch Data • N/P: Next/Prev Stock • B: Back to List • Q: Quit")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(status, view_layout.status);
    }

    /// Render date input with cursor
    fn render_date_input_with_cursor(&self) -> String {
        let mut result = self.date_input.clone();
        if self.cursor_position <= result.len() {
            result.insert(self.cursor_position, '|');
        } else {
            result.push('|');
        }
        result
    }
}

impl View for DataAnalysisView {
    fn render(&self, f: &mut Frame, area: Rect) {
        if self.selected_stock.is_some() {
            self.render_stock_detail_view(f, area);
        } else {
            self.render_stock_list_view(f, area);
        }
    }

    fn get_title(&self) -> String {
        "📊 Data Analysis".to_string()
    }

    fn get_status(&self) -> String {
        if self.state_manager.has_active_operations() {
            self.state_manager.get_status_text()
        } else if self.selected_stock.is_some() {
            "Stock Detail View - Enter date to fetch data".to_string()
        } else {
            "Stock List View - Select a stock to analyze".to_string()
        }
    }

    fn handle_key(&mut self, key: crossterm::event::KeyCode) -> Result<bool> {
        // Process any pending log messages
        self.process_pending_logs();
        
        // If we have active operations, only allow quit
        if self.state_manager.has_active_operations() {
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

        match key {
            crossterm::event::KeyCode::Up => {
                if self.selected_stock.is_none() && !self.available_stocks.is_empty() {
                    self.selected_stock_index = self.selected_stock_index.saturating_sub(1);
                    if self.selected_stock_index >= self.available_stocks.len() {
                        self.selected_stock_index = self.available_stocks.len().saturating_sub(1);
                    }
                }
                Ok(true)
            }
            crossterm::event::KeyCode::Down => {
                if self.selected_stock.is_none() && !self.available_stocks.is_empty() {
                    self.selected_stock_index = self.selected_stock_index.saturating_add(1);
                    if self.selected_stock_index >= self.available_stocks.len() {
                        self.selected_stock_index = 0;
                    }
                }
                Ok(true)
            }
            crossterm::event::KeyCode::Enter => {
                if self.selected_stock.is_none() && !self.available_stocks.is_empty() {
                    // Select the stock
                    self.selected_stock = Some(self.available_stocks[self.selected_stock_index].clone());
                    self.date_input = chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string();
                    self.cursor_position = self.date_input.len();
                    self.add_log_message(LogLevel::Info, &format!("Selected stock: {}", self.available_stocks[self.selected_stock_index].symbol));
                } else if self.selected_stock.is_some() {
                    // Fetch data for the selected date
                    if let Some(stock) = &self.selected_stock {
                        match self.parse_date_input(&self.date_input) {
                            Ok(date) => {
                                let symbol = stock.symbol.clone();
                                let date_clone = date;
                                let stock_clone = stock.clone();
                                
                                // Clone database reference to avoid borrow checker issues
                                let database = if let Some(db) = &self.database {
                                    db.clone()
                                } else {
                                    self.add_log_message(LogLevel::Error, "Database not available");
                                    return Ok(true);
                                };
                                
                                // Start async operation
                                let operation_id = format!("fetch_data_{}_{}", symbol, date);
                                let _ = self.state_manager.start_operation(operation_id.clone(), format!("Fetching data for {} on {}", symbol, date), true);
                                
                                // Spawn the actual work
                                let mut state_manager = self.state_manager.clone();
                                tokio::spawn(async move {
                                    match fetch_stock_data_for_date(&database, &symbol, date_clone).await {
                                        Ok(data) => {
                                            let _ = state_manager.complete_operation(&operation_id, Ok(format!("Successfully fetched data for {} on {}", symbol, date_clone)));
                                            let _ = state_manager.add_log_message(LogLevel::Success, &format!("Data: Open=${:.2}, Close=${:.2}, Volume={}", data.open, data.close, data.volume));
                                        }
                                        Err(e) => {
                                            let _ = state_manager.complete_operation(&operation_id, Err(format!("Failed to fetch data: {}", e)));
                                        }
                                    }
                                });
                                
                                self.add_log_message(LogLevel::Info, &format!("Fetching data for {} on {}", stock.symbol, date));
                            }
                            Err(_) => {
                                self.pending_log_message = Some("Invalid date format. Use YYYY-MM-DD".to_string());
                                self.pending_log_level = Some(LogLevel::Error);
                            }
                        }
                    }
                }
                Ok(true)
            }
            crossterm::event::KeyCode::Esc => {
                if self.selected_stock.is_some() {
                    self.selected_stock = None;
                    self.stock_data = None;
                    self.date_input.clear();
                    self.cursor_position = 0;
                    self.add_log_message(LogLevel::Info, "Returned to stock list");
                }
                Ok(true)
            }
            crossterm::event::KeyCode::Char('b') | crossterm::event::KeyCode::Char('B') => {
                // Back to stock list (alternative to Esc)
                if self.selected_stock.is_some() {
                    self.selected_stock = None;
                    self.stock_data = None;
                    self.date_input.clear();
                    self.cursor_position = 0;
                    self.add_log_message(LogLevel::Info, "Returned to stock list");
                }
                Ok(true)
            }
            crossterm::event::KeyCode::Char('n') | crossterm::event::KeyCode::Char('N') => {
                // Next stock (when viewing stock data)
                if self.selected_stock.is_some() && !self.available_stocks.is_empty() {
                    self.selected_stock_index = (self.selected_stock_index + 1) % self.available_stocks.len();
                    self.selected_stock = Some(self.available_stocks[self.selected_stock_index].clone());
                    self.stock_data = None;
                    self.date_input = chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string();
                    self.cursor_position = self.date_input.len();
                    self.add_log_message(LogLevel::Info, &format!("Switched to stock: {}", self.available_stocks[self.selected_stock_index].symbol));
                }
                Ok(true)
            }
            crossterm::event::KeyCode::Char('p') | crossterm::event::KeyCode::Char('P') => {
                // Previous stock (when viewing stock data)
                if self.selected_stock.is_some() && !self.available_stocks.is_empty() {
                    self.selected_stock_index = if self.selected_stock_index == 0 {
                        self.available_stocks.len() - 1
                    } else {
                        self.selected_stock_index - 1
                    };
                    self.selected_stock = Some(self.available_stocks[self.selected_stock_index].clone());
                    self.stock_data = None;
                    self.date_input = chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string();
                    self.cursor_position = self.date_input.len();
                    self.add_log_message(LogLevel::Info, &format!("Switched to stock: {}", self.available_stocks[self.selected_stock_index].symbol));
                }
                Ok(true)
            }
            crossterm::event::KeyCode::Char('r') | crossterm::event::KeyCode::Char('R') => {
                // Refresh stock list
                if self.selected_stock.is_none() {
                    let _ = self.load_available_stocks();
                }
                Ok(true)
            }
            crossterm::event::KeyCode::Char(c) => {
                if self.selected_stock.is_some() && (c.is_numeric() || c == '-') {
                    if self.cursor_position < self.date_input.len() {
                        self.date_input.insert(self.cursor_position, c);
                    } else {
                        self.date_input.push(c);
                    }
                    self.cursor_position += 1;
                }
                Ok(true)
            }
            crossterm::event::KeyCode::Backspace => {
                if self.selected_stock.is_some() && self.cursor_position > 0 {
                    self.date_input.remove(self.cursor_position - 1);
                    self.cursor_position -= 1;
                }
                Ok(true)
            }
            crossterm::event::KeyCode::Delete => {
                if self.selected_stock.is_some() && self.cursor_position < self.date_input.len() {
                    self.date_input.remove(self.cursor_position);
                }
                Ok(true)
            }
            crossterm::event::KeyCode::Left => {
                if self.selected_stock.is_some() && self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
                Ok(true)
            }
            crossterm::event::KeyCode::Right => {
                if self.selected_stock.is_some() && self.cursor_position < self.date_input.len() {
                    self.cursor_position += 1;
                }
                Ok(true)
            }
            _ => Ok(false)
        }
    }

    fn handle_state_update(&mut self, update: &crate::ui::state::StateUpdate) -> Result<bool> {
        // Process state updates from async operations
        match update {
            StateUpdate::LogMessage { level, message } => {
                self.add_log_message(level.clone(), message);
                Ok(true)
            }
            StateUpdate::OperationCompleted { id, result } => {
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
            _ => Ok(false)
        }
    }

    fn update(&mut self) -> Result<()> {
        // Process any pending log messages
        self.process_pending_logs();
        
        // Process state updates from async operations
        self.state_manager.process_updates();
        
        Ok(())
    }
}

/// Fetch stock data for a specific date from the database
async fn fetch_stock_data_for_date(
    database: &DatabaseManagerSqlx,
    symbol: &str,
    date: NaiveDate,
) -> Result<StockData> {
    // Get stock ID
    let db_stock = database.get_stock_by_symbol(symbol).await?
        .ok_or_else(|| anyhow::anyhow!("Stock {} not found", symbol))?;
    
    let stock_id = db_stock.id
        .ok_or_else(|| anyhow::anyhow!("Stock {} has no ID", symbol))?;
    
    // Get price data
    let price_data = database.get_price_on_date(stock_id, date).await?
        .ok_or_else(|| anyhow::anyhow!("No price data for {} on {}", symbol, date))?;
    
    // Get fundamentals data (P/E ratio, market cap) - TODO: Implement in SQLX
    let pe_ratio = None; // database.get_pe_ratio_on_date(stock_id, date).ok().flatten();
    let market_cap = None; // database.get_market_cap_on_date(stock_id, date).ok().flatten();
    
    Ok(StockData {
        symbol: symbol.to_string(),
        company_name: db_stock.company_name,
        date,
        open: price_data.open_price,
        high: price_data.high_price,
        low: price_data.low_price,
        close: price_data.close_price,
        volume: price_data.volume.unwrap_or(0),
        pe_ratio,
        market_cap,
    })
}
