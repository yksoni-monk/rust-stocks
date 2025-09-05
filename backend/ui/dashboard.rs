use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use chrono::NaiveDate;
use anyhow::Result;

use crate::database_sqlx::DatabaseManagerSqlx;
use crate::models::DatabaseStats;

#[allow(dead_code)]
pub struct Dashboard {
    pub database_stats: Option<DatabaseStats>,
}

// DatabaseStats moved to models/mod.rs to avoid duplication

impl Dashboard {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            database_stats: None,
        }
    }

    /// Refresh data from database
    #[allow(dead_code)]
    pub async fn refresh_data(&mut self, database: &DatabaseManagerSqlx) -> Result<()> {
        // Get basic database stats
        let stocks = database.get_active_stocks().await?;
        let total_stocks = stocks.len();
        
        // Get total price records from stats
        let stats = database.get_stats().await?;
        let total_price_records = stats.get("total_prices").unwrap_or(&0).clone() as usize;
        
        // Calculate coverage percentage (simplified)
        let data_coverage_percentage = if total_stocks > 0 {
            (total_price_records as f64 / (total_stocks * 1000) as f64) * 100.0 // Rough estimate
        } else {
            0.0
        };
        
        // Get date ranges from database
        let oldest_data_date = database.get_oldest_data_date().await.unwrap_or(None);
        let last_update_date = database.get_last_update_date().await.unwrap_or(None);
        
        self.database_stats = Some(DatabaseStats {
            total_stocks,
            total_price_records,
            data_coverage_percentage,
            oldest_data_date,
            last_update_date,
            top_pe_decliner: None, // Dashboard doesn't need PE decliner info
        });
        
        Ok(())
    }

    /// Render the dashboard view
    #[allow(dead_code)]
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(0),    // Main content
            ])
            .split(area);

        // Title
        let title = Paragraph::new("📊 Stock Analysis System - Dashboard")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        f.render_widget(title, chunks[0]);

        // Main content - split into sections
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // Database stats
                Constraint::Min(0),   // Quick actions
            ])
            .split(chunks[1]);

        self.render_database_stats(f, main_chunks[0]);
        self.render_quick_actions(f, main_chunks[1]);
    }

    #[allow(dead_code)]
    fn render_database_stats(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(34),
            ])
            .split(area);

        if let Some(stats) = &self.database_stats {
            // Total stocks
            let stocks_text = vec![
                Line::from(vec![
                    Span::styled("Total Stocks: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{}", stats.total_stocks),
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Coverage: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{:.1}%", stats.data_coverage_percentage),
                        if stats.data_coverage_percentage > 80.0 {
                            Style::default().fg(Color::Green)
                        } else {
                            Style::default().fg(Color::Yellow)
                        }
                    ),
                ]),
            ];
            
            let stocks_block = Paragraph::new(stocks_text)
                .block(Block::default().borders(Borders::ALL).title("🏢 Stocks"));
            f.render_widget(stocks_block, chunks[0]);

            // Total records
            let records_text = vec![
                Line::from(vec![
                    Span::styled("Price Records: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{}", stats.total_price_records),
                        Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Avg per Stock: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        if stats.total_stocks > 0 {
                            format!("{}", stats.total_price_records / stats.total_stocks)
                        } else {
                            "0".to_string()
                        },
                        Style::default().fg(Color::Cyan)
                    ),
                ]),
            ];
            
            let records_block = Paragraph::new(records_text)
                .block(Block::default().borders(Borders::ALL).title("📈 Data"));
            f.render_widget(records_block, chunks[1]);

            // Data dates
            let dates_text = vec![
                Line::from(vec![
                    Span::styled("Oldest Data: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        stats.oldest_data_date.map(|d| d.to_string()).unwrap_or("None".to_string()),
                        Style::default().fg(Color::Magenta)
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Last Update: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        stats.last_update_date.map(|d| d.to_string()).unwrap_or("Never".to_string()),
                        Style::default().fg(Color::White)
                    ),
                ]),
            ];
            
            let dates_block = Paragraph::new(dates_text)
                .block(Block::default().borders(Borders::ALL).title("📅 Dates"));
            f.render_widget(dates_block, chunks[2]);
        } else {
            let welcome_text = vec![
                Line::from("🚀 Welcome to Rust Stocks Analysis System!"),
                Line::from(""),
                Line::from("📋 Database is empty - no stock data found"),
                Line::from("💡 Use 'cargo run --bin update_sp500' to populate stock list"),
                Line::from("🔄 Then run data collection to fetch historical data"),
                Line::from(""),
                Line::from("⌨️  Press 'q' to quit, Tab to navigate views"),
            ];
            
            let loading = Paragraph::new(welcome_text)
                .block(Block::default().borders(Borders::ALL).title("📊 Stock Analysis Dashboard"))
                .style(Style::default().fg(Color::White));
            f.render_widget(loading, area);
        }
    }

    #[allow(dead_code)]
    fn render_quick_actions(&self, f: &mut Frame, area: Rect) {
        let actions = vec![
            ListItem::new("Tab - Switch to Data Collection view"),
            ListItem::new("S - Switch to Stock Analysis view"),
            ListItem::new("P - Switch to Progress Analyzer view"),
            ListItem::new("R - Refresh data"),
            ListItem::new("Q - Quit application"),
        ];

        let actions_list = List::new(actions)
            .block(Block::default().borders(Borders::ALL).title("⚡ Quick Actions"))
            .style(Style::default().fg(Color::Gray));

        f.render_widget(actions_list, area);
    }
}