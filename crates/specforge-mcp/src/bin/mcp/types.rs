//! Type definitions for MCP tool parameters and responses
//!
//! Contains all structs used for tool inputs and outputs.

use std::collections::HashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ============================================================================
// Default Value Helper Functions
// ============================================================================

pub fn default_true() -> bool {
    true
}

pub fn default_include_builtin() -> bool {
    true
}

pub fn default_category() -> String {
    "custom".to_string()
}

pub fn default_tail_lines() -> usize {
    100
}

pub fn default_limit_20() -> i64 {
    20
}

pub fn default_limit_10() -> i64 {
    10
}

pub fn default_output_limit() -> usize {
    5000
}

pub fn default_limit_50() -> usize {
    50
}

pub fn default_max_lines() -> usize {
    500
}

pub fn default_start_line() -> usize {
    1
}

// ============================================================================
// Parameter Types for Tools (must derive JsonSchema)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetProjectParams {
    /// The absolute path to the project directory
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetProjectsParams {
    /// Optional search query to filter projects by name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListWorktreesParams {
    /// The absolute path to the project directory
    pub project_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetWorktreeStatusParams {
    /// The absolute path to the worktree or project directory
    pub worktree_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetGitDiffParams {
    /// The absolute path to the worktree or project directory
    pub worktree_path: String,
}

// Workflow tool parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListWorkflowsParams {
    /// Optional project ID to filter workflows
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetWorkflowParams {
    /// The workflow ID
    pub workflow_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateWorkflowParams {
    /// Workflow name
    pub name: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional project ID to associate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AddWorkflowStepParams {
    /// Target workflow ID
    pub workflow_id: String,
    /// Step name
    pub name: String,
    /// Shell command to execute
    pub command: String,
    /// Optional working directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Optional timeout in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    /// Optional position (defaults to end)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<i32>,
}

/// Individual step input for batch creation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowStepInput {
    /// Step display name
    pub name: String,
    /// Shell command to execute
    pub command: String,
    /// Optional working directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Optional timeout in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

/// Parameters for add_workflow_steps tool (batch operation)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddWorkflowStepsParams {
    /// Target workflow ID - use actual ID from create_workflow or list_workflows
    pub workflow_id: String,
    /// Array of steps to add (max 10). Steps are added in array order.
    pub steps: Vec<WorkflowStepInput>,
}

/// Response for add_workflow_steps tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddWorkflowStepsResponse {
    /// Whether all steps were added successfully
    pub success: bool,
    /// The workflow ID steps were added to
    pub workflow_id: String,
    /// List of created step details
    pub created_steps: Vec<CreatedStepInfo>,
    /// Total number of steps in workflow after this operation
    pub total_workflow_steps: usize,
    /// Summary message
    pub message: String,
}

/// Info for a single created step in batch response
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatedStepInfo {
    /// Generated node ID for this step
    pub node_id: String,
    /// Step name as provided
    pub name: String,
    /// Assigned order position
    pub order: i32,
    /// Command as provided
    pub command: String,
}

/// Parameters for create_workflow_with_steps tool (atomic workflow + steps creation)
/// This tool creates a workflow and its steps in a single atomic operation,
/// preventing sync issues that can occur with separate create_workflow + add_workflow_steps calls.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkflowWithStepsParams {
    /// Workflow display name (required, 1-100 characters)
    pub name: String,
    /// Optional workflow description (max 500 characters)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional project ID to associate with (must be valid from list_projects)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    /// Array of steps to create (1-10 steps). Steps execute in array order.
    pub steps: Vec<WorkflowStepInput>,
}

/// Response for create_workflow_with_steps tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkflowWithStepsResponse {
    /// Operation success status
    pub success: bool,
    /// Created workflow ID (use with run_workflow, get_workflow, etc.)
    pub workflow_id: String,
    /// Workflow name as created
    pub workflow_name: String,
    /// Workflow description if provided
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Associated project ID if provided
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    /// List of created steps with their IDs and order positions
    pub created_steps: Vec<CreatedStepInfo>,
    /// Total number of steps created
    pub total_steps: usize,
    /// ISO 8601 timestamp when workflow was created
    pub created_at: String,
    /// Human-readable summary message
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListStepTemplatesParams {
    /// Filter by category (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Search query (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// Include built-in templates (default: true)
    #[serde(default = "default_include_builtin")]
    pub include_builtin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateStepTemplateParams {
    /// Template name
    pub name: String,
    /// Shell command
    pub command: String,
    /// Category (default: "custom")
    #[serde(default = "default_category")]
    pub category: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RunWorkflowParams {
    /// Workflow ID to execute
    pub workflow_id: String,
    /// Optional project path override (for working directory)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunNpmScriptParams {
    /// Project path (required - the directory containing package.json)
    pub project_path: String,
    /// Script name from package.json scripts (e.g., "build", "dev", "test")
    pub script_name: String,
    /// Optional arguments to pass to the script
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Timeout in milliseconds (default: 5 minutes, max: 1 hour)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    /// Run in background mode (default: false). When true, returns immediately with process ID.
    #[serde(default)]
    pub run_in_background: bool,
    /// Pattern to match in output to consider process started successfully.
    /// Examples: "ready in", "Local:", "Server running", "Compiled successfully"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_pattern: Option<String>,
    /// Timeout for success pattern matching in milliseconds (default: 30000ms)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunPackageManagerCommandParams {
    /// Project path (required - the directory containing package.json)
    pub project_path: String,
    /// Command to execute: "install", "update", "add", "remove", "ci", "audit", "outdated"
    pub command: String,
    /// Packages to add/remove (required for "add" and "remove" commands)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packages: Option<Vec<String>>,
    /// Additional flags (e.g., ["--save-dev", "--frozen-lockfile"])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<Vec<String>>,
    /// Timeout in milliseconds (default: 5 minutes, max: 30 minutes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

// ============================================================================
// Background Process Tool Parameters
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetBackgroundProcessOutputParams {
    /// The process ID returned from run_npm_script (e.g., "bp_abc123")
    pub process_id: String,
    /// Number of lines to return from the end (default: 100)
    #[serde(default = "default_tail_lines")]
    pub tail_lines: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StopBackgroundProcessParams {
    /// The process ID to stop
    pub process_id: String,
    /// Send SIGKILL instead of SIGTERM (default: false)
    #[serde(default)]
    pub force: bool,
}

// ============================================================================
// MCP Action Tool Parameters
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListActionsParams {
    /// Filter by action type (script, webhook, workflow)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_type: Option<String>,
    /// Filter by project ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    /// Only return enabled actions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_only: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetActionParams {
    /// Action ID to retrieve
    pub action_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunScriptParams {
    /// Action ID of the script to execute
    pub action_id: String,
    /// Additional arguments to pass to the script
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Environment variable overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    /// Working directory override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TriggerWebhookParams {
    /// Action ID of the webhook to trigger
    pub action_id: String,
    /// Variables for URL/payload template substitution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<HashMap<String, String>>,
    /// Payload override (replaces template)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetExecutionStatusParams {
    /// Execution ID to check
    pub execution_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListActionExecutionsParams {
    /// Filter by action ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    /// Filter by action type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_type: Option<String>,
    /// Filter by status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Maximum number of results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetActionPermissionsParams {
    /// Optional action ID to get specific permission
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
}

// ============================================================================
// Enhanced MCP Tool Parameters
// ============================================================================

/// Parameters for get_environment_info tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetEnvironmentInfoParams {
    /// Include PATH environment details (default: false)
    #[serde(default)]
    pub include_paths: bool,
    /// Optional project path to check project-specific toolchain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
}

/// Parameters for list_ai_providers tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListAIProvidersParams {
    /// Only return enabled providers (default: true)
    #[serde(default = "default_true")]
    pub enabled_only: bool,
}

/// Parameters for check_file_exists tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckFileExistsParams {
    /// Base project path - must be a registered project
    pub project_path: String,
    /// Relative paths to check (e.g., ['package.json', 'src/index.ts'])
    pub paths: Vec<String>,
}

/// Parameters for list_conversations tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListConversationsParams {
    /// Filter by project path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    /// Maximum number of conversations to return (default: 20, max: 100)
    #[serde(default = "default_limit_20")]
    pub limit: i64,
    /// Search in conversation titles
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_query: Option<String>,
}

/// Parameters for get_notifications tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetNotificationsParams {
    /// Filter by notification category
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Only return unread notifications (default: false)
    #[serde(default)]
    pub unread_only: bool,
    /// Maximum notifications to return (default: 20)
    #[serde(default = "default_limit_20")]
    pub limit: i64,
}

/// Parameters for mark_notifications_read tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MarkNotificationsReadParams {
    /// List of notification IDs to mark as read
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_ids: Option<Vec<String>>,
    /// Mark all notifications as read (default: false)
    #[serde(default)]
    pub mark_all: bool,
}

/// Parameters for get_security_scan_results tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetSecurityScanResultsParams {
    /// Path to the project - use actual path from list_projects
    pub project_path: String,
}

