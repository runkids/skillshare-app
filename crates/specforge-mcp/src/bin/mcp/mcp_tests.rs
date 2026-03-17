//! Comprehensive tests for MCP Server
//!
//! This module contains tests for:
//! - Tool categorization (security.rs)
//! - Permission checking (security.rs)
//! - Permission matrix (all mode/category/whitelist combinations)
//! - Rate limiting (state.rs)
//! - Circular buffer (background/types.rs)
//!
//! ## Running Tests
//!
//! ```bash
//! # Run all MCP tests
//! cd src-tauri && cargo test --bin specforge-mcp mcp_tests
//!
//! # Run specific test group
//! cargo test --bin specforge-mcp mcp_tests::tests::tool_category
//!
//! # Run with output
//! cargo test --bin specforge-mcp mcp_tests -- --nocapture
//! ```

use super::security::{get_tool_category, is_tool_allowed, ToolCategory};
use super::state::ToolRateLimiters;
use super::background::types::CircularBuffer;
use specforge_lib::models::mcp::{DevServerMode, MCPEncryptedSecrets, MCPPermissionMode, MCPServerConfig};

// ============================================================================
// Tool Name Constants
// ============================================================================

/// All ReadOnly tools (26 tools)
const READONLY_TOOLS: &[&str] = &[
    "list_projects",
    "get_project",
    "list_worktrees",
    "get_worktree_status",
    "get_git_diff",
    "list_workflows",
    "get_workflow",
    "list_step_templates",
    "list_actions",
    "get_action",
    "list_action_executions",
    "get_execution_status",
    "get_action_permissions",
    "get_background_process_output",
    "list_background_processes",
    "get_environment_info",
    "list_ai_providers",
    "check_file_exists",
    "list_conversations",
    "get_notifications",
    "get_security_scan_results",
    "list_deployments",
    "get_project_dependencies",
    "get_workflow_execution_details",
    "search_project_files",
    "read_project_file",
];

/// All Write tools (6 tools)
const WRITE_TOOLS: &[&str] = &[
    "create_workflow",
    "add_workflow_step",
    "create_step_template",
    "update_workflow",
    "delete_workflow_step",
    "mark_notifications_read",
];

/// All Execute tools (7 tools)
const EXECUTE_TOOLS: &[&str] = &[
    "run_workflow",
    "run_script",
    "trigger_webhook",
    "run_npm_script",
    "run_package_manager_command",
    "stop_background_process",
    "run_security_scan",
];

// ============================================================================
// Test Helpers
// ============================================================================

/// Create a test config with specified permission mode and allowed tools
fn create_test_config(mode: MCPPermissionMode, allowed_tools: Vec<String>) -> MCPServerConfig {
    MCPServerConfig {
        is_enabled: true,
        permission_mode: mode,
        dev_server_mode: DevServerMode::McpManaged,
        allowed_tools,
        log_requests: false,
        encrypted_secrets: MCPEncryptedSecrets::default(),
    }
}

/// Get a sample tool for each category
fn sample_readonly_tool() -> &'static str {
    "list_projects"
}

fn sample_write_tool() -> &'static str {
    "create_workflow"
}

fn sample_execute_tool() -> &'static str {
    "run_workflow"
}

// ============================================================================
// Tool Category Tests
// ============================================================================

#[cfg(test)]
mod tool_category_tests {
    use super::*;

    #[test]
    fn test_readonly_tools_return_readonly_category() {
        for tool in READONLY_TOOLS {
            assert_eq!(
                get_tool_category(tool),
                ToolCategory::ReadOnly,
                "Tool '{}' should be categorized as ReadOnly",
                tool
            );
        }
    }

    #[test]
    fn test_write_tools_return_write_category() {
        for tool in WRITE_TOOLS {
            assert_eq!(
                get_tool_category(tool),
                ToolCategory::Write,
                "Tool '{}' should be categorized as Write",
                tool
            );
        }
    }

