// Services module
// Business logic and background services

// Core services (shared between Tauri app and MCP)
pub mod crypto;
pub mod mcp_action;
pub mod security_guardian;
pub mod snapshot;

pub use crypto::*;

// Note: Tauri-dependent modules (ai, ai_assistant, ai_cli, deploy,
// file_watcher, incoming_webhook, notification) are in src-tauri/src/services/
