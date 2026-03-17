// MCP Action models for action execution via MCP protocol
// @see specs/021-mcp-actions/data-model.md

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Enums
// ============================================================================

/// Type of MCP action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MCPActionType {
    Script,
    Webhook,
    Workflow,
}

impl std::fmt::Display for MCPActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MCPActionType::Script => write!(f, "script"),
            MCPActionType::Webhook => write!(f, "webhook"),
            MCPActionType::Workflow => write!(f, "workflow"),
        }
    }
}

impl std::str::FromStr for MCPActionType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "script" => Ok(MCPActionType::Script),
            "webhook" => Ok(MCPActionType::Webhook),
            "workflow" => Ok(MCPActionType::Workflow),
            _ => Err(format!("Unknown action type: {}", s)),
        }
    }
}

/// Permission level for actions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PermissionLevel {
    /// Always require user confirmation
    #[default]
    RequireConfirm,
    /// Execute automatically without confirmation
    AutoApprove,
    /// Block execution entirely
    Deny,
}

impl std::fmt::Display for PermissionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionLevel::RequireConfirm => write!(f, "require_confirm"),
            PermissionLevel::AutoApprove => write!(f, "auto_approve"),
            PermissionLevel::Deny => write!(f, "deny"),
        }
    }
}

impl std::str::FromStr for PermissionLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "require_confirm" => Ok(PermissionLevel::RequireConfirm),
            "auto_approve" => Ok(PermissionLevel::AutoApprove),
            "deny" => Ok(PermissionLevel::Deny),
            _ => Err(format!("Unknown permission level: {}", s)),
        }
    }
}

/// Execution status for actions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    /// Waiting for user confirmation
    PendingConfirm,
    /// Queued for execution
    Queued,
    /// Currently executing
    Running,
    /// Completed successfully
    Completed,
    /// Execution failed
    Failed,
    /// Cancelled by user
    Cancelled,
    /// Timed out
    TimedOut,
    /// Denied by user (action request was rejected)
    Denied,
}

impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionStatus::PendingConfirm => write!(f, "pending_confirm"),
            ExecutionStatus::Queued => write!(f, "queued"),
            ExecutionStatus::Running => write!(f, "running"),
            ExecutionStatus::Completed => write!(f, "completed"),
            ExecutionStatus::Failed => write!(f, "failed"),
            ExecutionStatus::Cancelled => write!(f, "cancelled"),
            ExecutionStatus::TimedOut => write!(f, "timed_out"),
            ExecutionStatus::Denied => write!(f, "denied"),
        }
    }
}

impl std::str::FromStr for ExecutionStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending_confirm" => Ok(ExecutionStatus::PendingConfirm),
            "queued" => Ok(ExecutionStatus::Queued),
            "running" => Ok(ExecutionStatus::Running),
            "completed" => Ok(ExecutionStatus::Completed),
            "failed" => Ok(ExecutionStatus::Failed),
            "cancelled" => Ok(ExecutionStatus::Cancelled),
            "timed_out" => Ok(ExecutionStatus::TimedOut),
            "denied" => Ok(ExecutionStatus::Denied),
            _ => Err(format!("Unknown execution status: {}", s)),
        }
    }
}

// ============================================================================
// Core Entities
// ============================================================================

/// A registered action that can be triggered via MCP protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPAction {
    pub id: String,
    pub action_type: MCPActionType,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub config: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default = "default_true")]
    pub is_enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

fn default_true() -> bool {
    true
}

/// Permission configuration for actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPActionPermission {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_type: Option<MCPActionType>,
    pub permission_level: PermissionLevel,
    pub created_at: String,
}

/// Record of an action execution attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPActionExecution {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    pub action_type: MCPActionType,
    pub action_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_client: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    pub status: ExecutionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
}

// ============================================================================
// Configuration Schemas
// ============================================================================

/// Configuration for script-type actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptConfig {
    /// Command to execute
    pub command: String,
    /// Command arguments
    #[serde(default)]
    pub args: Vec<String>,
    /// Working directory (relative to project or absolute)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Environment variables to set
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Timeout in milliseconds (default: 60000)
    #[serde(default = "default_script_timeout")]
    pub timeout_ms: u64,
    /// Whether to use Volta for Node.js version management
    #[serde(default)]
    pub use_volta: bool,
}

fn default_script_timeout() -> u64 {
    60000
}

/// Configuration for webhook-type actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPWebhookConfig {
    /// Target URL (must be HTTPS in production)
    pub url: String,
    /// HTTP method
    #[serde(default = "default_method")]
    pub method: String,
    /// Custom headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Payload template (JSON with variable substitution)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_template: Option<String>,
    /// Timeout in milliseconds (default: 30000)
    #[serde(default = "default_webhook_timeout")]
    pub timeout_ms: u64,
    /// Retry count on failure (default: 0)
    #[serde(default)]
    pub retry_count: u8,
    /// Whether to validate SSL certificates
    #[serde(default = "default_true")]
    pub verify_ssl: bool,
}

