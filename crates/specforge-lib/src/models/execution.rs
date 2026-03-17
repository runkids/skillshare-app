// Execution data models
// Represents a script or workflow execution instance
// Feature 013: Extended to support parent-child execution tracking

use serde::{Deserialize, Serialize};

/// Execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Running,
    Completed,
    Failed,
    Paused,
    Cancelled,
}

impl Default for ExecutionStatus {
    fn default() -> Self {
        ExecutionStatus::Running
    }
}

/// Node execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

impl Default for NodeStatus {
    fn default() -> Self {
        NodeStatus::Pending
    }
}

/// Maximum recursion depth for child workflow executions
pub const MAX_EXECUTION_DEPTH: u32 = 5;

/// Represents a workflow execution instance
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Execution {
    pub id: String,
    pub workflow_id: String,
    pub status: ExecutionStatus,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    pub node_results: Vec<NodeResult>,
    /// Parent execution ID (if this is a child execution)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_execution_id: Option<String>,
    /// Parent node ID that triggered this execution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_node_id: Option<String>,
    /// Recursion depth (0 = top level)
    #[serde(default)]
    pub depth: u32,
}

impl Execution {
    pub fn new(id: String, workflow_id: String) -> Self {
        Self {
            id,
            workflow_id,
            status: ExecutionStatus::Running,
            started_at: chrono::Utc::now().to_rfc3339(),
            finished_at: None,
            node_results: Vec::new(),
            parent_execution_id: None,
            parent_node_id: None,
            depth: 0,
        }
    }

    /// Create a new child execution
    pub fn new_child(
        id: String,
        workflow_id: String,
        parent_execution_id: String,
        parent_node_id: String,
        depth: u32,
    ) -> Self {
        Self {
            id,
            workflow_id,
            status: ExecutionStatus::Running,
            started_at: chrono::Utc::now().to_rfc3339(),
            finished_at: None,
            node_results: Vec::new(),
            parent_execution_id: Some(parent_execution_id),
            parent_node_id: Some(parent_node_id),
            depth,
        }
    }

    /// Check if this is a child execution
    pub fn is_child(&self) -> bool {
        self.parent_execution_id.is_some()
    }

    /// Check if max depth is exceeded
    pub fn is_max_depth_exceeded(&self) -> bool {
        self.depth >= MAX_EXECUTION_DEPTH
    }

    pub fn complete(&mut self) {
        self.status = ExecutionStatus::Completed;
        self.finished_at = Some(chrono::Utc::now().to_rfc3339());
    }

    pub fn fail(&mut self) {
        self.status = ExecutionStatus::Failed;
        self.finished_at = Some(chrono::Utc::now().to_rfc3339());
    }

    pub fn pause(&mut self) {
        self.status = ExecutionStatus::Paused;
    }

    pub fn cancel(&mut self) {
        self.status = ExecutionStatus::Cancelled;
        self.finished_at = Some(chrono::Utc::now().to_rfc3339());
    }
}

// ============================================================================
// Child Execution Result (Feature 013)
// ============================================================================

/// Result of a child workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildExecutionResult {
    /// Child execution ID
    pub child_execution_id: String,
    /// Child workflow ID
    pub child_workflow_id: String,
    /// Child workflow name
    pub child_workflow_name: String,
    /// Execution status
    pub status: String,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Result of a node execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeResult {
    pub node_id: String,
    pub status: NodeStatus,
    pub output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    /// Exit code (for script nodes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Child execution result (for trigger-workflow nodes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_execution_result: Option<ChildExecutionResult>,
}

impl NodeResult {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            status: NodeStatus::Pending,
            output: String::new(),
            error_message: None,
            started_at: chrono::Utc::now().to_rfc3339(),
            finished_at: None,
            exit_code: None,
            child_execution_result: None,
        }
    }

    pub fn start(&mut self) {
        self.status = NodeStatus::Running;
        self.started_at = chrono::Utc::now().to_rfc3339();
    }

    pub fn complete(&mut self, exit_code: i32) {
        self.status = if exit_code == 0 {
            NodeStatus::Completed
        } else {
            NodeStatus::Failed
        };
        self.exit_code = Some(exit_code);
        self.finished_at = Some(chrono::Utc::now().to_rfc3339());
    }

    /// Complete with child execution result (for trigger-workflow nodes)
    pub fn complete_with_child(&mut self, result: ChildExecutionResult) {
        self.status = if result.status == "completed" {
            NodeStatus::Completed
        } else {
            NodeStatus::Failed
        };
        if result.status != "completed" {
            self.error_message = result.error_message.clone();
        }
        self.child_execution_result = Some(result);
        self.finished_at = Some(chrono::Utc::now().to_rfc3339());
    }

    pub fn skip(&mut self) {
        self.status = NodeStatus::Skipped;
        self.finished_at = Some(chrono::Utc::now().to_rfc3339());
    }

    pub fn append_output(&mut self, output: &str) {
        self.output.push_str(output);
    }
}
