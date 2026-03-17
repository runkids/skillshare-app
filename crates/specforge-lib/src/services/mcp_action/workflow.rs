// Workflow Executor for MCP Actions
// Executes SpecForge workflows with progress tracking
// @see specs/021-mcp-actions/research.md

use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Instant;

use crate::models::mcp_action::{
    MCPActionType, WorkflowActionConfig, WorkflowExecutionResult,
};

use super::ActionExecutor;

/// Workflow executor that delegates to existing workflow execution system
pub struct WorkflowExecutor {
    // Will hold reference to workflow repository/service in future
}

impl WorkflowExecutor {
    pub fn new() -> Self {
        Self {}
    }

    /// Parse workflow configuration from JSON parameters
    fn parse_config(&self, params: &serde_json::Value) -> Result<WorkflowActionConfig, String> {
        serde_json::from_value(params.clone())
            .map_err(|e| format!("Invalid workflow config: {}", e))
    }

    /// Execute a workflow by ID with parameter overrides
    async fn execute_workflow(
        &self,
        _config: &WorkflowActionConfig,
        _parameter_overrides: Option<&HashMap<String, serde_json::Value>>,
    ) -> Result<WorkflowExecutionResult, String> {
        let start = Instant::now();

        // TODO: Integrate with actual workflow execution system
        // For now, return a placeholder that indicates the workflow ID
        // The actual implementation will:
        // 1. Load workflow definition from WorkflowRepository
        // 2. Merge parameters with overrides
        // 3. Execute each step using existing step execution logic
        // 4. Track progress and report results

        let execution_id = uuid::Uuid::new_v4().to_string();

        // Placeholder implementation
        // In the actual implementation, this will delegate to the workflow service
        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(WorkflowExecutionResult {
            execution_id,
            status: "pending".to_string(),
            steps_completed: 0,
            steps_total: 0,
            step_results: vec![],
            duration_ms,
        })
    }
}

impl Default for WorkflowExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ActionExecutor for WorkflowExecutor {
    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        // Extract config
        let config: WorkflowActionConfig = if let Some(config_val) = params.get("config") {
            serde_json::from_value(config_val.clone())
                .map_err(|e| format!("Invalid workflow config: {}", e))?
        } else {
            self.parse_config(&params)?
        };

        // Extract optional parameter overrides
        let parameters: Option<HashMap<String, serde_json::Value>> = params
            .get("parameters")
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        let result = self.execute_workflow(&config, parameters.as_ref()).await?;

        serde_json::to_value(result)
            .map_err(|e| format!("Failed to serialize result: {}", e))
    }

    fn action_type(&self) -> MCPActionType {
        MCPActionType::Workflow
    }

    fn description(&self, params: &serde_json::Value) -> String {
        if let Some(config) = params.get("config") {
            if let Ok(workflow_config) =
                serde_json::from_value::<WorkflowActionConfig>(config.clone())
            {
                return format!("Execute workflow: {}", workflow_config.workflow_id);
            }
        }

        if let Some(workflow_id) = params.get("workflowId").and_then(|v| v.as_str()) {
            return format!("Execute workflow: {}", workflow_id);
        }

        "Execute workflow".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_executor_placeholder() {
        let executor = WorkflowExecutor::new();

        let params = serde_json::json!({
            "config": {
                "workflowId": "test-workflow-123"
            }
        });

        let result = executor.execute(params).await;
        assert!(result.is_ok());

        let result_val = result.unwrap();
        assert!(result_val.get("executionId").is_some());
        assert_eq!(result_val["status"], "pending");
    }

    #[test]
    fn test_action_type() {
        let executor = WorkflowExecutor::new();
        assert_eq!(executor.action_type(), MCPActionType::Workflow);
    }

    #[test]
    fn test_description() {
        let executor = WorkflowExecutor::new();

        let params = serde_json::json!({
            "config": {
                "workflowId": "deploy-to-production"
            }
        });

        let desc = executor.description(&params);
        assert!(desc.contains("deploy-to-production"));
    }
}
