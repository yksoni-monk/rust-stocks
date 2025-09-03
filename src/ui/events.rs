use anyhow::Result;
use crossterm::event::{Event, KeyCode, MouseEvent};
use tokio::sync::mpsc;
use crate::ui::state::{StateUpdate, LogLevel};

/// Unified TUI events
#[derive(Debug, Clone)]
pub enum TuiEvent {
    Key(KeyCode),
    Mouse(MouseEvent),
    Resize(u16, u16),
    StateUpdate(StateUpdate),
    Tick,
}

/// Event manager for handling all TUI events
pub struct EventManager {
    pub event_sender: mpsc::Sender<TuiEvent>,
    pub event_receiver: mpsc::Receiver<TuiEvent>,
}

impl EventManager {
    /// Create a new event manager
    pub fn new() -> Self {
        let (event_sender, event_receiver) = mpsc::channel::<TuiEvent>(100);
        Self {
            event_sender,
            event_receiver,
        }
    }

    /// Send an event
    pub async fn send_event(&self, event: TuiEvent) -> Result<()> {
        self.event_sender.send(event).await?;
        Ok(())
    }

    /// Send a key event
    pub async fn send_key(&self, key: KeyCode) -> Result<()> {
        self.send_event(TuiEvent::Key(key)).await
    }

    /// Send a state update
    pub async fn send_state_update(&self, update: StateUpdate) -> Result<()> {
        self.send_event(TuiEvent::StateUpdate(update)).await
    }

    /// Send a log message
    pub async fn send_log(&self, level: LogLevel, message: String) -> Result<()> {
        self.send_state_update(StateUpdate::LogMessage { level, message }).await
    }

    /// Send a tick event
    pub async fn send_tick(&self) -> Result<()> {
        self.send_event(TuiEvent::Tick).await
    }

    /// Try to receive an event (non-blocking)
    pub fn try_receive(&mut self) -> Option<TuiEvent> {
        self.event_receiver.try_recv().ok()
    }

    /// Receive an event (blocking)
    pub async fn receive(&mut self) -> Option<TuiEvent> {
        self.event_receiver.recv().await
    }
}

/// Event handler trait for views (sync version for trait object compatibility)
pub trait EventHandler {
    /// Handle a TUI event (sync version)
    fn handle_event(&mut self, event: &TuiEvent) -> Result<bool>; // Returns true if event was handled
    
    /// Handle key events specifically
    fn handle_key(&mut self, key: KeyCode) -> Result<bool> {
        self.handle_event(&TuiEvent::Key(key))
    }
    
    /// Handle state updates
    fn handle_state_update(&mut self, update: &StateUpdate) -> Result<bool> {
        self.handle_event(&TuiEvent::StateUpdate(update.clone()))
    }
}

/// Global event handler for the main application
pub struct GlobalEventHandler {
    pub event_manager: EventManager,
}

impl GlobalEventHandler {
    /// Create a new global event handler
    pub fn new() -> Self {
        Self {
            event_manager: EventManager::new(),
        }
    }

    /// Process terminal events and convert them to TUI events
    pub async fn process_terminal_events(&self) -> Result<()> {
        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key_event) = crossterm::event::read()? {
                self.event_manager.send_key(key_event.code).await?;
            }
        }
        Ok(())
    }

    /// Send a tick event for periodic updates
    pub async fn send_tick(&self) -> Result<()> {
        self.event_manager.send_tick().await
    }

    /// Send a log message
    pub async fn send_log(&self, level: LogLevel, message: String) -> Result<()> {
        self.event_manager.send_log(level, message).await
    }

    /// Get a clone of the event sender for use in async tasks
    pub fn get_event_sender(&self) -> mpsc::Sender<TuiEvent> {
        self.event_manager.event_sender.clone()
    }
}

/// Event routing for different views
pub struct EventRouter {
    pub handlers: Vec<Box<dyn EventHandler>>,
}

impl EventRouter {
    /// Create a new event router
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    /// Add an event handler
    pub fn add_handler(&mut self, handler: Box<dyn EventHandler>) {
        self.handlers.push(handler);
    }

    /// Route an event to all handlers
    pub fn route_event(&mut self, event: &TuiEvent) -> Result<bool> {
        for handler in &mut self.handlers {
            if handler.handle_event(event)? {
                return Ok(true); // Event was handled
            }
        }
        Ok(false) // Event was not handled
    }

    /// Route a key event
    pub fn route_key(&mut self, key: KeyCode) -> Result<bool> {
        self.route_event(&TuiEvent::Key(key))
    }

    /// Route a state update
    pub fn route_state_update(&mut self, update: &StateUpdate) -> Result<bool> {
        self.route_event(&TuiEvent::StateUpdate(update.clone()))
    }
}

/// Event loop for the TUI application
pub struct EventLoop {
    pub global_handler: GlobalEventHandler,
    pub router: EventRouter,
    pub running: bool,
}

impl EventLoop {
    /// Create a new event loop
    pub fn new() -> Self {
        Self {
            global_handler: GlobalEventHandler::new(),
            router: EventRouter::new(),
            running: true,
        }
    }

    /// Add an event handler to the router
    pub fn add_handler(&mut self, handler: Box<dyn EventHandler>) {
        self.router.add_handler(handler);
    }

    /// Run the event loop
    pub async fn run(&mut self) -> Result<()> {
        while self.running {
            // Process terminal events
            self.global_handler.process_terminal_events().await?;
            
            // Send tick event for periodic updates
            self.global_handler.send_tick().await?;
            
            // Process all pending events from the global handler
            while let Some(event) = self.global_handler.event_manager.try_receive() {
                match event {
                    TuiEvent::Key(KeyCode::Char('q')) | TuiEvent::Key(KeyCode::Char('Q')) => {
                        self.running = false;
                        break;
                    }
                    _ => {
                        self.router.route_event(&event)?;
                    }
                }
            }
            
            // Small delay to prevent busy waiting
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
        
        Ok(())
    }

    /// Stop the event loop
    pub fn stop(&mut self) {
        self.running = false;
    }
}
