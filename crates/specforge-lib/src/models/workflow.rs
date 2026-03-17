// Workflow data models
// Represents an automation workflow
// Feature 013: Extended to support multiple node types (trigger-workflow)

use crate::models::incoming_webhook::IncomingWebhookConfig;
use crate::models::webhook::WebhookConfig;
use serde::{Deserialize, Serialize};

/// Represents an automation workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workflow {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default)]
    pub nodes: Vec<WorkflowNode>,
    #[serde(default = "default_workflow_timestamp")]
    pub created_at: String,
    #[serde(default = "default_workflow_timestamp")]
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_executed_at: Option<String>,
    /// Outgoing webhook configuration for notifications
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webhook: Option<WebhookConfig>,
    /// Incoming webhook configuration for external triggers
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incoming_webhook: Option<IncomingWebhookConfig>,
}

fn default_workflow_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

impl Workflow {
    pub fn new(id: String, name: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            name,
            description: None,
            project_id: None,
            nodes: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
            last_executed_at: None,
            webhook: None,
            incoming_webhook: None,
        }
    }
}

// ============================================================================
// Node Types (Feature 013: Workflow Trigger Workflow)
// ============================================================================

/// Node type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum NodeType {
    Script,
    TriggerWorkflow,
}

impl Default for NodeType {
    fn default() -> Self {
        NodeType::Script
    }
}

/// Child failure handling strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OnChildFailure {
    Fail,
    Continue,
}

impl Default for OnChildFailure {
    fn default() -> Self {
        OnChildFailure::Fail
    }
}

/// Configuration for a script node
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptNodeConfig {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

impl Default for ScriptNodeConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            cwd: None,
            timeout: None,
        }
    }
}

/// Configuration for a trigger-workflow node
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerWorkflowConfig {
    /// Target workflow ID to trigger
    pub target_workflow_id: String,
    /// Whether to wait for child workflow completion
    #[serde(default = "default_true")]
    pub wait_for_completion: bool,
    /// How to handle child workflow failure
    #[serde(default)]
    pub on_child_failure: OnChildFailure,
}

fn default_true() -> bool {
    true
}

impl Default for TriggerWorkflowConfig {
    fn default() -> Self {
        Self {
            target_workflow_id: String::new(),
            wait_for_completion: true,
            on_child_failure: OnChildFailure::Fail,
        }
    }
}

/// Node configuration (Tagged Union)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum NodeConfig {
    #[serde(rename = "script")]
    Script(ScriptNodeConfig),
    #[serde(rename = "trigger-workflow")]
    TriggerWorkflow(TriggerWorkflowConfig),
}

impl Default for NodeConfig {
    fn default() -> Self {
        NodeConfig::Script(ScriptNodeConfig::default())
    }
}

/// Represents a node in a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowNode {
    pub id: String,
    #[serde(rename = "type", default = "default_node_type")]
    pub node_type: String,
    pub name: String,
    #[serde(default = "default_node_config")]
    pub config: serde_json::Value,
    #[serde(default)]
    pub order: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<NodePosition>,
}

fn default_node_type() -> String {
    "script".to_string()
}

fn default_node_config() -> serde_json::Value {
    serde_json::json!({})
}

impl WorkflowNode {
    pub fn new(id: String, name: String, command: String) -> Self {
        Self {
            id,
            node_type: String::from("script"),
            name,
            config: serde_json::json!({
                "command": command,
            }),
            order: 0,
            position: None,
        }
    }

    /// Create a new trigger-workflow node
    pub fn new_trigger(id: String, name: String, target_workflow_id: String) -> Self {
        Self {
            id,
            node_type: String::from("trigger-workflow"),
            name,
            config: serde_json::json!({
                "targetWorkflowId": target_workflow_id,
                "waitForCompletion": true,
                "onChildFailure": "fail",
            }),
            order: 0,
            position: None,
        }
    }

    /// Check if this is a script node
    pub fn is_script(&self) -> bool {
        self.node_type == "script"
    }

    /// Check if this is a trigger-workflow node
    pub fn is_trigger_workflow(&self) -> bool {
        self.node_type == "trigger-workflow"
    }

    /// Get script config (if this is a script node)
    pub fn get_script_config(&self) -> Option<ScriptNodeConfig> {
        if self.is_script() {
            serde_json::from_value(self.config.clone()).ok()
        } else {
            None
        }
    }

    /// Get trigger-workflow config (if this is a trigger-workflow node)
    pub fn get_trigger_workflow_config(&self) -> Option<TriggerWorkflowConfig> {
        if self.is_trigger_workflow() {
            serde_json::from_value(self.config.clone()).ok()
        } else {
            None
        }
    }
}

/// Position of a node on the canvas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

// ============================================================================
// Cycle Detection Types (Feature 013)
// ============================================================================

/// Cycle detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CycleDetectionResult {
    /// Whether a cycle exists
    pub has_cycle: bool,
    /// The cycle path (workflow IDs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycle_path: Option<Vec<String>>,
    /// The cycle path (workflow names for UI display)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycle_names: Option<Vec<String>>,
}

/// Available workflow for selection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableWorkflow {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub step_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_executed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycle_warning: Option<CycleDetectionResult>,
}
