use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Centralized layout management to prevent conflicts between views
#[allow(dead_code)]
pub struct TuiLayout {
    pub tab_bar: Rect,
    pub content: Rect,
    pub status_bar: Rect,
}

impl TuiLayout {
    /// Create a new layout from the given area
    pub fn new(area: Rect) -> Self {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tab bar
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Status bar
            ])
            .split(area);
            
        Self {
            tab_bar: chunks[0],
            content: chunks[1],
            status_bar: chunks[2],
        }
    }

    /// Render the tab bar
    #[allow(dead_code)]
    pub fn render_tab_bar(&self, f: &mut Frame, selected_tab: usize) {
        let titles = vec![
            "Data Collection",
            "Data Analysis",
        ];
        
        let tabs = ratatui::widgets::Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("Stock Analysis System"))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .select(selected_tab);
            
        f.render_widget(tabs, self.tab_bar);
    }

    /// Render the status bar
    #[allow(dead_code)]
    pub fn render_status_bar(&self, f: &mut Frame, status_text: &str) {
        let status_content = vec![
            Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::Gray)),
                Span::styled("Tab", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(" to switch views • ", Style::default().fg(Color::Gray)),
                Span::styled("R", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled(" to refresh • ", Style::default().fg(Color::Gray)),
                Span::styled("Q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled(" to quit", Style::default().fg(Color::Gray)),
            ]),
            Line::from(vec![
                Span::styled(status_text, Style::default().fg(Color::Cyan)),
            ]),
        ];
        
        let paragraph = Paragraph::new(status_content)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        
        f.render_widget(paragraph, self.status_bar);
    }

    /// Get content area for views to use
    #[allow(dead_code)]
    pub fn get_content_area(&self) -> Rect {
        self.content
    }
}

/// Helper struct for view-specific layouts
#[allow(dead_code)]
pub struct ViewLayout {
    pub title: Rect,
    pub main_content: Rect,
    pub status: Rect,
}

impl ViewLayout {
    /// Create a view layout within the given content area
    #[allow(dead_code)]
    pub fn new(content_area: Rect) -> Self {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Status
            ])
            .split(content_area);
            
        Self {
            title: chunks[0],
            main_content: chunks[1],
            status: chunks[2],
        }
    }

    /// Create a layout for data collection view (with logs)
    #[allow(dead_code)]
    pub fn for_data_collection(content_area: Rect) -> Self {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(12), // Actions list
                Constraint::Min(0),     // Logs
                Constraint::Length(3),  // Status
            ])
            .split(content_area);
            
        Self {
            title: chunks[0],
            main_content: chunks[1], // This will be further split for actions and logs
            status: chunks[3],
        }
    }

    /// Create a layout for data analysis view
    #[allow(dead_code)]
    pub fn for_data_analysis(content_area: Rect) -> Self {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Status
            ])
            .split(content_area);
            
        Self {
            title: chunks[0],
            main_content: chunks[1],
            status: chunks[2],
        }
    }

    /// Split main content into horizontal sections
    #[allow(dead_code)]
    pub fn split_main_content(&self, direction: Direction, constraints: &[Constraint]) -> Vec<Rect> {
        Layout::default()
            .direction(direction)
            .constraints(constraints)
            .split(self.main_content)
            .to_vec()
    }

    /// Split main content into vertical sections
    #[allow(dead_code)]
    pub fn split_main_content_vertical(&self, constraints: &[Constraint]) -> Vec<Rect> {
        self.split_main_content(Direction::Vertical, constraints)
    }

    /// Split main content into horizontal sections
    #[allow(dead_code)]
    pub fn split_main_content_horizontal(&self, constraints: &[Constraint]) -> Vec<Rect> {
        self.split_main_content(Direction::Horizontal, constraints)
    }
}
