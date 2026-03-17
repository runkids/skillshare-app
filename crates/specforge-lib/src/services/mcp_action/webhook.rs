// Webhook Executor for MCP Actions
// Triggers configured webhooks with parameter substitution
// @see specs/021-mcp-actions/research.md

use async_trait::async_trait;
use reqwest::{Client, Method};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::models::mcp_action::{MCPActionType, MCPWebhookConfig, WebhookExecutionResult};

use super::ActionExecutor;

/// Maximum response body size before truncation (10KB)
const MAX_RESPONSE_SIZE: usize = 10 * 1024;

/// Default timeout for webhook requests (30 seconds)
const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// Base delay for exponential backoff (milliseconds)
const RETRY_BASE_DELAY_MS: u64 = 1000;

/// Webhook executor for HTTP requests
pub struct WebhookExecutor {
    client: Client,
}

impl WebhookExecutor {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Parse webhook configuration from JSON parameters
    fn parse_config(&self, params: &serde_json::Value) -> Result<MCPWebhookConfig, String> {
        serde_json::from_value(params.clone())
            .map_err(|e| format!("Invalid webhook config: {}", e))
    }

    /// Substitute variables in a string template
    /// Variables are in the format {{variable_name}}
    fn substitute_variables(&self, template: &str, variables: &HashMap<String, String>) -> String {
        let mut result = template.to_string();

        for (key, value) in variables {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }

        result
    }

    /// Send the HTTP request with retry logic
    async fn send_request(
        &self,
        config: &MCPWebhookConfig,
        variables: Option<&HashMap<String, String>>,
        payload_override: Option<&serde_json::Value>,
    ) -> Result<WebhookExecutionResult, String> {
        let start = Instant::now();

        // Determine URL with variable substitution
        let url = if let Some(vars) = variables {
            self.substitute_variables(&config.url, vars)
        } else {
            config.url.clone()
        };

        // Parse HTTP method
        let method = Method::from_bytes(config.method.as_bytes())
            .map_err(|_| format!("Invalid HTTP method: {}", config.method))?;

        // Prepare request body
        let body: Option<String> = if let Some(payload) = payload_override {
            Some(serde_json::to_string(payload)
                .map_err(|e| format!("Failed to serialize payload: {}", e))?)
        } else if let Some(template) = &config.payload_template {
            let body_str = if let Some(vars) = variables {
                self.substitute_variables(template, vars)
            } else {
                template.clone()
            };
            Some(body_str)
        } else {
            None
        };

        // Get timeout and retry settings
        let timeout_ms = if config.timeout_ms > 0 {
            config.timeout_ms
        } else {
            DEFAULT_TIMEOUT_MS
        };

        let max_retries = config.retry_count;
        let mut retry_attempts: u8 = 0;
        let mut last_error: Option<String> = None;

        // Retry loop
        while retry_attempts <= max_retries {
            if retry_attempts > 0 {
                // Exponential backoff: 1s, 2s, 4s...
                let delay = RETRY_BASE_DELAY_MS * (1 << (retry_attempts - 1));
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }

            // Build request
            let mut request = self.client
                .request(method.clone(), &url)
                .timeout(Duration::from_millis(timeout_ms));

            // Add headers
            for (key, value) in &config.headers {
                let header_value = if let Some(vars) = variables {
                    self.substitute_variables(value, vars)
                } else {
                    value.clone()
                };
                request = request.header(key.as_str(), header_value);
            }

            // Add Content-Type if not set and we have a body
            if body.is_some() && !config.headers.contains_key("Content-Type") {
                request = request.header("Content-Type", "application/json");
            }

            // Add body
            if let Some(ref body_str) = body {
                request = request.body(body_str.clone());
            }

            // Send request
            match request.send().await {
                Ok(response) => {
                    let status_code = response.status().as_u16();

                    // Extract response headers
                    let response_headers: HashMap<String, String> = response
                        .headers()
                        .iter()
                        .map(|(k, v)| {
                            (k.as_str().to_string(), v.to_str().unwrap_or("").to_string())
                        })
                        .collect();

                    // Get response body
                    let body_bytes = response.bytes().await
                        .map_err(|e| format!("Failed to read response body: {}", e))?;

                    let mut response_body = String::from_utf8_lossy(&body_bytes).to_string();

                    // Truncate if too large
                    if response_body.len() > MAX_RESPONSE_SIZE {
                        response_body.truncate(MAX_RESPONSE_SIZE);
                        response_body.push_str("\n... (response truncated)");
                    }

                    let duration_ms = start.elapsed().as_millis() as u64;

                    // Check if status indicates success (2xx) or client error (4xx)
                    // Only retry on 5xx errors
                    if status_code >= 500 && retry_attempts < max_retries {
                        last_error = Some(format!("Server error: {}", status_code));
                        retry_attempts += 1;
                        continue;
                    }

                    return Ok(WebhookExecutionResult {
                        status_code,
                        response_body: Some(response_body),
                        response_headers,
                        duration_ms,
                        retry_attempts,
                    });
                }
                Err(e) => {
                    last_error = Some(format!("Request failed: {}", e));

                    if retry_attempts < max_retries {
                        retry_attempts += 1;
                        continue;
                    }
                }
            }
        }

        // All retries exhausted
        Err(last_error.unwrap_or_else(|| "Unknown error".to_string()))
    }
}

impl Default for WebhookExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ActionExecutor for WebhookExecutor {
    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        // Extract config
        let config: MCPWebhookConfig = if let Some(config_val) = params.get("config") {
            serde_json::from_value(config_val.clone())
                .map_err(|e| format!("Invalid webhook config: {}", e))?
        } else {
            self.parse_config(&params)?
        };

        // Extract optional variables for substitution
        let variables: Option<HashMap<String, String>> = params.get("variables")
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        // Extract optional payload override
        let payload = params.get("payload");

        let result = self.send_request(&config, variables.as_ref(), payload).await?;

        serde_json::to_value(result)
            .map_err(|e| format!("Failed to serialize result: {}", e))
    }

    fn action_type(&self) -> MCPActionType {
        MCPActionType::Webhook
    }

    fn description(&self, params: &serde_json::Value) -> String {
        if let Some(config) = params.get("config") {
            if let Ok(webhook_config) = serde_json::from_value::<MCPWebhookConfig>(config.clone()) {
                return format!(
                    "Trigger webhook: {} {}",
                    webhook_config.method,
                    webhook_config.url
                );
            }
        }

        if let Some(url) = params.get("url").and_then(|v| v.as_str()) {
            let method = params.get("method")
                .and_then(|v| v.as_str())
                .unwrap_or("POST");
            return format!("Trigger webhook: {} {}", method, url);
        }

        "Trigger webhook".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_substitution() {
        let executor = WebhookExecutor::new();

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "world".to_string());
        vars.insert("version".to_string(), "1.0".to_string());

        let template = "Hello {{name}}, version {{version}}!";
        let result = executor.substitute_variables(template, &vars);

        assert_eq!(result, "Hello world, version 1.0!");
    }

    #[test]
    fn test_action_type() {
        let executor = WebhookExecutor::new();
        assert_eq!(executor.action_type(), MCPActionType::Webhook);
    }

    #[test]
    fn test_description() {
        let executor = WebhookExecutor::new();

        let params = serde_json::json!({
            "config": {
                "url": "https://api.example.com/webhook",
                "method": "POST"
            }
        });

        let desc = executor.description(&params);
        assert!(desc.contains("POST"));
        assert!(desc.contains("api.example.com"));
    }
}
