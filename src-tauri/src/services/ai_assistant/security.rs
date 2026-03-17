// Security Module for AI Assistant
// Feature: AI Assistant Tab (022-ai-assistant-tab)
//
// Provides security measures for AI tool execution:
// - Path validation against registered projects
// - Project boundary enforcement
// - Tool execution permission checking

use std::path::{Path, PathBuf};
use crate::repositories::ProjectRepository;
use crate::utils::database::Database;

/// Errors that can occur during security validation
#[derive(Debug, Clone)]
pub enum SecurityError {
    /// Path does not exist
    PathNotFound { path: String },
    /// Path is not canonical (contains .. or symlinks that escape)
    InvalidPath { path: String, reason: String },
    /// Path is outside of registered projects
    PathOutsideProject { path: String },
    /// Project not found in database
    ProjectNotRegistered { path: String },
    /// Database error
    DatabaseError { message: String },
    /// Tool not allowed
    ToolNotAllowed { tool_name: String },
}

impl std::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityError::PathNotFound { path } => {
                write!(f, "Path not found: {}", path)
            }
            SecurityError::InvalidPath { path, reason } => {
                write!(f, "Invalid path '{}': {}", path, reason)
            }
            SecurityError::PathOutsideProject { path } => {
                write!(f, "Path '{}' is outside of any registered project", path)
            }
            SecurityError::ProjectNotRegistered { path } => {
                write!(f, "Project at '{}' is not registered in SpecForge", path)
            }
            SecurityError::DatabaseError { message } => {
                write!(f, "Database error: {}", message)
            }
            SecurityError::ToolNotAllowed { tool_name } => {
                write!(f, "Tool '{}' is not allowed", tool_name)
            }
        }
    }
}

impl std::error::Error for SecurityError {}

/// Validates paths and enforces project boundaries for AI tool execution
pub struct PathSecurityValidator {
    db: Database,
}

impl PathSecurityValidator {
    /// Create a new PathSecurityValidator
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Validate that a path is within a registered project
    ///
    /// This method:
    /// 1. Canonicalizes the path (resolves symlinks, .., etc.)
    /// 2. Checks if the path is within a registered project
    /// 3. Returns the validated canonical path
    pub fn validate_project_path(&self, path: &str) -> Result<PathBuf, SecurityError> {
        let input_path = Path::new(path);

        // Check if path exists
        if !input_path.exists() {
            return Err(SecurityError::PathNotFound {
                path: path.to_string(),
            });
        }

        // Canonicalize the path to resolve symlinks and relative components
        let canonical = std::fs::canonicalize(input_path)
            .map_err(|e| SecurityError::InvalidPath {
                path: path.to_string(),
                reason: e.to_string(),
            })?;

        // Get all registered projects
        let repo = ProjectRepository::new(self.db.clone());
        let projects = repo.list().map_err(|e| SecurityError::DatabaseError {
            message: e,
        })?;

        // Check if canonical path is within any registered project
        for project in &projects {
            let project_path = match std::fs::canonicalize(&project.path) {
                Ok(p) => p,
                Err(_) => continue, // Skip projects with invalid paths
            };

            // Check if the input path is the project path or a subdirectory
            if canonical.starts_with(&project_path) {
                return Ok(canonical);
            }
        }

        // Path is not within any registered project
        Err(SecurityError::PathOutsideProject {
            path: path.to_string(),
        })
    }

    /// Validate that a path exactly matches a registered project
    pub fn validate_exact_project_path(&self, path: &str) -> Result<PathBuf, SecurityError> {
        let input_path = Path::new(path);

        // Check if path exists
        if !input_path.exists() {
            return Err(SecurityError::PathNotFound {
                path: path.to_string(),
            });
        }

        // Canonicalize the path
        let canonical = std::fs::canonicalize(input_path)
            .map_err(|e| SecurityError::InvalidPath {
                path: path.to_string(),
                reason: e.to_string(),
            })?;

        // Check if this exact path is a registered project
        let repo = ProjectRepository::new(self.db.clone());

        // Try to find by the canonical path string
        let canonical_str = canonical.to_string_lossy().to_string();

        // Also check by the original path (in case it's stored differently)
        let exists = repo.exists_by_path(&canonical_str)
            .or_else(|_| repo.exists_by_path(path))
            .map_err(|e| SecurityError::DatabaseError { message: e })?;

        if exists {
            return Ok(canonical);
        }

        // Also check all projects for matching canonical paths
        let projects = repo.list().map_err(|e| SecurityError::DatabaseError {
            message: e,
        })?;

        for project in &projects {
            if let Ok(project_canonical) = std::fs::canonicalize(&project.path) {
                if project_canonical == canonical {
                    return Ok(canonical);
                }
            }
        }

        Err(SecurityError::ProjectNotRegistered {
            path: path.to_string(),
        })
    }