/// Parameters for run_security_scan tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunSecurityScanParams {
    /// Path to the project - must be a registered project
    pub project_path: String,
    /// Attempt to auto-fix vulnerabilities (default: false)
    #[serde(default)]
    pub fix: bool,
}

/// Parameters for list_deployments tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListDeploymentsParams {
    /// Path to the project
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    /// Filter by deployment platform (github_pages, netlify, cloudflare_pages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    /// Filter by deployment status (pending, building, success, failed, cancelled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Maximum deployments to return (default: 10)
    #[serde(default = "default_limit_10")]
    pub limit: i64,
}

/// Parameters for get_project_dependencies tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetProjectDependenciesParams {
    /// Path to the project
    pub project_path: String,
    /// Include devDependencies (default: true)
    #[serde(default = "default_true")]
    pub include_dev: bool,
    /// Include peerDependencies (default: false)
    #[serde(default)]
    pub include_peer: bool,
}

/// Parameters for update_workflow tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkflowParams {
    /// The workflow ID - use actual ID from list_workflows
    pub workflow_id: String,
    /// New workflow name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New workflow description (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Parameters for delete_workflow_step tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteWorkflowStepParams {
    /// The workflow ID
    pub workflow_id: String,
    /// The step/node ID to remove - use actual ID from get_workflow
    pub step_id: String,
}

