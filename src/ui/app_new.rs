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
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::{
    database_sqlx::DatabaseManagerSqlx,
    models::Config,
    ui::{
        View, ViewManager, TuiLayout,
        data_collection_new::DataCollectionView,
        data_analysis_new::DataAnalysisView,
        state::{AsyncStateManager, LogLevel, StateUpdate},
    },
};

/// Refactored StockTuiApp using the new architecture
pub struct StockTuiApp {
    pub should_quit: bool,
    pub view_manager: ViewManager,
    pub global_state_manager: AsyncStateManager,
    pub database: Arc<DatabaseManagerSqlx>,
    pub log_sender: broadcast::Sender<StateUpdate>,
    pub log_receiver: broadcast::Receiver<StateUpdate>,
}

impl StockTuiApp {
    pub fn new(config: &Config, database: DatabaseManagerSqlx) -> Result<Self> {
        // Create global state manager
        let global_state_manager = AsyncStateManager::new();
        let log_sender = global_state_manager.get_broadcast_sender();
        let log_receiver = global_state_manager.subscribe();
        
        // Create view manager
        let mut view_manager = ViewManager::new();
        
        // Create views
        let mut data_collection_view = DataCollectionView::new();
        let mut data_analysis_view = DataAnalysisView::new();
        
        // Set up database references
        let database_arc = Arc::new(database);
        data_analysis_view.set_database(database_arc.clone());
        
        // Add views to view manager
        view_manager.add_view(Box::new(data_collection_view));
        view_manager.add_view(Box::new(data_analysis_view));
        
        // Set initial view to data collection
        view_manager.switch_to_view(0)?;

        Ok(Self {
            should_quit: false,
            view_manager,
            global_state_manager,
            database: database_arc,
            log_sender,
            log_receiver,
        })
    }

    pub fn draw(&mut self, f: &mut Frame) {
        // Process any new state updates from broadcast channel
        while let Ok(update) = self.log_receiver.try_recv() {
            // Broadcast to all views
            if let Some(current_view) = self.view_manager.get_current_view_mut() {
                let _ = current_view.handle_state_update(&update);
            }
        }
        
        // Use centralized layout management
        let layout = TuiLayout::new(f.area());
        
        // Render tab bar
        self.render_tab_bar(f, layout.tab_bar);
        
        // Render current view using centralized layout
        if let Some(current_view) = self.view_manager.get_current_view() {
            current_view.render(f, layout.content);
        }

        // Render status bar
        self.render_status_bar(f, layout.status_bar);
    }

    fn render_tab_bar(&self, f: &mut Frame, area: Rect) {
        let titles = self.view_manager.get_view_titles();
        let current_index = self.view_manager.current_view_index;
        
        let tabs = ratatui::widgets::Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("Stock Analysis System"))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .select(current_index);
            
        f.render_widget(tabs, area);
    }

    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let status_text = if let Some(current_view) = self.view_manager.get_current_view() {
            current_view.get_status()
        } else {
            "No view available".to_string()
        };
        
        let status_lines = vec![
            Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::Gray)),
                Span::styled("Tab", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(" to switch views • ", Style::default().fg(Color::Gray)),
                Span::styled("Q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled(" to quit", Style::default().fg(Color::Gray)),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Gray)),
                Span::styled(status_text, Style::default().fg(Color::White)),
            ]),
        ];
        
        let paragraph = Paragraph::new(status_lines)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        
        f.render_widget(paragraph, area);
    }

    pub async fn handle_key_event(&mut self, key: KeyCode) -> Result<()> {
        // Handle global app events first
        match key {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.should_quit = true;
                return Ok(());
            }
            KeyCode::Tab => {
                self.next_view();
                return Ok(());
            }
            KeyCode::BackTab => {
                self.previous_view();
                return Ok(());
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.refresh_current_view();
                return Ok(());
            }
            _ => {}
        }
        
        // Route key events to the current view
        if let Some(current_view) = self.view_manager.get_current_view_mut() {
            let _ = current_view.handle_key(key);
        }
        
        Ok(())
    }

    fn next_view(&mut self) {
        let current_index = self.view_manager.current_view_index;
        let next_index = (current_index + 1) % self.view_manager.view_count();
        let _ = self.view_manager.switch_to_view(next_index);
    }

    fn previous_view(&mut self) {
        let current_index = self.view_manager.current_view_index;
        let prev_index = if current_index == 0 {
            self.view_manager.view_count() - 1
        } else {
            current_index - 1
        };
        let _ = self.view_manager.switch_to_view(prev_index);
    }

    fn refresh_current_view(&mut self) {
        // Update current view
        if let Some(current_view) = self.view_manager.get_current_view_mut() {
            let _ = current_view.update();
        }
        
        // Process global state updates
        self.global_state_manager.process_updates();
    }
}

/// Run the async TUI application with new architecture
pub async fn run_app_async(config: Config, database: DatabaseManagerSqlx) -> Result<()> {
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
                app.handle_key_event(key_event.code).await?;
                
                if app.should_quit {
                    break;
                }
            }
        }
        
        // Update views
        app.refresh_current_view();
        
        // Draw the UI
        terminal.draw(|f| app.draw(f))?;
    }
    
    // Cleanup
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    
    Ok(())
}
