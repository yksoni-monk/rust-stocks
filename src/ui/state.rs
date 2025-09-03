use anyhow::Result;
use chrono::{DateTime, Utc};
use tokio::sync::broadcast;
use std::collections::HashMap;

/// Application state for async operations
#[derive(Debug, Clone)]
pub enum AppState {
    Idle,
    Loading { operation: String, progress: f64 },
    Executing { operation: String, start_time: DateTime<Utc> },
    Error { message: String },
    Success { message: String },
}

/// State update events for async operations
#[derive(Debug, Clone)]
pub enum StateUpdate {
    OperationStarted { id: String, operation: String },
    OperationProgress { id: String, progress: f64, message: String },
    OperationCompleted { id: String, result: Result<String, String> },
    LogMessage { level: LogLevel, message: String },
    StockListUpdated { stocks: Vec<String> }, // New variant for stock list updates
}

/// Log levels
#[derive(Debug, Clone)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// Async operation tracking
#[derive(Debug, Clone)]
pub struct AsyncOperation {
    pub id: String,
    pub operation: String,
    pub start_time: DateTime<Utc>,
    pub progress: f64,
    pub current_message: String,
    pub is_cancellable: bool,
}

/// Completed operation record
#[derive(Debug, Clone)]
pub struct CompletedOperation {
    pub id: String,
    pub operation: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub result: Result<String, String>,
}

/// Log message with timestamp
#[derive(Debug, Clone)]
pub struct LogMessage {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
}

/// Centralized async state management
#[derive(Debug)]
pub struct AsyncStateManager {
    pub current_state: AppState,
    pub pending_operations: HashMap<String, AsyncOperation>,
    pub completed_operations: Vec<CompletedOperation>,
    pub log_messages: Vec<LogMessage>,
    pub broadcast_sender: broadcast::Sender<StateUpdate>,
    pub broadcast_receiver: broadcast::Receiver<StateUpdate>,
}

impl Clone for AsyncStateManager {
    fn clone(&self) -> Self {
        let (broadcast_sender, broadcast_receiver) = broadcast::channel::<StateUpdate>(100);
        
        Self {
            current_state: self.current_state.clone(),
            pending_operations: self.pending_operations.clone(),
            completed_operations: self.completed_operations.clone(),
            log_messages: self.log_messages.clone(),
            broadcast_sender,
            broadcast_receiver,
        }
    }
}

impl AsyncStateManager {
    /// Create a new async state manager
    pub fn new() -> Self {
        let (broadcast_sender, broadcast_receiver) = broadcast::channel::<StateUpdate>(100);
        
        Self {
            current_state: AppState::Idle,
            pending_operations: HashMap::new(),
            completed_operations: Vec::new(),
            log_messages: Vec::new(),
            broadcast_sender,
            broadcast_receiver,
        }
    }

    /// Start a new async operation
    pub fn start_operation(&mut self, id: String, operation: String, cancellable: bool) -> Result<()> {
        let op = AsyncOperation {
            id: id.clone(),
            operation: operation.clone(),
            start_time: Utc::now(),
            progress: 0.0,
            current_message: "Starting...".to_string(),
            is_cancellable: cancellable,
        };
        
        self.pending_operations.insert(id.clone(), op);
        self.current_state = AppState::Executing {
            operation: operation.clone(),
            start_time: Utc::now(),
        };
        
        // Send state update
        let _ = self.broadcast_sender.send(StateUpdate::OperationStarted {
            id,
            operation: operation.clone(),
        });
        
        self.add_log_message(LogLevel::Info, &format!("Started: {}", operation));
        Ok(())
    }

    /// Update operation progress
    pub fn update_progress(&mut self, id: &str, progress: f64, message: &str) -> Result<()> {
        if let Some(op) = self.pending_operations.get_mut(id) {
            op.progress = progress;
            op.current_message = message.to_string();
            
            // Update current state
            self.current_state = AppState::Loading {
                operation: op.operation.clone(),
                progress,
            };
            
            // Send state update
            let _ = self.broadcast_sender.send(StateUpdate::OperationProgress {
                id: id.to_string(),
                progress,
                message: message.to_string(),
            });
        }
        Ok(())
    }

