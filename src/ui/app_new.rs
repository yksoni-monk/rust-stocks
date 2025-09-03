use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    prelude::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame, Terminal,
};
use std::io;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::{
    database_sqlx::DatabaseManagerSqlx,
    models::Config,
    ui::{
        data_collection_new::DataCollectionView,
        data_analysis_new::DataAnalysisView,
        layout::TuiLayout,
        state::{AsyncStateManager, StateUpdate},
        View,
    },
};

/// Main TUI application with simplified architecture
pub struct StockTuiApp {
    pub should_quit: bool,
    pub current_view: usize, // 0 = data collection, 1 = data analysis
    pub data_collection_view: DataCollectionView,
    pub data_analysis_view: DataAnalysisView,
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
        
        // Create views
        let mut data_collection_view = DataCollectionView::new();
        let mut data_analysis_view = DataAnalysisView::new();
        
        // Set up database references
        let database_arc = Arc::new(database);
        data_collection_view.set_database(database_arc.clone());
        data_analysis_view.set_database(database_arc.clone());
        
        // Set the global state manager for the views
        data_collection_view.set_state_manager(global_state_manager.clone());
        data_analysis_view.set_state_manager(global_state_manager.clone());
        
        // Set the global broadcast sender for the views
        data_collection_view.set_global_broadcast_sender(log_sender.clone());
        data_analysis_view.set_global_broadcast_sender(log_sender.clone());

