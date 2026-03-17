// AI CLI Tool data models
// Feature: AI CLI Integration (020-ai-cli-integration)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Supported AI CLI tool types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CLIToolType {
    /// Claude Code CLI (claude)
    ClaudeCode,
    /// OpenAI Codex CLI (codex)
    Codex,
    /// Google Gemini CLI (gemini)
    GeminiCli,
}

impl CLIToolType {
    /// Returns the default binary name for this CLI tool
    pub fn binary_name(&self) -> &'static str {
        match self {
            CLIToolType::ClaudeCode => "claude",
            CLIToolType::Codex => "codex",
            CLIToolType::GeminiCli => "gemini",
        }
    }

    /// Returns the environment variable name for API key
    pub fn env_var_name(&self) -> &'static str {
        match self {
            CLIToolType::ClaudeCode => "ANTHROPIC_API_KEY",
            CLIToolType::Codex => "OPENAI_API_KEY",
            CLIToolType::GeminiCli => "GOOGLE_API_KEY",
        }
    }

    /// Returns the display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            CLIToolType::ClaudeCode => "Claude Code",
            CLIToolType::Codex => "Codex",
            CLIToolType::GeminiCli => "Gemini CLI",
        }
    }

    /// Returns all available CLI tool types
    pub fn all() -> Vec<CLIToolType> {
        vec![
            CLIToolType::ClaudeCode,
            CLIToolType::Codex,
            CLIToolType::GeminiCli,
        ]
    }
}

impl std::fmt::Display for CLIToolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CLIToolType::ClaudeCode => write!(f, "claude_code"),
            CLIToolType::Codex => write!(f, "codex"),
            CLIToolType::GeminiCli => write!(f, "gemini_cli"),
        }
    }
}

/// Authentication mode for CLI tools
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CLIAuthMode {
    /// CLI handles authentication internally (subscription users)
    /// e.g., `claude login`, `codex auth`
    #[default]
    CliNative,
    /// Use API key from SpecForge AI Providers
    ApiKey,
}

/// Configuration for a CLI tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CLIToolConfig {
    pub id: String,
    pub tool_type: CLIToolType,
    pub name: String,
    /// Custom binary path, None = auto-detect
    pub binary_path: Option<String>,
    pub is_enabled: bool,
    /// Authentication mode
    pub auth_mode: CLIAuthMode,
    /// Reference to ai_providers for API key mode
    pub api_key_provider_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CLIToolConfig {
    /// Create a new CLI tool config with auto-detect
    pub fn new(tool_type: CLIToolType) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            tool_type,
            name: tool_type.display_name().to_string(),
            binary_path: None,
            is_enabled: true,
            auth_mode: CLIAuthMode::CliNative,
            api_key_provider_id: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Detected CLI tool information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedCLITool {
    pub tool_type: CLIToolType,
    pub binary_path: String,
    pub version: Option<String>,
    pub is_authenticated: bool,
}

/// Request to execute an AI CLI command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AICLIExecuteRequest {
    /// Which CLI tool to use
    pub tool: CLIToolType,
    /// The prompt/instruction to send
    pub prompt: String,
    /// Project path for working directory
    pub project_path: String,
    /// Optional specific model override
    pub model: Option<String>,
    /// Additional context to include
    #[serde(default)]
    pub context: AICLIContext,
    /// CLI-specific options
    #[serde(default)]
    pub options: AICLIOptions,
}

/// Context options for AI CLI execution
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AICLIContext {
    /// Include git staged diff
    #[serde(default)]
    pub include_diff: bool,
    /// Specific files to include in context
    #[serde(default)]
    pub files: Vec<String>,
    /// Custom context text
    pub custom_context: Option<String>,
    /// Include MCP context (SpecForge project info)
    #[serde(default)]
    pub include_mcp_context: bool,
}

/// Options for AI CLI execution
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AICLIOptions {
    /// Max tokens for response
    pub max_tokens: Option<u32>,
    /// Temperature (0.0 - 1.0)
    pub temperature: Option<f32>,
    /// Timeout in seconds (default: 300)
    pub timeout_secs: Option<u64>,
    /// Whether to stream output (default: true)
    #[serde(default = "default_stream_output")]
    pub stream_output: bool,
}

