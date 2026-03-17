//! Background process types and constants
//!
//! Contains types and constants for background process management.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

// ============================================================================
// Constants
// ============================================================================

/// Maximum number of concurrent background processes
pub const MAX_BACKGROUND_PROCESSES: usize = 20;

/// Maximum output buffer size per process (1MB)
pub const MAX_OUTPUT_BUFFER_BYTES: usize = 1024 * 1024;

/// Maximum lines to keep in buffer
pub const MAX_OUTPUT_BUFFER_LINES: usize = 10000;

/// Default success pattern timeout (30 seconds)
pub const DEFAULT_SUCCESS_TIMEOUT_MS: u64 = 30_000;

/// Process cleanup interval (check every 60 seconds)
pub const CLEANUP_INTERVAL_SECS: u64 = 60;

/// Time after completion before process is removed (5 minutes)
pub const COMPLETED_PROCESS_TTL_SECS: u64 = 300;

// ============================================================================
// Types
// ============================================================================

/// Status of a background process
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundProcessStatus {
    /// Process is starting, waiting for success pattern
    Starting,
    /// Process is running (success pattern matched or no pattern specified)
    Running,
    /// Process completed successfully (exit code 0)
    Completed,
    /// Process failed (non-zero exit code)
    Failed,
    /// Process was stopped by user
    Stopped,
    /// Process timed out waiting for success pattern
    TimedOut,
}

impl std::fmt::Display for BackgroundProcessStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Starting => write!(f, "starting"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Stopped => write!(f, "stopped"),
            Self::TimedOut => write!(f, "timed_out"),
        }
    }
}

/// Information about a background process (returned to AI)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundProcessInfo {
    pub id: String,
    pub pid: u32,
    pub script_name: String,
    pub project_path: String,
    pub status: BackgroundProcessStatus,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Whether success pattern was matched
    pub pattern_matched: bool,
    /// Command that was executed
    pub command: String,
}

/// Output from a background process
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessOutput {
    pub process_id: String,
    pub status: BackgroundProcessStatus,
    pub pid: u32,
    pub script_name: String,
    pub started_at: String,
    pub output_lines: Vec<String>,
    pub has_more: bool,
    pub total_lines: usize,
}

/// Circular buffer for output with configurable max size
pub struct CircularBuffer {
    pub lines: VecDeque<String>,
    max_lines: usize,
    pub total_bytes: usize,
    max_bytes: usize,
}

impl CircularBuffer {
    pub fn new(max_lines: usize, max_bytes: usize) -> Self {
        Self {
            lines: VecDeque::new(),
            max_lines,
            total_bytes: 0,
            max_bytes,
        }
    }

    pub fn push(&mut self, line: String) {
        let line_len = line.len();

        // Remove old lines if we exceed limits
        while (self.lines.len() >= self.max_lines || self.total_bytes + line_len > self.max_bytes)
            && !self.lines.is_empty()
        {
            if let Some(old) = self.lines.pop_front() {
                self.total_bytes = self.total_bytes.saturating_sub(old.len());
            }
        }

        self.total_bytes += line_len;
        self.lines.push_back(line);
    }

    pub fn tail(&self, n: usize) -> Vec<String> {
        self.lines.iter().rev().take(n).rev().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }
}

/// Internal state for tracking a background process
pub struct BackgroundProcessState {
    pub info: BackgroundProcessInfo,
    /// Handle to the child process
    pub child: Option<tokio::process::Child>,
    /// Output buffer (combined stdout/stderr)
    pub output_buffer: CircularBuffer,
    /// Task handle for output reading
    pub _output_task: Option<tokio::task::JoinHandle<()>>,
    /// AI Activity log entry ID (for status updates)
    pub log_entry_id: Option<i64>,
    /// Start timestamp for duration calculation
    pub start_timestamp_ms: i64,
}