        Ok(Self {
            should_quit: false,
            current_view: 0, // Start with data collection
            data_collection_view,
            data_analysis_view,
            global_state_manager,
            database: database_arc,
            log_sender,
            log_receiver,
        })
    }

    pub fn draw(&mut self, f: &mut Frame) {
        // Process any new state updates from broadcast channel
        while let Ok(update) = self.log_receiver.try_recv() {
            // Debug log the received update
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("debug_tui.log") 
            {
                let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                let _ = writeln!(file, "[{}] App received state update: {:?}", timestamp, update);
            }
            
            // Broadcast to appropriate views based on update type
            match &update {
                crate::ui::state::StateUpdate::LogMessage { .. } => {
                    // LogMessage events should only go to the current view to avoid duplicates
                    match self.current_view {
                        0 => {
                            if let Ok(result) = self.data_collection_view.handle_state_update(&update) {
                                if let Ok(mut file) = std::fs::OpenOptions::new()
                                    .create(true)
                                    .append(true)
                                    .open("debug_tui.log") 
                                {
                                    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                                    let _ = writeln!(file, "[{}] Data collection view handled LogMessage, result: {}", timestamp, result);
                                }
                            }
                        }
                        1 => {
                            if let Ok(result) = self.data_analysis_view.handle_state_update(&update) {
                                if let Ok(mut file) = std::fs::OpenOptions::new()
                                    .create(true)
                                    .append(true)
                                    .open("debug_tui.log") 
                                {
                                    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                                    let _ = writeln!(file, "[{}] Data analysis view handled LogMessage, result: {}", timestamp, result);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                crate::ui::state::StateUpdate::StockListUpdated { .. } => {
                    // StockListUpdated should go to both views since both need the stock data
                    if let Ok(result) = self.data_collection_view.handle_state_update(&update) {
                        if let Ok(mut file) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("debug_tui.log") 
                        {
                            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                            let _ = writeln!(file, "[{}] Data collection view handled StockListUpdated, result: {}", timestamp, result);
                        }
                    }
                    if let Ok(result) = self.data_analysis_view.handle_state_update(&update) {
                        if let Ok(mut file) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("debug_tui.log") 
                        {
                            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                            let _ = writeln!(file, "[{}] Data analysis view handled StockListUpdated, result: {}, available_stocks count: {}", 
                                          timestamp, result, self.data_analysis_view.available_stocks.len());
                        }
                    }
                }
                _ => {
                    // Other updates go only to the current view
                    match self.current_view {
                        0 => {
                            if let Ok(result) = self.data_collection_view.handle_state_update(&update) {
                                if let Ok(mut file) = std::fs::OpenOptions::new()
                                    .create(true)
                                    .append(true)
                                    .open("debug_tui.log") 
                                {
                                    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                                    let _ = writeln!(file, "[{}] Data collection view handled update, result: {}", timestamp, result);
                                }
                            }
                        }
                        1 => {
                            if let Ok(result) = self.data_analysis_view.handle_state_update(&update) {
                                if let Ok(mut file) = std::fs::OpenOptions::new()
                                    .create(true)
                                    .append(true)
                                    .open("debug_tui.log") 
                                {
                                    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                                    let _ = writeln!(file, "[{}] Data analysis view handled update, result: {}, available_stocks count: {}", 
                                                  timestamp, result, self.data_analysis_view.available_stocks.len());
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        
        // Use centralized layout management
        let layout = TuiLayout::new(f.area());
        
        // Render tab bar
        self.render_tab_bar(f, layout.tab_bar);
        
        // Render current view using centralized layout
        match self.current_view {
            0 => {
                self.data_collection_view.render(f, layout.content);
            }
            1 => {
                self.data_analysis_view.render(f, layout.content);
            }
            _ => {}
        }

        // Render status bar
        self.render_status_bar(f, layout.status_bar);
    }

    fn render_tab_bar(&self, f: &mut Frame, area: Rect) {
        let titles = vec!["Data Collection", "Data Analysis"];
        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("Stock Analysis System"))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .select(self.current_view);
            
        f.render_widget(tabs, area);
    }

    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let status_text = match self.current_view {
            0 => self.data_collection_view.get_status(),
            1 => self.data_analysis_view.get_status(),
            _ => "No view available".to_string(),
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
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("debug_tui.log") 
                {
                    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                    let _ = writeln!(file, "[{}] Quit key pressed", timestamp);
                }
                self.should_quit = true;
                return Ok(());
            }
            KeyCode::Tab => {
                let old_view = self.current_view;
                self.current_view = (self.current_view + 1) % 2;
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("debug_tui.log") 
                {
                    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                    let _ = writeln!(file, "[{}] Tab pressed - switching from view {} to view {}", timestamp, old_view, self.current_view);
                }
                return Ok(());
            }
            KeyCode::BackTab => {
                let old_view = self.current_view;
                self.current_view = if self.current_view == 0 { 1 } else { 0 };
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("debug_tui.log") 
                {
                    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                    let _ = writeln!(file, "[{}] BackTab pressed - switching from view {} to view {}", timestamp, old_view, self.current_view);
                }
                return Ok(());
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("debug_tui.log") 
                {
                    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                    let _ = writeln!(file, "[{}] Refresh key pressed", timestamp);
                }
                self.refresh_current_view();
                return Ok(());
            }
            _ => {}
        }
        
        // Debug log current view and key routing
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("debug_tui.log") 
        {
            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let _ = writeln!(file, "[{}] App routing key {:?} to view {} (current_view value confirmed)", timestamp, key, self.current_view);
        }
        
        // Route key events to the current view
        match self.current_view {
            0 => {
                if let Ok(result) = self.data_collection_view.handle_key(key) {
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("debug_tui.log") 
                    {
                        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                        let _ = writeln!(file, "[{}] Data collection view handled key, result: {}", timestamp, result);
                    }
                }
            }
            1 => {
                match self.data_analysis_view.handle_key(key) {
                    Ok(result) => {
                        if let Ok(mut file) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("debug_tui.log") 
                        {
                            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                            let _ = writeln!(file, "[{}] Data analysis view handled key, result: {}", timestamp, result);
                        }
                    }
                    Err(e) => {
                        if let Ok(mut file) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("debug_tui.log") 
                        {
                            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                            let _ = writeln!(file, "[{}] Data analysis view ERROR handling key: {}", timestamp, e);
                        }
                    }
                }
            }
            _ => {}
        }
        
        Ok(())
    }

    fn refresh_current_view(&mut self) {
        // Update current view
        match self.current_view {
            0 => {
                let _ = self.data_collection_view.update();
            }
            1 => {
                let _ = self.data_analysis_view.update();
            }
            _ => {}
        }
        
        // Process global state updates
        self.global_state_manager.process_updates();
    }
}

/// Run the async TUI application with simplified architecture
pub async fn run_app_async(config: Config, database: DatabaseManagerSqlx) -> Result<()> {
    // Debug log function entry - truncate file to start fresh
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("debug_tui.log") 
    {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(file, "[{}] run_app_async starting", timestamp);
    }
    
    let mut app = StockTuiApp::new(&config, database)?;
    
    // Debug log after app creation
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("debug_tui.log") 
    {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(file, "[{}] StockTuiApp created", timestamp);
    }
    
    // Enable raw mode
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("debug_tui.log") 
    {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(file, "[{}] Enabling raw mode", timestamp);
    }
    
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("debug_tui.log") 
    {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(file, "[{}] Terminal setup complete", timestamp);
    }
    
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Debug log that the event loop is starting
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("debug_tui.log") 
    {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(file, "[{}] Event loop starting", timestamp);
    }

    // Main async event loop with proper event handling
    let mut loop_iteration = 0;
    loop {
        loop_iteration += 1;
        
        // Debug log every 100 iterations to show the loop is running
        if loop_iteration % 100 == 0 {
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("debug_tui.log") 
            {
                let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                let _ = writeln!(file, "[{}] Event loop iteration {}", timestamp, loop_iteration);
            }
        }
        
        // Process all available events without blocking
        let mut events_processed = false;
        
        // Process multiple events in quick succession to handle rapid key presses
        let mut event_count = 0;
        const MAX_EVENTS_PER_CYCLE: usize = 10; // Limit to prevent blocking
        
        while event_count < MAX_EVENTS_PER_CYCLE && crossterm::event::poll(std::time::Duration::from_millis(0))? {
            // Debug log that we found an event
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("debug_tui.log") 
            {
                let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                let _ = writeln!(file, "[{}] Event detected, reading...", timestamp);
            }
            if let Event::Key(key_event) = event::read()? {
                // Debug log key events
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("debug_tui.log") 
                {
                    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                    let _ = writeln!(file, "[{}] Key event: {:?}", timestamp, key_event.code);
                }
                
                app.handle_key_event(key_event.code).await?;
                events_processed = true;
                event_count += 1;
                
                if app.should_quit {
                    break;
                }
            }
        }
        
        // Check if we should quit after processing events
        if app.should_quit {
            break;
        }
        
        // Always refresh views and draw UI to ensure responsiveness
        app.refresh_current_view();
        terminal.draw(|f| app.draw(f))?;
        
        // Small delay to maintain reasonable frame rate
        // Shorter delay when events were processed for better responsiveness
        let delay_ms = if events_processed { 8 } else { 16 };
        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
    }
    
    // Cleanup
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    
    Ok(())
}