fn default_method() -> String {
    "POST".to_string()
}

fn default_webhook_timeout() -> u64 {
    30000
}

/// Configuration for workflow-type actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowActionConfig {
    /// Workflow ID to execute
    pub workflow_id: String,
    /// Override parameters (merged with workflow defaults)
    #[serde(default)]
    pub parameters: HashMap<String, serde_json::Value>,
    /// Whether to report step-by-step progress
    #[serde(default = "default_true")]
    pub report_progress: bool,
}

// ============================================================================
// Result Schemas
// ============================================================================

/// Result of script execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub truncated: bool,
    pub duration_ms: u64,
}

/// Result of webhook execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookExecutionResult {
    pub status_code: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_body: Option<String>,
    #[serde(default)]
    pub response_headers: HashMap<String, String>,
    pub duration_ms: u64,
    #[serde(default)]
    pub retry_attempts: u8,
}

/// Result of workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowExecutionResult {
    pub execution_id: String,
    pub status: String,
    pub steps_completed: usize,
    pub steps_total: usize,
    pub step_results: Vec<StepResult>,
    pub duration_ms: u64,
}

/// Result of a single workflow step
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepResult {
    pub step_id: String,
    pub step_name: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub duration_ms: u64,
}

// ============================================================================
// Request/Response Types for MCP Tools
// ============================================================================

/// Parameters for run_script MCP tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunScriptParams {
    /// ID of a registered script action (mutually exclusive with command)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    /// Ad-hoc command to execute (mutually exclusive with action_id)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Command arguments (only used with command parameter)
    #[serde(default)]
    pub args: Vec<String>,
    /// Working directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Execution timeout in milliseconds
    #[serde(default = "default_script_timeout")]
    pub timeout_ms: u64,
    /// Environment variables to set
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Parameters for trigger_webhook MCP tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerWebhookParams {
    /// ID of a registered webhook action (mutually exclusive with url)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    /// Webhook URL (mutually exclusive with action_id)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// HTTP method
    #[serde(default = "default_method")]
    pub method: String,
    /// Custom HTTP headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Request payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    /// Request timeout in milliseconds
    #[serde(default = "default_webhook_timeout")]
    pub timeout_ms: u64,
}

/// Parameters for run_workflow MCP tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunWorkflowParams {
    /// ID of the workflow to execute
    pub workflow_id: String,
    /// Override parameters for workflow steps
    #[serde(default)]
    pub parameters: HashMap<String, serde_json::Value>,
    /// If true, wait for workflow to complete
    #[serde(default = "default_true")]
    pub wait_for_completion: bool,
}

/// Response for action execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionExecutionResponse {
    pub execution_id: String,
    pub status: ExecutionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Pending action request for user confirmation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingActionRequest {
    pub execution_id: String,
    pub action_type: MCPActionType,
    pub action_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub source_client: Option<String>,
    pub parameters: Option<serde_json::Value>,
    pub requested_at: String,
}

// ============================================================================
// Filter and Query Types
// ============================================================================

/// Filter options for listing actions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_type: Option<MCPActionType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_enabled: Option<bool>,
}

/// Filter options for listing executions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_type: Option<MCPActionType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ExecutionStatus>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    20
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_type_serialization() {
        assert_eq!(MCPActionType::Script.to_string(), "script");
        assert_eq!(MCPActionType::Webhook.to_string(), "webhook");
        assert_eq!(MCPActionType::Workflow.to_string(), "workflow");
    }

    #[test]
    fn test_action_type_parsing() {
        assert_eq!("script".parse::<MCPActionType>().unwrap(), MCPActionType::Script);
        assert_eq!("WEBHOOK".parse::<MCPActionType>().unwrap(), MCPActionType::Webhook);
        assert!("invalid".parse::<MCPActionType>().is_err());
    }

    #[test]
    fn test_permission_level_default() {
        let level: PermissionLevel = Default::default();
        assert_eq!(level, PermissionLevel::RequireConfirm);
    }

    #[test]
    fn test_execution_status_serialization() {
        assert_eq!(ExecutionStatus::PendingConfirm.to_string(), "pending_confirm");
        assert_eq!(ExecutionStatus::Running.to_string(), "running");
        assert_eq!(ExecutionStatus::Completed.to_string(), "completed");
    }

    #[test]
    fn test_script_config_defaults() {
        let config: ScriptConfig = serde_json::from_str(r#"{"command": "npm test"}"#).unwrap();
        assert_eq!(config.timeout_ms, 60000);
        assert!(config.args.is_empty());
        assert!(!config.use_volta);
    }

    #[test]
    fn test_webhook_config_defaults() {
        let config: MCPWebhookConfig = serde_json::from_str(r#"{"url": "https://example.com"}"#).unwrap();
        assert_eq!(config.method, "POST");
        assert_eq!(config.timeout_ms, 30000);
        assert!(config.verify_ssl);
    }
}
