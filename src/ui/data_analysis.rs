use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::database::DatabaseManager;

/// Data analysis view state
pub struct DataAnalysisView {
    pub selected_stock_index: usize,
    pub available_stocks: Vec<StockInfo>,
    pub selected_stock: Option<StockInfo>,
    pub date_input: String,
    pub cursor_position: usize,
    pub stock_data: Option<StockData>,
    pub is_loading: bool,
    pub error_message: Option<String>,
}

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

impl DataAnalysisView {
    pub fn new() -> Self {
        Self {
            selected_stock_index: 0,
            available_stocks: Vec::new(),
            selected_stock: None,
            date_input: String::new(),
            cursor_position: 0,
            stock_data: None,
            is_loading: false,
            error_message: None,
        }
    }

    /// Load available stocks from database
    pub fn load_available_stocks(&mut self, database: &DatabaseManager) -> Result<()> {
        self.is_loading = true;
        self.error_message = None;
        
        match database.get_active_stocks() {
            Ok(stocks) => {
                let mut stock_infos = Vec::new();
                
                for stock in stocks {
                    // Get data statistics for this stock
                    if let Some(stock_id) = stock.id {
                        if let Ok(stats) = database.get_stock_data_stats(stock_id) {
                            stock_infos.push(StockInfo {
                                symbol: stock.symbol.clone(),
                                company_name: stock.company_name.clone(),
                                data_points: stats.data_points,
                                latest_date: stats.latest_date,
                                earliest_date: stats.earliest_date,
                            });
                        }
                    }
                }
                
                // Sort by symbol for easier browsing
                stock_infos.sort_by(|a, b| a.symbol.cmp(&b.symbol));
                self.available_stocks = stock_infos;
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to load stocks: {}", e));
            }
        }
        
        self.is_loading = false;
        Ok(())
    }

