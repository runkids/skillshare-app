// MCP Server data models
// Feature: AI CLI Integration (020-ai-cli-integration)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// MCP Tool Registry (Single Source of Truth)
// ============================================================================

/// Tool permission category for access control
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MCPToolPermissionCategory {
    /// Read-only operations (always allowed in read_only mode)
    Read,
    /// Execute operations (run commands, workflows)
    Execute,
    /// Write operations (create, update, delete)
    Write,
}

impl MCPToolPermissionCategory {
    /// Convert to string for serialization
    pub fn as_str(&self) -> &'static str {
        match self {
            MCPToolPermissionCategory::Read => "read",
            MCPToolPermissionCategory::Execute => "execute",
            MCPToolPermissionCategory::Write => "write",
        }
    }
}

/// Tool definition with all metadata for UI display and permission management
#[derive(Debug, Clone)]
pub struct MCPToolDefinition {
    /// Tool name (used in MCP calls)
    pub name: &'static str,
    /// Human-readable description
    pub description: &'static str,
    /// UI display category (e.g., "Project Management", "Git Worktree")
    pub display_category: &'static str,
    /// Permission category for access control
    pub permission_category: MCPToolPermissionCategory,
    /// Which permission types are applicable for this tool
    pub applicable_permissions: &'static [&'static str],
}

