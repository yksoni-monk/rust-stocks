use anyhow::Result;
use ratatui::{Frame, prelude::Rect};

/// View contract for all TUI views (non-async for trait object compatibility)
pub trait View {
    /// Render the view
    fn render(&self, f: &mut Frame, area: Rect);
    
    /// Get the view title
    fn get_title(&self) -> String;
    
    /// Get the view status text
    fn get_status(&self) -> String;
    
    /// Check if the view is active
    fn is_active(&self) -> bool {
        true
    }
    
    /// Handle view-specific key events (sync version)
    fn handle_key(&mut self, _key: crossterm::event::KeyCode) -> Result<bool> {
        Ok(false) // Default: not handled
    }
    
    /// Handle view-specific state updates (sync version)
    fn handle_state_update(&mut self, _update: &crate::ui::state::StateUpdate) -> Result<bool> {
        Ok(false) // Default: not handled
    }
    
    /// Update the view (called periodically)
    fn update(&mut self) -> Result<()> {
        Ok(()) // Default: no update needed
    }
}

/// View manager for handling multiple views
pub struct ViewManager {
    pub views: Vec<Box<dyn View>>,
    pub current_view_index: usize,
}

impl ViewManager {
    /// Create a new view manager
    pub fn new() -> Self {
        Self {
            views: Vec::new(),
            current_view_index: 0,
        }
    }
    
    /// Add a view
    pub fn add_view(&mut self, view: Box<dyn View>) {
        self.views.push(view);
    }
    
    /// Get the current view
    pub fn get_current_view(&self) -> Option<&dyn View> {
        self.views.get(self.current_view_index).map(|v| v.as_ref())
    }
    
    /// Get the current view mutably
    pub fn get_current_view_mut(&mut self) -> Option<&mut (dyn View + '_)> {
        if self.current_view_index < self.views.len() {
            Some(self.views[self.current_view_index].as_mut())
        } else {
            None
        }
    }
    
    /// Switch to the next view
    pub fn next_view(&mut self) {
        if !self.views.is_empty() {
            self.current_view_index = (self.current_view_index + 1) % self.views.len();
        }
    }
    
    /// Switch to the previous view
    pub fn previous_view(&mut self) {
        if !self.views.is_empty() {
            self.current_view_index = if self.current_view_index == 0 {
                self.views.len() - 1
            } else {
                self.current_view_index - 1
            };
        }
    }
    
    /// Switch to a specific view by index
    pub fn switch_to_view(&mut self, index: usize) -> Result<()> {
        if index < self.views.len() {
            self.current_view_index = index;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid view index: {}", index))
        }
    }
    
    /// Get the number of views
    pub fn view_count(&self) -> usize {
        self.views.len()
    }
    
    /// Get view titles for tab bar
    pub fn get_view_titles(&self) -> Vec<String> {
        self.views.iter().map(|v| v.get_title()).collect()
    }
    
    /// Render the current view
    pub fn render_current_view(&self, f: &mut Frame, area: Rect) {
        if let Some(view) = self.get_current_view() {
            view.render(f, area);
        }
    }
    
    /// Get the current view's status
    pub fn get_current_status(&self) -> String {
        self.get_current_view()
            .map(|v| v.get_status())
            .unwrap_or_else(|| "No view available".to_string())
    }
    
    /// Handle key event for current view
    pub fn handle_current_view_key(&mut self, key: crossterm::event::KeyCode) -> Result<bool> {
        if let Some(view) = self.get_current_view_mut() {
            view.handle_key(key)
        } else {
            Ok(false)
        }
    }
    
    /// Handle state update for current view
    pub fn handle_current_view_state_update(&mut self, update: &crate::ui::state::StateUpdate) -> Result<bool> {
        if let Some(view) = self.get_current_view_mut() {
            view.handle_state_update(update)
        } else {
            Ok(false)
        }
    }
    
    /// Update current view
    pub fn update_current_view(&mut self) -> Result<()> {
        if let Some(view) = self.get_current_view_mut() {
            view.update()
        } else {
            Ok(())
        }
    }
}

/// Base view implementation with common functionality
pub struct BaseView {
    pub title: String,
    pub status: String,
    pub active: bool,
}

impl BaseView {
    /// Create a new base view
    pub fn new(title: String) -> Self {
        Self {
            title,
            status: "Ready".to_string(),
            active: true,
        }
    }
    
    /// Set the status
    pub fn set_status(&mut self, status: String) {
        self.status = status;
    }
    
    /// Set active state
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }
}

impl View for BaseView {
    fn render(&self, _f: &mut Frame, _area: Rect) {
        // Default empty render - subclasses should override
    }
    
    fn get_title(&self) -> String {
        self.title.clone()
    }
    
    fn get_status(&self) -> String {
        self.status.clone()
    }
    
    fn is_active(&self) -> bool {
        self.active
    }
}

/// View factory for creating different types of views
pub struct ViewFactory;

impl ViewFactory {
    /// Create a data collection view
    pub fn create_data_collection_view() -> Box<dyn View> {
        // This will be implemented when we refactor the existing DataCollectionView
        Box::new(BaseView::new("Data Collection".to_string()))
    }
    
    /// Create a data analysis view
    pub fn create_data_analysis_view() -> Box<dyn View> {
        // This will be implemented when we refactor the existing DataAnalysisView
        Box::new(BaseView::new("Data Analysis".to_string()))
    }
    
    /// Create a dashboard view
    pub fn create_dashboard_view() -> Box<dyn View> {
        // This will be implemented when we refactor the existing Dashboard
        Box::new(BaseView::new("Dashboard".to_string()))
    }
}

/// View state for tracking view-specific data
#[derive(Debug, Clone)]
pub struct ViewState {
    pub title: String,
    pub status: String,
    pub is_active: bool,
    pub last_update: std::time::Instant,
}

impl ViewState {
    /// Create a new view state
    pub fn new(title: String) -> Self {
        Self {
            title,
            status: "Ready".to_string(),
            is_active: true,
            last_update: std::time::Instant::now(),
        }
    }
    
    /// Update the status
    pub fn update_status(&mut self, status: String) {
        self.status = status;
        self.last_update = std::time::Instant::now();
    }
    
    /// Set active state
    pub fn set_active(&mut self, active: bool) {
        self.is_active = active;
        self.last_update = std::time::Instant::now();
    }
    
    /// Check if the view needs refresh
    pub fn needs_refresh(&self) -> bool {
        self.last_update.elapsed() > std::time::Duration::from_millis(100)
    }
}
