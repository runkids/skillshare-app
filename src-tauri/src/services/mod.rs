// Services module
// Business logic and background services

// Re-export shared services from specforge-lib
pub use specforge_lib::services::crypto;
pub use specforge_lib::services::crypto::*;

// Tauri-dependent services (local)
pub mod config_service;
pub mod file_watcher;
pub mod notification;

// Spec-driven development services
pub mod schema_service;
pub mod spec_service;

pub use config_service::*;
pub use file_watcher::*;
pub use notification::*;