    /// Complete an operation
    pub fn complete_operation(&mut self, id: &str, result: Result<String, String>) -> Result<()> {
        if let Some(op) = self.pending_operations.remove(id) {
            let completed_op = CompletedOperation {
                id: id.to_string(),
                operation: op.operation.clone(),
                start_time: op.start_time,
                end_time: Utc::now(),
                result: result.clone(),
            };
            
            self.completed_operations.push(completed_op);
            
            // Update current state
            match &result {
                Ok(msg) => {
                    self.current_state = AppState::Success {
                        message: msg.clone(),
                    };
                    self.add_log_message(LogLevel::Success, &format!("Completed: {} - {}", op.operation, msg));
                }
                Err(err) => {
                    self.current_state = AppState::Error {
                        message: err.clone(),
                    };
                    self.add_log_message(LogLevel::Error, &format!("Failed: {} - {}", op.operation, err));
                }
            }
            
            // Send state update
            let _ = self.broadcast_sender.send(StateUpdate::OperationCompleted {
                id: id.to_string(),
                result,
            });
        }
        Ok(())
    }

    /// Cancel an operation
    pub fn cancel_operation(&mut self, id: &str) -> Result<()> {
        if let Some(op) = self.pending_operations.remove(id) {
            self.add_log_message(LogLevel::Warning, &format!("Cancelled: {}", op.operation));
            
            // Update current state
            self.current_state = AppState::Idle;
            
            // Send state update
            let _ = self.broadcast_sender.send(StateUpdate::OperationCompleted {
                id: id.to_string(),
                result: Err("Operation cancelled".to_string()),
            });
        }
        Ok(())
    }

    /// Add a log message
    pub fn add_log_message(&mut self, level: LogLevel, message: &str) {
        let log_msg = LogMessage {
            timestamp: Utc::now(),
            level: level.clone(),
            message: message.to_string(),
        };
        
        self.log_messages.push(log_msg);
        
        // Keep only last 100 log messages
        if self.log_messages.len() > 100 {
            self.log_messages.remove(0);
        }
        
        // Send log update
        let _ = self.broadcast_sender.send(StateUpdate::LogMessage {
            level,
            message: message.to_string(),
        });
    }

    /// Process incoming state updates
    pub fn process_updates(&mut self) {
        while let Ok(update) = self.broadcast_receiver.try_recv() {
            match update {
                StateUpdate::LogMessage { level, message } => {
                    let log_msg = LogMessage {
                        timestamp: Utc::now(),
                        level,
                        message,
                    };
                    self.log_messages.push(log_msg);
                    
                    // Keep only last 100 log messages
                    if self.log_messages.len() > 100 {
                        self.log_messages.remove(0);
                    }
                }
                _ => {
                    // Other updates are handled by the specific methods
                }
            }
        }
    }

    /// Get the current status text for display
    pub fn get_status_text(&self) -> String {
        match &self.current_state {
            AppState::Idle => "Ready".to_string(),
            AppState::Loading { operation, progress } => {
                format!("Loading {}: {:.1}%", operation, progress * 100.0)
            }
            AppState::Executing { operation, start_time } => {
                let duration = Utc::now() - *start_time;
                format!("Executing {}: {}s", operation, duration.num_seconds())
            }
            AppState::Error { message } => format!("Error: {}", message),
            AppState::Success { message } => format!("Success: {}", message),
        }
    }

    /// Check if any operations are currently running
    pub fn has_active_operations(&self) -> bool {
        !self.pending_operations.is_empty()
    }

    /// Get all active operations
    pub fn get_active_operations(&self) -> Vec<&AsyncOperation> {
        self.pending_operations.values().collect()
    }

    /// Get recent log messages (last N)
    pub fn get_recent_logs(&self, count: usize) -> Vec<&LogMessage> {
        let start = if self.log_messages.len() > count {
            self.log_messages.len() - count
        } else {
            0
        };
        self.log_messages[start..].iter().collect()
    }

    /// Get the broadcast sender
    pub fn get_broadcast_sender(&self) -> broadcast::Sender<StateUpdate> {
        self.broadcast_sender.clone()
    }

    /// Subscribe to state updates
    pub fn subscribe(&self) -> broadcast::Receiver<StateUpdate> {
        self.broadcast_sender.subscribe()
    }
}
