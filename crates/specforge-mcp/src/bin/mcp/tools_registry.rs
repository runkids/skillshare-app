//! MCP Tool Registry for bin/mcp
//!
//! Re-exports tool definitions from specforge_lib::models::mcp
//! to provide a consistent interface within the MCP server binary.

// Re-export from specforge_lib
pub use specforge_lib::models::mcp::{
    MCPToolPermissionCategory as PermissionCategory,
    MCP_ALL_TOOLS as ALL_TOOLS,
    get_mcp_tool_permission_category as get_permission_category,
};

// Additional re-exports used by tests
#[cfg(test)]
pub use specforge_lib::models::mcp::get_mcp_tool as get_tool;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_tools_has_entries() {
        assert!(!ALL_TOOLS.is_empty());
        // Should have at least 33 tools based on current implementation
        assert!(ALL_TOOLS.len() >= 33, "Expected at least 33 tools, got {}", ALL_TOOLS.len());
    }

    #[test]
    fn test_get_permission_category() {
        assert_eq!(get_permission_category("list_projects"), PermissionCategory::Read);
        assert_eq!(get_permission_category("run_workflow"), PermissionCategory::Execute);
        assert_eq!(get_permission_category("create_workflow"), PermissionCategory::Write);
        // Unknown tool should default to Execute
        assert_eq!(get_permission_category("unknown_tool"), PermissionCategory::Execute);
    }

    #[test]
    fn test_get_tool() {
        let tool = get_tool("list_projects").unwrap();
        assert_eq!(tool.name, "list_projects");
        assert_eq!(tool.display_category, "Project Management");

        assert!(get_tool("nonexistent_tool").is_none());
    }

    #[test]
    fn test_unique_tool_names() {
        let mut names: Vec<&str> = ALL_TOOLS.iter().map(|t| t.name).collect();
        let original_len = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), original_len, "Tool names must be unique");
    }
}