    #[test]
    fn test_execute_tools_return_execute_category() {
        for tool in EXECUTE_TOOLS {
            assert_eq!(
                get_tool_category(tool),
                ToolCategory::Execute,
                "Tool '{}' should be categorized as Execute",
                tool
            );
        }
    }

    #[test]
    fn test_unknown_tool_defaults_to_execute() {
        // Unknown tools should default to Execute (most restrictive)
        assert_eq!(get_tool_category("unknown_tool"), ToolCategory::Execute);
        assert_eq!(get_tool_category("malicious_command"), ToolCategory::Execute);
        assert_eq!(get_tool_category("foo_bar_baz"), ToolCategory::Execute);
    }

    #[test]
    fn test_empty_tool_name_defaults_to_execute() {
        assert_eq!(get_tool_category(""), ToolCategory::Execute);
    }

    #[test]
    fn test_tool_count_matches_expected() {
        // Verify we have the expected number of tools
        assert_eq!(READONLY_TOOLS.len(), 26, "Expected 26 ReadOnly tools");
        assert_eq!(WRITE_TOOLS.len(), 6, "Expected 6 Write tools");
        assert_eq!(EXECUTE_TOOLS.len(), 7, "Expected 7 Execute tools");
    }
}

// ============================================================================
// Permission Mode Tests
// ============================================================================

#[cfg(test)]
mod permission_mode_tests {
    use super::*;

    // --- ReadOnly Mode ---

    #[test]
    fn test_readonly_mode_allows_readonly_tools() {
        let config = create_test_config(MCPPermissionMode::ReadOnly, vec![]);
        for tool in READONLY_TOOLS {
            assert!(
                is_tool_allowed(tool, &config).is_ok(),
                "ReadOnly mode should allow ReadOnly tool '{}'",
                tool
            );
        }
    }

    #[test]
    fn test_readonly_mode_blocks_write_tools() {
        let config = create_test_config(MCPPermissionMode::ReadOnly, vec![]);
        for tool in WRITE_TOOLS {
            let result = is_tool_allowed(tool, &config);
            assert!(
                result.is_err(),
                "ReadOnly mode should block Write tool '{}'",
                tool
            );
            let err = result.unwrap_err();
            assert!(
                err.contains("read-only mode"),
                "Error should mention 'read-only mode': {}",
                err
            );
        }
    }

    #[test]
    fn test_readonly_mode_blocks_execute_tools() {
        let config = create_test_config(MCPPermissionMode::ReadOnly, vec![]);
        for tool in EXECUTE_TOOLS {
            let result = is_tool_allowed(tool, &config);
            assert!(
                result.is_err(),
                "ReadOnly mode should block Execute tool '{}'",
                tool
            );
        }
    }

    // --- ExecuteWithConfirm Mode ---

    #[test]
    fn test_execute_with_confirm_allows_readonly_tools() {
        let config = create_test_config(MCPPermissionMode::ExecuteWithConfirm, vec![]);
        for tool in READONLY_TOOLS {
            assert!(
                is_tool_allowed(tool, &config).is_ok(),
                "ExecuteWithConfirm mode should allow ReadOnly tool '{}'",
                tool
            );
        }
    }

    #[test]
    fn test_execute_with_confirm_allows_write_tools() {
        let config = create_test_config(MCPPermissionMode::ExecuteWithConfirm, vec![]);
        for tool in WRITE_TOOLS {
            assert!(
                is_tool_allowed(tool, &config).is_ok(),
                "ExecuteWithConfirm mode should allow Write tool '{}'",
                tool
            );
        }
    }

    #[test]
    fn test_execute_with_confirm_allows_execute_tools() {
        let config = create_test_config(MCPPermissionMode::ExecuteWithConfirm, vec![]);
        for tool in EXECUTE_TOOLS {
            assert!(
                is_tool_allowed(tool, &config).is_ok(),
                "ExecuteWithConfirm mode should allow Execute tool '{}'",
                tool
            );
        }
    }

    // --- FullAccess Mode ---

