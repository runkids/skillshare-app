// Webhook data models for workflow webhook support
// @see specs/012-workflow-webhook-support/data-model.md

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Webhook configuration for a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookConfig {
    /// Whether the webhook is enabled
    pub enabled: bool,
    /// Webhook URL (must be HTTPS)
    pub url: String,
    /// Trigger condition
    pub trigger: WebhookTrigger,
    /// Custom HTTP headers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    /// Custom payload template (JSON format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_template: Option<String>,
}

/// Webhook trigger condition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum WebhookTrigger {
    /// Trigger only on successful completion
    OnSuccess,
    /// Trigger only on failure
    OnFailure,
    /// Always trigger regardless of status
    Always,
}

/// Result of testing a webhook
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookTestResult {
    /// Whether the test was successful
    pub success: bool,
    /// HTTP response status code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    /// Response body (truncated to 1000 chars)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_body: Option<String>,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Response time in milliseconds
    pub response_time: u64,
}

/// Webhook delivery event payload for frontend notification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookDeliveryPayload {
    /// Execution ID
    pub execution_id: String,
    /// Workflow ID
    pub workflow_id: String,
    /// Timestamp when delivery was attempted
    pub attempted_at: String,
    /// Whether delivery was successful
    pub success: bool,
    /// HTTP status code if received
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Response time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_time: Option<u64>,
}

/// Default payload template for webhook
pub const DEFAULT_PAYLOAD_TEMPLATE: &str = r#"{
  "workflow": {
    "id": "{{workflow_id}}",
    "name": "{{workflow_name}}"
  },
  "execution": {
    "id": "{{execution_id}}",
    "status": "{{status}}",
    "duration_ms": {{duration}},
    "timestamp": "{{timestamp}}"
  },
  "error": "{{error_message}}"
}"#;

/// Supported template variables
pub const SUPPORTED_VARIABLES: &[&str] = &[
    "workflow_id",
    "workflow_name",
    "execution_id",
    "status",
    "duration",
    "timestamp",
    "error_message",
];
