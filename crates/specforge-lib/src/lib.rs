// SpecForge Shared Library
// Contains models, repositories, services, and utilities shared between
// the Tauri app and the MCP server.

pub mod models;
pub mod repositories;
pub mod services;
pub mod utils;

// Re-export models for convenience
pub use models::*;