/// All MCP tools registered in the system
/// This is the single source of truth for tool definitions
pub static MCP_ALL_TOOLS: &[MCPToolDefinition] = &[
    // ============================================================================
    // Project Management
    // ============================================================================
    MCPToolDefinition {
        name: "list_projects",
        description: "List all registered projects with detailed info",
        display_category: "Project Management",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "get_project",
        description: "Get project details (scripts, workflows, git info)",
        display_category: "Project Management",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "get_project_dependencies",
        description: "Get dependencies from package.json",
        display_category: "Project Management",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },

    // ============================================================================
    // Git Worktree
    // ============================================================================
    MCPToolDefinition {
        name: "list_worktrees",
        description: "List all git worktrees for a project",
        display_category: "Git Worktree",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "get_worktree_status",
        description: "Get git status (branch, staged, modified, untracked)",
        display_category: "Git Worktree",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "get_git_diff",
        description: "Get staged changes diff for commit messages",
        display_category: "Git Worktree",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },

    // ============================================================================
    // Workflows
    // ============================================================================
    MCPToolDefinition {
        name: "list_workflows",
        description: "List all workflows, filter by project",
        display_category: "Workflows",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "get_workflow",
        description: "Get detailed workflow info with all steps",
        display_category: "Workflows",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "create_workflow",
        description: "Create a new workflow",
        display_category: "Workflows",
        permission_category: MCPToolPermissionCategory::Write,
        applicable_permissions: &["read", "write"],
    },
    MCPToolDefinition {
        name: "create_workflow_with_steps",
        description: "Create a workflow with steps atomically (recommended, max 10 steps)",
        display_category: "Workflows",
        permission_category: MCPToolPermissionCategory::Write,
        applicable_permissions: &["read", "write"],
    },
    MCPToolDefinition {
        name: "add_workflow_step",
        description: "Add a script step to a workflow",
        display_category: "Workflows",
        permission_category: MCPToolPermissionCategory::Write,
        applicable_permissions: &["read", "write"],
    },
    MCPToolDefinition {
        name: "add_workflow_steps",
        description: "Add multiple steps to a workflow atomically (batch, max 10)",
        display_category: "Workflows",
        permission_category: MCPToolPermissionCategory::Write,
        applicable_permissions: &["read", "write"],
    },
    MCPToolDefinition {
        name: "update_workflow",
        description: "Update workflow name/description",
        display_category: "Workflows",
        permission_category: MCPToolPermissionCategory::Write,
        applicable_permissions: &["read", "write"],
    },
    MCPToolDefinition {
        name: "delete_workflow_step",
        description: "Remove a step from a workflow",
        display_category: "Workflows",
        permission_category: MCPToolPermissionCategory::Write,
        applicable_permissions: &["read", "write"],
    },
    MCPToolDefinition {
        name: "run_workflow",
        description: "Execute a workflow synchronously",
        display_category: "Workflows",
        permission_category: MCPToolPermissionCategory::Execute,
        applicable_permissions: &["read", "execute"],
    },
    MCPToolDefinition {
        name: "get_workflow_execution_details",
        description: "Get execution logs",
        display_category: "Workflows",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },

    // ============================================================================
    // Templates
    // ============================================================================
    MCPToolDefinition {
        name: "list_step_templates",
        description: "List available step templates",
        display_category: "Templates",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "create_step_template",
        description: "Create a reusable step template",
        display_category: "Templates",
        permission_category: MCPToolPermissionCategory::Write,
        applicable_permissions: &["read", "write"],
    },

    // ============================================================================
    // NPM/Package Scripts
    // ============================================================================
    MCPToolDefinition {
        name: "run_npm_script",
        description: "Run npm/yarn/pnpm scripts (volta/corepack support)",
        display_category: "NPM/Package Scripts",
        permission_category: MCPToolPermissionCategory::Execute,
        applicable_permissions: &["read", "execute"],
    },
    MCPToolDefinition {
        name: "run_package_manager_command",
        description: "Run package manager commands (install, update, etc.)",
        display_category: "NPM/Package Scripts",
        permission_category: MCPToolPermissionCategory::Execute,
        applicable_permissions: &["read", "execute"],
    },

    // ============================================================================
    // Background Processes
    // ============================================================================
    MCPToolDefinition {
        name: "get_background_process_output",
        description: "Get output from a background process",
        display_category: "Background Processes",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "stop_background_process",
        description: "Stop/terminate a background process",
        display_category: "Background Processes",
        permission_category: MCPToolPermissionCategory::Execute,
        applicable_permissions: &["read", "execute"],
    },
    MCPToolDefinition {
        name: "list_background_processes",
        description: "List all background processes",
        display_category: "Background Processes",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },

    // ============================================================================
    // MCP Actions
    // ============================================================================
    MCPToolDefinition {
        name: "list_actions",
        description: "List all MCP actions",
        display_category: "MCP Actions",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "get_action",
        description: "Get action details by ID",
        display_category: "MCP Actions",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "run_script",
        description: "Execute a script action",
        display_category: "MCP Actions",
        permission_category: MCPToolPermissionCategory::Execute,
        applicable_permissions: &["read", "execute"],
    },
    MCPToolDefinition {
        name: "trigger_webhook",
        description: "Trigger a webhook action",
        display_category: "MCP Actions",
        permission_category: MCPToolPermissionCategory::Execute,
        applicable_permissions: &["read", "execute"],
    },
    MCPToolDefinition {
        name: "get_execution_status",
        description: "Get action execution status",
        display_category: "MCP Actions",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "list_action_executions",
        description: "List recent executions",
        display_category: "MCP Actions",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "get_action_permissions",
        description: "Get permission configuration",
        display_category: "MCP Actions",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },

    // ============================================================================
    // AI Assistant
    // ============================================================================
    MCPToolDefinition {
        name: "list_ai_providers",
        description: "List configured AI providers",
        display_category: "AI Assistant",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "list_conversations",
        description: "List past AI conversations",
        display_category: "AI Assistant",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },

    // ============================================================================
    // Notifications
    // ============================================================================
    MCPToolDefinition {
        name: "get_notifications",
        description: "Get recent notifications",
        display_category: "Notifications",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "mark_notifications_read",
        description: "Mark notifications as read",
        display_category: "Notifications",
        permission_category: MCPToolPermissionCategory::Write,
        applicable_permissions: &["read", "write"],
    },

    // ============================================================================
    // Security
    // ============================================================================
    MCPToolDefinition {
        name: "get_security_scan_results",
        description: "Get vulnerability scan results",
        display_category: "Security",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "run_security_scan",
        description: "Run npm/yarn/pnpm audit",
        display_category: "Security",
        permission_category: MCPToolPermissionCategory::Execute,
        applicable_permissions: &["read", "execute"],
    },
    MCPToolDefinition {
        name: "check_dependency_integrity",
        description: "Check dependency integrity against reference snapshot",
        display_category: "Security",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "get_security_insights",
        description: "Get security insights and risk overview for a project",
        display_category: "Security",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "export_security_report",
        description: "Generate and export security audit report (JSON/MD/HTML)",
        display_category: "Security",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },

    // ============================================================================
    // Time Machine (Execution Snapshots)
    // ============================================================================
    MCPToolDefinition {
        name: "list_execution_snapshots",
        description: "List Time Machine snapshots for a project",
        display_category: "Time Machine",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "get_snapshot_details",
        description: "Get detailed snapshot info including dependencies",
        display_category: "Time Machine",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "compare_snapshots",
        description: "Compare two snapshots and get dependency changes",
        display_category: "Time Machine",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "search_snapshots",
        description: "Search snapshots by package name, version, or date range",
        display_category: "Time Machine",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "replay_execution",
        description: "Replay a workflow execution from a snapshot",
        display_category: "Time Machine",
        permission_category: MCPToolPermissionCategory::Execute,
        applicable_permissions: &["read", "execute"],
    },
    MCPToolDefinition {
        name: "capture_snapshot",
        description: "Manually capture a Time Machine snapshot for a project",
        display_category: "Time Machine",
        permission_category: MCPToolPermissionCategory::Execute,
        applicable_permissions: &["read", "execute"],
    },

    // ============================================================================
    // Deployments
    // ============================================================================
    MCPToolDefinition {
        name: "list_deployments",
        description: "List deployment history",
        display_category: "Deployments",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },

    // ============================================================================
    // File Operations
    // ============================================================================
    MCPToolDefinition {
        name: "check_file_exists",
        description: "Check if files exist in project",
        display_category: "File Operations",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "search_project_files",
        description: "Search files by pattern",
        display_category: "File Operations",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
    MCPToolDefinition {
        name: "read_project_file",
        description: "Read file content (security-limited)",
        display_category: "File Operations",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },

    // ============================================================================
    // System
    // ============================================================================
    MCPToolDefinition {
        name: "get_environment_info",
        description: "Get system tool versions and paths",
        display_category: "System",
        permission_category: MCPToolPermissionCategory::Read,
        applicable_permissions: &["read"],
    },
];

/// Get the permission category for a tool by name
pub fn get_mcp_tool_permission_category(tool_name: &str) -> MCPToolPermissionCategory {
    MCP_ALL_TOOLS
        .iter()
        .find(|t| t.name == tool_name)
        .map(|t| t.permission_category)
        .unwrap_or(MCPToolPermissionCategory::Execute) // Unknown tools default to Execute (most restrictive)
}

/// Get a tool definition by name
pub fn get_mcp_tool(tool_name: &str) -> Option<&'static MCPToolDefinition> {
    MCP_ALL_TOOLS.iter().find(|t| t.name == tool_name)
}

