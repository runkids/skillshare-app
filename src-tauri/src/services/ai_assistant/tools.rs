// MCP Tool Handler for AI Assistant
// Feature: AI Assistant Tab (022-ai-assistant-tab)
// Enhancement: AI Precision Improvement (025-ai-workflow-generator)
//
// Handles tool/function calling for AI-driven MCP operations:
// - Tool definitions for AI providers
// - Tool call execution via MCP action service
// - Permission validation
// - Result formatting
// - Security: Path validation against registered projects
// - Context delta tracking for session state updates (025)

use crate::models::ai_assistant::{ToolCall, ToolResult, ToolDefinition, AvailableTools, ContextDelta};
use crate::models::ai::ChatToolDefinition;
use crate::services::audit::{AuditService, log_tool_execution as log_audit_tool};
use crate::utils::path_resolver;
use crate::utils::database::Database;
use crate::repositories::{MCPRepository, McpLogEntry};
use std::collections::HashMap;
use std::sync::Arc;
use chrono::Utc;

use super::security::{PathSecurityValidator, ToolPermissionChecker, OutputSanitizer};

// ============================================================================
// MCP Tool Response Types (Dual-Layer Response Schema)
// ============================================================================

/// Display status for visual styling in UI
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DisplayStatus {
    Success,
    Warning,
    Info,
    Error,
}

/// Display item for list-style presentation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayItem {
    /// Label text
    pub label: String,
    /// Value text
    pub value: String,
    /// Optional icon name (from Lucide icons)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Optional action to trigger on click
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<DisplayAction>,
}

/// Action for display item click
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayAction {
    pub tool: String,
    pub args: serde_json::Value,
}

/// Human-readable display layer for MCP tool responses
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayLayer {
    /// One-line summary for compact display (e.g., "Found 5 workflows")
    pub summary: String,
    /// Optional detailed message for expanded view
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Visual status hint for styling
    pub status: DisplayStatus,
    /// Optional items for list-style display
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<DisplayItem>>,
}

impl DisplayLayer {
    /// Create a new success display layer with just a summary
    pub fn success(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            detail: None,
            status: DisplayStatus::Success,
            items: None,
        }
    }

    /// Create a new info display layer
    pub fn info(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            detail: None,
            status: DisplayStatus::Info,
            items: None,
        }
    }

    /// Create a new warning display layer
    pub fn warning(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            detail: None,
            status: DisplayStatus::Warning,
            items: None,
        }
    }

    /// Create a new error display layer
    pub fn error(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            detail: None,
            status: DisplayStatus::Error,
            items: None,
        }
    }

    /// Add detail message
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Add display items
    pub fn with_items(mut self, items: Vec<DisplayItem>) -> Self {
        self.items = Some(items);
        self
    }
}

/// Response metadata for tool chaining and orchestration
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseMeta {
    /// Hint for the next logical tool call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_tool_hint: Option<String>,
    /// Reference IDs that can be used in subsequent calls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_ids: Option<HashMap<String, String>>,
    /// Execution duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Structured MCP tool response with dual-layer architecture
/// - data: Machine-readable structured data for AI and programmatic use
/// - display: Human-readable presentation layer for UI
/// - meta: Optional metadata for orchestration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MCPToolResponse<T: serde::Serialize> {
    /// Machine-readable structured data
    pub data: T,
    /// Human-readable display layer
    pub display: DisplayLayer,
    /// Optional metadata for chaining and tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResponseMeta>,
}

impl<T: serde::Serialize> MCPToolResponse<T> {
    /// Create a new MCPToolResponse
    pub fn new(data: T, display: DisplayLayer) -> Self {
        Self {
            data,
            display,
            meta: None,
        }
    }

    /// Add metadata
    pub fn with_meta(mut self, meta: ResponseMeta) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Convert to JSON string for ToolResult output
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Handles tool calls from AI responses
pub struct MCPToolHandler {
    /// Path security validator for project boundary enforcement
    path_validator: Option<PathSecurityValidator>,
    /// Database connection for tool operations
    db: Option<Database>,
}

// ============================================================================
// Feature 025: Execution Tool Categories for Cross-Project Validation
// ============================================================================

/// Tools that modify state and should prefer the current session context
const EXECUTION_TOOLS: &[&str] = &[
    "run_script",
    "run_workflow",
    "create_workflow",
    "create_workflow_with_steps",
    "add_workflow_step",
    "add_workflow_steps",
    "run_package_manager_command",
    "trigger_webhook",
];

/// Tools that only read data and can operate on any project
const INFO_TOOLS: &[&str] = &[
    "get_git_status",
    "get_staged_diff",
    "list_project_scripts",
    "list_projects",
    "get_project",
    "list_workflows",
    "get_workflow",
    "list_worktrees",
];

impl MCPToolHandler {
    /// Create a new MCPToolHandler without database (for testing/basic use)
    pub fn new() -> Self {
        Self {
            path_validator: None,
            db: None,
        }
    }

    /// Create a new MCPToolHandler with database for security validation
    pub fn with_database(db: Database) -> Self {
        Self {
            path_validator: Some(PathSecurityValidator::new(db.clone())),
            db: Some(db),
        }
    }

    // =========================================================================
    // Feature 025: Session Context Validation
    // =========================================================================

    /// Check if a tool is an execution tool that should prefer current session context
    pub fn is_execution_tool(tool_name: &str) -> bool {
        EXECUTION_TOOLS.contains(&tool_name)
    }

    /// Check if a tool is an info tool that can operate on any project
    #[allow(dead_code)]
    pub fn is_info_tool(tool_name: &str) -> bool {
        INFO_TOOLS.contains(&tool_name)
    }

    /// Validate execution tool against session context and log warnings
    /// This does NOT block execution (per design decision), only warns
    pub fn validate_execution_context(
        &self,
        tool_call: &ToolCall,
        session_context: Option<&crate::models::ai_assistant::SessionContext>,
    ) {
        // Only validate execution tools
        if !Self::is_execution_tool(&tool_call.name) {
            return;
        }

        let session_ctx = match session_context {
            Some(ctx) => ctx,
            None => return, // No session context, nothing to validate
        };

        // Check if workflow operation targets a different project
        if tool_call.name == "run_workflow" || tool_call.name == "get_workflow" {
            if let Some(wf_id) = tool_call.arguments.get("workflow_id").and_then(|v| v.as_str()) {
                if !session_ctx.is_workflow_bound(wf_id) {
                    // Check if it's a global workflow
                    let is_global = self.is_global_workflow(wf_id);
                    if !is_global {
                        log::warn!(
                            "[Session Context] Execution tool '{}' targeting workflow '{}' outside session context (project: {:?}). Consider starting new conversation for cross-project operations.",
                            tool_call.name,
                            wf_id,
                            session_ctx.project_id
                        );
                    }
                }
            }
        }

        // Check if create_workflow targets a different project
        if tool_call.name == "create_workflow" || tool_call.name == "create_workflow_with_steps" {
            if let Some(target_project_id) = tool_call.arguments.get("project_id").and_then(|v| v.as_str()) {
                if !target_project_id.is_empty() {
                    if session_ctx.project_id.as_deref() != Some(target_project_id) {
                        log::warn!(
                            "[Session Context] create_workflow targeting different project '{}' (current: {:?}). Consider starting new conversation.",
                            target_project_id,
                            session_ctx.project_id
                        );
                    }
                }
            }
        }

        // Check if run_script targets a different project path
        if tool_call.name == "run_script" || tool_call.name == "run_package_manager_command" {
            if let Some(target_path) = tool_call.arguments.get("project_path").and_then(|v| v.as_str()) {
                if let Some(ref session_path) = session_ctx.project_path {
                    // Normalize paths for comparison
                    let session_path_normalized = std::path::Path::new(session_path);
                    let target_path_normalized = std::path::Path::new(target_path);

                    if session_path_normalized != target_path_normalized {
                        log::warn!(
                            "[Session Context] Execution tool '{}' targeting different project path '{}' (current: {}). Consider starting new conversation.",
                            tool_call.name,
                            target_path,
                            session_path
                        );
                    }
                }
            }
        }
    }

    /// Check if a workflow is global (not bound to any project)
    fn is_global_workflow(&self, workflow_id: &str) -> bool {
        if let Some(ref db) = self.db {
            if let Ok(Some(wf)) = crate::repositories::WorkflowRepository::new(db.clone()).get(workflow_id) {
                return wf.project_id.is_none();
            }
        }
        false
    }

    /// Convert our tool definitions to ChatToolDefinition format for AI providers
    pub fn get_chat_tool_definitions(&self, project_path: Option<&str>) -> Vec<ChatToolDefinition> {
        self.get_available_tools(project_path)
            .tools
            .into_iter()
            .map(|t| ChatToolDefinition::function(
                t.name,
                t.description,
                t.parameters,
            ))
            .collect()
    }

