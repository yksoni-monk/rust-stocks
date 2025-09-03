pub mod dashboard;
pub mod data_collection;
pub mod data_analysis;
pub mod data_collection_new;
pub mod data_analysis_new;
pub mod app;
pub mod app_new;
pub mod layout;
pub mod state;
pub mod events;
pub mod view;

// Re-export main components for convenience
pub use app::run_app_async;
pub use app_new::run_app_async as run_app_async_new;
pub use layout::{TuiLayout, ViewLayout};
pub use state::{AsyncStateManager, AppState, StateUpdate, LogLevel};
pub use events::{TuiEvent, EventHandler, EventManager, EventLoop};
pub use view::{View, ViewManager, BaseView, ViewFactory};