/// Get all unique display categories
pub fn get_mcp_display_categories() -> Vec<&'static str> {
    let mut categories: Vec<&'static str> = MCP_ALL_TOOLS
        .iter()
        .map(|t| t.display_category)
        .collect();
    categories.sort();
    categories.dedup();
    categories
}

/// MCP Server permission modes
///
/// Permission levels control what AI assistants can do:
/// - `ReadOnly`: Can only query data (list projects, get status, view workflows)
/// - `ExecuteWithConfirm`: Can read + execute workflows, but cannot create/modify
/// - `FullAccess`: Full access including creating and modifying workflows (dangerous)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MCPPermissionMode {
    /// Only allow read operations (default)
    #[default]
    ReadOnly,
    /// Allow read + execute, but not write operations
    ExecuteWithConfirm,
    /// Full access including create/modify (dangerous)
    FullAccess,
}

/// Dev server mode for MCP
///
/// Controls how dev server commands (npm run dev, etc.) are handled:
/// - `McpManaged`: MCP manages background processes independently (default)
/// - `UiIntegrated`: Processes are tracked in SpecForge UI via events
/// - `RejectWithHint`: Reject dev server commands with a hint to use SpecForge UI
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DevServerMode {
    /// MCP manages background processes independently (default)
    #[default]
    McpManaged,
    /// Processes are tracked in SpecForge UI via events
    UiIntegrated,
    /// Reject dev server commands with a hint to use SpecForge UI
    RejectWithHint,
}

impl std::fmt::Display for DevServerMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DevServerMode::McpManaged => write!(f, "mcp_managed"),
            DevServerMode::UiIntegrated => write!(f, "ui_integrated"),
            DevServerMode::RejectWithHint => write!(f, "reject_with_hint"),
        }
    }
}