    /// Check if a path is safe (no path traversal attempts)
    pub fn is_path_safe(&self, path: &str) -> bool {
        // Reject paths with obvious traversal attempts
        let dangerous_patterns = [
            "..",
            "~",
            "$HOME",
            "${HOME}",
            "%USERPROFILE%",
        ];

        for pattern in dangerous_patterns {
            if path.contains(pattern) {
                return false;
            }
        }

        // Reject paths with null bytes
        if path.contains('\0') {
            return false;
        }

        // Reject very long paths (potential buffer overflow attempts)
        if path.len() > 4096 {
            return false;
        }

        true
    }

    /// Sanitize and validate a path for tool execution
    ///
    /// Returns the canonical path if valid, or an error if the path is invalid or outside project bounds.
    pub fn sanitize_tool_path(&self, path: &str) -> Result<PathBuf, SecurityError> {
        // First check for obvious path safety issues
        if !self.is_path_safe(path) {
            return Err(SecurityError::InvalidPath {
                path: path.to_string(),
                reason: "Path contains potentially dangerous patterns".to_string(),
            });
        }

        // Then validate against registered projects
        self.validate_project_path(path)
    }
}

/// Tool permission checker for AI-driven operations
pub struct ToolPermissionChecker;

impl ToolPermissionChecker {
    /// Tools that can be executed without confirmation (read-only operations)
    const AUTO_ALLOWED_TOOLS: &'static [&'static str] = &[
        "get_git_status",
        "get_staged_diff",
        "list_project_scripts",
        "list_workflows",
        "list_projects",
        "get_project",
        "get_workflow",
        "list_worktrees",
        "list_actions",
        "get_action",
        "list_action_executions",
        "get_execution_status",
        "list_step_templates",
        // New tools synced with MCP Server
        "get_worktree_status",
        "get_git_diff",
        "get_action_permissions",
        "list_background_processes",
        "get_background_process_output",
    ];

    /// Tools that require user confirmation before execution
    const CONFIRMATION_REQUIRED_TOOLS: &'static [&'static str] = &[
        "run_script",
        "run_npm_script",
        "run_workflow",
        "trigger_webhook",
        "create_workflow",
        "create_workflow_with_steps",
        "add_workflow_step",
        "add_workflow_steps",
        // New tools synced with MCP Server
        "create_step_template",
        "stop_background_process",
    ];

    /// Tools that are explicitly blocked
    const BLOCKED_TOOLS: &'static [&'static str] = &[
        "execute_command",  // Would allow arbitrary shell execution
        "write_file",       // Would allow arbitrary file writes
        "delete_file",      // Would allow arbitrary file deletion
        "read_file",        // Could expose sensitive files
        "modify_env",       // Could modify environment variables
    ];

    /// Check if a tool is allowed to be executed
    pub fn is_tool_allowed(tool_name: &str) -> bool {
        !Self::BLOCKED_TOOLS.contains(&tool_name)
    }

    /// Check if a tool requires user confirmation
    pub fn requires_confirmation(tool_name: &str) -> bool {
        // If it's in auto-allowed, no confirmation needed
        if Self::AUTO_ALLOWED_TOOLS.contains(&tool_name) {
            return false;
        }

        // If it's explicitly in confirmation-required, yes
        if Self::CONFIRMATION_REQUIRED_TOOLS.contains(&tool_name) {
            return true;
        }

        // Default to requiring confirmation for unknown tools
        true
    }

    /// Validate a tool call can proceed
    pub fn validate_tool_call(tool_name: &str) -> Result<(), SecurityError> {
        if Self::BLOCKED_TOOLS.contains(&tool_name) {
            return Err(SecurityError::ToolNotAllowed {
                tool_name: tool_name.to_string(),
            });
        }
        Ok(())
    }

    /// Get the list of allowed tools for documentation/AI context
    pub fn get_allowed_tools() -> Vec<&'static str> {
        let mut tools = Vec::new();
        tools.extend(Self::AUTO_ALLOWED_TOOLS);
        tools.extend(Self::CONFIRMATION_REQUIRED_TOOLS);
        tools
    }
}

/// Output sanitizer for tool execution results
pub struct OutputSanitizer;