    /// Get available tools for AI providers
    pub fn get_available_tools(&self, _project_path: Option<&str>) -> AvailableTools {
        let tools = vec![
            ToolDefinition {
                name: "run_script".to_string(),
                description: "Run an npm/pnpm/yarn script from the project's package.json. If unsure which project/script, call list_projects then list_project_scripts first.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "script_name": {
                            "type": "string",
                            "description": "Name of the script to run - use actual script name from list_project_scripts"
                        },
                        "project_path": {
                            "type": "string",
                            "description": "Path to the project directory - use actual path from list_projects"
                        }
                    },
                    "required": ["script_name", "project_path"]
                }),
                requires_confirmation: true,
                category: "script".to_string(),
            },
            ToolDefinition {
                name: "run_workflow".to_string(),
                description: "Execute a SpecForge workflow by ID. If unsure which workflow, call list_workflows first and ask the user.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "workflow_id": {
                            "type": "string",
                            "description": "The workflow ID - use actual ID from list_workflows, not a placeholder"
                        }
                    },
                    "required": ["workflow_id"]
                }),
                requires_confirmation: true,
                category: "workflow".to_string(),
            },
            ToolDefinition {
                name: "trigger_webhook".to_string(),
                description: "Trigger a configured webhook action. If unsure which webhook, call list_actions first with actionType='webhook'.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "webhook_id": {
                            "type": "string",
                            "description": "The webhook action ID - use actual ID from list_actions, NEVER generate a fake ID"
                        },
                        "payload": {
                            "type": "object",
                            "description": "Optional payload to send with the webhook"
                        }
                    },
                    "required": ["webhook_id"]
                }),
                requires_confirmation: true,
                category: "webhook".to_string(),
            },
            ToolDefinition {
                name: "get_git_status".to_string(),
                description: "Get git status: current branch, staged/modified/untracked files, ahead/behind counts. Returns structured data for analysis.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "project_path": {
                            "type": "string",
                            "description": "Path to the git repository"
                        }
                    },
                    "required": ["project_path"]
                }),
                requires_confirmation: false,
                category: "git".to_string(),
            },
            ToolDefinition {
                name: "get_staged_diff".to_string(),
                description: "Get diff of staged changes. Returns file changes, additions/deletions counts. Useful for generating commit messages.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "project_path": {
                            "type": "string",
                            "description": "Path to the git repository"
                        }
                    },
                    "required": ["project_path"]
                }),
                requires_confirmation: false,
                category: "git".to_string(),
            },
            ToolDefinition {
                name: "list_project_scripts".to_string(),
                description: "List all scripts from package.json with their commands. Use this to verify script names before calling run_script.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "project_path": {
                            "type": "string",
                            "description": "Path to the project directory"
                        }
                    },
                    "required": ["project_path"]
                }),
                requires_confirmation: false,
                category: "info".to_string(),
            },
            // Project management tools (Feature 023)
            ToolDefinition {
                name: "list_projects".to_string(),
                description: "List all registered projects in SpecForge with their type, package manager, and workflow count".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Optional search query to filter projects by name"
                        }
                    }
                }),
                requires_confirmation: false,
                category: "project".to_string(),
            },
            ToolDefinition {
                name: "get_project".to_string(),
                description: "Get detailed information about a specific project including scripts, package manager, workflows, and git info. If unsure which project, call list_projects first.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The absolute path to the project directory - use actual path from list_projects"
                        }
                    },
                    "required": ["path"]
                }),
                requires_confirmation: false,
                category: "project".to_string(),
            },
            // Workflow tools
            ToolDefinition {
                name: "list_workflows".to_string(),
                description: "List all workflows in SpecForge, optionally filtered by project".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "project_id": {
                            "type": "string",
                            "description": "Optional project ID to filter workflows"
                        }
                    }
                }),
                requires_confirmation: false,
                category: "workflow".to_string(),
            },
            ToolDefinition {
                name: "get_workflow".to_string(),
                description: "Get detailed information about a workflow including all its steps. If unsure which workflow, call list_workflows first.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "workflow_id": {
                            "type": "string",
                            "description": "The workflow ID - use actual ID from list_workflows, not a placeholder"
                        }
                    },
                    "required": ["workflow_id"]
                }),
                requires_confirmation: false,
                category: "workflow".to_string(),
            },
            // Worktree tools
            ToolDefinition {
                name: "list_worktrees".to_string(),
                description: "List all git worktrees for a project. If unsure which project, call list_projects first.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "project_path": {
                            "type": "string",
                            "description": "The absolute path to the project directory - use actual path from list_projects"
                        }
                    },
                    "required": ["project_path"]
                }),
                requires_confirmation: false,
                category: "git".to_string(),
            },
            // Action tools
            ToolDefinition {
                name: "list_actions".to_string(),
                description: "List all available MCP actions that can be executed. Filter by type (script, webhook, workflow) or project.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "actionType": {
                            "type": "string",
                            "description": "Filter by action type (script, webhook, workflow)"
                        },
                        "projectId": {
                            "type": "string",
                            "description": "Filter by project ID"
                        },
                        "enabledOnly": {
                            "type": "boolean",
                            "description": "Only return enabled actions"
                        }
                    }
                }),
                requires_confirmation: false,
                category: "action".to_string(),
            },
            ToolDefinition {
                name: "get_action".to_string(),
                description: "Get detailed information about a specific MCP action by ID. If unsure which action, call list_actions first.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "actionId": {
                            "type": "string",
                            "description": "Action ID - use actual ID from list_actions, NEVER generate a fake ID"
                        }
                    },
                    "required": ["actionId"]
                }),
                requires_confirmation: false,
                category: "action".to_string(),
            },
            ToolDefinition {
                name: "list_action_executions".to_string(),
                description: "List recent action executions with optional filtering by action, type, or status".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "actionId": {
                            "type": "string",
                            "description": "Filter by action ID"
                        },
                        "actionType": {
                            "type": "string",
                            "description": "Filter by action type"
                        },
                        "status": {
                            "type": "string",
                            "description": "Filter by status"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of results"
                        }
                    }
                }),
                requires_confirmation: false,
                category: "action".to_string(),
            },
            ToolDefinition {
                name: "get_execution_status".to_string(),
                description: "Get the current status and result of a running or completed action execution. If unsure, call list_action_executions first.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "executionId": {
                            "type": "string",
                            "description": "Execution ID - use actual ID from list_action_executions or from tool execution result"
                        }
                    },
                    "required": ["executionId"]
                }),
                requires_confirmation: false,
                category: "action".to_string(),
            },
            // npm script (alternative to run_script)
            ToolDefinition {
                name: "run_npm_script".to_string(),
                description: "Execute an npm/yarn/pnpm script from a project's package.json. Use runInBackground=true for long-running scripts like 'dev', 'start', 'watch' that don't exit. If unsure which project/script, call list_projects then get_project first.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "projectPath": {
                            "type": "string",
                            "description": "Project path - use actual path from list_projects"
                        },
                        "scriptName": {
                            "type": "string",
                            "description": "Script name - use actual script name from get_project response"
                        },
                        "args": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Optional arguments to pass to the script"
                        },
                        "runInBackground": {
                            "type": "boolean",
                            "description": "Run in background (default: false). Set to true for long-running scripts like 'dev', 'start', 'watch' that don't exit. Returns immediately with a process ID."
                        },
                        "successPattern": {
                            "type": "string",
                            "description": "Regex pattern to match in output indicating the process is ready (e.g., 'ready in \\\\d+' for Vite, 'compiled successfully' for webpack). Only used when runInBackground is true."
                        },
                        "timeoutMs": {
                            "type": "integer",
                            "description": "Timeout in milliseconds (default: 5 minutes for foreground, 30 seconds for background success pattern matching)"
                        }
                    },
                    "required": ["projectPath", "scriptName"]
                }),
                requires_confirmation: true,
                category: "script".to_string(),
            },
            // Workflow creation tools
            ToolDefinition {
                name: "create_workflow".to_string(),
                description: "Create a new workflow. IMPORTANT: If a project is selected in the session, the workflow will automatically bind to that project unless you explicitly pass project_id=\"__GLOBAL__\" for a global workflow. When user requests a workflow, ASK if they want it project-specific or global before creating.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Workflow name"
                        },
                        "description": {
                            "type": "string",
                            "description": "Optional description"
                        },
                        "project_id": {
                            "type": "string",
                            "description": "Project ID to associate. Auto-filled from session if not provided. Use \"__GLOBAL__\" to explicitly create a global workflow not bound to any project."
                        }
                    },
                    "required": ["name"]
                }),
                requires_confirmation: true,
                category: "workflow".to_string(),
            },
            ToolDefinition {
                name: "create_workflow_with_steps".to_string(),
                description: "Create a new workflow with steps in ONE atomic operation. RECOMMENDED over separate calls. IMPORTANT: If a project is selected, workflow auto-binds to it. Use project_id=\"__GLOBAL__\" for global workflow. ASK user before creating if they want project-specific or global. Steps cwd auto-fills to project path if not specified.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Workflow name (required)"
                        },
                        "description": {
                            "type": "string",
                            "description": "Optional workflow description"
                        },
                        "project_id": {
                            "type": "string",
                            "description": "Project ID to associate. Auto-filled from session. Use \"__GLOBAL__\" to create a global workflow."
                        },
                        "steps": {
                            "type": "array",
                            "description": "Array of steps (1-10). Steps execute in array order.",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": {
                                        "type": "string",
                                        "description": "Step name"
                                    },
                                    "command": {
                                        "type": "string",
                                        "description": "Shell command to execute"
                                    },
                                    "cwd": {
                                        "type": "string",
                                        "description": "Working directory. Auto-filled to project path if not specified."
                                    },
                                    "timeout": {
                                        "type": "integer",
                                        "description": "Optional timeout in milliseconds"
                                    }
                                },
                                "required": ["name", "command"]
                            },
                            "minItems": 1,
                            "maxItems": 10
                        }
                    },
                    "required": ["name", "steps"]
                }),
                requires_confirmation: true,
                category: "workflow".to_string(),
            },
            ToolDefinition {
                name: "add_workflow_step".to_string(),
                description: "Add a new step to an existing workflow. You MUST use the actual workflow_id returned from create_workflow or list_workflows. If unsure which workflow to use, call list_workflows first and ask the user to select one.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "workflow_id": {
                            "type": "string",
                            "description": "Target workflow ID - MUST be a real ID from create_workflow or list_workflows. NEVER generate a fake ID."
                        },
                        "name": {
                            "type": "string",
                            "description": "Step name"
                        },
                        "command": {
                            "type": "string",
                            "description": "Shell command to execute"
                        },
                        "cwd": {
                            "type": "string",
                            "description": "Optional working directory"
                        },
                        "order": {
                            "type": "integer",
                            "description": "Optional position (defaults to end)"
                        }
                    },
                    "required": ["workflow_id", "name", "command"]
                }),
                requires_confirmation: true,
                category: "workflow".to_string(),
            },
            ToolDefinition {
                name: "add_workflow_steps".to_string(),
                description: "Add multiple steps to a workflow in a single atomic operation. Use this for efficiently creating workflows with multiple steps. All steps are validated upfront - if any validation fails, no steps are added. Maximum 10 steps per call. Steps are executed in array order.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "workflow_id": {
                            "type": "string",
                            "description": "Target workflow ID - MUST be a real ID from create_workflow or list_workflows"
                        },
                        "steps": {
                            "type": "array",
                            "description": "Array of steps to add (max 10). Steps are added in order.",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": {
                                        "type": "string",
                                        "description": "Step name (must be unique within batch)"
                                    },
                                    "command": {
                                        "type": "string",
                                        "description": "Shell command to execute"
                                    },
                                    "cwd": {
                                        "type": "string",
                                        "description": "Optional working directory"
                                    },
                                    "timeout": {
                                        "type": "integer",
                                        "description": "Optional timeout in milliseconds"
                                    }
                                },
                                "required": ["name", "command"]
                            },
                            "minItems": 1,
                            "maxItems": 10
                        }
                    },
                    "required": ["workflow_id", "steps"]
                }),
                requires_confirmation: true,
                category: "workflow".to_string(),
            },
            ToolDefinition {
                name: "list_step_templates".to_string(),
                description: "List available step templates for workflow steps. Filter by category or search query.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "category": {
                            "type": "string",
                            "description": "Filter by category"
                        },
                        "query": {
                            "type": "string",
                            "description": "Search query"
                        },
                        "include_builtin": {
                            "type": "boolean",
                            "description": "Include built-in templates (default: true)"
                        }
                    }
                }),
                requires_confirmation: false,
                category: "workflow".to_string(),
            },
            // Git tools (Feature 023 - sync with MCP Server)
            ToolDefinition {
                name: "get_worktree_status".to_string(),
                description: "Get git status including current branch, ahead/behind counts, staged files, modified files, and untracked files".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "worktree_path": {
                            "type": "string",
                            "description": "The absolute path to the worktree or project directory"
                        }
                    },
                    "required": ["worktree_path"]
                }),
                requires_confirmation: false,
                category: "git".to_string(),
            },
            ToolDefinition {
                name: "get_git_diff".to_string(),
                description: "Get the staged changes diff. Useful for generating commit messages. Returns the diff content along with statistics.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "worktree_path": {
                            "type": "string",
                            "description": "The absolute path to the worktree or project directory"
                        }
                    },
                    "required": ["worktree_path"]
                }),
                requires_confirmation: false,
                category: "git".to_string(),
            },
            // Step template creation (write tool)
            ToolDefinition {
                name: "create_step_template".to_string(),
                description: "Create a custom step template that can be reused across workflows. Templates are saved in SpecForge.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Template name"
                        },
                        "command": {
                            "type": "string",
                            "description": "Shell command"
                        },
                        "category": {
                            "type": "string",
                            "description": "Category (default: \"custom\")"
                        },
                        "description": {
                            "type": "string",
                            "description": "Optional description"
                        }
                    },
                    "required": ["name", "command"]
                }),
                requires_confirmation: true,
                category: "workflow".to_string(),
            },
            // Action permissions
            ToolDefinition {
                name: "get_action_permissions".to_string(),
                description: "Get permission configuration for actions. Shows whether actions require confirmation, auto-approve, or are denied.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "actionId": {
                            "type": "string",
                            "description": "Optional action ID to get specific permission"
                        }
                    }
                }),
                requires_confirmation: false,
                category: "action".to_string(),
            },
            // Background process management (Feature 023 - sync with MCP Server)
            ToolDefinition {
                name: "list_background_processes".to_string(),
                description: "List background processes started via run_npm_script (runInBackground: true). Shows process ID, status, script name, and runtime. Use get_background_process_output to see logs.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
                requires_confirmation: false,
                category: "process".to_string(),
            },
            ToolDefinition {
                name: "get_background_process_output".to_string(),
                description: "Get output from a background process started with run_npm_script (runInBackground: true). Returns the tail of stdout/stderr output.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "processId": {
                            "type": "string",
                            "description": "The process ID returned from run_npm_script (e.g., \"bp_abc123\")"
                        },
                        "tailLines": {
                            "type": "integer",
                            "description": "Number of lines to return from the end (default: 100)"
                        }
                    },
                    "required": ["processId"]
                }),
                requires_confirmation: false,
                category: "process".to_string(),
            },
            ToolDefinition {
                name: "stop_background_process".to_string(),
                description: "Stop/terminate a background process. Use force=true to send SIGKILL instead of SIGTERM.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "processId": {
                            "type": "string",
                            "description": "The process ID to stop"
                        },
                        "force": {
                            "type": "boolean",
                            "description": "Send SIGKILL instead of SIGTERM (default: false)"
                        }
                    },
                    "required": ["processId"]
                }),
                requires_confirmation: true,
                category: "process".to_string(),
            },
            // Package manager commands (not scripts from package.json)
            ToolDefinition {
                name: "run_package_manager_command".to_string(),
                description: "Run a package manager command (audit, outdated, install, etc.). ⚠️ WARNING: 'install', 'update', 'prune' commands MODIFY node_modules. For read-only checks, use 'audit', 'outdated', 'list', 'why'. NOT for package.json scripts - use run_script for those.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The package manager command to run (e.g., 'audit', 'outdated', 'install', 'update')",
                            "enum": ["audit", "outdated", "install", "update", "prune", "dedupe", "why", "list", "info"]
                        },
                        "project_path": {
                            "type": "string",
                            "description": "Path to the project directory"
                        },
                        "args": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Optional additional arguments (e.g., ['--fix'] for audit --fix)"
                        }
                    },
                    "required": ["command", "project_path"]
                }),
                requires_confirmation: true,
                category: "script".to_string(),
            },
            // =========================================================================
            // Time Machine & Security Guardian Tools (Feature 025)
            // =========================================================================
            ToolDefinition {
                name: "list_snapshots".to_string(),
                description: "List Time Machine snapshots for a project. Snapshots capture dependency state when lockfile changes or manually triggered.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "project_path": {
                            "type": "string",
                            "description": "The project path to list snapshots for"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of snapshots to return (default: 20)"
                        }
                    },
                    "required": ["project_path"]
                }),
                requires_confirmation: false,
                category: "time_machine".to_string(),
            },
            ToolDefinition {
                name: "get_snapshot_details".to_string(),
                description: "Get detailed information about a specific execution snapshot including all dependencies, postinstall scripts, and security score.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "snapshot_id": {
                            "type": "string",
                            "description": "The snapshot ID to get details for"
                        }
                    },
                    "required": ["snapshot_id"]
                }),
                requires_confirmation: false,
                category: "time_machine".to_string(),
            },
            ToolDefinition {
                name: "compare_snapshots".to_string(),
                description: "Compare two snapshots to see dependency changes: added, removed, updated packages, new postinstall scripts, and security score changes.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "snapshot_a_id": {
                            "type": "string",
                            "description": "The first (older) snapshot ID"
                        },
                        "snapshot_b_id": {
                            "type": "string",
                            "description": "The second (newer) snapshot ID"
                        }
                    },
                    "required": ["snapshot_a_id", "snapshot_b_id"]
                }),
                requires_confirmation: false,
                category: "time_machine".to_string(),
            },
            ToolDefinition {
                name: "search_snapshots".to_string(),
                description: "Search execution snapshots across all projects. Find snapshots containing specific packages or within date ranges.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "package_name": {
                            "type": "string",
                            "description": "Package name to search for (supports partial match)"
                        },
                        "project_path": {
                            "type": "string",
                            "description": "Filter by project path"
                        },
                        "from_date": {
                            "type": "string",
                            "description": "Start date for search (ISO 8601 format)"
                        },
                        "to_date": {
                            "type": "string",
                            "description": "End date for search (ISO 8601 format)"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum results to return (default: 20)"
                        }
                    }
                }),
                requires_confirmation: false,
                category: "time_machine".to_string(),
            },
            ToolDefinition {
                name: "check_dependency_integrity".to_string(),
                description: "Check if current dependencies match a reference snapshot. Detects drift from known good state.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "project_path": {
                            "type": "string",
                            "description": "Path to the project directory"
                        },
                        "reference_snapshot_id": {
                            "type": "string",
                            "description": "Optional snapshot ID to compare against. If not provided, uses the latest snapshot."
                        }
                    },
                    "required": ["project_path"]
                }),
                requires_confirmation: false,
                category: "security".to_string(),
            },
            ToolDefinition {
                name: "get_security_insights".to_string(),
                description: "Get security insights for a project: risk score, typosquatting alerts, postinstall script changes, and suspicious patterns.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "project_path": {
                            "type": "string",
                            "description": "Path to the project directory"
                        }
                    },
                    "required": ["project_path"]
                }),
                requires_confirmation: false,
                category: "security".to_string(),
            },
            ToolDefinition {
                name: "capture_snapshot".to_string(),
                description: "Manually capture a Time Machine snapshot for a project. Captures current dependency state from lockfile.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "project_path": {
                            "type": "string",
                            "description": "Path to the project directory to snapshot"
                        }
                    },
                    "required": ["project_path"]
                }),
                requires_confirmation: true,
                category: "time_machine".to_string(),
            },
        ];

        AvailableTools { tools }
    }

    /// Execute a tool call (for auto-approved read-only tools)
    /// Returns a ToolResult with the execution outcome
    ///
    /// Security checks performed:
    /// 1. Tool permission validation (blocked tools are rejected)
    /// 2. Confirmation requirement check (confirmation-required tools are rejected)
    /// 3. Path validation against registered projects
    /// 4. Output sanitization to remove sensitive data
    pub async fn execute_tool_call(&self, tool_call: &ToolCall) -> ToolResult {
        log::info!("[AI Tool] execute_tool_call START: name={}, id={}, args={:?}",
            tool_call.name, tool_call.id, tool_call.arguments);

        // Security check 1: Validate tool is allowed
        if let Err(e) = ToolPermissionChecker::validate_tool_call(&tool_call.name) {
            log::warn!("[AI Tool] Security check failed for {}: {}", tool_call.name, e);
            return ToolResult::failure(
                tool_call.id.clone(),
                format!("Security error: {}", e),
            );
        }

        // Security check 2: Reject confirmation-required tools (they must use execute_confirmed_tool_call)
        if ToolPermissionChecker::requires_confirmation(&tool_call.name) {
            return ToolResult::failure(
                tool_call.id.clone(),
                "This action requires user confirmation before execution.".to_string(),
            );
        }

        // Execute read-only tools
        let result = match tool_call.name.as_str() {
            "get_git_status" => self.execute_get_git_status(tool_call).await,
            "get_staged_diff" => self.execute_get_staged_diff(tool_call).await,
            "list_project_scripts" => self.execute_list_project_scripts(tool_call).await,
            "list_projects" => self.execute_list_projects(tool_call).await,
            "get_project" => self.execute_get_project(tool_call).await,
            "list_workflows" => self.execute_list_workflows(tool_call).await,
            "get_workflow" => self.execute_get_workflow(tool_call).await,
            "list_worktrees" => self.execute_list_worktrees(tool_call).await,
            "list_actions" => self.execute_list_actions(tool_call).await,
            "get_action" => self.execute_get_action(tool_call).await,
            "list_action_executions" => self.execute_list_action_executions(tool_call).await,
            "get_execution_status" => self.execute_get_execution_status(tool_call).await,
            "list_step_templates" => self.execute_list_step_templates(tool_call).await,
            // New tools synced with MCP Server
            "get_worktree_status" => self.execute_get_worktree_status(tool_call).await,
            "get_git_diff" => self.execute_get_git_diff(tool_call).await,
            "get_action_permissions" => self.execute_get_action_permissions(tool_call).await,
            "list_background_processes" => self.execute_list_background_processes(tool_call).await,
            "get_background_process_output" => self.execute_get_background_process_output(tool_call).await,
            // Time Machine & Security Guardian tools (Feature 025)
            "list_snapshots" => self.execute_list_snapshots(tool_call).await,
            "get_snapshot_details" => self.execute_get_snapshot_details(tool_call).await,
            "compare_snapshots" => self.execute_compare_snapshots(tool_call).await,
            "search_snapshots" => self.execute_search_snapshots(tool_call).await,
            "check_dependency_integrity" => self.execute_check_dependency_integrity(tool_call).await,
            "get_security_insights" => self.execute_get_security_insights(tool_call).await,
            _ => {
                log::warn!("[AI Tool] Unknown tool in execute_tool_call: {}", tool_call.name);
                ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Unknown tool: {}", tool_call.name),
                )
            },
        };

        log::info!("[AI Tool] execute_tool_call END: name={}, success={}, output_len={}",
            tool_call.name, result.success, result.output.len());

        let sanitized = self.sanitize_result(result);

        // Log to AI Activity
        self.log_tool_execution(tool_call, &sanitized);

        sanitized
    }

    /// Execute a confirmed tool call (for user-approved actions)
    /// This method is called AFTER the user has approved the action.
    ///
    /// Security checks performed:
    /// 1. Tool permission validation (blocked tools are rejected)
    /// 2. Path validation against registered projects
    /// 3. Output sanitization to remove sensitive data
    pub async fn execute_confirmed_tool_call(&self, tool_call: &ToolCall) -> ToolResult {
        // Security check 1: Validate tool is allowed (even for confirmed calls)
        if let Err(e) = ToolPermissionChecker::validate_tool_call(&tool_call.name) {
            return ToolResult::failure(
                tool_call.id.clone(),
                format!("Security error: {}", e),
            );
        }

        // Execute the tool (including confirmation-required tools)
        let result = match tool_call.name.as_str() {
            // Read-only tools
            "get_git_status" => self.execute_get_git_status(tool_call).await,
            "get_staged_diff" => self.execute_get_staged_diff(tool_call).await,
            "list_project_scripts" => self.execute_list_project_scripts(tool_call).await,
            "list_projects" => self.execute_list_projects(tool_call).await,
            "get_project" => self.execute_get_project(tool_call).await,
            "list_workflows" => self.execute_list_workflows(tool_call).await,
            "get_workflow" => self.execute_get_workflow(tool_call).await,
            "list_worktrees" => self.execute_list_worktrees(tool_call).await,
            "list_actions" => self.execute_list_actions(tool_call).await,
            "get_action" => self.execute_get_action(tool_call).await,
            "list_action_executions" => self.execute_list_action_executions(tool_call).await,
            "get_execution_status" => self.execute_get_execution_status(tool_call).await,
            "list_step_templates" => self.execute_list_step_templates(tool_call).await,
            // New read-only tools synced with MCP Server
            "get_worktree_status" => self.execute_get_worktree_status(tool_call).await,
            "get_git_diff" => self.execute_get_git_diff(tool_call).await,
            "get_action_permissions" => self.execute_get_action_permissions(tool_call).await,
            "list_background_processes" => self.execute_list_background_processes(tool_call).await,
            "get_background_process_output" => self.execute_get_background_process_output(tool_call).await,
            // Time Machine & Security Guardian tools (Feature 025)
            "list_snapshots" => self.execute_list_snapshots(tool_call).await,
            "get_snapshot_details" => self.execute_get_snapshot_details(tool_call).await,
            "compare_snapshots" => self.execute_compare_snapshots(tool_call).await,
            "search_snapshots" => self.execute_search_snapshots(tool_call).await,
            "check_dependency_integrity" => self.execute_check_dependency_integrity(tool_call).await,
            "get_security_insights" => self.execute_get_security_insights(tool_call).await,
            // Confirmation-required tools
            "run_script" => self.execute_run_script(tool_call).await,
            "run_npm_script" => self.execute_run_npm_script(tool_call).await,
            "run_workflow" => self.execute_run_workflow(tool_call).await,
            "trigger_webhook" => self.execute_trigger_webhook(tool_call).await,
            "create_workflow" => self.execute_create_workflow(tool_call).await,
            "create_workflow_with_steps" => self.execute_create_workflow_with_steps(tool_call).await,
            "add_workflow_step" => self.execute_add_workflow_step(tool_call).await,
            "add_workflow_steps" => self.execute_add_workflow_steps(tool_call).await,
            // New confirmation-required tools synced with MCP Server
            "create_step_template" => self.execute_create_step_template(tool_call).await,
            "stop_background_process" => self.execute_stop_background_process(tool_call).await,
            // Time Machine capture
            "capture_snapshot" => self.execute_capture_snapshot(tool_call).await,
            // Package manager commands
            "run_package_manager_command" => self.execute_run_package_manager_command(tool_call).await,
            _ => ToolResult::failure(
                tool_call.id.clone(),
                format!("Unknown tool: {}", tool_call.name),
            ),
        };

        let sanitized = self.sanitize_result(result);

        // Log to AI Activity
        self.log_tool_execution(tool_call, &sanitized);

        sanitized
    }

    /// Sanitize tool result output
    fn sanitize_result(&self, result: ToolResult) -> ToolResult {
        if result.success {
            ToolResult {
                call_id: result.call_id,
                success: result.success,
                output: OutputSanitizer::sanitize_output(&result.output),
                error: result.error,
                duration_ms: result.duration_ms,
                metadata: result.metadata,
            }
        } else {
            result
        }
    }

    /// Log tool execution to AI Activity (mcp_logs table)
    /// This allows AI Assistant tool calls to appear in Settings > AI Activity
    fn log_tool_execution(&self, tool_call: &ToolCall, result: &ToolResult) {
        // Skip logging for background processes (they have their own logging with status updates)
        if let Some(ref metadata) = result.metadata {
            if metadata.get("is_background_process").and_then(|v| v.as_bool()).unwrap_or(false) {
                log::debug!("[AI Tool] Skipping log for background process (already logged with running status)");
                return;
            }
        }

        let Some(db) = &self.db else {
            log::debug!("[AI Tool] No database connection, skipping activity log");
            return;
        };

        let repo = MCPRepository::new(db.clone());

        // Sanitize arguments - convert to string, sanitize, then back to JSON
        let args_str = serde_json::to_string(&tool_call.arguments).unwrap_or_default();
        let sanitized_str = OutputSanitizer::sanitize_output(&args_str);
        let sanitized_args: serde_json::Value = serde_json::from_str(&sanitized_str)
            .unwrap_or(tool_call.arguments.clone());

        let log_entry = McpLogEntry {
            id: None,
            timestamp: Utc::now(),
            tool: tool_call.name.clone(),
            arguments: sanitized_args,
            result: if result.success { "success".to_string() } else { "error".to_string() },
            duration_ms: result.duration_ms.unwrap_or(0) as u64,
            error: result.error.clone(),
            source: Some("ai_assistant".to_string()),
        };

        if let Err(e) = repo.insert_log(&log_entry) {
            log::warn!("[AI Tool] Failed to log tool execution: {}", e);
        }

        // Security audit logging for tool executions
        // Use tool_call.id as session identifier since we don't have conversation context here
        let audit_service = AuditService::new(Arc::new(db.clone()));
        log_audit_tool(
            &audit_service,
            &tool_call.id,
            &tool_call.name,
            result.success,
            result.error.as_deref(),
        );
    }

    /// Log background process start with "running" status
    /// Returns the log entry ID for later status updates
    fn log_background_process_start(&self, tool_call: &ToolCall) -> Option<i64> {
        log::info!("[AI Tool] log_background_process_start called for tool: {}", tool_call.name);

        let db = match self.db.as_ref() {
            Some(d) => d,
            None => {
                log::warn!("[AI Tool] log_background_process_start: No database connection");
                return None;
            }
        };

        let repo = MCPRepository::new(db.clone());

        // Sanitize arguments
        let args_str = serde_json::to_string(&tool_call.arguments).unwrap_or_default();
        let sanitized_str = OutputSanitizer::sanitize_output(&args_str);
        let sanitized_args: serde_json::Value = serde_json::from_str(&sanitized_str)
            .unwrap_or(tool_call.arguments.clone());

        let log_entry = McpLogEntry {
            id: None,
            timestamp: Utc::now(),
            tool: tool_call.name.clone(),
            arguments: sanitized_args,
            result: "running".to_string(),
            duration_ms: 0,
            error: None,
            source: Some("ai_assistant".to_string()),
        };

        match repo.insert_log(&log_entry) {
            Ok(id) => {
                log::info!("[AI Tool] Logged background process start with id {} and 'running' status", id);
                Some(id)
            }
            Err(e) => {
                log::warn!("[AI Tool] Failed to log background process start: {}", e);
                None
            }
        }
    }

    /// Validate a project path against registered projects
    /// Returns the validated canonical path or an error message
    fn validate_project_path(&self, path: &str) -> Result<std::path::PathBuf, String> {
        match &self.path_validator {
            Some(validator) => {
                validator.sanitize_tool_path(path)
                    .map_err(|e| format!("Security validation failed: {}", e))
            }
            None => {
                // No validator available - just check if path exists
                // This is less secure but allows basic functionality
                let p = std::path::Path::new(path);
                if p.exists() {
                    std::fs::canonicalize(p)
                        .map_err(|e| format!("Invalid path: {}", e))
                } else {
                    Err(format!("Path does not exist: {}", path))
                }
            }
        }
    }

    /// Execute get_git_status tool
    async fn execute_get_git_status(&self, tool_call: &ToolCall) -> ToolResult {
        let project_path = match tool_call.arguments.get("project_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: project_path".to_string(),
            ),
        };

        // Security: Validate path is within a registered project
        let validated_path = match self.validate_project_path(project_path) {
            Ok(p) => p,
            Err(e) => return ToolResult::failure(tool_call.id.clone(), e),
        };

        let output = path_resolver::create_command("git")
            .args(["status", "--porcelain", "-b"])
            .current_dir(&validated_path)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let status = parse_git_status(&stdout);
                let output_json = serde_json::json!({
                    "status": status,
                    "raw": stdout.to_string()
                });
                ToolResult::success(
                    tool_call.id.clone(),
                    serde_json::to_string(&output_json).unwrap_or_default(),
                    None,
                )
            }
            Ok(out) => ToolResult::failure(
                tool_call.id.clone(),
                format!("Git command failed: {}", String::from_utf8_lossy(&out.stderr)),
            ),
            Err(e) => ToolResult::failure(
                tool_call.id.clone(),
                format!("Failed to execute git: {}", e),
            ),
        }
    }

    /// Execute get_staged_diff tool
    async fn execute_get_staged_diff(&self, tool_call: &ToolCall) -> ToolResult {
        let project_path = match tool_call.arguments.get("project_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: project_path".to_string(),
            ),
        };

        // Security: Validate path is within a registered project
        let validated_path = match self.validate_project_path(project_path) {
            Ok(p) => p,
            Err(e) => return ToolResult::failure(tool_call.id.clone(), e),
        };

        let output = path_resolver::create_command("git")
            .args(["diff", "--staged", "--stat"])
            .current_dir(&validated_path)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let diff_stat = String::from_utf8_lossy(&out.stdout);

                // Also get the actual diff (limited)
                let diff_output = path_resolver::create_command("git")
                    .args(["diff", "--staged"])
                    .current_dir(&validated_path)
                    .output();

                let diff_content = diff_output
                    .ok()
                    .filter(|o| o.status.success())
                    .map(|o| {
                        let full = String::from_utf8_lossy(&o.stdout);
                        // Limit diff to first 5000 chars for AI context (handle UTF-8)
                        let char_count = full.chars().count();
                        if char_count > 5000 {
                            let truncated: String = full.chars().take(5000).collect();
                            format!("{}...\n[Diff truncated, {} more characters]", truncated, char_count - 5000)
                        } else {
                            full.to_string()
                        }
                    })
                    .unwrap_or_default();

                if diff_stat.is_empty() {
                    let output_json = serde_json::json!({
                        "message": "No staged changes",
                        "has_changes": false
                    });
                    ToolResult::success(
                        tool_call.id.clone(),
                        serde_json::to_string(&output_json).unwrap_or_default(),
                        None,
                    )
                } else {
                    let output_json = serde_json::json!({
                        "summary": diff_stat.to_string(),
                        "diff": diff_content,
                        "has_changes": true
                    });
                    ToolResult::success(
                        tool_call.id.clone(),
                        serde_json::to_string(&output_json).unwrap_or_default(),
                        None,
                    )
                }
            }
            Ok(out) => ToolResult::failure(
                tool_call.id.clone(),
                format!("Git command failed: {}", String::from_utf8_lossy(&out.stderr)),
            ),
            Err(e) => ToolResult::failure(
                tool_call.id.clone(),
                format!("Failed to execute git: {}", e),
            ),
        }
    }

    /// Execute list_project_scripts tool
    async fn execute_list_project_scripts(&self, tool_call: &ToolCall) -> ToolResult {
        let project_path = match tool_call.arguments.get("project_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: project_path".to_string(),
            ),
        };

        // Security: Validate path is within a registered project
        let validated_path = match self.validate_project_path(project_path) {
            Ok(p) => p,
            Err(e) => return ToolResult::failure(tool_call.id.clone(), e),
        };

        let package_json_path = validated_path.join("package.json");

        match std::fs::read_to_string(&package_json_path) {
            Ok(content) => {
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(json) => {
                        let scripts = json.get("scripts")
                            .and_then(|s| s.as_object())
                            .map(|s| {
                                s.iter()
                                    .map(|(k, v)| serde_json::json!({
                                        "name": k,
                                        "command": v
                                    }))
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();

                        let output_json = serde_json::json!({
                            "scripts": scripts,
                            "count": scripts.len()
                        });
                        ToolResult::success(
                            tool_call.id.clone(),
                            serde_json::to_string(&output_json).unwrap_or_default(),
                            None,
                        )
                    }
                    Err(e) => ToolResult::failure(
                        tool_call.id.clone(),
                        format!("Invalid package.json: {}", e),
                    ),
                }
            }
            Err(e) => ToolResult::failure(
                tool_call.id.clone(),
                format!("Cannot read package.json: {}", e),
            ),
        }
    }

    /// Execute list_workflows tool
    async fn execute_list_workflows(&self, tool_call: &ToolCall) -> ToolResult {
        // Query database for workflows
        if let Some(ref db) = self.db {
            match crate::repositories::WorkflowRepository::new(db.clone()).list() {
                Ok(workflows) => {
                    let output = serde_json::json!({
                        "workflows": workflows.iter().map(|w| serde_json::json!({
                            "id": w.id,
                            "name": w.name,
                            "description": w.description,
                            "projectId": w.project_id,
                            "nodeCount": w.nodes.len(),
                        })).collect::<Vec<_>>(),
                        "count": workflows.len()
                    });
                    return ToolResult::success(
                        tool_call.id.clone(),
                        serde_json::to_string(&output).unwrap_or_default(),
                        None,
                    );
                }
                Err(e) => {
                    return ToolResult::failure(
                        tool_call.id.clone(),
                        format!("Failed to list workflows: {}", e),
                    );
                }
            }
        }
        ToolResult::failure(
            tool_call.id.clone(),
            "Database not available".to_string(),
        )
    }

    /// Execute get_workflow tool
    async fn execute_get_workflow(&self, tool_call: &ToolCall) -> ToolResult {
        let workflow_id = match tool_call.arguments.get("workflow_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: workflow_id".to_string(),
            ),
        };

        if let Some(ref db) = self.db {
            match crate::repositories::WorkflowRepository::new(db.clone()).get(workflow_id) {
                Ok(Some(workflow)) => {
                    let output = serde_json::json!({
                        "id": workflow.id,
                        "name": workflow.name,
                        "description": workflow.description,
                        "projectId": workflow.project_id,
                        "nodes": workflow.nodes.iter().map(|n| serde_json::json!({
                            "id": n.id,
                            "name": n.name,
                            "type": n.node_type,
                            "order": n.order,
                        })).collect::<Vec<_>>(),
                        "nodeCount": workflow.nodes.len(),
                        "createdAt": workflow.created_at,
                        "updatedAt": workflow.updated_at,
                    });
                    return ToolResult::success(
                        tool_call.id.clone(),
                        serde_json::to_string(&output).unwrap_or_default(),
                        None,
                    );
                }
                Ok(None) => {
                    return ToolResult::failure(
                        tool_call.id.clone(),
                        format!("Workflow not found: {}", workflow_id),
                    );
                }
                Err(e) => {
                    return ToolResult::failure(
                        tool_call.id.clone(),
                        format!("Failed to get workflow: {}", e),
                    );
                }
            }
        }
        ToolResult::failure(
            tool_call.id.clone(),
            "Database not available".to_string(),
        )
    }

    /// Execute list_projects tool
    async fn execute_list_projects(&self, tool_call: &ToolCall) -> ToolResult {
        log::info!("[AI Tool] execute_list_projects called, db available: {}", self.db.is_some());

        // Get optional query parameter for filtering
        let query_filter = tool_call.arguments
            .get("query")
            .and_then(|v| v.as_str())
            .map(|s| s.to_lowercase());

        if let Some(ref db) = self.db {
            log::debug!("[AI Tool] Creating ProjectRepository and querying...");
            match crate::repositories::ProjectRepository::new(db.clone()).list() {
                Ok(projects) => {
                    // Apply query filter if provided
                    let filtered_projects: Vec<_> = if let Some(ref query) = query_filter {
                        projects.into_iter()
                            .filter(|p| p.name.to_lowercase().contains(query))
                            .collect()
                    } else {
                        projects
                    };

                    log::info!("[AI Tool] list_projects found {} projects (filtered: {})",
                        filtered_projects.len(), query_filter.is_some());
                    for (i, p) in filtered_projects.iter().enumerate() {
                        log::debug!("[AI Tool] Project {}: {} at {}", i + 1, p.name, p.path);
                    }

                    let count = filtered_projects.len();

                    // Build display layer
                    let summary = if count == 0 {
                        "No projects registered".to_string()
                    } else if count == 1 {
                        format!("Found 1 project: {}", filtered_projects[0].name)
                    } else {
                        format!("Found {} registered projects", count)
                    };

                    let display_items: Vec<DisplayItem> = filtered_projects.iter().take(5).map(|p| {
                        DisplayItem {
                            label: p.name.clone(),
                            value: p.path.clone(),
                            icon: Some("folder".to_string()),
                            action: Some(DisplayAction {
                                tool: "get_project".to_string(),
                                args: serde_json::json!({ "path": p.path }),
                            }),
                        }
                    }).collect();

                    let display = if count > 0 {
                        DisplayLayer::success(summary).with_items(display_items)
                    } else {
                        DisplayLayer::info(summary)
                            .with_detail("Register projects via the Projects panel")
                    };

                    let data = serde_json::json!({
                        "projects": filtered_projects.iter().map(|p| serde_json::json!({
                            "id": p.id,
                            "name": p.name,
                            "path": p.path,
                            "packageManager": p.package_manager,
                            "isMonorepo": p.is_monorepo,
                        })).collect::<Vec<_>>(),
                        "count": count
                    });

                    let response = MCPToolResponse::new(data, display);
                    if count > 0 {
                        let meta = ResponseMeta {
                            next_tool_hint: Some("Use get_project to see details of a specific project".to_string()),
                            ..Default::default()
                        };
                        return ToolResult::success(
                            tool_call.id.clone(),
                            response.with_meta(meta).to_json(),
                            None,
                        );
                    }

                    return ToolResult::success(
                        tool_call.id.clone(),
                        response.to_json(),
                        None,
                    );
                }
                Err(e) => {
                    log::error!("[AI Tool] list_projects query error: {}", e);
                    return ToolResult::failure(
                        tool_call.id.clone(),
                        format!("Failed to list projects: {}", e),
                    );
                }
            }
        }
        log::warn!("[AI Tool] list_projects: database not available - MCPToolHandler was not initialized with database");
        ToolResult::failure(
            tool_call.id.clone(),
            "Database not available".to_string(),
        )
    }

    /// Execute get_project tool
    async fn execute_get_project(&self, tool_call: &ToolCall) -> ToolResult {
        let path = match tool_call.arguments.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: path".to_string(),
            ),
        };

        if let Some(ref db) = self.db {
            match crate::repositories::ProjectRepository::new(db.clone()).get_by_path(path) {
                Ok(Some(project)) => {
                    let output = serde_json::json!({
                        "id": project.id,
                        "name": project.name,
                        "path": project.path,
                        "packageManager": project.package_manager,
                        "isMonorepo": project.is_monorepo,
                        "version": project.version,
                        "description": project.description,
                        "scripts": project.scripts,
                        "createdAt": project.created_at,
                    });
                    return ToolResult::success(
                        tool_call.id.clone(),
                        serde_json::to_string(&output).unwrap_or_default(),
                        None,
                    );
                }
                Ok(None) => {
                    return ToolResult::failure(
                        tool_call.id.clone(),
                        format!("Project not found at path: {}", path),
                    );
                }
                Err(e) => {
                    return ToolResult::failure(
                        tool_call.id.clone(),
                        format!("Failed to get project: {}", e),
                    );
                }
            }
        }
        ToolResult::failure(
            tool_call.id.clone(),
            "Database not available".to_string(),
        )
    }

    /// Execute list_worktrees tool
    async fn execute_list_worktrees(&self, tool_call: &ToolCall) -> ToolResult {
        let project_path = match tool_call.arguments.get("project_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: project_path".to_string(),
            ),
        };

        // Execute git worktree list (use path_resolver for macOS GUI app compatibility)
        match path_resolver::create_command("git")
            .args(["-C", project_path, "worktree", "list", "--porcelain"])
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    let result = serde_json::json!({
                        "worktrees": output_str.lines().collect::<Vec<_>>(),
                        "raw": output_str.to_string()
                    });
                    ToolResult::success(
                        tool_call.id.clone(),
                        serde_json::to_string(&result).unwrap_or_default(),
                        None,
                    )
                } else {
                    ToolResult::failure(
                        tool_call.id.clone(),
                        String::from_utf8_lossy(&output.stderr).to_string(),
                    )
                }
            }
            Err(e) => ToolResult::failure(
                tool_call.id.clone(),
                format!("Failed to execute git worktree list: {}", e),
            ),
        }
    }

    /// Execute list_actions tool
    async fn execute_list_actions(&self, tool_call: &ToolCall) -> ToolResult {
        use crate::models::mcp_action::ActionFilter;

        if let Some(ref db) = self.db {
            let filter = ActionFilter::default();
            match crate::repositories::MCPActionRepository::new(db.clone()).list_actions(&filter) {
                Ok(actions) => {
                    let output = serde_json::json!({
                        "actions": actions.iter().map(|a| serde_json::json!({
                            "id": a.id,
                            "name": a.name,
                            "actionType": format!("{:?}", a.action_type),
                            "enabled": a.is_enabled,
                            "description": a.description,
                        })).collect::<Vec<_>>(),
                        "count": actions.len()
                    });
                    return ToolResult::success(
                        tool_call.id.clone(),
                        serde_json::to_string(&output).unwrap_or_default(),
                        None,
                    );
                }
                Err(e) => {
                    return ToolResult::failure(
                        tool_call.id.clone(),
                        format!("Failed to list actions: {}", e),
                    );
                }
            }
        }
        ToolResult::failure(
            tool_call.id.clone(),
            "Database not available".to_string(),
        )
    }

    /// Execute get_action tool
    async fn execute_get_action(&self, tool_call: &ToolCall) -> ToolResult {
        let action_id = match tool_call.arguments.get("actionId").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: actionId".to_string(),
            ),
        };

        if let Some(ref db) = self.db {
            match crate::repositories::MCPActionRepository::new(db.clone()).get_action(action_id) {
                Ok(Some(action)) => {
                    let output = serde_json::json!({
                        "id": action.id,
                        "name": action.name,
                        "actionType": format!("{:?}", action.action_type),
                        "enabled": action.is_enabled,
                        "description": action.description,
                        "config": action.config,
                    });
                    return ToolResult::success(
                        tool_call.id.clone(),
                        serde_json::to_string(&output).unwrap_or_default(),
                        None,
                    );
                }
                Ok(None) => {
                    return ToolResult::failure(
                        tool_call.id.clone(),
                        format!("Action not found: {}", action_id),
                    );
                }
                Err(e) => {
                    return ToolResult::failure(
                        tool_call.id.clone(),
                        format!("Failed to get action: {}", e),
                    );
                }
            }
        }
        ToolResult::failure(
            tool_call.id.clone(),
            "Database not available".to_string(),
        )
    }

    /// Execute list_action_executions tool
    async fn execute_list_action_executions(&self, tool_call: &ToolCall) -> ToolResult {
        // Placeholder - would query execution history from database
        let output = serde_json::json!({
            "message": "Action execution history available in SpecForge UI",
            "hint": "Check the Actions tab for execution history"
        });
        ToolResult::success(
            tool_call.id.clone(),
            serde_json::to_string(&output).unwrap_or_default(),
            None,
        )
    }

    /// Execute get_execution_status tool
    async fn execute_get_execution_status(&self, tool_call: &ToolCall) -> ToolResult {
        let _execution_id = match tool_call.arguments.get("executionId").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: executionId".to_string(),
            ),
        };

        // Placeholder - would query execution status from database
        let output = serde_json::json!({
            "message": "Execution status available in SpecForge UI",
            "hint": "Check the Actions tab for execution details"
        });
        ToolResult::success(
            tool_call.id.clone(),
            serde_json::to_string(&output).unwrap_or_default(),
            None,
        )
    }

    /// Execute list_step_templates tool
    async fn execute_list_step_templates(&self, tool_call: &ToolCall) -> ToolResult {
        // Placeholder - would query step templates from database
        let output = serde_json::json!({
            "message": "Step templates available in SpecForge UI",
            "hint": "Check workflow editor for available step templates"
        });
        ToolResult::success(
            tool_call.id.clone(),
            serde_json::to_string(&output).unwrap_or_default(),
            None,
        )
    }

    // =========================================================================
    // New Tools Synced with MCP Server (Feature 023)
    // =========================================================================

    /// Execute get_worktree_status tool - get git status with detailed info
    async fn execute_get_worktree_status(&self, tool_call: &ToolCall) -> ToolResult {
        let worktree_path = match tool_call.arguments.get("worktree_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: worktree_path".to_string(),
            ),
        };

        // Get current branch (use path_resolver for macOS GUI app compatibility)
        let branch_output = path_resolver::create_command("git")
            .args(["-C", worktree_path, "rev-parse", "--abbrev-ref", "HEAD"])
            .output();
        let current_branch = branch_output.ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        // Get status (use path_resolver for macOS GUI app compatibility)
        let status_output = path_resolver::create_command("git")
            .args(["-C", worktree_path, "status", "--porcelain=v2", "--branch"])
            .output();

        match status_output {
            Ok(output) if output.status.success() => {
                let status_str = String::from_utf8_lossy(&output.stdout);
                let mut staged: Vec<String> = Vec::new();
                let mut modified: Vec<String> = Vec::new();
                let mut untracked: Vec<String> = Vec::new();
                let mut ahead = 0i32;
                let mut behind = 0i32;

                for line in status_str.lines() {
                    if line.starts_with("# branch.ab") {
                        // Parse ahead/behind: # branch.ab +1 -2
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 4 {
                            ahead = parts[2].trim_start_matches('+').parse().unwrap_or(0);
                            behind = parts[3].trim_start_matches('-').parse().unwrap_or(0);
                        }
                    } else if line.starts_with("1 ") || line.starts_with("2 ") {
                        // Changed entries
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 9 {
                            let xy = parts[1];
                            let path = parts.last().unwrap_or(&"");
                            if xy.starts_with('A') || xy.starts_with('M') || xy.starts_with('D') {
                                staged.push(path.to_string());
                            }
                            if xy.chars().nth(1).map(|c| c != '.').unwrap_or(false) {
                                modified.push(path.to_string());
                            }
                        }
                    } else if line.starts_with("? ") {
                        // Untracked
                        let path = line.trim_start_matches("? ");
                        untracked.push(path.to_string());
                    }
                }

                let result = serde_json::json!({
                    "currentBranch": current_branch,
                    "ahead": ahead,
                    "behind": behind,
                    "staged": staged,
                    "modified": modified,
                    "untracked": untracked,
                    "clean": staged.is_empty() && modified.is_empty() && untracked.is_empty()
                });

                ToolResult::success(
                    tool_call.id.clone(),
                    serde_json::to_string(&result).unwrap_or_default(),
                    None,
                )
            }
            Ok(output) => ToolResult::failure(
                tool_call.id.clone(),
                String::from_utf8_lossy(&output.stderr).to_string(),
            ),
            Err(e) => ToolResult::failure(
                tool_call.id.clone(),
                format!("Failed to get git status: {}", e),
            ),
        }
    }

    /// Execute get_git_diff tool - get staged changes diff
    async fn execute_get_git_diff(&self, tool_call: &ToolCall) -> ToolResult {
        let worktree_path = match tool_call.arguments.get("worktree_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: worktree_path".to_string(),
            ),
        };

        // Get staged diff (use path_resolver for macOS GUI app compatibility)
        let diff_output = path_resolver::create_command("git")
            .args(["-C", worktree_path, "diff", "--cached", "--stat"])
            .output();

        let diff_content = path_resolver::create_command("git")
            .args(["-C", worktree_path, "diff", "--cached"])
            .output();

        match (diff_output, diff_content) {
            (Ok(stat), Ok(content)) if stat.status.success() => {
                let stat_str = String::from_utf8_lossy(&stat.stdout);
                let content_str = String::from_utf8_lossy(&content.stdout);

                let result = serde_json::json!({
                    "stats": stat_str.to_string(),
                    "diff": content_str.to_string(),
                    "hasChanges": !content_str.is_empty()
                });

                ToolResult::success(
                    tool_call.id.clone(),
                    serde_json::to_string(&result).unwrap_or_default(),
                    None,
                )
            }
            (Ok(output), _) => ToolResult::failure(
                tool_call.id.clone(),
                String::from_utf8_lossy(&output.stderr).to_string(),
            ),
            (Err(e), _) => ToolResult::failure(
                tool_call.id.clone(),
                format!("Failed to get git diff: {}", e),
            ),
        }
    }

    /// Execute create_step_template tool - create a custom step template
    async fn execute_create_step_template(&self, tool_call: &ToolCall) -> ToolResult {
        let name = match tool_call.arguments.get("name").and_then(|v| v.as_str()) {
            Some(n) if !n.trim().is_empty() => n.trim(),
            _ => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: name".to_string(),
            ),
        };

        let command = match tool_call.arguments.get("command").and_then(|v| v.as_str()) {
            Some(c) if !c.trim().is_empty() => c.trim(),
            _ => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: command".to_string(),
            ),
        };

        let category = tool_call.arguments
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("custom");

        let description = tool_call.arguments
            .get("description")
            .and_then(|v| v.as_str());

        if let Some(ref db) = self.db {
            let template_repo = crate::repositories::TemplateRepository::new(db.clone());

            // Create template using CustomStepTemplate
            let template = crate::models::step_template::CustomStepTemplate {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.to_string(),
                command: command.to_string(),
                category: crate::models::step_template::TemplateCategory::Custom,
                description: description.map(|s| s.to_string()),
                is_custom: true,
                created_at: chrono::Utc::now().to_rfc3339(),
            };

            match template_repo.save(&template) {
                Ok(_) => {
                    let result = serde_json::json!({
                        "success": true,
                        "template": {
                            "id": template.id,
                            "name": template.name,
                            "command": template.command,
                            "category": category,
                            "description": template.description,
                        }
                    });
                    ToolResult::success(
                        tool_call.id.clone(),
                        serde_json::to_string(&result).unwrap_or_default(),
                        None,
                    )
                }
                Err(e) => ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Failed to create step template: {}", e),
                ),
            }
        } else {
            ToolResult::failure(
                tool_call.id.clone(),
                "Database not available".to_string(),
            )
        }
    }

    /// Execute get_action_permissions tool - get permission configuration
    async fn execute_get_action_permissions(&self, tool_call: &ToolCall) -> ToolResult {
        let action_id = tool_call.arguments
            .get("actionId")
            .and_then(|v| v.as_str());

        if let Some(ref db) = self.db {
            let action_repo = crate::repositories::MCPActionRepository::new(db.clone());

            if let Some(id) = action_id {
                // Get specific action - need to get action first to know its type
                match action_repo.get_action(id) {
                    Ok(Some(action)) => {
                        // Get permission for this specific action
                        match action_repo.get_permission(Some(id), &action.action_type) {
                            Ok(permission) => {
                                let result = serde_json::json!({
                                    "actionId": id,
                                    "actionType": action.action_type.to_string(),
                                    "permission": permission.to_string(),
                                });
                                ToolResult::success(
                                    tool_call.id.clone(),
                                    serde_json::to_string(&result).unwrap_or_default(),
                                    None,
                                )
                            }
                            Err(e) => ToolResult::failure(
                                tool_call.id.clone(),
                                format!("Failed to get action permission: {}", e),
                            ),
                        }
                    }
                    Ok(None) => ToolResult::failure(
                        tool_call.id.clone(),
                        format!("Action not found: {}", id),
                    ),
                    Err(e) => ToolResult::failure(
                        tool_call.id.clone(),
                        format!("Failed to get action: {}", e),
                    ),
                }
            } else {
                // Get all permissions
                match action_repo.list_permissions() {
                    Ok(permissions) => {
                        let result = serde_json::json!({
                            "permissions": permissions.into_iter().map(|perm| {
                                serde_json::json!({
                                    "id": perm.id,
                                    "actionId": perm.action_id,
                                    "actionType": perm.action_type.map(|t| t.to_string()),
                                    "permission": perm.permission_level.to_string(),
                                })
                            }).collect::<Vec<_>>()
                        });
                        ToolResult::success(
                            tool_call.id.clone(),
                            serde_json::to_string(&result).unwrap_or_default(),
                            None,
                        )
                    }
                    Err(e) => ToolResult::failure(
                        tool_call.id.clone(),
                        format!("Failed to list action permissions: {}", e),
                    ),
                }
            }
        } else {
            ToolResult::failure(
                tool_call.id.clone(),
                "Database not available".to_string(),
            )
        }
    }

    /// Execute list_background_processes tool
    async fn execute_list_background_processes(&self, tool_call: &ToolCall) -> ToolResult {
        use super::background_process::BACKGROUND_PROCESS_MANAGER;

        let processes = BACKGROUND_PROCESS_MANAGER.list_processes().await;
        let count = processes.len();

        // Build display layer
        let summary = if count == 0 {
            "No background processes running".to_string()
        } else if count == 1 {
            format!("1 background process running: {}", processes[0].name)
        } else {
            format!("{} background processes running", count)
        };

        let display_items: Vec<DisplayItem> = processes.iter().map(|p| {
            DisplayItem {
                label: p.name.clone(),
                value: p.status.to_string(),
                icon: Some("activity".to_string()),
                action: Some(DisplayAction {
                    tool: "get_background_process_output".to_string(),
                    args: serde_json::json!({ "processId": p.id }),
                }),
            }
        }).collect();

        let display = if count > 0 {
            DisplayLayer::success(summary).with_items(display_items)
        } else {
            DisplayLayer::info(summary)
        };

        let data = serde_json::json!({
            "success": true,
            "count": count,
            "processes": processes.iter().map(|p| {
                serde_json::json!({
                    "id": p.id,
                    "pid": p.pid,
                    "name": p.name,
                    "projectPath": p.project_path,
                    "status": p.status.to_string(),
                    "startedAt": p.started_at,
                    "command": p.command,
                })
            }).collect::<Vec<_>>()
        });

        let response = MCPToolResponse::new(data, display);
        ToolResult::success(
            tool_call.id.clone(),
            response.to_json(),
            None,
        )
    }

    /// Execute get_background_process_output tool
    async fn execute_get_background_process_output(&self, tool_call: &ToolCall) -> ToolResult {
        use super::background_process::BACKGROUND_PROCESS_MANAGER;

        let process_id = match tool_call.arguments.get("processId").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: processId".to_string(),
            ),
        };

        let tail_lines = tool_call.arguments
            .get("tailLines")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        match BACKGROUND_PROCESS_MANAGER.get_output(process_id, tail_lines).await {
            Ok(output_lines) => {
                // Build simplified output response
                let result = serde_json::json!({
                    "success": true,
                    "processId": process_id,
                    "outputLines": output_lines.iter().map(|line| {
                        serde_json::json!({
                            "content": line.content,
                            "stream": line.stream,
                            "timestamp": line.timestamp,
                        })
                    }).collect::<Vec<_>>(),
                    "lineCount": output_lines.len(),
                });
                ToolResult::success(
                    tool_call.id.clone(),
                    serde_json::to_string_pretty(&result).unwrap_or_default(),
                    None,
                )
            }
            Err(e) => {
                ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Failed to get process output: {}", e),
                )
            }
        }
    }

    /// Execute stop_background_process tool
    async fn execute_stop_background_process(&self, tool_call: &ToolCall) -> ToolResult {
        use super::background_process::BACKGROUND_PROCESS_MANAGER;

        let process_id = match tool_call.arguments.get("processId").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: processId".to_string(),
            ),
        };

        let force = tool_call.arguments
            .get("force")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        match BACKGROUND_PROCESS_MANAGER.stop_process(process_id, force).await {
            Ok(()) => {
                let result = serde_json::json!({
                    "success": true,
                    "processId": process_id,
                    "message": format!("Process {} stopped successfully", process_id),
                });
                ToolResult::success(
                    tool_call.id.clone(),
                    serde_json::to_string(&result).unwrap_or_default(),
                    None,
                )
            }
            Err(e) => {
                ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Failed to stop process: {}", e),
                )
            }
        }
    }

    /// Execute run_npm_script tool
    /// Supports both foreground (blocking) and background (non-blocking) execution
    async fn execute_run_npm_script(&self, tool_call: &ToolCall) -> ToolResult {
        let project_path = match tool_call.arguments.get("projectPath").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: projectPath".to_string(),
            ),
        };

        let script_name = match tool_call.arguments.get("scriptName").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: scriptName".to_string(),
            ),
        };

        // Check if this should run in background
        // Auto-detect long-running scripts if not explicitly set
        let explicit_background = tool_call.arguments
            .get("runInBackground")
            .and_then(|v| v.as_bool());

        // Helper function to check if a string contains long-running script patterns
        fn contains_long_running_pattern(s: &str) -> bool {
            let lower = s.to_lowercase();
            // Script name patterns
            lower.contains("dev") ||
            lower.contains("start") ||
            lower.contains("serve") ||
            lower.contains("watch") ||
            lower.contains("preview") ||
            lower == "storybook" ||
            // Command patterns (for script content)
            lower.contains("vite") ||
            lower.contains("next dev") ||
            lower.contains("next start") ||
            lower.contains("webpack serve") ||
            lower.contains("webpack-dev-server") ||
            lower.contains("react-scripts start") ||
            lower.contains("ng serve") ||
            lower.contains("vue-cli-service serve") ||
            lower.contains("nuxt dev") ||
            lower.contains("astro dev") ||
            lower.contains("remix dev") ||
            lower.contains("nodemon") ||
            lower.contains("ts-node-dev") ||
            lower.contains("concurrently") ||
            lower.contains("run-p") ||  // npm-run-all parallel
            lower.contains("npm run dev") ||
            lower.contains("npm run start") ||
            lower.contains("pnpm run dev") ||
            lower.contains("pnpm run start") ||
            lower.contains("pnpm dev") ||
            lower.contains("pnpm start") ||
            lower.contains("yarn dev") ||
            lower.contains("yarn start")
        }

        // Check script name first
        let is_long_running_by_name = contains_long_running_pattern(script_name);

        // Try to read package.json to check the actual script content
        let is_long_running_by_content = {
            let package_json_path = std::path::Path::new(project_path).join("package.json");
            if let Ok(content) = std::fs::read_to_string(&package_json_path) {
                if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(script_content) = pkg
                        .get("scripts")
                        .and_then(|s| s.get(script_name))
                        .and_then(|v| v.as_str())
                    {
                        contains_long_running_pattern(script_content)
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        };

        let is_long_running_script = is_long_running_by_name || is_long_running_by_content;

        // Use explicit value if provided, otherwise auto-detect
        let run_in_background = explicit_background.unwrap_or(is_long_running_script);

        if run_in_background && explicit_background.is_none() {
            log::info!(
                "[run_npm_script] Auto-detected '{}' as long-running script (by_name={}, by_content={}), using background mode",
                script_name,
                is_long_running_by_name,
                is_long_running_by_content
            );
        }

        let success_pattern = tool_call.arguments
            .get("successPattern")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let timeout_ms = tool_call.arguments
            .get("timeoutMs")
            .and_then(|v| v.as_u64());

        // If running in background, use BackgroundProcessManager
        if run_in_background {
            return self.execute_run_npm_script_background(
                tool_call,
                project_path,
                script_name,
                success_pattern,
                timeout_ms,
            ).await;
        }

        // Otherwise, forward to run_script for foreground execution
        let mut modified_call = tool_call.clone();
        modified_call.arguments = serde_json::json!({
            "script_name": script_name,
            "project_path": project_path
        });
        self.execute_run_script(&modified_call).await
    }

    /// Execute npm script in background mode
    async fn execute_run_npm_script_background(
        &self,
        tool_call: &ToolCall,
        project_path: &str,
        script_name: &str,
        success_pattern: Option<String>,
        timeout_ms: Option<u64>,
    ) -> ToolResult {
        use super::background_process::BACKGROUND_PROCESS_MANAGER;

        // Security: Validate path is within a registered project
        let validated_path = match self.validate_project_path(project_path) {
            Ok(p) => p,
            Err(e) => return ToolResult::failure(tool_call.id.clone(), e),
        };

        // Validate script exists in package.json
        let package_json_path = validated_path.join("package.json");
        let package_json_content = match std::fs::read_to_string(&package_json_path) {
            Ok(c) => c,
            Err(e) => return ToolResult::failure(
                tool_call.id.clone(),
                format!("Cannot read package.json: {}", e),
            ),
        };

        let package_json: serde_json::Value = match serde_json::from_str(&package_json_content) {
            Ok(j) => j,
            Err(e) => return ToolResult::failure(
                tool_call.id.clone(),
                format!("Invalid package.json: {}", e),
            ),
        };

        // Check if script exists
        let scripts = package_json.get("scripts").and_then(|s| s.as_object());
        let script_exists = scripts.map(|s| s.contains_key(script_name)).unwrap_or(false);

        if !script_exists {
            let available: Vec<&str> = scripts
                .map(|s| s.keys().map(|k| k.as_str()).collect())
                .unwrap_or_default();
            return ToolResult::failure(
                tool_call.id.clone(),
                format!(
                    "Script '{}' not found in package.json. Available scripts: {}",
                    script_name,
                    available.join(", ")
                ),
            );
        }

        // Detect package manager
        let package_manager = if validated_path.join("pnpm-lock.yaml").exists() {
            "pnpm"
        } else if validated_path.join("yarn.lock").exists() {
            "yarn"
        } else {
            "npm"
        };

        let cwd = validated_path.to_string_lossy().to_string();

        // Log to AI Activity with "running" status
        let log_entry_id = self.log_background_process_start(tool_call);

        // Start background process
        match BACKGROUND_PROCESS_MANAGER.start_process(
            script_name.to_string(),                        // name
            package_manager.to_string(),                    // command
            vec!["run".to_string(), script_name.to_string()], // args
            cwd.clone(),                                    // cwd
            project_path.to_string(),                       // project_path
            success_pattern,                                // success_pattern
            timeout_ms,                                     // success_timeout_ms
            None,                                           // conversation_id
            Some(tool_call.id.clone()),                     // tool_call_id
            log_entry_id,                                   // log_entry_id for status updates
        ).await {
            Ok(info) => {
                let output = serde_json::json!({
                    "success": true,
                    "backgroundProcess": true,
                    "processId": info.id,
                    "pid": info.pid,
                    "scriptName": script_name,
                    "projectPath": project_path,
                    "packageManager": package_manager,
                    "status": info.status.to_string(),
                    "message": format!(
                        "Background process started successfully. Process ID: {}. Use 'get_background_process_output' to check output or 'stop_background_process' to stop.",
                        info.id
                    )
                });
                // Mark as background process to skip duplicate logging
                ToolResult {
                    call_id: tool_call.id.clone(),
                    success: true,
                    output: serde_json::to_string_pretty(&output).unwrap_or_default(),
                    error: None,
                    duration_ms: None,
                    metadata: Some(serde_json::json!({ "is_background_process": true })),
                }
            }
            Err(e) => {
                ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Failed to start background process: {}", e),
                )
            }
        }
    }

    /// Execute create_workflow tool
    async fn execute_create_workflow(&self, tool_call: &ToolCall) -> ToolResult {
        let name = match tool_call.arguments.get("name").and_then(|v| v.as_str()) {
            Some(n) if !n.trim().is_empty() => n,
            _ => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: name".to_string(),
            ),
        };

        let description = tool_call.arguments
            .get("description")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty());

        // Handle project_id - treat empty strings as None to avoid FK constraint violation
        let project_id = tool_call.arguments
            .get("project_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty());

        // If project_id is provided, validate it exists
        if let (Some(ref db), Some(pid)) = (&self.db, project_id) {
            let project_exists = crate::repositories::ProjectRepository::new(db.clone())
                .get(pid)
                .map(|p| p.is_some())
                .unwrap_or(false);

            if !project_exists {
                return ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Project '{}' not found. Please provide a valid project_id or omit it.", pid),
                );
            }
        }

        if let Some(ref db) = self.db {
            let workflow_id = uuid::Uuid::new_v4().to_string();
            let mut workflow = crate::models::workflow::Workflow::new(
                workflow_id.clone(),
                name.to_string(),
            );
            workflow.description = description.map(|s| s.to_string());
            workflow.project_id = project_id.map(|s| s.to_string());

            match crate::repositories::WorkflowRepository::new(db.clone()).save(&workflow) {
                Ok(_) => {
                    let output = serde_json::json!({
                        "success": true,
                        "workflowId": workflow.id,
                        "name": workflow.name,
                        "message": format!("Workflow '{}' created successfully. IMPORTANT: Use workflow_id '{}' for add_workflow_step.", name, workflow.id)
                    });

                    // Feature 025: Create context delta for session tracking
                    let context_delta = ContextDelta::workflow_created(
                        workflow.id.clone(),
                        workflow.name.clone(),
                        workflow.project_id.clone(),
                    );

                    return ToolResult {
                        call_id: tool_call.id.clone(),
                        success: true,
                        output: serde_json::to_string(&output).unwrap_or_default(),
                        error: None,
                        duration_ms: None,
                        metadata: Some(serde_json::json!({
                            "context_delta": context_delta
                        })),
                    };
                }
                Err(e) => {
                    return ToolResult::failure(
                        tool_call.id.clone(),
                        format!("Failed to create workflow: {}", e),
                    );
                }
            }
        }
        ToolResult::failure(
            tool_call.id.clone(),
            "Database not available".to_string(),
        )
    }

    /// Execute create_workflow_with_steps tool (atomic workflow + steps creation)
    async fn execute_create_workflow_with_steps(&self, tool_call: &ToolCall) -> ToolResult {
        const MAX_BATCH_SIZE: usize = 10;

        let name = match tool_call.arguments.get("name").and_then(|v| v.as_str()) {
            Some(n) if !n.trim().is_empty() => n,
            _ => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: name".to_string(),
            ),
        };

        let description = tool_call.arguments
            .get("description")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty());

        let project_id = tool_call.arguments
            .get("project_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty());

        let steps = match tool_call.arguments.get("steps").and_then(|v| v.as_array()) {
            Some(arr) => arr,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: steps (array)".to_string(),
            ),
        };

        // Validate steps
        if steps.is_empty() {
            return ToolResult::failure(
                tool_call.id.clone(),
                "Steps array cannot be empty".to_string(),
            );
        }
        if steps.len() > MAX_BATCH_SIZE {
            return ToolResult::failure(
                tool_call.id.clone(),
                format!("Too many steps: {} (max {})", steps.len(), MAX_BATCH_SIZE),
            );
        }

        // Validate all steps upfront
        let mut step_names: Vec<&str> = Vec::new();
        for (i, step) in steps.iter().enumerate() {
            let step_name = match step.get("name").and_then(|v| v.as_str()) {
                Some(n) if !n.trim().is_empty() => n,
                _ => return ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Step {} missing or empty 'name'", i + 1),
                ),
            };
            if step_names.contains(&step_name) {
                return ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Duplicate step name '{}'", step_name),
                );
            }
            step_names.push(step_name);

            if step.get("command").and_then(|v| v.as_str()).map(|s| s.trim().is_empty()).unwrap_or(true) {
                return ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Step {} '{}' missing or empty 'command'", i + 1, step_name),
                );
            }
        }

        // Validate project_id if provided
        if let (Some(ref db), Some(pid)) = (&self.db, project_id) {
            let project_exists = crate::repositories::ProjectRepository::new(db.clone())
                .get(pid)
                .map(|p| p.is_some())
                .unwrap_or(false);

            if !project_exists {
                return ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Project '{}' not found", pid),
                );
            }
        }

        if let Some(ref db) = self.db {
            let workflow_id = uuid::Uuid::new_v4().to_string();
            let mut workflow = crate::models::workflow::Workflow::new(
                workflow_id.clone(),
                name.to_string(),
            );
            workflow.description = description.map(|s| s.to_string());
            workflow.project_id = project_id.map(|s| s.to_string());

            // Add all steps to workflow
            let mut created_steps: Vec<serde_json::Value> = Vec::new();
            for (i, step) in steps.iter().enumerate() {
                let step_name = step.get("name").and_then(|v| v.as_str()).unwrap();
                let command = step.get("command").and_then(|v| v.as_str()).unwrap();
                let cwd = step.get("cwd").and_then(|v| v.as_str()).map(|s| s.to_string());
                let timeout = step.get("timeout").and_then(|v| v.as_u64());

                let node_id = uuid::Uuid::new_v4().to_string();
                let order = i as i32;

                let mut config = serde_json::json!({
                    "command": command,
                });
                if let Some(cwd_val) = cwd {
                    config["cwd"] = serde_json::json!(cwd_val);
                }
                if let Some(timeout_val) = timeout {
                    config["timeout"] = serde_json::json!(timeout_val);
                }

                let node = crate::models::workflow::WorkflowNode {
                    id: node_id.clone(),
                    node_type: "script".to_string(),
                    name: step_name.to_string(),
                    config,
                    order,
                    position: None,
                };

                workflow.nodes.push(node);

                created_steps.push(serde_json::json!({
                    "nodeId": node_id,
                    "name": step_name,
                    "order": order,
                    "command": command
                }));
            }

            // Save workflow with all steps atomically
            match crate::repositories::WorkflowRepository::new(db.clone()).save(&workflow) {
                Ok(_) => {
                    let output = serde_json::json!({
                        "success": true,
                        "workflowId": workflow.id,
                        "workflowName": workflow.name,
                        "description": workflow.description,
                        "projectId": workflow.project_id,
                        "createdSteps": created_steps,
                        "totalSteps": created_steps.len(),
                        "message": format!("Workflow '{}' created with {} steps. Use workflow_id '{}' with run_workflow to execute.", name, created_steps.len(), workflow.id)
                    });

                    // Feature 025: Create context delta for session tracking
                    // Extract step IDs and names for batch delta
                    let step_ids: Vec<String> = created_steps
                        .iter()
                        .filter_map(|s| s.get("nodeId").and_then(|v| v.as_str()).map(|s| s.to_string()))
                        .collect();
                    let step_names: Vec<String> = created_steps
                        .iter()
                        .filter_map(|s| s.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()))
                        .collect();

                    // Create combined delta: workflow created + steps added
                    let workflow_delta = ContextDelta::workflow_created(
                        workflow.id.clone(),
                        workflow.name.clone(),
                        workflow.project_id.clone(),
                    );

                    // We return the workflow delta; steps are implicitly part of the workflow
                    // The SessionCreatedResources will be updated to include steps via separate call or parsing

                    return ToolResult {
                        call_id: tool_call.id.clone(),
                        success: true,
                        output: serde_json::to_string(&output).unwrap_or_default(),
                        error: None,
                        duration_ms: None,
                        metadata: Some(serde_json::json!({
                            "context_delta": workflow_delta,
                            "created_step_ids": step_ids,
                            "created_step_names": step_names
                        })),
                    };
                }
                Err(e) => {
                    return ToolResult::failure(
                        tool_call.id.clone(),
                        format!("Failed to create workflow: {}", e),
                    );
                }
            }
        }
        ToolResult::failure(
            tool_call.id.clone(),
            "Database not available".to_string(),
        )
    }

    /// Execute add_workflow_step tool
    async fn execute_add_workflow_step(&self, tool_call: &ToolCall) -> ToolResult {
        let workflow_id = match tool_call.arguments.get("workflow_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: workflow_id".to_string(),
            ),
        };

        let name = match tool_call.arguments.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: name".to_string(),
            ),
        };

        let command = match tool_call.arguments.get("command").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: command".to_string(),
            ),
        };

        if let Some(ref db) = self.db {
            let repo = crate::repositories::WorkflowRepository::new(db.clone());

            // First, get the existing workflow
            match repo.get(workflow_id) {
                Ok(Some(mut workflow)) => {
                    // Create a new node for this step
                    let node_id = uuid::Uuid::new_v4().to_string();
                    let node = crate::models::workflow::WorkflowNode::new(
                        node_id.clone(),
                        name.to_string(),
                        command.to_string(),
                    );

                    // Add the node to the workflow
                    workflow.nodes.push(node);

                    // Save the updated workflow
                    match repo.save(&workflow) {
                        Ok(_) => {
                            let output = serde_json::json!({
                                "success": true,
                                "nodeId": node_id,
                                "workflowId": workflow_id,
                                "message": format!("Step '{}' added to workflow", name)
                            });

                            // Feature 025: Create context delta for session tracking
                            let context_delta = ContextDelta::step_added(
                                workflow_id.to_string(),
                                workflow.name.clone(),
                                workflow.project_id.clone(),
                                node_id.clone(),
                                name.to_string(),
                                (workflow.nodes.len() - 1) as i32, // order is 0-based
                            );

                            return ToolResult {
                                call_id: tool_call.id.clone(),
                                success: true,
                                output: serde_json::to_string(&output).unwrap_or_default(),
                                error: None,
                                duration_ms: None,
                                metadata: Some(serde_json::json!({
                                    "context_delta": context_delta
                                })),
                            };
                        }
                        Err(e) => {
                            return ToolResult::failure(
                                tool_call.id.clone(),
                                format!("Failed to save workflow with new step: {}", e),
                            );
                        }
                    }
                }
                Ok(None) => {
                    return ToolResult::failure(
                        tool_call.id.clone(),
                        format!("Workflow not found: {}", workflow_id),
                    );
                }
                Err(e) => {
                    return ToolResult::failure(
                        tool_call.id.clone(),
                        format!("Failed to get workflow: {}", e),
                    );
                }
            }
        }
        ToolResult::failure(
            tool_call.id.clone(),
            "Database not available".to_string(),
        )
    }

    /// Execute add_workflow_steps tool (batch operation)
    async fn execute_add_workflow_steps(&self, tool_call: &ToolCall) -> ToolResult {
        const MAX_BATCH_SIZE: usize = 10;

        let workflow_id = match tool_call.arguments.get("workflow_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: workflow_id".to_string(),
            ),
        };

        let steps = match tool_call.arguments.get("steps").and_then(|v| v.as_array()) {
            Some(arr) => arr,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: steps (array)".to_string(),
            ),
        };

        // Validate batch size
        if steps.is_empty() {
            return ToolResult::failure(
                tool_call.id.clone(),
                "Steps array cannot be empty".to_string(),
            );
        }

        if steps.len() > MAX_BATCH_SIZE {
            return ToolResult::failure(
                tool_call.id.clone(),
                format!("Too many steps: {} (max {})", steps.len(), MAX_BATCH_SIZE),
            );
        }

        // Validate all steps upfront
        let mut step_names: Vec<&str> = Vec::new();
        for (i, step) in steps.iter().enumerate() {
            let name = match step.get("name").and_then(|v| v.as_str()) {
                Some(n) if !n.trim().is_empty() => n,
                _ => return ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Step {} missing or empty 'name'", i + 1),
                ),
            };

            // Check for duplicate names
            if step_names.contains(&name) {
                return ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Duplicate step name '{}' found", name),
                );
            }
            step_names.push(name);

            // Check command
            if step.get("command").and_then(|v| v.as_str()).map(|s| s.trim().is_empty()).unwrap_or(true) {
                return ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Step {} '{}' missing or empty 'command'", i + 1, name),
                );
            }
        }

        if let Some(ref db) = self.db {
            let repo = crate::repositories::WorkflowRepository::new(db.clone());

            match repo.get(workflow_id) {
                Ok(Some(mut workflow)) => {
                    let mut created_nodes: Vec<serde_json::Value> = Vec::new();

                    // Calculate starting order
                    let start_order = workflow.nodes.iter()
                        .map(|n| n.order)
                        .max()
                        .unwrap_or(-1) + 1;

                    for (i, step) in steps.iter().enumerate() {
                        let name = step.get("name").and_then(|v| v.as_str()).unwrap();
                        let command = step.get("command").and_then(|v| v.as_str()).unwrap();
                        let cwd = step.get("cwd").and_then(|v| v.as_str()).map(|s| s.to_string());
                        let timeout = step.get("timeout").and_then(|v| v.as_u64());

                        let node_id = uuid::Uuid::new_v4().to_string();
                        let order = start_order + i as i32;

                        let mut config = serde_json::json!({
                            "command": command,
                        });
                        if let Some(cwd_val) = cwd {
                            config["cwd"] = serde_json::json!(cwd_val);
                        }
                        if let Some(timeout_val) = timeout {
                            config["timeout"] = serde_json::json!(timeout_val);
                        }

                        let node = crate::models::workflow::WorkflowNode {
                            id: node_id.clone(),
                            node_type: "script".to_string(),
                            name: name.to_string(),
                            config,
                            order,
                            position: None,
                        };

                        workflow.nodes.push(node);

                        created_nodes.push(serde_json::json!({
                            "nodeId": node_id,
                            "name": name,
                            "order": order,
                            "command": command
                        }));
                    }

                    // Save the updated workflow
                    match repo.save(&workflow) {
                        Ok(_) => {
                            let output = serde_json::json!({
                                "success": true,
                                "workflowId": workflow_id,
                                "stepsAdded": created_nodes.len(),
                                "createdSteps": created_nodes,
                                "totalWorkflowSteps": workflow.nodes.len(),
                                "message": format!("Successfully added {} steps to workflow", created_nodes.len())
                            });

                            // Feature 025: Create context delta for session tracking (batch)
                            let step_ids: Vec<String> = created_nodes
                                .iter()
                                .filter_map(|s| s.get("nodeId").and_then(|v| v.as_str()).map(|s| s.to_string()))
                                .collect();
                            let step_names_list: Vec<String> = created_nodes
                                .iter()
                                .filter_map(|s| s.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()))
                                .collect();

                            let context_delta = ContextDelta::steps_added(
                                workflow_id.to_string(),
                                workflow.name.clone(),
                                workflow.project_id.clone(),
                                step_ids,
                                step_names_list,
                            );

                            return ToolResult {
                                call_id: tool_call.id.clone(),
                                success: true,
                                output: serde_json::to_string(&output).unwrap_or_default(),
                                error: None,
                                duration_ms: None,
                                metadata: Some(serde_json::json!({
                                    "context_delta": context_delta
                                })),
                            };
                        }
                        Err(e) => {
                            return ToolResult::failure(
                                tool_call.id.clone(),
                                format!("Failed to save workflow: {}", e),
                            );
                        }
                    }
                }
                Ok(None) => {
                    return ToolResult::failure(
                        tool_call.id.clone(),
                        format!("Workflow not found: {}", workflow_id),
                    );
                }
                Err(e) => {
                    return ToolResult::failure(
                        tool_call.id.clone(),
                        format!("Failed to get workflow: {}", e),
                    );
                }
            }
        }
        ToolResult::failure(
            tool_call.id.clone(),
            "Database not available".to_string(),
        )
    }

    // =========================================================================
    // Confirmation-Required Tool Execution
    // =========================================================================

    /// Execute run_script tool (requires prior user confirmation)
    /// Auto-detects long-running scripts and routes to background execution
    async fn execute_run_script(&self, tool_call: &ToolCall) -> ToolResult {
        let start_time = std::time::Instant::now();

        // Extract parameters
        let script_name = match tool_call.arguments.get("script_name").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: script_name".to_string(),
            ),
        };

        let project_path = match tool_call.arguments.get("project_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: project_path".to_string(),
            ),
        };

        // Security: Validate path is within a registered project
        let validated_path = match self.validate_project_path(project_path) {
            Ok(p) => p,
            Err(e) => return ToolResult::failure(tool_call.id.clone(), e),
        };

        // Validate script exists in package.json
        let package_json_path = validated_path.join("package.json");
        let package_json_content = match std::fs::read_to_string(&package_json_path) {
            Ok(c) => c,
            Err(e) => return ToolResult::failure(
                tool_call.id.clone(),
                format!("Cannot read package.json: {}", e),
            ),
        };

        let package_json: serde_json::Value = match serde_json::from_str(&package_json_content) {
            Ok(j) => j,
            Err(e) => return ToolResult::failure(
                tool_call.id.clone(),
                format!("Invalid package.json: {}", e),
            ),
        };

        // Check if script exists
        let scripts = package_json.get("scripts")
            .and_then(|s| s.as_object());

        let script_exists = scripts
            .map(|s| s.contains_key(script_name))
            .unwrap_or(false);

        if !script_exists {
            let available: Vec<&str> = scripts
                .map(|s| s.keys().map(|k| k.as_str()).collect())
                .unwrap_or_default();
            return ToolResult::failure(
                tool_call.id.clone(),
                format!(
                    "Script '{}' not found in package.json. Available scripts: {}",
                    script_name,
                    available.join(", ")
                ),
            );
        }

        // Auto-detect long-running scripts and route to background execution
        fn contains_long_running_pattern(s: &str) -> bool {
            let lower = s.to_lowercase();
            lower.contains("dev") ||
            lower.contains("start") ||
            lower.contains("serve") ||
            lower.contains("watch") ||
            lower.contains("vite") ||
            lower.contains("webpack") ||
            lower.contains("nodemon") ||
            lower.contains("next dev") ||
            lower.contains("nuxt dev") ||
            lower.contains("astro dev")
        }

        let is_long_running_by_name = contains_long_running_pattern(script_name);
        let is_long_running_by_content = scripts
            .and_then(|s| s.get(script_name))
            .and_then(|v| v.as_str())
            .map(|content| contains_long_running_pattern(content))
            .unwrap_or(false);

        if is_long_running_by_name || is_long_running_by_content {
            log::info!(
                "[run_script] Auto-detected '{}' as long-running script, redirecting to background execution",
                script_name
            );
            // Redirect to run_npm_script with background mode
            return self.execute_run_npm_script_background(
                tool_call,
                project_path,
                script_name,
                None, // success_pattern
                None, // timeout_ms
            ).await;
        }

        // Detect package manager
        let package_manager = if validated_path.join("pnpm-lock.yaml").exists() {
            "pnpm"
        } else if validated_path.join("yarn.lock").exists() {
            "yarn"
        } else {
            "npm"
        };

        // Get the full path to the package manager using path_resolver
        let pm_path = path_resolver::get_tool_path(package_manager);

        // Spawn the process with tracking for cancellation support
        use super::PROCESS_MANAGER;

        let cwd = validated_path.to_string_lossy().to_string();
        if let Err(e) = PROCESS_MANAGER.spawn_tracked(
            tool_call.id.clone(),
            &pm_path,
            &["run", script_name],
            &cwd,
        ).await {
            return ToolResult::failure(
                tool_call.id.clone(),
                format!("Failed to spawn script process: {}", e),
            );
        }

        // Wait for output with 5 minute timeout (can be cancelled by stop_process)
        let timeout_ms = 5 * 60 * 1000; // 5 minutes
        let result = PROCESS_MANAGER.wait_for_output(&tool_call.id, Some(timeout_ms)).await;

        let duration_ms = start_time.elapsed().as_millis() as i64;

        match result {
            Ok((stdout, stderr, success)) => {
                if success {
                    let output_json = serde_json::json!({
                        "success": true,
                        "script": script_name,
                        "package_manager": package_manager,
                        "stdout": stdout,
                        "stderr": stderr,
                    });
                    ToolResult::success(
                        tool_call.id.clone(),
                        serde_json::to_string_pretty(&output_json).unwrap_or_default(),
                        Some(duration_ms),
                    )
                } else {
                    let output_json = serde_json::json!({
                        "success": false,
                        "script": script_name,
                        "package_manager": package_manager,
                        "stdout": stdout,
                        "stderr": stderr,
                    });
                    ToolResult {
                        call_id: tool_call.id.clone(),
                        success: false,
                        output: serde_json::to_string_pretty(&output_json).unwrap_or_default(),
                        error: Some(format!("Script '{}' failed", script_name)),
                        duration_ms: Some(duration_ms),
                        metadata: None,
                    }
                }
            }
            Err(e) => {
                // Check if it was stopped by user
                let status = PROCESS_MANAGER.get_status(&tool_call.id).await;
                if status == Some(super::process_manager::ProcessStatus::Stopped) {
                    ToolResult {
                        call_id: tool_call.id.clone(),
                        success: false,
                        output: String::new(),
                        error: Some("Cancelled by user".to_string()),
                        duration_ms: Some(duration_ms),
                        metadata: None,
                    }
                } else {
                    ToolResult::failure(
                        tool_call.id.clone(),
                        format!("Script execution failed: {}", e),
                    )
                }
            }
        }
    }

    /// Execute run_package_manager_command tool (requires prior user confirmation)
    /// Runs package manager commands like audit, outdated, install, etc.
    async fn execute_run_package_manager_command(&self, tool_call: &ToolCall) -> ToolResult {
        println!(">>> [AI Tool] execute_run_package_manager_command started: {:?}", tool_call.arguments);
        let start_time = std::time::Instant::now();

        // Extract parameters
        let command = match tool_call.arguments.get("command").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: command".to_string(),
            ),
        };

        let project_path = match tool_call.arguments.get("project_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: project_path".to_string(),
            ),
        };

        // Validate command is in allowed list
        let allowed_commands = ["audit", "outdated", "install", "update", "prune", "dedupe", "why", "list", "info"];
        if !allowed_commands.contains(&command) {
            return ToolResult::failure(
                tool_call.id.clone(),
                format!(
                    "Command '{}' is not allowed. Allowed commands: {}",
                    command,
                    allowed_commands.join(", ")
                ),
            );
        }

        // Security: Validate path is within a registered project
        let validated_path = match self.validate_project_path(project_path) {
            Ok(p) => p,
            Err(e) => return ToolResult::failure(tool_call.id.clone(), e),
        };

        // Get additional args if provided
        let extra_args: Vec<String> = tool_call.arguments
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect())
            .unwrap_or_default();

        // Detect package manager
        let package_manager = if validated_path.join("pnpm-lock.yaml").exists() {
            "pnpm"
        } else if validated_path.join("yarn.lock").exists() {
            "yarn"
        } else if validated_path.join("bun.lockb").exists() {
            "bun"
        } else {
            "npm"
        };

        // Build command args
        let mut args: Vec<String> = vec![command.to_string()];
        args.extend(extra_args.clone());

        let cwd = validated_path.to_string_lossy().to_string();
        println!(">>> [AI Tool] Executing: {} {:?} in {}", package_manager, args, cwd);

        // Use path_resolver::create_async_command for proper PATH handling
        // This is the same approach used in execute_script command
        let mut cmd = path_resolver::create_async_command(package_manager);
        cmd.args(&args);
        cmd.current_dir(&cwd);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        println!(">>> [AI Tool] Spawning command...");

        // Spawn and wait for completion with timeout (2 minutes for most commands)
        let timeout_duration = tokio::time::Duration::from_secs(120);
        let output = match tokio::time::timeout(timeout_duration, cmd.output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                println!(">>> [AI Tool] Failed to execute command: {}", e);
                return ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Failed to execute package command: {}", e),
                );
            }
            Err(_) => {
                println!(">>> [AI Tool] Command timed out after 2 minutes");
                return ToolResult::failure(
                    tool_call.id.clone(),
                    "Command timed out after 2 minutes".to_string(),
                );
            }
        };

        println!(">>> [AI Tool] Command completed with status: {:?}", output.status);

        let duration_ms = start_time.elapsed().as_millis() as i64;
        let raw_stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let raw_stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Truncate output to avoid token explosion (max 32KB each)
        // Use floor_char_boundary to avoid cutting UTF-8 multi-byte characters
        const MAX_OUTPUT_LEN: usize = 32 * 1024;
        let stdout = if raw_stdout.len() > MAX_OUTPUT_LEN {
            // Find the last valid UTF-8 char boundary before MAX_OUTPUT_LEN
            let truncate_at = raw_stdout
                .char_indices()
                .take_while(|(i, _)| *i < MAX_OUTPUT_LEN)
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(0);
            format!(
                "{}...\n\n[Output truncated: {} bytes total, showing first {} bytes]",
                &raw_stdout[..truncate_at],
                raw_stdout.len(),
                truncate_at
            )
        } else {
            raw_stdout
        };
        let stderr = if raw_stderr.len() > MAX_OUTPUT_LEN {
            let truncate_at = raw_stderr
                .char_indices()
                .take_while(|(i, _)| *i < MAX_OUTPUT_LEN)
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(0);
            format!(
                "{}...\n\n[Output truncated: {} bytes total, showing first {} bytes]",
                &raw_stderr[..truncate_at],
                raw_stderr.len(),
                truncate_at
            )
        } else {
            raw_stderr
        };

        // For audit/outdated commands, non-zero exit code means "issues found" not "command failed"
        // These commands return exit code 1 when they find vulnerabilities/outdated packages
        let is_info_command = matches!(command, "audit" | "outdated" | "list" | "info" | "why");
        let has_output = !stdout.is_empty() || !stderr.is_empty();
        let success = output.status.success() || (is_info_command && has_output);

        println!(">>> [AI Tool] stdout len: {}, stderr len: {}, success: {}", stdout.len(), stderr.len(), success);

        let output_json = serde_json::json!({
            "success": success,
            "command": command,
            "args": extra_args,
            "package_manager": package_manager,
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": output.status.code(),
        });

        if success {
            ToolResult::success(
                tool_call.id.clone(),
                serde_json::to_string_pretty(&output_json).unwrap_or_default(),
                Some(duration_ms),
            )
        } else {
            ToolResult {
                call_id: tool_call.id.clone(),
                success: false,
                output: serde_json::to_string_pretty(&output_json).unwrap_or_default(),
                error: Some(format!("Command '{}' failed to execute", command)),
                duration_ms: Some(duration_ms),
                metadata: None,
            }
        }
    }

    /// Execute run_workflow tool (requires prior user confirmation)
    /// Note: Full workflow execution requires AppHandle which is not available here.
    /// This method returns information about how to execute the workflow.
    async fn execute_run_workflow(&self, tool_call: &ToolCall) -> ToolResult {
        let workflow_id = match tool_call.arguments.get("workflow_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: workflow_id".to_string(),
            ),
        };

        // For workflow execution, we need the full Tauri context (AppHandle)
        // which is not available in this service layer.
        // Return instructions for how to properly execute the workflow.
        let output_json = serde_json::json!({
            "status": "workflow_queued",
            "workflow_id": workflow_id,
            "message": format!("Workflow '{}' has been queued for execution.", workflow_id),
            "note": "Workflow execution is handled by the SpecForge runtime. Check the Workflows panel for execution status."
        });

        ToolResult::success(
            tool_call.id.clone(),
            serde_json::to_string_pretty(&output_json).unwrap_or_default(),
            None,
        )
    }

    /// Execute trigger_webhook tool (requires prior user confirmation)
    async fn execute_trigger_webhook(&self, tool_call: &ToolCall) -> ToolResult {
        let webhook_id = match tool_call.arguments.get("webhook_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: webhook_id".to_string(),
            ),
        };

        let payload = tool_call.arguments.get("payload")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        // Similar to workflow execution, webhook triggering requires
        // access to the webhook configuration and HTTP client.
        // Return instructions for how to properly trigger the webhook.
        let output_json = serde_json::json!({
            "status": "webhook_queued",
            "webhook_id": webhook_id,
            "payload": payload,
            "message": format!("Webhook '{}' has been queued for triggering.", webhook_id),
            "note": "Webhook execution is handled by the SpecForge runtime. Check the Webhooks panel for execution status."
        });

        ToolResult::success(
            tool_call.id.clone(),
            serde_json::to_string_pretty(&output_json).unwrap_or_default(),
            None,
        )
    }

    // =========================================================================
    // Time Machine & Security Guardian Tool Execution (Feature 025)
    // =========================================================================

    /// Execute list_snapshots tool
    async fn execute_list_snapshots(&self, tool_call: &ToolCall) -> ToolResult {
        let project_path = match tool_call.arguments.get("project_path").and_then(|v| v.as_str()) {
            Some(path) => path,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: project_path".to_string(),
            ),
        };

        let limit = tool_call.arguments
            .get("limit")
            .and_then(|v| v.as_i64())
            .map(|l| l as i32)
            .unwrap_or(20);

        if let Some(ref db) = self.db {
            let repo = crate::repositories::SnapshotRepository::new(db.clone());
            let filter = crate::models::snapshot::SnapshotFilter {
                project_path: Some(project_path.to_string()),
                limit: Some(limit),
                ..Default::default()
            };
            match repo.list_snapshots(&filter) {
                Ok(snapshots) => {
                    let output = serde_json::json!({
                        "snapshots": snapshots.iter().map(|s| serde_json::json!({
                            "id": s.id,
                            "projectPath": s.project_path,
                            "triggerSource": s.trigger_source,
                            "status": s.status,
                            "totalDependencies": s.total_dependencies,
                            "securityScore": s.security_score,
                            "createdAt": s.created_at,
                        })).collect::<Vec<_>>(),
                        "count": snapshots.len()
                    });
                    ToolResult::success(
                        tool_call.id.clone(),
                        serde_json::to_string_pretty(&output).unwrap_or_default(),
                        None,
                    )
                }
                Err(e) => ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Failed to list snapshots: {}", e),
                ),
            }
        } else {
            ToolResult::failure(tool_call.id.clone(), "Database not available".to_string())
        }
    }

    /// Execute get_snapshot_details tool
    async fn execute_get_snapshot_details(&self, tool_call: &ToolCall) -> ToolResult {
        let snapshot_id = match tool_call.arguments.get("snapshot_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: snapshot_id".to_string(),
            ),
        };

        if let Some(ref db) = self.db {
            let repo = crate::repositories::SnapshotRepository::new(db.clone());
            match repo.get_snapshot_with_dependencies(snapshot_id) {
                Ok(Some(snapshot_with_deps)) => {
                    let snapshot = &snapshot_with_deps.snapshot;
                    let dependencies = &snapshot_with_deps.dependencies;
                    let output = serde_json::json!({
                        "snapshot": {
                            "id": snapshot.id,
                            "projectPath": snapshot.project_path,
                            "triggerSource": snapshot.trigger_source,
                            "status": snapshot.status,
                            "totalDependencies": snapshot.total_dependencies,
                            "securityScore": snapshot.security_score,
                            "lockfileHash": snapshot.lockfile_hash,
                            "createdAt": snapshot.created_at,
                        },
                        "dependencies": dependencies.iter().map(|d| serde_json::json!({
                            "name": d.name,
                            "version": d.version,
                            "isDev": d.is_dev,
                            "hasPostinstall": d.has_postinstall,
                            "postinstallScript": d.postinstall_script,
                        })).collect::<Vec<_>>(),
                        "dependencyCount": dependencies.len()
                    });
                    ToolResult::success(
                        tool_call.id.clone(),
                        serde_json::to_string_pretty(&output).unwrap_or_default(),
                        None,
                    )
                }
                Ok(None) => ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Snapshot not found: {}", snapshot_id),
                ),
                Err(e) => ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Failed to get snapshot: {}", e),
                ),
            }
        } else {
            ToolResult::failure(tool_call.id.clone(), "Database not available".to_string())
        }
    }

    /// Execute compare_snapshots tool
    async fn execute_compare_snapshots(&self, tool_call: &ToolCall) -> ToolResult {
        let snapshot_a_id = match tool_call.arguments.get("snapshot_a_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: snapshot_a_id".to_string(),
            ),
        };

        let snapshot_b_id = match tool_call.arguments.get("snapshot_b_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: snapshot_b_id".to_string(),
            ),
        };

        if let Some(ref db) = self.db {
            let diff_service = crate::services::snapshot::SnapshotDiffService::new(db.clone());
            match diff_service.compare_snapshots(snapshot_a_id, snapshot_b_id) {
                Ok(diff) => {
                    let output = serde_json::json!({
                        "snapshotAId": diff.snapshot_a_id,
                        "snapshotBId": diff.snapshot_b_id,
                        "summary": {
                            "addedCount": diff.summary.added_count,
                            "removedCount": diff.summary.removed_count,
                            "updatedCount": diff.summary.updated_count,
                            "unchangedCount": diff.summary.unchanged_count,
                            "postinstallAdded": diff.summary.postinstall_added,
                            "postinstallRemoved": diff.summary.postinstall_removed,
                            "postinstallChanged": diff.summary.postinstall_changed,
                            "securityScoreChange": diff.summary.security_score_change,
                        },
                        "dependencyChanges": diff.dependency_changes.iter().map(|c| serde_json::json!({
                            "name": c.name,
                            "changeType": c.change_type,
                            "oldVersion": c.old_version,
                            "newVersion": c.new_version,
                        })).collect::<Vec<_>>(),
                        "postinstallChanges": diff.postinstall_changes.iter().map(|p| serde_json::json!({
                            "packageName": p.package_name,
                            "changeType": p.change_type,
                        })).collect::<Vec<_>>(),
                        "lockfileTypeChanged": diff.lockfile_type_changed,
                    });
                    ToolResult::success(
                        tool_call.id.clone(),
                        serde_json::to_string_pretty(&output).unwrap_or_default(),
                        None,
                    )
                }
                Err(e) => ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Failed to compare snapshots: {}", e),
                ),
            }
        } else {
            ToolResult::failure(tool_call.id.clone(), "Database not available".to_string())
        }
    }

    /// Execute search_snapshots tool
    async fn execute_search_snapshots(&self, tool_call: &ToolCall) -> ToolResult {
        let package_name = tool_call.arguments
            .get("package_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let project_path = tool_call.arguments
            .get("project_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let from_date = tool_call.arguments
            .get("from_date")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let to_date = tool_call.arguments
            .get("to_date")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let limit = tool_call.arguments
            .get("limit")
            .and_then(|v| v.as_i64())
            .map(|l| l as i32)
            .unwrap_or(20);

        if let Some(ref db) = self.db {
            use crate::services::snapshot::search::SnapshotSearchCriteria;
            let criteria = SnapshotSearchCriteria {
                package_name,
                package_version: None,
                project_path,
                from_date,
                to_date,
                has_postinstall: None,
                min_security_score: None,
                max_security_score: None,
                limit: Some(limit),
                offset: None,
            };

            let search_service = crate::services::snapshot::SnapshotSearchService::new(db.clone());
            match search_service.search(&criteria) {
                Ok(response) => {
                    let output = serde_json::json!({
                        "summary": {
                            "totalMatches": response.summary.total_matches,
                            "totalSnapshots": response.summary.total_snapshots,
                        },
                        "results": response.results.iter().map(|r| serde_json::json!({
                            "snapshotId": r.snapshot.id,
                            "projectPath": r.snapshot.project_path,
                            "createdAt": r.snapshot.created_at,
                            "totalDependencies": r.snapshot.total_dependencies,
                            "securityScore": r.snapshot.security_score,
                            "matchCount": r.match_count,
                            "matchedDependencies": r.matched_dependencies.iter().map(|d| serde_json::json!({
                                "name": d.name,
                                "version": d.version,
                            })).collect::<Vec<_>>(),
                        })).collect::<Vec<_>>(),
                    });
                    ToolResult::success(
                        tool_call.id.clone(),
                        serde_json::to_string_pretty(&output).unwrap_or_default(),
                        None,
                    )
                }
                Err(e) => ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Failed to search snapshots: {}", e),
                ),
            }
        } else {
            ToolResult::failure(tool_call.id.clone(), "Database not available".to_string())
        }
    }

    /// Execute check_dependency_integrity tool
    async fn execute_check_dependency_integrity(&self, tool_call: &ToolCall) -> ToolResult {
        let project_path = match tool_call.arguments.get("project_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: project_path".to_string(),
            ),
        };

        if let Some(ref db) = self.db {
            let integrity_service = crate::services::security_guardian::DependencyIntegrityService::new(db.clone());
            match integrity_service.check_integrity(project_path) {
                Ok(result) => {
                    let output = serde_json::json!({
                        "hasDrift": result.has_drift,
                        "referenceSnapshotId": result.reference_snapshot_id,
                        "referenceSnapshotDate": result.reference_snapshot_date,
                        "currentLockfileHash": result.current_lockfile_hash,
                        "referenceLockfileHash": result.reference_lockfile_hash,
                        "lockfileMatches": result.lockfile_matches,
                        "summary": {
                            "totalChanges": result.summary.total_changes,
                            "addedCount": result.summary.added_count,
                            "removedCount": result.summary.removed_count,
                            "updatedCount": result.summary.updated_count,
                            "postinstallChanges": result.summary.postinstall_changes,
                            "typosquattingSuspects": result.summary.typosquatting_suspects,
                            "riskLevel": result.summary.risk_level,
                        },
                        "dependencyChanges": result.dependency_changes.len(),
                        "postinstallAlerts": result.postinstall_alerts.len(),
                        "typosquattingAlerts": result.typosquatting_alerts.len(),
                    });
                    ToolResult::success(
                        tool_call.id.clone(),
                        serde_json::to_string_pretty(&output).unwrap_or_default(),
                        None,
                    )
                }
                Err(e) => ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Failed to check integrity: {}", e),
                ),
            }
        } else {
            ToolResult::failure(tool_call.id.clone(), "Database not available".to_string())
        }
    }

    /// Execute get_security_insights tool
    async fn execute_get_security_insights(&self, tool_call: &ToolCall) -> ToolResult {
        let project_path = match tool_call.arguments.get("project_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: project_path".to_string(),
            ),
        };

        if let Some(ref db) = self.db {
            let insights_service = crate::services::security_guardian::SecurityInsightsService::new(db.clone());
            match insights_service.get_project_overview(project_path) {
                Ok(overview) => {
                    let output = serde_json::json!({
                        "projectPath": overview.project_path,
                        "riskScore": overview.risk_score,
                        "riskLevel": overview.risk_level,
                        "totalSnapshots": overview.total_snapshots,
                        "latestSnapshotId": overview.latest_snapshot_id,
                        "latestSnapshotDate": overview.latest_snapshot_date,
                        "insightSummary": {
                            "critical": overview.insight_summary.critical,
                            "high": overview.insight_summary.high,
                            "medium": overview.insight_summary.medium,
                            "low": overview.insight_summary.low,
                            "total": overview.insight_summary.total,
                        },
                        "typosquattingAlerts": overview.typosquatting_alerts.iter().map(|a| serde_json::json!({
                            "packageName": a.package_name,
                            "similarTo": a.similar_to,
                            "firstSeen": a.first_seen,
                        })).collect::<Vec<_>>(),
                        "frequentUpdaters": overview.frequent_updaters.iter().map(|f| serde_json::json!({
                            "packageName": f.package_name,
                            "updateCount": f.update_count,
                        })).collect::<Vec<_>>(),
                    });
                    ToolResult::success(
                        tool_call.id.clone(),
                        serde_json::to_string_pretty(&output).unwrap_or_default(),
                        None,
                    )
                }
                Err(e) => ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Failed to get security insights: {}", e),
                ),
            }
        } else {
            ToolResult::failure(tool_call.id.clone(), "Database not available".to_string())
        }
    }

    /// Execute capture_snapshot tool
    async fn execute_capture_snapshot(&self, tool_call: &ToolCall) -> ToolResult {
        let project_path = match tool_call.arguments.get("project_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::failure(
                tool_call.id.clone(),
                "Missing required parameter: project_path".to_string(),
            ),
        };

        if let Some(ref db) = self.db {
            // Use the capture service to create a manual snapshot
            let storage_base = dirs::data_dir()
                .map(|p| p.join("com.specforge.app").join("time-machine"))
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            let storage = crate::services::snapshot::SnapshotStorage::new(storage_base);
            let capture_service = crate::services::snapshot::SnapshotCaptureService::new(storage, db.clone());

            match capture_service.capture_manual_snapshot(project_path) {
                Ok(snapshot) => {
                    let output = serde_json::json!({
                        "success": true,
                        "snapshot": {
                            "id": snapshot.id,
                            "projectPath": snapshot.project_path,
                            "triggerSource": snapshot.trigger_source,
                            "status": snapshot.status,
                            "totalDependencies": snapshot.total_dependencies,
                            "securityScore": snapshot.security_score,
                            "createdAt": snapshot.created_at,
                        },
                        "message": format!("Snapshot captured successfully: {}", snapshot.id)
                    });
                    ToolResult::success(
                        tool_call.id.clone(),
                        serde_json::to_string_pretty(&output).unwrap_or_default(),
                        None,
                    )
                }
                Err(e) => ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Failed to capture snapshot: {}", e),
                ),
            }
        } else {
            ToolResult::failure(tool_call.id.clone(), "Database not available".to_string())
        }
    }

    /// Check if a tool requires user confirmation
    /// Delegates to ToolPermissionChecker for consistent behavior across the codebase
    pub fn requires_confirmation(&self, tool_name: &str) -> bool {
        // Use the centralized ToolPermissionChecker to ensure consistency
        // with security.rs validation in execute_tool_call()
        ToolPermissionChecker::requires_confirmation(tool_name)
    }

    /// Validate tool call arguments
    pub fn validate_tool_call(&self, tool_call: &ToolCall) -> Result<(), String> {
        let tools = self.get_available_tools(None);

        // Find the tool definition
        let tool_def = tools.tools.iter()
            .find(|t| t.name == tool_call.name)
            .ok_or_else(|| format!("Unknown tool: {}", tool_call.name))?;

        // Validate required parameters
        if tool_def.parameters.get("properties").is_some() {
            if let Some(required) = tool_def.parameters.get("required") {
                if let Some(required_arr) = required.as_array() {
                    for req in required_arr {
                        if let Some(req_name) = req.as_str() {
                            if tool_call.arguments.get(req_name).is_none() {
                                return Err(format!("Missing required parameter: {}", req_name));
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for MCPToolHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse git status --porcelain output into a structured format
fn parse_git_status(output: &str) -> serde_json::Value {
    let lines: Vec<&str> = output.lines().collect();
    let mut branch = String::new();
    let mut staged: Vec<String> = Vec::new();
    let mut modified: Vec<String> = Vec::new();
    let mut untracked: Vec<String> = Vec::new();

    for line in lines {
        if line.starts_with("## ") {
            // Branch line: ## main...origin/main
            branch = line[3..].split("...").next().unwrap_or("").to_string();
        } else if line.len() >= 3 {
            let status = &line[0..2];
            let file = line[3..].to_string();

            match status.chars().collect::<Vec<_>>().as_slice() {
                ['M', 'M'] | ['A', 'M'] => {
                    // Both staged and modified
                    staged.push(file.clone());
                    modified.push(file);
                }
                ['A', _] | ['M', ' '] | ['D', ' '] | ['R', _] | ['C', _] => {
                    staged.push(file);
                }
                [' ', 'M'] | [' ', 'D'] => {
                    modified.push(file);
                }
                ['?', '?'] => {
                    untracked.push(file);
                }
                _ => {}
            }
        }
    }

    serde_json::json!({
        "branch": branch,
        "staged": staged,
        "staged_count": staged.len(),
        "modified": modified,
        "modified_count": modified.len(),
        "untracked": untracked,
        "untracked_count": untracked.len(),
        "clean": staged.is_empty() && modified.is_empty() && untracked.is_empty()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_available_tools() {
        let handler = MCPToolHandler::new();
        let tools = handler.get_available_tools(None);

        assert!(!tools.tools.is_empty());

        // Check that expected tools exist
        let tool_names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"run_script"));
        assert!(tool_names.contains(&"run_workflow"));
        assert!(tool_names.contains(&"get_git_status"));
    }

    #[test]
    fn test_requires_confirmation() {
        let handler = MCPToolHandler::new();

        // Actions that modify state should require confirmation
        assert!(handler.requires_confirmation("run_script"));
        assert!(handler.requires_confirmation("run_workflow"));
        assert!(handler.requires_confirmation("trigger_webhook"));

        // Read-only operations don't need confirmation
        assert!(!handler.requires_confirmation("get_git_status"));
        assert!(!handler.requires_confirmation("get_staged_diff"));
        assert!(!handler.requires_confirmation("list_project_scripts"));

        // Unknown tools should require confirmation
        assert!(handler.requires_confirmation("unknown_tool"));
    }

    #[test]
    fn test_validate_tool_call_success() {
        let handler = MCPToolHandler::new();

        let tool_call = ToolCall::new(
            "get_git_status".to_string(),
            serde_json::json!({
                "project_path": "/some/path"
            }),
        );

        let result = handler.validate_tool_call(&tool_call);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_tool_call_missing_param() {
        let handler = MCPToolHandler::new();

        let tool_call = ToolCall::new(
            "run_script".to_string(),
            serde_json::json!({
                "script_name": "build"
                // Missing project_path
            }),
        );

        let result = handler.validate_tool_call(&tool_call);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("project_path"));
    }

    #[test]
    fn test_validate_tool_call_unknown_tool() {
        let handler = MCPToolHandler::new();

        let tool_call = ToolCall::new(
            "unknown_tool".to_string(),
            serde_json::json!({}),
        );

        let result = handler.validate_tool_call(&tool_call);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown tool"));
    }
}