/// Parameters for get_workflow_execution_details tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetWorkflowExecutionDetailsParams {
    /// The execution ID from list_action_executions
    pub execution_id: String,
    /// Include full stdout/stderr output (default: true)
    #[serde(default = "default_true")]
    pub include_output: bool,
    /// Max characters per step output (default: 5000)
    #[serde(default = "default_output_limit")]
    pub truncate_output: usize,
}

/// Parameters for search_project_files tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchProjectFilesParams {
    /// Base project path - must be a registered project
    pub project_path: String,
    /// File name pattern (glob syntax, e.g., '*.ts', 'src/**/*.tsx')
    pub pattern: String,
    /// Maximum files to return (default: 50)
    #[serde(default = "default_limit_50")]
    pub max_results: usize,
    /// Include directory matches (default: false)
    #[serde(default)]
    pub include_directories: bool,
}

/// Parameters for read_project_file tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadProjectFileParams {
    /// Base project path - must be a registered project
    pub project_path: String,
    /// Relative path to the file within the project
    pub file_path: String,
    /// Maximum lines to read (default: 500)
    #[serde(default = "default_max_lines")]
    pub max_lines: usize,
    /// Line to start reading from (1-based, default: 1)
    #[serde(default = "default_start_line")]
    pub start_line: usize,
}

// ============================================================================
// Response Types for Tools
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    /// Project ID (if registered in SpecForge, null if not registered)
    pub id: Option<String>,
    /// Project path
    pub path: String,
    /// Project name
    pub name: String,
    /// Project description
    pub description: Option<String>,
    /// Git remote URL
    pub git_remote: Option<String>,
    /// Current git branch
    pub current_branch: Option<String>,
    /// Package manager detected (npm, yarn, pnpm, bun)
    pub package_manager: Option<String>,
    /// Available scripts from package.json
    pub scripts: Option<HashMap<String, String>>,
    /// Project type (node, rust, python, etc.)
    pub project_type: Option<String>,
    /// Node.js version from .nvmrc, .node-version, or package.json engines
    pub node_version: Option<String>,
    /// Associated workflows in SpecForge
    pub workflows: Option<Vec<WorkflowRef>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRef {
    pub id: String,
    pub name: String,
}