    #[test]
    fn test_full_access_allows_all_tools() {
        let config = create_test_config(MCPPermissionMode::FullAccess, vec![]);

        for tool in READONLY_TOOLS
            .iter()
            .chain(WRITE_TOOLS)
            .chain(EXECUTE_TOOLS)
        {
            assert!(
                is_tool_allowed(tool, &config).is_ok(),
                "FullAccess mode should allow tool '{}'",
                tool
            );
        }
    }
}

// ============================================================================
// Whitelist Tests
// ============================================================================

#[cfg(test)]
mod whitelist_tests {
    use super::*;

    #[test]
    fn test_empty_whitelist_allows_all_based_on_mode() {
        // Empty whitelist + FullAccess = all tools allowed
        let config = create_test_config(MCPPermissionMode::FullAccess, vec![]);
        assert!(is_tool_allowed("list_projects", &config).is_ok());
        assert!(is_tool_allowed("run_workflow", &config).is_ok());
    }

    #[test]
    fn test_whitelist_blocks_unlisted_tools() {
        let config = create_test_config(
            MCPPermissionMode::FullAccess,
            vec!["list_projects".to_string()],
        );

        // Listed tool should be allowed
        assert!(is_tool_allowed("list_projects", &config).is_ok());

        // Unlisted tool should be blocked
        let result = is_tool_allowed("get_project", &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not in the allowed tools list"));
    }

    #[test]
    fn test_whitelist_allows_multiple_listed_tools() {
        let config = create_test_config(
            MCPPermissionMode::FullAccess,
            vec![
                "list_projects".to_string(),
                "get_project".to_string(),
                "run_workflow".to_string(),
            ],
        );

        assert!(is_tool_allowed("list_projects", &config).is_ok());
        assert!(is_tool_allowed("get_project", &config).is_ok());
        assert!(is_tool_allowed("run_workflow", &config).is_ok());
        assert!(is_tool_allowed("list_workflows", &config).is_err());
    }

    #[test]
    fn test_whitelist_checked_before_permission_mode() {
        // Even in FullAccess mode, whitelist should be checked first
        let config = create_test_config(
            MCPPermissionMode::FullAccess,
            vec!["list_projects".to_string()],
        );

        // This Execute tool is not in whitelist - should fail with whitelist error
        let result = is_tool_allowed("run_workflow", &config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("not in the allowed tools list"),
            "Should fail due to whitelist, not permission mode: {}",
            err
        );
    }

    #[test]
    fn test_whitelist_error_contains_tool_name() {
        let config = create_test_config(
            MCPPermissionMode::FullAccess,
            vec!["list_projects".to_string()],
        );

        let result = is_tool_allowed("unknown_tool", &config);
        let err = result.unwrap_err();
        assert!(
            err.contains("unknown_tool"),
            "Error should contain the tool name: {}",
            err
        );
    }

    #[test]
    fn test_whitelist_with_readonly_mode_still_enforces_mode() {
        // Tool in whitelist but wrong category for mode
        let config = create_test_config(
            MCPPermissionMode::ReadOnly,
            vec!["run_workflow".to_string()], // Execute tool in whitelist
        );

        // Should fail because ReadOnly mode doesn't allow Execute tools
        let result = is_tool_allowed("run_workflow", &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("read-only mode"));
    }
}

// ============================================================================
// Permission Matrix Tests (Full Coverage)
// ============================================================================

#[cfg(test)]
mod permission_matrix_tests {
    use super::*;

