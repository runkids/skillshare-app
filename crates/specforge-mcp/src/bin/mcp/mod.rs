//! MCP Server modules for SpecForge
//!
//! This module contains components extracted from mcp_server.rs
//! for better maintainability and organization.
//!
//! ## Module Structure
//!
//! - `types`: Parameter and response type definitions (~680 lines)
//! - `security`: Tool categorization and permission checking (~80 lines)
//! - `state`: Rate limiters and concurrency controls (~70 lines)
//! - `templates`: Built-in workflow step templates (~220 lines)
//! - `store`: Database access and local data types (~370 lines)
//! - `background/`: Background process management (~595 lines)
//! - `tools_registry`: Centralized tool definitions (~300 lines)
//! - `instance_manager`: Smart multi-instance management with heartbeat (~400 lines)
//!
//! The main tool implementations remain in `mcp_server.rs` due to
//! `rmcp` crate's requirement that all `#[tool]` methods be in a
//! single `#[tool_router] impl` block.

// Extracted modules
pub mod types;
pub mod security;
pub mod state;
pub mod templates;
pub mod store;
pub mod background;
pub mod tools_registry;
pub mod instance_manager;

// Re-export commonly used items
pub use security::{ToolCategory, get_tool_category, is_tool_allowed};
pub use tools_registry::ALL_TOOLS;
pub use state::{RATE_LIMITER, TOOL_RATE_LIMITERS, ACTION_SEMAPHORE};
pub use templates::get_builtin_templates;
pub use store::{
    read_store_data, write_store_data, log_request, open_database, get_database_path,
    Project, Workflow, WorkflowNode, CustomStepTemplate,
};
pub use background::{
    BackgroundProcessStatus, BACKGROUND_PROCESS_MANAGER, CLEANUP_INTERVAL_SECS,
};
pub use instance_manager::InstanceManager;

// Test module (only compiled in test builds)
#[cfg(test)]
mod mcp_tests;
