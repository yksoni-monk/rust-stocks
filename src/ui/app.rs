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
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::io;
use tokio::sync::broadcast;
use chrono::{DateTime, Utc};

use crate::{
    database::DatabaseManager,
    models::Config,
    ui::{dashboard::Dashboard, data_collection::DataCollectionView, data_analysis::DataAnalysisView},
};

/// Log message for broadcast channel
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

pub struct StockTuiApp {
    pub selected_tab: usize,
    pub should_quit: bool,
    pub dashboard: Dashboard,
    pub data_collection: DataCollectionView,
    pub data_analysis: DataAnalysisView,
    pub database: DatabaseManager,
    pub log_sender: broadcast::Sender<LogMessage>,
    pub log_receiver: broadcast::Receiver<LogMessage>,
}

impl StockTuiApp {
    pub fn new(_config: &Config, database: DatabaseManager) -> Result<Self> {
        let dashboard = Dashboard::new();
        let data_collection = DataCollectionView::new();
        let data_analysis = DataAnalysisView::new();
        
        // Create broadcast channel for logs
        let (log_sender, log_receiver) = broadcast::channel::<LogMessage>(100);

        Ok(Self {
            selected_tab: 0,
            should_quit: false,
            dashboard,
            data_collection,
            data_analysis,
            database,
            log_sender,
            log_receiver,
        })
    }

    pub fn draw(&mut self, f: &mut Frame) {
        // Process any new log messages from broadcast channel
        while let Ok(log_msg) = self.log_receiver.try_recv() {
            self.data_collection.add_log_message_from_broadcast(log_msg);
        }
        
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
            1 => self.data_analysis.render(f, chunks[1]),
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

    #[allow(dead_code)]
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
            return self.data_collection.handle_key_event(key, self.log_sender.clone());
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
                // Route key events to the appropriate view
                match self.selected_tab {
                    0 => self.data_collection.handle_key_event(key, self.log_sender.clone())?,
                    1 => self.data_analysis.handle_key_event(key, &self.database)?,
                    _ => {}
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
        
        // Load available stocks for data analysis
        let _ = self.data_analysis.load_available_stocks(&self.database);
        
        Ok(())
    }
}



/// Run the async TUI application
pub async fn run_app_async(config: Config, database: DatabaseManager) -> Result<()> {
    let mut app = StockTuiApp::new(&config, database)?;
    
    // Enable raw mode
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Main async event loop
    loop {
        // Handle terminal events with timeout
        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key_event) = event::read()? {
                app.handle_key_event(key_event.code)?;
                
                if app.should_quit {
                    break;
                }
            }
        }
        
        // Draw the UI
        terminal.draw(|f| app.draw(f))?;
    }
    
    // Cleanup
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    
    Ok(())
}