    // Helper to run a matrix test
    fn check_permission(
        mode: MCPPermissionMode,
        tool: &str,
        whitelist: Option<&[&str]>,
        expected_ok: bool,
        expected_error_contains: Option<&str>,
    ) {
        let allowed_tools = whitelist
            .map(|w| w.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default();
        let config = create_test_config(mode.clone(), allowed_tools);
        let result = is_tool_allowed(tool, &config);

        if expected_ok {
            assert!(
                result.is_ok(),
                "Expected Ok for mode={:?}, tool={}, whitelist={:?}",
                mode,
                tool,
                whitelist
            );
        } else {
            assert!(
                result.is_err(),
                "Expected Err for mode={:?}, tool={}, whitelist={:?}",
                mode,
                tool,
                whitelist
            );
            if let Some(expected) = expected_error_contains {
                let err = result.unwrap_err();
                assert!(
                    err.contains(expected),
                    "Error should contain '{}': {}",
                    expected,
                    err
                );
            }
        }
    }

    // --- ReadOnly Mode Matrix ---

    #[test]
    fn test_matrix_readonly_readonly_empty_whitelist() {
        check_permission(
            MCPPermissionMode::ReadOnly,
            sample_readonly_tool(),
            None, // empty whitelist
            true,
            None,
        );
    }

    #[test]
    fn test_matrix_readonly_readonly_in_whitelist() {
        check_permission(
            MCPPermissionMode::ReadOnly,
            sample_readonly_tool(),
            Some(&["list_projects"]),
            true,
            None,
        );
    }

    #[test]
    fn test_matrix_readonly_readonly_not_in_whitelist() {
        check_permission(
            MCPPermissionMode::ReadOnly,
            sample_readonly_tool(),
            Some(&["get_project"]), // list_projects not in whitelist
            false,
            Some("not in the allowed tools list"),
        );
    }

    #[test]
    fn test_matrix_readonly_write_empty_whitelist() {
        check_permission(
            MCPPermissionMode::ReadOnly,
            sample_write_tool(),
            None,
            false,
            Some("read-only mode"),
        );
    }

    #[test]
    fn test_matrix_readonly_write_in_whitelist() {
        check_permission(
            MCPPermissionMode::ReadOnly,
            sample_write_tool(),
            Some(&["create_workflow"]),
            false,
            Some("read-only mode"),
        );
    }

    #[test]
    fn test_matrix_readonly_write_not_in_whitelist() {
        check_permission(
            MCPPermissionMode::ReadOnly,
            sample_write_tool(),
            Some(&["list_projects"]),
            false,
            Some("not in the allowed tools list"),
        );
    }

    #[test]
    fn test_matrix_readonly_execute_empty_whitelist() {
        check_permission(
            MCPPermissionMode::ReadOnly,
            sample_execute_tool(),
            None,
            false,
            Some("read-only mode"),
        );
    }

    #[test]
    fn test_matrix_readonly_execute_in_whitelist() {
        check_permission(
            MCPPermissionMode::ReadOnly,
            sample_execute_tool(),
            Some(&["run_workflow"]),
            false,
            Some("read-only mode"),
        );
    }

    #[test]
    fn test_matrix_readonly_execute_not_in_whitelist() {
        check_permission(
            MCPPermissionMode::ReadOnly,
            sample_execute_tool(),
            Some(&["list_projects"]),
            false,
            Some("not in the allowed tools list"),
        );
    }

    // --- ExecuteWithConfirm Mode Matrix ---

    #[test]
    fn test_matrix_execute_confirm_readonly_empty_whitelist() {
        check_permission(
            MCPPermissionMode::ExecuteWithConfirm,
            sample_readonly_tool(),
            None,
            true,
            None,
        );
    }

    #[test]
    fn test_matrix_execute_confirm_write_empty_whitelist() {
        check_permission(
            MCPPermissionMode::ExecuteWithConfirm,
            sample_write_tool(),
            None,
            true,
            None,
        );
    }

    #[test]
    fn test_matrix_execute_confirm_execute_empty_whitelist() {
        check_permission(
            MCPPermissionMode::ExecuteWithConfirm,
            sample_execute_tool(),
            None,
            true,
            None,
        );
    }

    #[test]
    fn test_matrix_execute_confirm_any_not_in_whitelist() {
        check_permission(
            MCPPermissionMode::ExecuteWithConfirm,
            sample_readonly_tool(),
            Some(&["get_project"]),
            false,
            Some("not in the allowed tools list"),
        );
    }

    // --- FullAccess Mode Matrix ---

    #[test]
    fn test_matrix_full_access_readonly_empty_whitelist() {
        check_permission(
            MCPPermissionMode::FullAccess,
            sample_readonly_tool(),
            None,
            true,
            None,
        );
    }

    #[test]
    fn test_matrix_full_access_write_empty_whitelist() {
        check_permission(
            MCPPermissionMode::FullAccess,
            sample_write_tool(),
            None,
            true,
            None,
        );
    }

    #[test]
    fn test_matrix_full_access_execute_empty_whitelist() {
        check_permission(
            MCPPermissionMode::FullAccess,
            sample_execute_tool(),
            None,
            true,
            None,
        );
    }

    #[test]
    fn test_matrix_full_access_any_not_in_whitelist() {
        check_permission(
            MCPPermissionMode::FullAccess,
            sample_execute_tool(),
            Some(&["list_projects"]),
            false,
            Some("not in the allowed tools list"),
        );
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_case_sensitivity() {
        // Tool names are case-sensitive
        assert_eq!(get_tool_category("LIST_PROJECTS"), ToolCategory::Execute);
        assert_eq!(get_tool_category("List_Projects"), ToolCategory::Execute);
        assert_eq!(get_tool_category("listProjects"), ToolCategory::Execute);
    }

    #[test]
    fn test_whitespace_handling() {
        // Whitespace in tool names should be treated as unknown
        assert_eq!(get_tool_category(" list_projects"), ToolCategory::Execute);
        assert_eq!(get_tool_category("list_projects "), ToolCategory::Execute);
        assert_eq!(get_tool_category(" "), ToolCategory::Execute);
    }

    #[test]
    fn test_special_characters() {
        // Special characters in tool names
        assert_eq!(get_tool_category("list-projects"), ToolCategory::Execute);
        assert_eq!(get_tool_category("list.projects"), ToolCategory::Execute);
        assert_eq!(get_tool_category("list/projects"), ToolCategory::Execute);
    }

    #[test]
    fn test_null_like_strings() {
        assert_eq!(get_tool_category("null"), ToolCategory::Execute);
        assert_eq!(get_tool_category("undefined"), ToolCategory::Execute);
        assert_eq!(get_tool_category("None"), ToolCategory::Execute);
    }

    #[test]
    fn test_very_long_tool_name() {
        let long_name = "a".repeat(10000);
        assert_eq!(get_tool_category(&long_name), ToolCategory::Execute);
    }

    #[test]
    fn test_error_message_mentions_settings() {
        let config = create_test_config(MCPPermissionMode::ReadOnly, vec![]);
        let result = is_tool_allowed("run_workflow", &config);
        let err = result.unwrap_err();
        assert!(
            err.contains("SpecForge settings"),
            "Error should guide user to change settings: {}",
            err
        );
    }

    #[test]
    fn test_config_default_permission_mode() {
        let config = MCPServerConfig::default();
        assert_eq!(config.permission_mode, MCPPermissionMode::ReadOnly);
    }

    #[test]
    fn test_config_default_is_disabled() {
        let config = MCPServerConfig::default();
        assert!(!config.is_enabled);
    }
}

// ============================================================================
// Rate Limiter Tests
// ============================================================================

#[cfg(test)]
mod rate_limiter_tests {
    use super::*;

    #[test]
    fn test_rate_limiter_default_creation() {
        let _limiter = ToolRateLimiters::default();
        // Just verify it can be created without panic
        assert!(true, "ToolRateLimiters should be created successfully");
    }

    #[test]
    fn test_limit_description_readonly() {
        let limiter = ToolRateLimiters::default();
        let desc = limiter.get_limit_description(ToolCategory::ReadOnly);
        assert!(desc.contains("200"));
        assert!(desc.contains("read-only"));
    }

    #[test]
    fn test_limit_description_write() {
        let limiter = ToolRateLimiters::default();
        let desc = limiter.get_limit_description(ToolCategory::Write);
        assert!(desc.contains("30"));
        assert!(desc.contains("write"));
    }

    #[test]
    fn test_limit_description_execute() {
        let limiter = ToolRateLimiters::default();
        let desc = limiter.get_limit_description(ToolCategory::Execute);
        assert!(desc.contains("10"));
        assert!(desc.contains("execute"));
    }

    #[test]
    fn test_rate_limiter_initial_check_succeeds() {
        let limiter = ToolRateLimiters::default();
        // First request should always succeed
        assert!(limiter.check(ToolCategory::ReadOnly).is_ok());
        assert!(limiter.check(ToolCategory::Write).is_ok());
        assert!(limiter.check(ToolCategory::Execute).is_ok());
    }
}

// ============================================================================
// CircularBuffer Tests
// ============================================================================

#[cfg(test)]
mod circular_buffer_tests {
    use super::*;

    #[test]
    fn test_empty_buffer() {
        let buffer = CircularBuffer::new(100, 1024);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.tail(10).is_empty());
    }

    #[test]
    fn test_push_within_limits() {
        let mut buffer = CircularBuffer::new(100, 1024);
        buffer.push("line 1".to_string());
        buffer.push("line 2".to_string());
        buffer.push("line 3".to_string());

        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.tail(3), vec!["line 1", "line 2", "line 3"]);
    }

