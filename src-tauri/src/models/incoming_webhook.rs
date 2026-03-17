/**
 * Incoming Webhook Models
 * Data structures for incoming webhook configuration
 * @see specs/012-workflow-webhook-support
 */
use serde::{Deserialize, Serialize};

/// Default port for incoming webhook server
pub const DEFAULT_INCOMING_WEBHOOK_PORT: u16 = 9876;

// Note: IncomingWebhookConfig is re-exported from specforge-lib
// This local definition is kept for reference but specforge-lib's version takes precedence

/// Information about a running webhook server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningServerInfo {
    /// Workflow ID this server serves
    pub workflow_id: String,
    /// Workflow name for display
    pub workflow_name: String,
    /// Port the server is listening on
    pub port: u16,
    /// Whether the server is running
    pub running: bool,
}

/// Incoming webhook server status (multi-server)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingWebhookServerStatus {
    /// List of all running servers
    pub running_servers: Vec<RunningServerInfo>,
    /// Total number of running servers
    pub running_count: u32,
}

/// Webhook trigger response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookTriggerResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_id: Option<String>,
    pub message: String,
}
