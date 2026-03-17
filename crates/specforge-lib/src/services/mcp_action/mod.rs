// MCP Action Service
// Provides action execution services for MCP protocol
// @see specs/021-mcp-actions/research.md

pub mod script;
pub mod webhook;
pub mod workflow;

use async_trait::async_trait;
use crate::models::mcp_action::MCPActionType;

/// Trait for action executors
#[async_trait]
pub trait ActionExecutor: Send + Sync {
    /// Execute the action with given parameters
    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value, String>;

    /// Get the action type this executor handles
    fn action_type(&self) -> MCPActionType;

    /// Get a description for logging/display
    fn description(&self, params: &serde_json::Value) -> String;
}

/// Factory for creating action executors
pub fn create_executor(action_type: MCPActionType) -> Box<dyn ActionExecutor> {
    match action_type {
        MCPActionType::Script => Box::new(script::ScriptExecutor::new()),
        MCPActionType::Webhook => Box::new(webhook::WebhookExecutor::new()),
        MCPActionType::Workflow => Box::new(workflow::WorkflowExecutor::new()),
    }
}

// Re-export executors for convenience
pub use script::ScriptExecutor;
pub use webhook::WebhookExecutor;
pub use workflow::WorkflowExecutor;