impl OutputSanitizer {
    /// Sensitive patterns to redact from output
    const SENSITIVE_PATTERNS: &'static [&'static str] = &[
        "api_key",
        "apikey",
        "api-key",
        "secret",
        "password",
        "passwd",
        "token",
        "credential",
        "private_key",
        "privatekey",
    ];

    /// Sanitize tool output before returning to AI
    pub fn sanitize_output(output: &str) -> String {
        let mut result = output.to_string();

        // Redact lines that look like they contain secrets
        for line in output.lines() {
            let lower = line.to_lowercase();
            for pattern in Self::SENSITIVE_PATTERNS {
                if lower.contains(pattern) && (lower.contains('=') || lower.contains(':')) {
                    // Replace the entire line with a redacted version
                    result = result.replace(line, "[LINE REDACTED - may contain sensitive data]");
                    break;
                }
            }
        }

        // Redact common secret patterns
        result = Self::redact_secrets(&result);

        result
    }

    /// Redact common secret patterns from text
    fn redact_secrets(text: &str) -> String {
        let mut result = text.to_string();

        // JWT tokens
        let jwt_pattern = regex::Regex::new(r"eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*").ok();
        if let Some(re) = jwt_pattern {
            result = re.replace_all(&result, "[JWT_REDACTED]").to_string();
        }

        // API key prefixes
        let api_key_prefixes = ["sk-", "pk_", "rk_", "ghp_", "gho_", "github_pat_", "glpat-", "npm_", "sk-ant-"];
        for prefix in api_key_prefixes {
            if let Some(re) = regex::Regex::new(&format!(r"{}[a-zA-Z0-9_-]{{10,}}", regex::escape(prefix))).ok() {
                result = re.replace_all(&result, "[API_KEY_REDACTED]").to_string();
            }
        }

        result
    }

    /// Truncate output to a maximum length
    pub fn truncate_output(output: &str, max_chars: usize) -> String {
        let char_count = output.chars().count();
        if char_count > max_chars {
            let truncated: String = output.chars().take(max_chars).collect();
            format!("{}...\n[Output truncated, {} more characters]", truncated, char_count - max_chars)
        } else {
            output.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn setup_test_db() -> Database {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        // Keep the dir alive by leaking it for the duration of the test
        std::mem::forget(dir);
        Database::new(db_path).expect("Failed to create test database")
    }

    #[test]
    fn test_is_path_safe() {
        let validator = PathSecurityValidator::new(setup_test_db());

        // Safe paths
        assert!(validator.is_path_safe("/Users/test/project"));
        assert!(validator.is_path_safe("/home/user/code"));

        // Dangerous paths
        assert!(!validator.is_path_safe("/Users/test/../../../etc/passwd"));
        assert!(!validator.is_path_safe("~/.ssh/id_rsa"));
        assert!(!validator.is_path_safe("$HOME/.bashrc"));
        assert!(!validator.is_path_safe("path\0with\0nulls"));
    }

    #[test]
    fn test_tool_permission_checker() {
        // Auto-allowed tools
        assert!(ToolPermissionChecker::is_tool_allowed("get_git_status"));
        assert!(!ToolPermissionChecker::requires_confirmation("get_git_status"));

        // Read-only query tools should be auto-allowed
        assert!(ToolPermissionChecker::is_tool_allowed("list_projects"));
        assert!(!ToolPermissionChecker::requires_confirmation("list_projects"));
        assert!(ToolPermissionChecker::is_tool_allowed("get_project"));
        assert!(!ToolPermissionChecker::requires_confirmation("get_project"));

        // Confirmation required tools
        assert!(ToolPermissionChecker::is_tool_allowed("run_script"));
        assert!(ToolPermissionChecker::requires_confirmation("run_script"));

        // Blocked tools
        assert!(!ToolPermissionChecker::is_tool_allowed("execute_command"));
        assert!(ToolPermissionChecker::validate_tool_call("execute_command").is_err());
    }

    #[test]
    fn test_output_sanitizer() {
        // Test line-level secret redaction (lines with sensitive keywords + = or :)
        let output = "API_KEY=sk-abc123def456ghi789";
        let sanitized = OutputSanitizer::sanitize_output(output);
        assert!(sanitized.contains("REDACTED"), "API_KEY line should be redacted");

        // Test JWT redaction (standalone JWT without sensitive keyword prefix)
        // Note: Line-level redaction will catch "token: ..." lines first
        let jwt_output = "Here is the JWT eyJhbGciOiJIUzI1NiJ9.eyJ0ZXN0IjoxfQ.abc123def456";
        let sanitized = OutputSanitizer::sanitize_output(jwt_output);
        assert!(sanitized.contains("JWT_REDACTED"), "JWT should be redacted: {}", sanitized);

        // Test API key prefix redaction
        let api_key_output = "Using key sk-abcdefghij1234567890";
        let sanitized = OutputSanitizer::sanitize_output(api_key_output);
        assert!(sanitized.contains("API_KEY_REDACTED"), "API key prefix should be redacted: {}", sanitized);

        // Test truncation
        let long_output = "a".repeat(10000);
        let truncated = OutputSanitizer::truncate_output(&long_output, 1000);
        assert!(truncated.len() < 10000);
        assert!(truncated.contains("truncated"));
    }

    #[test]
    fn test_allowed_tools_list() {
        let tools = ToolPermissionChecker::get_allowed_tools();
        assert!(!tools.is_empty());
        assert!(tools.contains(&"get_git_status"));
        assert!(tools.contains(&"run_script"));
        assert!(!tools.contains(&"execute_command"));
    }
}
