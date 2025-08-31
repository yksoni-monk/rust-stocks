use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal, Frame,
};
use std::io;

use crate::models::Config;
use crate::database::DatabaseManager;
use super::dashboard::Dashboard;

pub struct StockTuiApp {
    pub database: DatabaseManager,
    pub dashboard: Dashboard,
    pub should_quit: bool,
    pub selected_tab: usize,
}

impl StockTuiApp {
    pub fn new(config: &Config) -> Result<Self> {
        let database = DatabaseManager::new(&config.database_path)?;
        
        Ok(Self {
            database,
            dashboard: Dashboard::new(),
            should_quit: false,
            selected_tab: 0,
        })
    }

    pub fn draw(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tab bar
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Status bar
            ])
            .split(f.area());

        // Render tab bar
        self.render_tab_bar(f, chunks[0]);
        
        // Render content based on selected tab
        match self.selected_tab {
            0 => self.dashboard.render(f, chunks[1]),
            1 => self.render_data_collection_view(f, chunks[1]),
            2 => self.render_stock_analysis_view(f, chunks[1]),
            3 => self.render_progress_analyzer_view(f, chunks[1]),
            4 => self.render_settings_view(f, chunks[1]),
            _ => self.dashboard.render(f, chunks[1]),
        }

        // Render status bar
        self.render_status_bar(f, chunks[2]);
    }

    fn render_tab_bar(&self, f: &mut Frame, area: Rect) {
        let titles = vec![
            "Dashboard",
            "Data Collection", 
            "Stock Analysis",
            "Progress",
            "Settings"
        ];
        
        let tabs = ratatui::widgets::Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("Stock Analysis System"))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .select(self.selected_tab);
            
        f.render_widget(tabs, area);
    }

    fn render_data_collection_view(&self, f: &mut Frame, area: Rect) {
        let items = vec![
            ListItem::new("📋 Update S&P 500 company list"),
            ListItem::new("📈 Collect historical data (2020-2025)"),
            ListItem::new("🔄 Incremental daily updates"),
            ListItem::new("📊 Validate data integrity"),
            ListItem::new(""),
            ListItem::new("Instructions:"),
            ListItem::new("• Run: cargo run --bin update_sp500"),
            ListItem::new("• Run: cargo run --bin collect_with_detailed_logs -- -s 20240101"),
        ];
        
        let list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("🚀 Data Collection"))
            .style(Style::default().fg(Color::White));
            
        f.render_widget(list, area);
    }

    fn render_stock_analysis_view(&self, f: &mut Frame, area: Rect) {
        let paragraph = Paragraph::new(vec![
            Line::from("🔍 Stock Analysis Features"),
            Line::from(""),
            Line::from("• P/E Ratio Analysis"),
            Line::from("• Price Performance Tracking"), 
            Line::from("• Market Trend Analysis"),
            Line::from("• Portfolio Optimization"),
            Line::from(""),
            Line::from("Coming Soon: Advanced analytics and stock screening"),
        ])
        .block(Block::default()
            .borders(Borders::ALL)
            .title("📊 Stock Analysis"))
        .style(Style::default().fg(Color::White));
        
        f.render_widget(paragraph, area);
    }

    fn render_progress_analyzer_view(&self, f: &mut Frame, area: Rect) {
        let paragraph = Paragraph::new(vec![
            Line::from("📈 Progress Analysis"),
            Line::from(""),
            Line::from("• Data collection completion: In Progress"),
            Line::from("• Target: All S&P 500 companies"),
            Line::from("• Date range: Jan 1, 2020 - Present"),
            Line::from("• Expected records: ~1.5M price points"),
            Line::from(""),
            Line::from("Status will be shown here once data collection starts"),
        ])
        .block(Block::default()
            .borders(Borders::ALL) 
            .title("🎯 Progress Analyzer"))
        .style(Style::default().fg(Color::White));
        
        f.render_widget(paragraph, area);
    }

    fn render_settings_view(&self, f: &mut Frame, area: Rect) {
        let paragraph = Paragraph::new(vec![
            Line::from("⚙️ System Settings"),
            Line::from(""),
            Line::from("• Schwab API Configuration"),
            Line::from("• Database Settings"),
            Line::from("• Collection Parameters"),
            Line::from("• Display Preferences"),
            Line::from(""),
            Line::from("Configuration files:"),
            Line::from("• .env - API credentials"),
            Line::from("• stocks.db - SQLite database"),
        ])
        .block(Block::default()
            .borders(Borders::ALL)
            .title("⚙️ Settings"))
        .style(Style::default().fg(Color::White));
        
        f.render_widget(paragraph, area);
    }

    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let status_text = vec![
            Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::Gray)),
                Span::styled("Tab", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(" to switch views • ", Style::default().fg(Color::Gray)),
                Span::styled("R", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled(" to refresh • ", Style::default().fg(Color::Gray)),
                Span::styled("Q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled(" to quit", Style::default().fg(Color::Gray)),
            ]),
        ];
        
        let paragraph = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        
        f.render_widget(paragraph, area);
    }

    pub fn handle_key_event(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.should_quit = true;
            }
            KeyCode::Tab => {
                self.next_tab();
            }
            KeyCode::BackTab => {
                self.previous_tab(); 
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.refresh_data()?;
            }
            KeyCode::Char('1') => {
                self.select_tab(0);
            }
            KeyCode::Char('2') => {
                self.select_tab(1);
            }
            KeyCode::Char('3') => {
                self.select_tab(2);
            }
            KeyCode::Char('4') => {
                self.select_tab(3);
            }
            KeyCode::Char('5') => {
                self.select_tab(4);
            }
            _ => {}
        }
        Ok(())
    }

    fn next_tab(&mut self) {
        self.selected_tab = (self.selected_tab + 1) % 5;
    }

    fn previous_tab(&mut self) {
        self.selected_tab = if self.selected_tab == 0 { 4 } else { self.selected_tab - 1 };
    }

    fn select_tab(&mut self, tab: usize) {
        if tab < 5 {
            self.selected_tab = tab;
        }
    }

    fn refresh_data(&mut self) -> Result<()> {
        // Try to refresh dashboard data, ignore errors
        let _ = self.dashboard.refresh_data(&self.database);
        Ok(())
    }
}

/// Run the main TUI application
pub fn run_app() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    
    // Load configuration and create app
    let config = Config::from_env()?;
    let mut app = StockTuiApp::new(&config)?;

    // Initial data refresh
    let _ = app.refresh_data();

    // Main application loop
    let result = loop {
        // Draw the UI
        if let Err(e) = terminal.draw(|f| app.draw(f)) {
            break Err(e.into());
        }

        // Handle events
        if let Ok(Event::Key(key)) = event::read() {
            if key.kind == KeyEventKind::Press {
                if let Err(e) = app.handle_key_event(key.code) {
                    break Err(e);
                }

                if app.should_quit {
                    break Ok(());
                }
            }
        }
    };

    // Cleanup terminal
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    result
}