/// Project summary for list_projects response
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectListItem {
    /// Project ID
    pub id: String,
    /// Project name
    pub name: String,
    /// Project path
    pub path: String,
    /// Project description
    pub description: Option<String>,
    /// Project type (node, rust, python, tauri, nextjs, etc.)
    pub project_type: Option<String>,
    /// Package manager (npm, yarn, pnpm, bun)
    pub package_manager: Option<String>,
    /// Current git branch
    pub current_branch: Option<String>,
    /// Number of workflows associated with this project
    pub workflow_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorktreeInfo {
    pub path: String,
    pub branch: String,
    pub is_main: bool,
    pub is_bare: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GitStatusInfo {
    pub branch: String,
    pub ahead: i32,
    pub behind: i32,
    pub staged: Vec<String>,
    pub modified: Vec<String>,
    pub untracked: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiffInfo {
    pub diff: String,
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

// Workflow response types
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowSummary {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub step_count: usize,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_executed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkflowResponse {
    pub workflow_id: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddStepResponse {
    pub node_id: String,
    pub workflow_id: String,
    pub name: String,
    pub order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StepTemplateInfo {
    pub id: String,
    pub name: String,
    pub command: String,
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub is_custom: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTemplateResponse {
    pub template_id: String,
    pub name: String,
    pub category: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunWorkflowResponse {
    pub success: bool,
    pub workflow_id: String,
    pub workflow_name: String,
    pub steps_executed: usize,
    pub total_steps: usize,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_step: Option<FailedStepInfo>,
    pub output_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FailedStepInfo {
    pub node_id: String,
    pub node_name: String,
    pub exit_code: i32,
    pub error_message: String,
}

// ============================================================================
// Time Machine & Security Guardian Tool Parameters
// ============================================================================

/// Parameters for check_dependency_integrity tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckDependencyIntegrityParams {
    /// Path to the project - use actual path from list_projects
    pub project_path: String,
    /// Optional workflow ID to use for reference snapshot
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
}

/// Parameters for get_security_insights tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetSecurityInsightsParams {
    /// Path to the project - use actual path from list_projects
    pub project_path: String,
}

/// Parameters for list_execution_snapshots tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListExecutionSnapshotsParams {
    /// Project path to filter snapshots
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    /// Maximum number of snapshots to return (default: 10)
    #[serde(default = "default_snapshot_limit")]
    pub limit: i32,
}

fn default_snapshot_limit() -> i32 {
    10
}

/// Parameters for get_snapshot_details tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetSnapshotDetailsParams {
    /// Snapshot ID to retrieve
    pub snapshot_id: String,
    /// Whether to include full dependency list (default: false)
    #[serde(default)]
    pub include_dependencies: bool,
}

/// Parameters for compare_snapshots tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompareSnapshotsParams {
    /// ID of the base snapshot (older)
    pub snapshot_a_id: String,
    /// ID of the comparison snapshot (newer)
    pub snapshot_b_id: String,
}

/// Parameters for search_snapshots tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchSnapshotsParams {
    /// Package name to search for
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_name: Option<String>,
    /// Package version to filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_version: Option<String>,
    /// Project path to filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    /// Start date (ISO 8601 format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_date: Option<String>,
    /// End date (ISO 8601 format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_date: Option<String>,
    /// Maximum number of results (default: 20)
    #[serde(default = "default_search_limit")]
    pub limit: i32,
}

/// Parameters for capture_snapshot tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSnapshotParams {
    /// Path to the project directory to snapshot
    pub project_path: String,
}

fn default_search_limit() -> i32 {
    20
}

/// Parameters for replay_execution tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReplayExecutionParams {
    /// Snapshot ID to replay from
    pub snapshot_id: String,
    /// Option for handling mismatches: "abort", "view_diff", "restore_lockfile", "proceed_with_current"
    #[serde(default = "default_replay_option")]
    pub option: String,
    /// Force replay even if there are significant mismatches
    #[serde(default)]
    pub force: bool,
}

fn default_replay_option() -> String {
    "abort".to_string()
}

/// Parameters for export_security_report tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExportSecurityReportParams {
    /// Path to the project
    pub project_path: String,
    /// Export format: "json", "markdown", or "html"
    #[serde(default = "default_export_format")]
    pub format: String,
}

fn default_export_format() -> String {
    "markdown".to_string()
}