impl std::fmt::Display for MCPPermissionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MCPPermissionMode::ReadOnly => write!(f, "read_only"),
            MCPPermissionMode::ExecuteWithConfirm => write!(f, "execute_with_confirm"),
            MCPPermissionMode::FullAccess => write!(f, "full_access"),
        }
    }
}

/// Encrypted secrets storage for MCP configuration
/// Uses AES-256-GCM encryption with machine-derived key
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MCPEncryptedSecrets {
    /// Encrypted nonce (base64)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
    /// Encrypted ciphertext (base64)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ciphertext: Option<String>,
}

impl MCPEncryptedSecrets {
    /// Check if secrets are stored
    pub fn is_empty(&self) -> bool {
        self.nonce.is_none() || self.ciphertext.is_none()
    }
}

/// MCP Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPServerConfig {
    /// Whether MCP Server is enabled
    #[serde(default)]
    pub is_enabled: bool,
    /// Default permission mode
    #[serde(default)]
    pub permission_mode: MCPPermissionMode,
    /// Dev server mode - controls how dev server commands are handled
    #[serde(default)]
    pub dev_server_mode: DevServerMode,
    /// List of allowed tools (empty = all tools allowed based on permission mode)
    #[serde(default = "default_allowed_tools")]
    pub allowed_tools: Vec<String>,
    /// Whether to log all requests
    #[serde(default = "default_true")]
    pub log_requests: bool,
    /// Encrypted secrets (API keys, tokens, etc.)
    /// Use set_secret/get_secret methods to access
    #[serde(default, skip_serializing_if = "MCPEncryptedSecrets::is_empty")]
    pub encrypted_secrets: MCPEncryptedSecrets,
}

fn default_true() -> bool {
    true
}

fn default_allowed_tools() -> Vec<String> {
    vec![
        "list_projects".to_string(),
        "get_project".to_string(),
        "list_worktrees".to_string(),
        "get_worktree_status".to_string(),
    ]
}

impl Default for MCPServerConfig {
    fn default() -> Self {
        Self {
            is_enabled: false,
            permission_mode: MCPPermissionMode::ReadOnly,
            dev_server_mode: DevServerMode::default(),
            allowed_tools: default_allowed_tools(),
            log_requests: true,
            encrypted_secrets: MCPEncryptedSecrets::default(),
        }
    }
}

impl MCPServerConfig {
    /// Store encrypted secrets using AES-256-GCM
    ///
    /// The secrets are stored as a JSON object that can contain multiple key-value pairs.
    /// Example: {"webhook_token": "abc123", "api_key": "xyz789"}
    ///
    /// Uses machine-derived key for encryption - data can only be decrypted on the same machine.
    pub fn set_secrets(&mut self, secrets: &std::collections::HashMap<String, String>) -> Result<(), String> {
        use crate::services::crypto::{encrypt, CryptoError};

        if secrets.is_empty() {
            self.encrypted_secrets = MCPEncryptedSecrets::default();
            return Ok(());
        }

        let json = serde_json::to_string(secrets)
            .map_err(|e| format!("Failed to serialize secrets: {}", e))?;

        let encrypted = encrypt(&json).map_err(|e: CryptoError| e.to_string())?;

        self.encrypted_secrets = MCPEncryptedSecrets {
            nonce: Some(encrypted.nonce),
            ciphertext: Some(encrypted.ciphertext),
        };

        Ok(())
    }

    /// Retrieve decrypted secrets
    ///
    /// Returns None if no secrets are stored or decryption fails.
    pub fn get_secrets(&self) -> Option<std::collections::HashMap<String, String>> {
        use crate::services::crypto::{decrypt, EncryptedData};

        if self.encrypted_secrets.is_empty() {
            return None;
        }

        let encrypted = EncryptedData {
            nonce: self.encrypted_secrets.nonce.clone()?,
            ciphertext: self.encrypted_secrets.ciphertext.clone()?,
        };

        let json = decrypt(&encrypted).ok()?;
        serde_json::from_str(&json).ok()
    }

    /// Get a single secret by key
    pub fn get_secret(&self, key: &str) -> Option<String> {
        self.get_secrets()?.get(key).cloned()
    }

