// Script Executor for MCP Actions
// Executes predefined scripts with proper environment setup
// @see specs/021-mcp-actions/research.md

use async_trait::async_trait;
use std::process::Stdio;
use std::time::Instant;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

use crate::models::mcp_action::{MCPActionType, ScriptConfig, ScriptExecutionResult};
use crate::utils::path_resolver;

use super::ActionExecutor;

/// Maximum output size before truncation (100KB)
const MAX_OUTPUT_SIZE: usize = 100 * 1024;

/// Default timeout for script execution (60 seconds)
const DEFAULT_TIMEOUT_MS: u64 = 60_000;

/// Script executor for running predefined commands
pub struct ScriptExecutor {
    // No state needed for now
}

impl ScriptExecutor {
    pub fn new() -> Self {
        Self {}
    }

    /// Parse script configuration from JSON parameters
    fn parse_config(&self, params: &serde_json::Value) -> Result<ScriptConfig, String> {
        serde_json::from_value(params.clone())
            .map_err(|e| format!("Invalid script config: {}", e))
    }

    /// Run a command with the specified configuration
    async fn run_command(&self, config: &ScriptConfig, cwd_override: Option<&str>) -> Result<ScriptExecutionResult, String> {
        let start = Instant::now();

        // Build the command using path_resolver for proper macOS GUI app support
        let mut cmd = if config.use_volta {
            // Use volta run for Volta-managed projects
            let volta_path = path_resolver::get_tool_path("volta");
            let mut c = Command::new(&volta_path);
            c.arg("run").arg(&config.command);
            c.args(&config.args);
            c
        } else {
            // Use path_resolver to find the command
            let tool_path = path_resolver::get_tool_path(&config.command);
            let mut c = Command::new(&tool_path);
            c.args(&config.args);
            c
        };

        // Set working directory
        let working_dir = cwd_override.or(config.cwd.as_deref());
        if let Some(cwd) = working_dir {
            cmd.current_dir(cwd);
        }

        // Set environment variables from path_resolver
        let home = path_resolver::get_home_dir();
        if let Some(ref home) = home {
            cmd.env("HOME", home);

            // Volta support
            let volta_home = format!("{}/.volta", home);
            if std::path::Path::new(&volta_home).exists() {
                cmd.env("VOLTA_HOME", &volta_home);
            }
        }

        cmd.env("PATH", path_resolver::get_path());

        if let Some(sock) = path_resolver::get_ssh_auth_sock() {
            cmd.env("SSH_AUTH_SOCK", &sock);
        }

        cmd.env("LANG", "en_US.UTF-8");
        cmd.env("LC_ALL", "en_US.UTF-8");
        cmd.env("TERM", "xterm-256color");
        cmd.env("FORCE_COLOR", "1");

        // Apply custom environment variables from config
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // Configure stdout/stderr capture
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Get timeout duration
        let timeout_ms = if config.timeout_ms > 0 {
            config.timeout_ms
        } else {
            DEFAULT_TIMEOUT_MS
        };

        // Spawn and wait with timeout
        let child = cmd.spawn()
            .map_err(|e| format!("Failed to spawn command '{}': {}", config.command, e))?;

        let result = timeout(Duration::from_millis(timeout_ms), child.wait_with_output()).await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(output)) => {
                let mut stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let mut stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let mut truncated = false;

                // Truncate output if too large
                if stdout.len() > MAX_OUTPUT_SIZE {
                    stdout.truncate(MAX_OUTPUT_SIZE);
                    stdout.push_str("\n... (output truncated)");
                    truncated = true;
                }
                if stderr.len() > MAX_OUTPUT_SIZE {
                    stderr.truncate(MAX_OUTPUT_SIZE);
                    stderr.push_str("\n... (output truncated)");
                    truncated = true;
                }

                Ok(ScriptExecutionResult {
                    exit_code: output.status.code().unwrap_or(-1),
                    stdout,
                    stderr,
                    truncated,
                    duration_ms,
                })
            }
            Ok(Err(e)) => {
                Err(format!("Command execution failed: {}", e))
            }
            Err(_) => {
                // Timeout occurred
                Err(format!(
                    "Command timed out after {}ms",
                    timeout_ms
                ))
            }
        }
    }
}

impl Default for ScriptExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ActionExecutor for ScriptExecutor {
    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        // Extract config and optional overrides
        let config: ScriptConfig = if let Some(config_val) = params.get("config") {
            serde_json::from_value(config_val.clone())
                .map_err(|e| format!("Invalid script config: {}", e))?
        } else {
            self.parse_config(&params)?
        };

        // Extract cwd override if provided
        let cwd_override = params.get("cwd")
            .and_then(|v| v.as_str());

        let result = self.run_command(&config, cwd_override).await?;

        serde_json::to_value(result)
            .map_err(|e| format!("Failed to serialize result: {}", e))
    }

    fn action_type(&self) -> MCPActionType {
        MCPActionType::Script
    }

    fn description(&self, params: &serde_json::Value) -> String {
        if let Some(config) = params.get("config") {
            if let Ok(script_config) = serde_json::from_value::<ScriptConfig>(config.clone()) {
                return format!(
                    "Execute script: {} {}",
                    script_config.command,
                    script_config.args.join(" ")
                );
            }
        }

        if let Some(cmd) = params.get("command").and_then(|v| v.as_str()) {
            return format!("Execute script: {}", cmd);
        }

        "Execute script".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_script_executor_echo() {
        let executor = ScriptExecutor::new();

        let params = serde_json::json!({
            "config": {
                "command": "echo",
                "args": ["hello", "world"],
                "timeoutMs": 5000
            }
        });

        let result = executor.execute(params).await;
        assert!(result.is_ok(), "Echo command should succeed: {:?}", result);

        let result_val = result.unwrap();
        assert_eq!(result_val["exitCode"], 0);
        assert!(result_val["stdout"].as_str().unwrap().contains("hello world"));
    }

    #[tokio::test]
    async fn test_script_executor_timeout() {
        let executor = ScriptExecutor::new();

        let params = serde_json::json!({
            "config": {
                "command": "sleep",
                "args": ["10"],
                "timeoutMs": 100  // Very short timeout
            }
        });

        let result = executor.execute(params).await;
        assert!(result.is_err(), "Sleep should timeout");
        assert!(result.unwrap_err().contains("timed out"));
    }

    #[test]
    fn test_action_type() {
        let executor = ScriptExecutor::new();
        assert_eq!(executor.action_type(), MCPActionType::Script);
    }

    #[test]
    fn test_description() {
        let executor = ScriptExecutor::new();

        let params = serde_json::json!({
            "config": {
                "command": "npm",
                "args": ["run", "build"]
            }
        });

        let desc = executor.description(&params);
        assert!(desc.contains("npm"));
        assert!(desc.contains("run build"));
    }
}
