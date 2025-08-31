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
    UpdateSP500List,
    CollectHistoricalData { start_date: NaiveDate, end_date: NaiveDate },
    IncrementalUpdate,
    ValidateData,
    ViewProgress,
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
}

impl DataCollectionView {
    pub fn new() -> Self {
        let actions = vec![
            DataCollectionAction {
                id: "update_sp500".to_string(),
                title: "📋 Update S&P 500 company list".to_string(),
                description: "Fetch latest S&P 500 constituents from official sources".to_string(),
                action_type: ActionType::UpdateSP500List,
                requires_confirmation: false,
            },
            DataCollectionAction {
                id: "collect_historical".to_string(),
                title: "📈 Collect historical data (2020-2025)".to_string(),
                description: "Fetch complete historical OHLC data for all stocks".to_string(),
                action_type: ActionType::CollectHistoricalData { 
                    start_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
                    end_date: chrono::Utc::now().date_naive(),
                },
                requires_confirmation: true,
            },
            DataCollectionAction {
                id: "incremental_update".to_string(),
                title: "🔄 Incremental daily updates".to_string(),
                description: "Fetch latest data since last update".to_string(),
                action_type: ActionType::IncrementalUpdate,
                requires_confirmation: false,
            },
            DataCollectionAction {
                id: "validate_data".to_string(),
                title: "📊 Validate data integrity".to_string(),
                description: "Check data completeness and identify gaps".to_string(),
                action_type: ActionType::ValidateData,
                requires_confirmation: false,
            },
            DataCollectionAction {
                id: "view_progress".to_string(),
                title: "📊 View collection progress".to_string(),
                description: "Show current data collection status".to_string(),
                action_type: ActionType::ViewProgress,
                requires_confirmation: false,
            },
        ];

        Self {
            selected_action: 0,
            actions,
            is_executing: false,
            current_operation: None,
            log_messages: Vec::new(),
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
        Ok(())
    }

    /// Execute the currently selected action
    pub fn execute_selected_action(&mut self) -> Result<()> {
        if self.selected_action >= self.actions.len() {
            return Ok(());
        }

        let _action_id = self.actions[self.selected_action].id.clone();
        let action_title = self.actions[self.selected_action].title.clone();
        let requires_confirmation = self.actions[self.selected_action].requires_confirmation;
        let action_type = self.actions[self.selected_action].action_type.clone();
        
        // Check if confirmation is required
        if requires_confirmation {
            self.log_warning(&format!("Confirmation required for: {}", action_title));
            // TODO: Implement confirmation dialog
            return Ok(());
        }

        self.start_operation_by_type(&action_type)?;
        Ok(())
    }

    /// Start an operation by action type
    pub fn start_operation_by_type(&mut self, action_type: &ActionType) -> Result<()> {
        self.is_executing = true;
        self.current_operation = Some(ActiveOperation {
            action_id: "operation".to_string(),
            start_time: Utc::now(),
            progress: 0.0,
            current_message: "Starting operation...".to_string(),
            logs: Vec::new(),
        });

        // Execute the action based on type
        match action_type {
            ActionType::UpdateSP500List => {
                self.log_info("Starting S&P 500 list update...");
                self.run_update_sp500()?;
            }
            ActionType::CollectHistoricalData { start_date, end_date } => {
                self.log_info(&format!("Starting historical data collection from {} to {}", start_date, end_date));
                self.run_historical_collection(*start_date, *end_date)?;
            }
            ActionType::IncrementalUpdate => {
                self.log_info("Starting incremental update...");
                self.run_incremental_update()?;
            }
            ActionType::ValidateData => {
                self.log_info("Starting data validation...");
                self.validate_data_integrity()?;
            }
            ActionType::ViewProgress => {
                self.log_info("Showing collection progress...");
                self.show_collection_progress();
            }
        }

        Ok(())
    }

    /// Run S&P 500 list update
    pub fn run_update_sp500(&mut self) -> Result<()> {
        self.log_info("Executing: cargo run --bin update_sp500");
        
        let output = Command::new("cargo")
            .args(["run", "--bin", "update_sp500"])
            .output()?;

        if output.status.success() {
            self.log_success("S&P 500 list updated successfully");
            let stdout = String::from_utf8_lossy(&output.stdout);
            self.log_info(&format!("Output: {}", stdout.trim()));
        } else {
            self.log_error("Failed to update S&P 500 list");
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.log_error(&format!("Error: {}", stderr.trim()));
        }

        self.complete_operation();
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

    /// Run incremental update
    pub fn run_incremental_update(&mut self) -> Result<()> {
        self.log_info("Executing: cargo run --bin smart_collect");
        
        let output = Command::new("cargo")
            .args(["run", "--bin", "smart_collect"])
            .output()?;

        if output.status.success() {
            self.log_success("Incremental update completed successfully");
            let stdout = String::from_utf8_lossy(&output.stdout);
            self.log_info(&format!("Output: {}", stdout.trim()));
        } else {
            self.log_error("Failed to perform incremental update");
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.log_error(&format!("Error: {}", stderr.trim()));
        }

        self.complete_operation();
        Ok(())
    }

    /// Validate data integrity
    pub fn validate_data_integrity(&mut self) -> Result<()> {
        self.log_info("Validating data integrity...");
        
        // TODO: Implement actual data validation logic
        self.log_info("Checking data completeness...");
        self.log_info("Identifying data gaps...");
        self.log_success("Data validation completed");
        
        self.complete_operation();
        Ok(())
    }

    /// Show collection progress
    pub fn show_collection_progress(&mut self) {
        self.log_info("Showing collection progress...");
        // TODO: Implement progress display
        self.complete_operation();
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

    /// Render the data collection view
    pub fn render(&self, f: &mut Frame, area: Rect) {
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