fn default_stream_output() -> bool {
    true
}

/// Output event for streaming CLI output
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AICLIOutputEvent {
    /// Unique execution ID
    pub execution_id: String,
    /// Output type: "stdout", "stderr", "status"
    pub output_type: String,
    /// The content
    pub content: String,
    /// Whether this is the final output
    pub is_final: bool,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl AICLIOutputEvent {
    pub fn stdout(execution_id: &str, content: String) -> Self {
        Self {
            execution_id: execution_id.to_string(),
            output_type: "stdout".to_string(),
            content,
            is_final: false,
            timestamp: Utc::now(),
        }
    }

    pub fn stderr(execution_id: &str, content: String) -> Self {
        Self {
            execution_id: execution_id.to_string(),
            output_type: "stderr".to_string(),
            content,
            is_final: false,
            timestamp: Utc::now(),
        }
    }

    pub fn status(execution_id: &str, content: String, is_final: bool) -> Self {
        Self {
            execution_id: execution_id.to_string(),
            output_type: "status".to_string(),
            content,
            is_final,
            timestamp: Utc::now(),
        }
    }
}

/// Execution result for AI CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AICLIExecuteResult {
    /// Unique execution ID
    pub execution_id: String,
    /// Exit code (None if still running)
    pub exit_code: Option<i32>,
    /// Full stdout output
    pub stdout: String,
    /// Full stderr output
    pub stderr: String,
    /// Execution duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Whether execution was cancelled
    pub cancelled: bool,
}

/// CLI execution log entry (for audit/history)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CLIExecutionLog {
    pub id: String,
    pub tool_type: CLIToolType,
    pub project_path: Option<String>,
    /// SHA256 hash of prompt (not storing actual prompt for privacy)
    pub prompt_hash: String,
    pub model: Option<String>,
    pub execution_time_ms: Option<u64>,
    pub exit_code: Option<i32>,
    pub tokens_used: Option<u32>,
    pub created_at: DateTime<Utc>,
}

impl CLIExecutionLog {
    /// Create a new execution log entry
    pub fn new(tool_type: CLIToolType, project_path: Option<String>, prompt: &str) -> Self {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(prompt.as_bytes());
        let prompt_hash = format!("{:x}", hasher.finalize());

        Self {
            id: Uuid::new_v4().to_string(),
            tool_type,
            project_path,
            prompt_hash,
            model: None,
            execution_time_ms: None,
            exit_code: None,
            tokens_used: None,
            created_at: Utc::now(),
        }
    }
}

/// Error types for AI CLI operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AICLIErrorCode {
    /// CLI tool not found on system
    ToolNotFound,
    /// Failed to spawn process
    SpawnFailed,
    /// API key not configured
    ApiKeyMissing,
    /// Process timed out
    Timeout,
    /// Process exited with non-zero code
    NonZeroExit,
    /// Working directory not found
    WorkingDirNotFound,
    /// Security violation (e.g., forbidden argument)
    SecurityViolation,
    /// Process was cancelled
    Cancelled,
    /// Unknown error
    Unknown,
}

impl std::fmt::Display for AICLIErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AICLIErrorCode::ToolNotFound => write!(f, "CLI_TOOL_NOT_FOUND"),
            AICLIErrorCode::SpawnFailed => write!(f, "CLI_SPAWN_FAILED"),
            AICLIErrorCode::ApiKeyMissing => write!(f, "CLI_API_KEY_MISSING"),
            AICLIErrorCode::Timeout => write!(f, "CLI_TIMEOUT"),
            AICLIErrorCode::NonZeroExit => write!(f, "CLI_NON_ZERO_EXIT"),
            AICLIErrorCode::WorkingDirNotFound => write!(f, "CLI_WORKDIR_NOT_FOUND"),
            AICLIErrorCode::SecurityViolation => write!(f, "CLI_SECURITY_VIOLATION"),
            AICLIErrorCode::Cancelled => write!(f, "CLI_CANCELLED"),
            AICLIErrorCode::Unknown => write!(f, "CLI_UNKNOWN_ERROR"),
        }
    }
}