    /// Set a single secret (preserving existing secrets)
    pub fn set_secret(&mut self, key: &str, value: &str) -> Result<(), String> {
        let mut secrets = self.get_secrets().unwrap_or_default();
        secrets.insert(key.to_string(), value.to_string());
        self.set_secrets(&secrets)
    }

    /// Remove a single secret
    pub fn remove_secret(&mut self, key: &str) -> Result<(), String> {
        let mut secrets = self.get_secrets().unwrap_or_default();
        secrets.remove(key);
        self.set_secrets(&secrets)
    }

    /// Check if a secret exists
    pub fn has_secret(&self, key: &str) -> bool {
        self.get_secrets()
            .map(|s| s.contains_key(key))
            .unwrap_or(false)
    }
}

/// MCP session information (runtime state, not persisted)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPSession {
    /// Session ID
    pub id: String,
    /// Connected client name
    pub client_name: String,
    /// Client version (if provided)
    pub client_version: Option<String>,
    /// When this session was connected
    pub connected_at: DateTime<Utc>,
    /// Last activity time
    pub last_activity: DateTime<Utc>,
    /// Request count
    pub request_count: u32,
}

impl MCPSession {
    pub fn new(client_name: String, client_version: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            client_name,
            client_version,
            connected_at: now,
            last_activity: now,
            request_count: 0,
        }
    }

    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
        self.request_count += 1;
    }
}

/// MCP request result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MCPRequestResult {
    Success,
    PermissionDenied,
    UserCancelled,
    Error,
}

impl std::fmt::Display for MCPRequestResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MCPRequestResult::Success => write!(f, "success"),
            MCPRequestResult::PermissionDenied => write!(f, "permission_denied"),
            MCPRequestResult::UserCancelled => write!(f, "user_cancelled"),
            MCPRequestResult::Error => write!(f, "error"),
        }
    }
}

/// MCP request log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPRequestLog {
    /// Log entry ID
    pub id: String,
    /// Associated session ID
    pub session_id: String,
    /// Tool name that was called
    pub tool_name: String,
    /// Arguments passed to the tool
    pub arguments: serde_json::Value,
    /// Execution result
    pub result: MCPRequestResult,
    /// Error message if failed
    pub error_message: Option<String>,
    /// When this request was executed
    pub executed_at: DateTime<Utc>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
}

impl MCPRequestLog {
    pub fn new(
        session_id: String,
        tool_name: String,
        arguments: serde_json::Value,
        result: MCPRequestResult,
        error_message: Option<String>,
        duration_ms: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id,
            tool_name,
            arguments,
            result,
            error_message,
            executed_at: Utc::now(),
            duration_ms,
        }
    }
}

/// MCP Server status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPStatus {
    /// Whether the server is running
    pub is_running: bool,
    /// Current permission mode
    pub permission_mode: MCPPermissionMode,
    /// Number of connected clients
    pub connected_clients: usize,
    /// Active sessions
    pub sessions: Vec<MCPSessionInfo>,
}

/// Simplified session info for status display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPSessionInfo {
    pub id: String,
    pub client_name: String,
    pub connected_at: DateTime<Utc>,
    pub request_count: u32,
}

impl From<&MCPSession> for MCPSessionInfo {
    fn from(session: &MCPSession) -> Self {
        Self {
            id: session.id.clone(),
            client_name: session.client_name.clone(),
            connected_at: session.connected_at,
            request_count: session.request_count,
        }
    }
}

/// Request to update MCP configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMCPConfigRequest {
    pub permission_mode: Option<MCPPermissionMode>,
    pub dev_server_mode: Option<DevServerMode>,
    pub allowed_tools: Option<Vec<String>>,
    pub log_requests: Option<bool>,
}

/// Request to get MCP logs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetLogsRequest {
    /// Maximum number of logs to return
    pub limit: Option<usize>,
    /// Filter by session ID
    pub session_id: Option<String>,
}

impl Default for GetLogsRequest {
    fn default() -> Self {
        Self {
            limit: Some(100),
            session_id: None,
        }
    }
}

/// Pending MCP request (waiting for user confirmation)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingMCPRequest {
    /// Request ID
    pub request_id: String,
    /// Session ID
    pub session_id: String,
    /// Tool name
    pub tool_name: String,
    /// Arguments
    pub arguments: serde_json::Value,
    /// When this request was received
    pub received_at: DateTime<Utc>,
}

