use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal, Frame,
};
use std::io;
use tracing::info;

use crate::analysis::{AnalysisEngine, SummaryStats};
use crate::models::{StockAnalysis, StockDetail, Stock};

pub mod components;
pub mod dashboard;
pub mod app;
// pub use components::*;

/// Main application state
pub struct StockApp {
    analysis_engine: AnalysisEngine,
    current_view: AppView,
    list_state: ListState,
    
    // Data
    pe_decliners: Vec<StockAnalysis>,
    current_offset: usize,
    search_results: Vec<Stock>,
    current_stock_detail: Option<StockDetail>,
    summary_stats: Option<SummaryStats>,
    
    // UI state
    search_query: String,
    status_message: String,
    is_loading: bool,
}

/// Application views
#[derive(Debug, Clone, PartialEq)]
pub enum AppView {
    Dashboard,
    PEDecliners,
    StockDetail(String),
    Search,
    Loading,
}

impl StockApp {
    /// Create a new stock application
    pub fn new(analysis_engine: AnalysisEngine) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            analysis_engine,
            current_view: AppView::Loading,
            list_state,
            pe_decliners: Vec::new(),
            current_offset: 0,
            search_results: Vec::new(),
            current_stock_detail: None,
            summary_stats: None,
            search_query: String::new(),
            status_message: "Loading...".to_string(),
            is_loading: true,
        }
    }

    /// Run the application
    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        io::stdout().execute(EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)?;

        let result = self.run_app(&mut terminal).await;

        // Cleanup
        disable_raw_mode()?;
        io::stdout().execute(LeaveAlternateScreen)?;

        result
    }

    /// Main application loop
    async fn run_app(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        // Initial data load
        self.load_initial_data().await?;

        loop {
            terminal.draw(|f| self.render(f))?;

            if let Event::Key(key) = event::read()? {
                match self.current_view {
                    AppView::Loading => {
                        // No input during loading
                    }
                    AppView::Dashboard => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Char('p') => {
                                self.switch_to_pe_decliners().await?;
                            }
                            KeyCode::Char('s') => {
                                self.current_view = AppView::Search;
                                self.search_query.clear();
                            }
                            KeyCode::Char('r') => {
                                self.refresh_data().await?;
                            }
                            _ => {}
                        }
                    }
                    AppView::PEDecliners => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Char('b') => {
                                self.current_view = AppView::Dashboard;
                            }
                            KeyCode::Down => self.next_item(),
                            KeyCode::Up => self.previous_item(),
                            KeyCode::Enter => {
                                if let Some(selected) = self.list_state.selected() {
                                    if selected < self.pe_decliners.len() {
                                        let symbol = self.pe_decliners[selected].stock.symbol.clone();
                                        self.load_stock_detail(&symbol).await?;
                                    }
                                }
                            }
                            KeyCode::Char('n') => {
                                self.load_next_pe_decliners().await?;
                            }
                            KeyCode::Char('p') => {
                                self.load_previous_pe_decliners().await?;
                            }
                            _ => {}
                        }
                    }
                    AppView::StockDetail(_) => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Char('b') => {
                                self.current_view = AppView::PEDecliners;
                            }
                            _ => {}
                        }
                    }
                    AppView::Search => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                self.current_view = AppView::Dashboard;
                            }
                            KeyCode::Enter => {
                                if !self.search_query.is_empty() {
                                    self.perform_search().await?;
                                }
                            }
                            KeyCode::Backspace => {
                                self.search_query.pop();
                            }
                            KeyCode::Char(c) => {
                                self.search_query.push(c);
                            }
                            KeyCode::Down => self.next_item(),
                            KeyCode::Up => self.previous_item(),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    /// Load initial application data
    async fn load_initial_data(&mut self) -> Result<()> {
        self.is_loading = true;
        self.status_message = "Loading summary statistics...".to_string();

        // Load summary stats
        self.summary_stats = Some(self.analysis_engine.get_summary_stats().await?);
        
        self.is_loading = false;
        self.current_view = AppView::Dashboard;
        self.status_message = "Ready".to_string();
        
        info!("Initial data loaded successfully");
        Ok(())
    }

    /// Switch to P/E decliners view and load data
    async fn switch_to_pe_decliners(&mut self) -> Result<()> {
        self.is_loading = true;
        self.status_message = "Loading P/E decliners...".to_string();
        
        self.current_offset = 0;
        self.pe_decliners = self.analysis_engine.get_top_pe_decliners(10, 0).await?;
        
        self.list_state = ListState::default();
        if !self.pe_decliners.is_empty() {
            self.list_state.select(Some(0));
        }
        
        self.current_view = AppView::PEDecliners;
        self.is_loading = false;
        self.status_message = format!("Showing top {} P/E decliners", self.pe_decliners.len());
        
        Ok(())
    }

    /// Load next page of P/E decliners
    async fn load_next_pe_decliners(&mut self) -> Result<()> {
        self.is_loading = true;
        self.current_offset += 10;
        
        let next_decliners = self.analysis_engine.get_top_pe_decliners(10, self.current_offset).await?;
        
        if !next_decliners.is_empty() {
            self.pe_decliners = next_decliners;
            self.list_state = ListState::default();
            self.list_state.select(Some(0));
            self.status_message = format!("Showing P/E decliners {} - {}", 
                                        self.current_offset + 1, 
                                        self.current_offset + self.pe_decliners.len());
        } else {
            self.current_offset -= 10; // Revert if no more data
            self.status_message = "No more data available".to_string();
        }
        
        self.is_loading = false;
        Ok(())
    }

    /// Load previous page of P/E decliners
    async fn load_previous_pe_decliners(&mut self) -> Result<()> {
        if self.current_offset >= 10 {
            self.is_loading = true;
            self.current_offset -= 10;
            
            self.pe_decliners = self.analysis_engine.get_top_pe_decliners(10, self.current_offset).await?;
            
            self.list_state = ListState::default();
            self.list_state.select(Some(0));
            self.status_message = format!("Showing P/E decliners {} - {}", 
                                        self.current_offset + 1, 
                                        self.current_offset + self.pe_decliners.len());
            self.is_loading = false;
        }
        Ok(())
    }

    /// Load detailed information for a stock
    async fn load_stock_detail(&mut self, symbol: &str) -> Result<()> {
        self.is_loading = true;
        self.status_message = format!("Loading details for {}...", symbol);
        
        self.current_stock_detail = self.analysis_engine.get_stock_details(symbol).await?;
        self.current_view = AppView::StockDetail(symbol.to_string());
        
        self.is_loading = false;
        self.status_message = if self.current_stock_detail.is_some() {
            format!("Showing details for {}", symbol)
        } else {
            format!("No detailed data available for {}", symbol)
        };
        
        Ok(())
    }

    /// Perform stock search
    async fn perform_search(&mut self) -> Result<()> {
        self.is_loading = true;
        self.status_message = format!("Searching for '{}'...", self.search_query);
        
        self.search_results = self.analysis_engine.search_stocks(&self.search_query).await?;
        
        self.list_state = ListState::default();
        if !self.search_results.is_empty() {
            self.list_state.select(Some(0));
        }
        
        self.is_loading = false;
        self.status_message = format!("Found {} results for '{}'", 
                                    self.search_results.len(), 
                                    self.search_query);
        
        Ok(())
    }

    /// Refresh all data
    async fn refresh_data(&mut self) -> Result<()> {
        self.load_initial_data().await
    }

    /// Move to next item in current list
    fn next_item(&mut self) {
        let max_index = match self.current_view {
            AppView::PEDecliners => self.pe_decliners.len(),
            AppView::Search => self.search_results.len(),
            _ => 0,
        };

        if max_index > 0 {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i >= max_index - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        }
    }

    /// Move to previous item in current list
    fn previous_item(&mut self) {
        let max_index = match self.current_view {
            AppView::PEDecliners => self.pe_decliners.len(),
            AppView::Search => self.search_results.len(),
            _ => 0,
        };

        if max_index > 0 {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        max_index - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        }
    }

    /// Render the current view
    fn render(&mut self, f: &mut Frame) {
        match self.current_view {
            AppView::Loading => self.render_loading(f),
            AppView::Dashboard => self.render_dashboard(f),
            AppView::PEDecliners => self.render_pe_decliners(f),
            AppView::StockDetail(_) => self.render_stock_detail(f),
            AppView::Search => self.render_search(f),
        }
    }

    /// Render loading screen
    fn render_loading(&self, f: &mut Frame) {
        let loading = Paragraph::new("Loading...")
            .block(Block::default().borders(Borders::ALL).title("Stock Analysis"))
            .style(Style::default().fg(Color::Yellow));
        
        f.render_widget(loading, f.area());
    }

    /// Render dashboard
    fn render_dashboard(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(f.area());

        // Title
        let title = Paragraph::new("📈 Stock Analysis Dashboard")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(title, chunks[0]);

        // Main content
        let mut content_lines = vec![
            Line::from(""),
            Line::from("📊 System Status:"),
            Line::from(""),
        ];

        if let Some(stats) = &self.summary_stats {
            content_lines.extend(vec![
                Line::from(vec![
                    Span::raw("  📋 Total Stocks: "),
                    Span::styled(stats.total_stocks.to_string(), Style::default().fg(Color::Green)),
                ]),
                Line::from(vec![
                    Span::raw("  💾 Price Records: "),
                    Span::styled(stats.total_price_records.to_string(), Style::default().fg(Color::Green)),
                ]),
                Line::from(vec![
                    Span::raw("  📅 Last Update: "),
                    Span::styled(
                        stats.last_update_date
                            .map(|d| d.to_string())
                            .unwrap_or_else(|| "Never".to_string()),
                        Style::default().fg(Color::Yellow)
                    ),
                ]),
            ]);

            if let Some((symbol, decline)) = &stats.top_pe_decliner {
                content_lines.extend(vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::raw("  📉 Top P/E Decliner: "),
                        Span::styled(symbol.clone(), Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                        Span::raw(" ("),
                        Span::styled(format!("{:.1}%", decline), Style::default().fg(Color::Red)),
                        Span::raw(" decline)"),
                    ]),
                ]);
            }
        }

        content_lines.extend(vec![
            Line::from(""),
            Line::from("🎯 Available Actions:"),
            Line::from(""),
            Line::from("  p - View P/E Ratio Decliners"),
            Line::from("  s - Search Stocks"),
            Line::from("  r - Refresh Data"),
            Line::from("  q - Quit"),
        ]);

        let content = Paragraph::new(content_lines)
            .block(Block::default().borders(Borders::ALL))
            .wrap(ratatui::widgets::Wrap { trim: true });
        f.render_widget(content, chunks[1]);

        // Status bar
        let status = Paragraph::new(self.status_message.as_str())
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(status, chunks[2]);
    }

    /// Render P/E decliners list
    fn render_pe_decliners(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(f.area());

        // Title
        let title = Paragraph::new("📉 Top P/E Ratio Decliners")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Red));
        f.render_widget(title, chunks[0]);

        // P/E decliners list
        let items: Vec<ListItem> = self.pe_decliners
            .iter()
            .enumerate()
            .map(|(i, analysis)| {
                let content = vec![
                    Line::from(vec![
                        Span::styled(format!("{}. ", i + 1 + self.current_offset), Style::default().fg(Color::Blue)),
                        Span::styled(&analysis.stock.symbol, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                        Span::raw(" - "),
                        Span::raw(&analysis.stock.company_name),
                    ]),
                    Line::from(vec![
                        Span::raw("   P/E Decline: "),
                        Span::styled(format!("{:.1}%", analysis.pe_decline_percent), Style::default().fg(Color::Red)),
                        Span::raw(" | Price: $"),
                        Span::styled(format!("{:.2}", analysis.current_price), Style::default().fg(Color::Green)),
                        Span::raw(" ("),
                        Span::styled(
                            format!("{:+.1}%", analysis.price_change_percent),
                            if analysis.price_change_percent >= 0.0 { 
                                Style::default().fg(Color::Green) 
                            } else { 
                                Style::default().fg(Color::Red) 
                            }
                        ),
                        Span::raw(")"),
                    ]),
                ];
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(Style::default().bg(Color::LightBlue).fg(Color::Black).add_modifier(Modifier::BOLD))
            .highlight_symbol("→ ");

        f.render_stateful_widget(list, chunks[1], &mut self.list_state);

        // Controls
        let controls = Paragraph::new("🎮 ↑/↓: Navigate | Enter: View Details | n: Next Page | p: Previous Page | b: Back | q: Quit")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(controls, chunks[2]);
    }

    /// Render stock detail view
    fn render_stock_detail(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(f.area());

        // Title
        let title = if let AppView::StockDetail(ref symbol) = self.current_view {
            format!("📋 Stock Details - {}", symbol)
        } else {
            "📋 Stock Details".to_string()
        };

        let title_widget = Paragraph::new(title)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(title_widget, chunks[0]);

        // Stock details
        if let Some(detail) = &self.current_stock_detail {
            let mut content_lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("Company: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&detail.stock.company_name),
                ]),
                Line::from(vec![
                    Span::styled("Symbol: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&detail.stock.symbol),
                ]),
            ];

            if let Some(sector) = &detail.stock.sector {
                content_lines.push(Line::from(vec![
                    Span::styled("Sector: ", Style::default().fg(Color::Yellow)),
                    Span::raw(sector),
                ]));
            }

            content_lines.extend(vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("Current Price: $", Style::default().fg(Color::Green)),
                    Span::styled(format!("{:.2}", detail.current_price.close_price), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                ]),
            ]);

            if let Some(pe_ratio) = detail.current_price.pe_ratio {
                content_lines.push(Line::from(vec![
                    Span::styled("P/E Ratio: ", Style::default().fg(Color::Blue)),
                    Span::styled(format!("{:.2}", pe_ratio), Style::default().fg(Color::Blue)),
                ]));
            }

            if let Some(volume) = detail.current_price.volume {
                content_lines.push(Line::from(vec![
                    Span::styled("Volume: ", Style::default().fg(Color::Magenta)),
                    Span::styled(format!("{}", volume), Style::default().fg(Color::Magenta)),
                ]));
            }

            content_lines.extend(vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("Price History: ", Style::default().fg(Color::Yellow)),
                    Span::raw(format!("{} days of data", detail.price_history.len())),
                ]),
                Line::from(vec![
                    Span::styled("P/E Trend: ", Style::default().fg(Color::Yellow)),
                    Span::raw(format!("{} data points", detail.pe_trend.len())),
                ]),
            ]);

            let details_paragraph = Paragraph::new(content_lines)
                .block(Block::default().borders(Borders::ALL))
                .wrap(ratatui::widgets::Wrap { trim: true });
            f.render_widget(details_paragraph, chunks[1]);
        } else {
            let no_data = Paragraph::new("No detailed data available for this stock.")
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Red));
            f.render_widget(no_data, chunks[1]);
        }

        // Controls
        let controls = Paragraph::new("🎮 b: Back | q: Quit")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(controls, chunks[2]);
    }

    /// Render search interface
    fn render_search(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(f.area());

        // Title
        let title = Paragraph::new("🔍 Stock Search")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Green));
        f.render_widget(title, chunks[0]);

        // Search input
        let search_input = Paragraph::new(self.search_query.as_str())
            .block(Block::default().borders(Borders::ALL).title("Search Query"))
            .style(Style::default().fg(Color::White));
        f.render_widget(search_input, chunks[1]);

        // Search results
        if !self.search_results.is_empty() {
            let items: Vec<ListItem> = self.search_results
                .iter()
                .map(|stock| {
                    let content = vec![
                        Line::from(vec![
                            Span::styled(&stock.symbol, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                            Span::raw(" - "),
                            Span::raw(&stock.company_name),
                        ]),
                    ];
                    ListItem::new(content)
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Search Results"))
                .highlight_style(Style::default().bg(Color::LightBlue).fg(Color::Black).add_modifier(Modifier::BOLD))
                .highlight_symbol("→ ");

            f.render_stateful_widget(list, chunks[2], &mut self.list_state);
        } else if !self.search_query.is_empty() {
            let no_results = Paragraph::new("No results found. Try a different search term.")
                .block(Block::default().borders(Borders::ALL).title("Search Results"))
                .style(Style::default().fg(Color::Red));
            f.render_widget(no_results, chunks[2]);
        } else {
            let help = Paragraph::new("Enter a stock symbol (e.g., AAPL) or company name (e.g., Apple)")
                .block(Block::default().borders(Borders::ALL).title("Search Results"))
                .style(Style::default().fg(Color::Gray));
            f.render_widget(help, chunks[2]);
        }

        // Controls
        let controls = Paragraph::new("🎮 Type to search | Enter: Search | ↑/↓: Navigate results | Esc: Back")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(controls, chunks[3]);
    }
}