    #[test]
    fn test_tail_returns_last_n_lines() {
        let mut buffer = CircularBuffer::new(100, 1024);
        for i in 1..=10 {
            buffer.push(format!("line {}", i));
        }

        let last_3 = buffer.tail(3);
        assert_eq!(last_3, vec!["line 8", "line 9", "line 10"]);

        let last_5 = buffer.tail(5);
        assert_eq!(last_5, vec!["line 6", "line 7", "line 8", "line 9", "line 10"]);
    }

    #[test]
    fn test_tail_with_more_than_available() {
        let mut buffer = CircularBuffer::new(100, 1024);
        buffer.push("line 1".to_string());
        buffer.push("line 2".to_string());

        let result = buffer.tail(10); // Request more than available
        assert_eq!(result, vec!["line 1", "line 2"]);
    }

    #[test]
    fn test_eviction_by_max_lines() {
        let mut buffer = CircularBuffer::new(3, 10000); // Max 3 lines
        buffer.push("line 1".to_string());
        buffer.push("line 2".to_string());
        buffer.push("line 3".to_string());
        buffer.push("line 4".to_string()); // Should evict "line 1"

        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.tail(3), vec!["line 2", "line 3", "line 4"]);
    }

    #[test]
    fn test_eviction_by_max_bytes() {
        let mut buffer = CircularBuffer::new(100, 20); // Max 20 bytes
        buffer.push("aaaaa".to_string()); // 5 bytes
        buffer.push("bbbbb".to_string()); // 5 bytes = 10 total
        buffer.push("ccccc".to_string()); // 5 bytes = 15 total
        buffer.push("ddddddddddd".to_string()); // 11 bytes, should evict until fits

        // After eviction, only the last line that fits should remain
        assert!(buffer.total_bytes <= 20);
        assert!(buffer.lines.contains(&"ddddddddddd".to_string()));
    }

    #[test]
    fn test_len_reflects_current_count() {
        let mut buffer = CircularBuffer::new(5, 1000);
        assert_eq!(buffer.len(), 0);

        buffer.push("a".to_string());
        assert_eq!(buffer.len(), 1);

        buffer.push("b".to_string());
        buffer.push("c".to_string());
        assert_eq!(buffer.len(), 3);
    }

    #[test]
    fn test_total_bytes_tracking() {
        let mut buffer = CircularBuffer::new(100, 1000);
        buffer.push("hello".to_string()); // 5 bytes
        assert_eq!(buffer.total_bytes, 5);

        buffer.push("world".to_string()); // 5 bytes
        assert_eq!(buffer.total_bytes, 10);
    }
}
