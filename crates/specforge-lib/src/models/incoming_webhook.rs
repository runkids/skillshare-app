/**
 * Incoming Webhook Models
 * Data structures for incoming webhook configuration
 * @see specs/012-workflow-webhook-support
 */
use serde::{Deserialize, Serialize};

/// Default port for incoming webhook server
pub const DEFAULT_INCOMING_WEBHOOK_PORT: u16 = 9876;

fn default_port() -> u16 {
    DEFAULT_INCOMING_WEBHOOK_PORT
}

fn default_rate_limit() -> usize {
    60 // 60 requests per minute
}

/// Incoming Webhook configuration (per workflow)
/// Each workflow can have its own port and token
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingWebhookConfig {
    /// Whether incoming webhook is enabled
    pub enabled: bool,
    /// API Token for authentication (UUID v4) - legacy, kept for backward compatibility
    pub token: String,
    /// Token creation timestamp (ISO 8601)
    pub token_created_at: String,
    /// Server listening port (per-workflow, default: 9876)
    #[serde(default = "default_port")]
    pub port: u16,
    /// HMAC secret for signature verification (new security feature)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    /// Whether to require HMAC signature (default: false for backward compatibility)
    #[serde(default)]
    pub require_signature: bool,
    /// Rate limit: max requests per minute (default: 60)
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_minute: usize,
}

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
