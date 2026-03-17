//! Background process management module
//!
//! Contains background process types and manager for running npm scripts
//! and other commands in the background.

pub mod types;
pub mod manager;

// Re-export commonly used items
pub use types::{BackgroundProcessStatus, CLEANUP_INTERVAL_SECS};
pub use manager::BACKGROUND_PROCESS_MANAGER;
