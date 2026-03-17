//! Security and permission handling for MCP tools
//!
//! Contains tool categorization and permission checking logic.
//! Uses centralized tool definitions from tools_registry.

use specforge_lib::models::mcp::{MCPPermissionMode, MCPServerConfig};
use super::tools_registry::{PermissionCategory, get_permission_category};

/// Tool permission category
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolCategory {
    /// Read-only operations (always allowed)
    ReadOnly,
    /// Write operations (create, update, delete)
    Write,
    /// Execute operations (run commands, workflows)
    Execute,
}

impl From<PermissionCategory> for ToolCategory {
    fn from(pc: PermissionCategory) -> Self {
        match pc {
            PermissionCategory::Read => ToolCategory::ReadOnly,
            PermissionCategory::Execute => ToolCategory::Execute,
            PermissionCategory::Write => ToolCategory::Write,
        }
    }
}

/// Get the permission category for a tool
/// Uses centralized tool definitions from tools_registry
pub fn get_tool_category(tool_name: &str) -> ToolCategory {
    get_permission_category(tool_name).into()
}

/// Check if a tool is allowed based on permission mode and allowed_tools list
pub fn is_tool_allowed(tool_name: &str, config: &MCPServerConfig) -> Result<(), String> {
    // Check allowed_tools whitelist first (if non-empty)
    if !config.allowed_tools.is_empty() && !config.allowed_tools.contains(&tool_name.to_string()) {
        return Err(format!(
            "Tool '{}' is not in the allowed tools list. Allowed: {:?}",
            tool_name, config.allowed_tools
        ));
    }

    // Check permission mode
    let category = get_tool_category(tool_name);

    match config.permission_mode {
        MCPPermissionMode::ReadOnly => {
            if category != ToolCategory::ReadOnly {
                return Err(format!(
                    "Tool '{}' requires write/execute permission, but MCP server is in read-only mode. \
                    Change permission mode in SpecForge settings to enable this tool.",
                    tool_name
                ));
            }
        }
        MCPPermissionMode::ExecuteWithConfirm | MCPPermissionMode::FullAccess => {
            // Allow all tools
        }
    }

    Ok(())
}
