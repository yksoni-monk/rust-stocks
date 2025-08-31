use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame, Terminal,
};
use std::io;

use crate::{
    database::DatabaseManager,
    models::Config,
    ui::{dashboard::Dashboard, data_collection::DataCollectionView},
};

pub struct StockTuiApp {
    pub selected_tab: usize,
    pub should_quit: bool,
    pub dashboard: Dashboard,
    pub data_collection: DataCollectionView,
    pub database: DatabaseManager,
}

impl StockTuiApp {
    pub fn new(config: &Config) -> Result<Self> {
        let database = DatabaseManager::new(&config.database_path)?;
        let dashboard = Dashboard::new();
        let data_collection = DataCollectionView::new();

        Ok(Self {
            selected_tab: 0,
            should_quit: false,
            dashboard,
            data_collection,
            database,
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
            0 => self.data_collection.render(f, chunks[1]),
            1 => self.render_data_analysis_view(f, chunks[1]),
            _ => self.data_collection.render(f, chunks[1]),
        }

        // Render status bar
        self.render_status_bar(f, chunks[2]);
    }

    fn render_tab_bar(&self, f: &mut Frame, area: Rect) {
        let titles = vec![
            "Data Collection",
            "Data Analysis",
        ];
        
        let tabs = ratatui::widgets::Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("Stock Analysis System"))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .select(self.selected_tab);
            
        f.render_widget(tabs, area);
    }

    fn render_data_analysis_view(&self, f: &mut Frame, area: Rect) {
        let paragraph = Paragraph::new(vec![
            Line::from("📊 Data Analysis"),
            Line::from(""),
            Line::from("Category: Data Analysis"),
            Line::from(""),
            Line::from("1. View data for any stock (latest, date range etc)"),
            Line::from("2. Analyse data (e.g. show top 10 stocks with largest % fall in P/E ratio)"),
            Line::from("3. Show stock with largest growth for a date range"),
            Line::from(""),
            Line::from("Coming Soon: Advanced analytics and stock screening"),
        ])
        .block(Block::default()
            .borders(Borders::ALL)
            .title("📊 Data Analysis"))
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
        // If we're in data collection view and it's executing, let it handle the event
        if self.selected_tab == 0 && self.data_collection.is_executing {
            return self.data_collection.handle_key_event(key);
        }

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
            _ => {
                // If we're in data collection view, let it handle other keys
                if self.selected_tab == 0 {
                    self.data_collection.handle_key_event(key)?;
                }
            }
        }
        Ok(())
    }

    fn next_tab(&mut self) {
        self.selected_tab = (self.selected_tab + 1) % 2;
    }

    fn previous_tab(&mut self) {
        self.selected_tab = if self.selected_tab == 0 { 1 } else { 0 };
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
    execute!(io::stdout(), EnterAlternateScreen)?;
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
            if let Err(e) = app.handle_key_event(key.code) {
                break Err(e);
            }

            if app.should_quit {
                break Ok(());
            }
        }
    };

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    result
}