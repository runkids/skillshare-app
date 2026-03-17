// Data models module
// Rust structs that map to TypeScript interfaces

pub mod ai;
pub mod ai_assistant;
pub mod apk;
pub mod cli_tool;
pub mod deploy;
pub mod mcp_action;
pub mod execution;
pub mod git;
pub mod incoming_webhook;
pub mod ipa;
pub mod mcp;
pub mod monorepo;
pub mod project;
pub mod security;
pub mod security_insight;
pub mod snapshot;
pub mod step_template;
pub mod toolchain;
pub mod version;
pub mod webhook;
pub mod workflow;
pub mod worktree;
pub mod worktree_sessions;

// Re-export all models for convenience
pub use apk::*;
pub use execution::*;
pub use ipa::*;
pub use project::*;
pub use workflow::*;
pub use worktree::*;
pub use worktree_sessions::*;
// Re-export security types except PackageManager (already exported from project)
pub use git::*;
pub use incoming_webhook::*;
pub use monorepo::*;
pub use security::{
    CvssInfo, DependencyCount, FixInfo, ScanError, ScanErrorCode, ScanStatus, SecurityScanData,
    SecurityScanSummary, Severity, VulnItem, VulnScanResult, VulnSummary, WorkspaceVulnSummary,
};
pub use step_template::*;
pub use version::*;
pub use webhook::*;
// Note: toolchain types are not re-exported to avoid conflict with version::VoltaConfig
// Use crate::models::toolchain::* explicitly when needed
// Note: ai and mcp types are not glob re-exported to keep namespace clean
// Use crate::models::ai::* or crate::models::mcp::* explicitly when needed