impl PendingMCPRequest {
    pub fn new(session_id: String, tool_name: String, arguments: serde_json::Value) -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            session_id,
            tool_name,
            arguments,
            received_at: Utc::now(),
        }
    }
}

// ============================================================================
// MCP Tool Types (for structured responses)
// ============================================================================

/// Project info for MCP tools
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPProjectInfo {
    pub path: String,
    pub name: String,
    pub is_active: bool,
}

/// Detailed project info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPProjectDetails {
    pub path: String,
    pub name: String,
    pub git_remote: Option<String>,
    pub current_branch: Option<String>,
    pub worktree_count: usize,
    pub workflow_count: usize,
}

/// Worktree info for MCP tools
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPWorktreeInfo {
    pub path: String,
    pub branch: String,
    pub is_main: bool,
    pub is_bare: bool,
}

/// Worktree status for MCP tools
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPWorktreeStatus {
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub staged: Vec<String>,
    pub modified: Vec<String>,
    pub untracked: Vec<String>,
}

/// Workflow info for MCP tools
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPWorkflowInfo {
    pub id: String,
    pub name: String,
    pub step_count: usize,
}

/// Workflow details for MCP tools
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPWorkflowDetails {
    pub id: String,
    pub name: String,
    pub steps: Vec<MCPWorkflowStep>,
}

/// Workflow step for MCP tools
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPWorkflowStep {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub step_type: String,
    pub command: Option<String>,
}

/// Git diff info for MCP tools
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPGitDiff {
    pub diff: String,
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

/// Workflow execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPWorkflowExecutionResult {
    pub execution_id: String,
    pub status: String,
    pub steps_completed: usize,
    pub steps_total: usize,
    pub duration_ms: u64,
}

/// Shell command execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPShellResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

/// Commit result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPCommitResult {
    pub commit_hash: String,
    pub message: String,
}

/// MCP Error response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPError {
    pub code: String,
    pub message: String,
}

impl MCPError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn project_not_found(path: &str) -> Self {
        Self::new("PROJECT_NOT_FOUND", format!("專案路徑不存在: {}", path))
    }

    pub fn worktree_not_found(path: &str) -> Self {
        Self::new("WORKTREE_NOT_FOUND", format!("Worktree 路徑不存在: {}", path))
    }

    pub fn workflow_not_found(id: &str) -> Self {
        Self::new("WORKFLOW_NOT_FOUND", format!("Workflow ID 不存在: {}", id))
    }

    pub fn permission_denied(operation: &str) -> Self {
        Self::new("PERMISSION_DENIED", format!("權限不足，操作被拒絕: {}", operation))
    }

    pub fn user_cancelled() -> Self {
        Self::new("USER_CANCELLED", "使用者在確認對話框中取消")
    }

    pub fn execution_timeout() -> Self {
        Self::new("EXECUTION_TIMEOUT", "執行超時")
    }

    pub fn execution_failed(reason: &str) -> Self {
        Self::new("EXECUTION_FAILED", format!("執行失敗: {}", reason))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_config_deserialization() {
        let json = r#"{
            "allowedTools": [],
            "isEnabled": true,
            "logRequests": true,
            "permissionMode": "full_access"
        }"#;
        
        let config: MCPServerConfig = serde_json::from_str(json).expect("Should parse");
        
        assert_eq!(config.is_enabled, true);
        assert_eq!(config.permission_mode, MCPPermissionMode::FullAccess);
        assert_eq!(config.log_requests, true);
        assert!(config.allowed_tools.is_empty());
    }
    
    #[test]
    fn test_mcp_permission_mode_deserialization() {
        assert_eq!(
            serde_json::from_str::<MCPPermissionMode>(r#""read_only""#).unwrap(),
            MCPPermissionMode::ReadOnly
        );
        assert_eq!(
            serde_json::from_str::<MCPPermissionMode>(r#""execute_with_confirm""#).unwrap(),
            MCPPermissionMode::ExecuteWithConfirm
        );
        assert_eq!(
            serde_json::from_str::<MCPPermissionMode>(r#""full_access""#).unwrap(),
            MCPPermissionMode::FullAccess
        );
    }
}