    /// Handle key events
    pub fn handle_key_event(&mut self, key: KeyCode, database: &DatabaseManager) -> Result<()> {
        match key {
            KeyCode::Up => {
                if self.selected_stock.is_none() && !self.available_stocks.is_empty() {
                    self.selected_stock_index = self.selected_stock_index.saturating_sub(1);
                    if self.selected_stock_index >= self.available_stocks.len() {
                        self.selected_stock_index = self.available_stocks.len().saturating_sub(1);
                    }
                }
            }
            KeyCode::Down => {
                if self.selected_stock.is_none() && !self.available_stocks.is_empty() {
                    self.selected_stock_index = self.selected_stock_index.saturating_add(1);
                    if self.selected_stock_index >= self.available_stocks.len() {
                        self.selected_stock_index = 0;
                    }
                }
            }
            KeyCode::Enter => {
                if self.selected_stock.is_none() && !self.available_stocks.is_empty() {
                    // Select the stock
                    self.selected_stock = Some(self.available_stocks[self.selected_stock_index].clone());
                    self.date_input = chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string();
                    self.cursor_position = self.date_input.len();
                } else if self.selected_stock.is_some() {
                    // Fetch data for the selected date
                    self.fetch_stock_data(database)?;
                }
            }
            KeyCode::Esc => {
                if self.selected_stock.is_some() {
                    self.selected_stock = None;
                    self.stock_data = None;
                    self.date_input.clear();
                    self.cursor_position = 0;
                }
            }
            KeyCode::Char(c) => {
                if self.selected_stock.is_some() && (c.is_numeric() || c == '-') {
                    if self.cursor_position < self.date_input.len() {
                        self.date_input.insert(self.cursor_position, c);
                    } else {
                        self.date_input.push(c);
                    }
                    self.cursor_position += 1;
                }
            }
            KeyCode::Backspace => {
                if self.selected_stock.is_some() && self.cursor_position > 0 {
                    self.date_input.remove(self.cursor_position - 1);
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Delete => {
                if self.selected_stock.is_some() && self.cursor_position < self.date_input.len() {
                    self.date_input.remove(self.cursor_position);
                }
            }
            KeyCode::Left => {
                if self.selected_stock.is_some() && self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Right => {
                if self.selected_stock.is_some() && self.cursor_position < self.date_input.len() {
                    self.cursor_position += 1;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Fetch stock data for a specific date
    fn fetch_stock_data(&mut self, database: &DatabaseManager) -> Result<()> {
        if let Some(stock) = &self.selected_stock {
            match NaiveDate::parse_from_str(&self.date_input, "%Y-%m-%d") {
                Ok(date) => {
                    self.is_loading = true;
                    self.error_message = None;
                    
                    // Get stock ID
                    if let Ok(Some(db_stock)) = database.get_stock_by_symbol(&stock.symbol) {
                        if let Some(stock_id) = db_stock.id {
                            // Get price data
                            if let Ok(Some(price_data)) = database.get_price_on_date(stock_id, date) {
                                // Get fundamentals data (P/E ratio, market cap)
                                let pe_ratio = database.get_pe_ratio_on_date(stock_id, date).ok().flatten();
                                let market_cap = database.get_market_cap_on_date(stock_id, date).ok().flatten();
                                
                                self.stock_data = Some(StockData {
                                    symbol: stock.symbol.clone(),
                                    company_name: stock.company_name.clone(),
                                    date,
                                    open: price_data.open_price,
                                    high: price_data.high_price,
                                    low: price_data.low_price,
                                    close: price_data.close_price,
                                    volume: price_data.volume.unwrap_or(0),
                                    pe_ratio,
                                    market_cap,
                                });
                            } else {
                                self.error_message = Some(format!("No price data available for {} on {}", stock.symbol, date));
                            }
                        } else {
                            self.error_message = Some(format!("Stock {} has no ID in database", stock.symbol));
                        }
                    } else {
                        self.error_message = Some(format!("Stock {} not found in database", stock.symbol));
                    }
                    
                    self.is_loading = false;
                }
                Err(_) => {
                    self.error_message = Some("Invalid date format. Use YYYY-MM-DD".to_string());
                }
            }
        }
        Ok(())
    }

    /// Render the data analysis view
    pub fn render(&self, f: &mut Frame, area: Rect) {
        if self.selected_stock.is_some() {
            self.render_stock_detail_view(f, area);
        } else {
            self.render_stock_list_view(f, area);
        }
    }

    /// Render the stock list view
    fn render_stock_list_view(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(0),    // Stock list
                Constraint::Length(3), // Status
            ])
            .split(area);

        // Title
        let title = Paragraph::new("📊 Data Analysis - Available Stocks")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        f.render_widget(title, chunks[0]);

        // Stock list
        if self.is_loading {
            let loading = Paragraph::new("Loading available stocks...")
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(loading, chunks[1]);
        } else if let Some(error) = &self.error_message {
            let error_widget = Paragraph::new(format!("Error: {}", error))
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Red));
            f.render_widget(error_widget, chunks[1]);
        } else if self.available_stocks.is_empty() {
            let empty = Paragraph::new("No stocks with data found in database.\nUse Data Collection to fetch stock data first.")
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Gray));
            f.render_widget(empty, chunks[1]);
        } else {
            let items: Vec<ListItem> = self.available_stocks
                .iter()
                .enumerate()
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
            f.render_widget(list, chunks[1]);
        }

        // Status
        let status = Paragraph::new("↑/↓: Navigate • Enter: Select Stock • R: Refresh • Q: Quit")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(status, chunks[2]);
    }

    /// Render the stock detail view
    fn render_stock_detail_view(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Length(5), // Date input
                Constraint::Min(0),    // Data display
                Constraint::Length(3), // Status
            ])
            .split(area);

        // Header
        if let Some(stock) = &self.selected_stock {
            let header = Paragraph::new(format!("📊 {} - {}", stock.symbol, stock.company_name))
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
            f.render_widget(header, chunks[0]);
        }

        // Date input
        let date_input_with_cursor = self.render_date_input_with_cursor();
        let date_input = Paragraph::new(format!("Enter date (YYYY-MM-DD): {}", date_input_with_cursor))
            .block(Block::default().borders(Borders::ALL).title("Date Selection"))
            .style(Style::default().fg(Color::White));
        f.render_widget(date_input, chunks[1]);

        // Data display
        if self.is_loading {
            let loading = Paragraph::new("Loading stock data...")
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(loading, chunks[2]);
        } else if let Some(error) = &self.error_message {
            let error_widget = Paragraph::new(format!("Error: {}", error))
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Red));
            f.render_widget(error_widget, chunks[2]);
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
            f.render_widget(data_widget, chunks[2]);
        } else {
            let empty = Paragraph::new("Enter a date and press Enter to view stock data")
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Gray));
            f.render_widget(empty, chunks[2]);
        }

        // Status
        let status = Paragraph::new("←/→: Navigate • Type: Enter Date • Enter: Fetch Data • Esc: Back to List • Q: Quit")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(status, chunks[3]);
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
