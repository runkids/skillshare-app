// Shared Store Module
// Provides utilities for MCP Server and Tauri App
//
// This module provides:
// 1. App identifier and path utilities
// 2. Input validation (path, command, string length, timeout)
// 3. Output sanitization (secret redaction)
// 4. Error sanitization
// 5. Rate limiting

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

// ============================================================================
// App Constants
// ============================================================================

/// App identifier for Tauri (must match tauri.conf.json identifier)
#[cfg(target_os = "macos")]
pub const APP_IDENTIFIER: &str = "com.specforge.app";

#[cfg(not(target_os = "macos"))]
pub const APP_IDENTIFIER: &str = "com.specforge.app";

/// Get the application data directory
pub fn get_app_data_dir() -> Result<PathBuf, String> {
    dirs::data_dir()
        .map(|p| p.join(APP_IDENTIFIER))
        .ok_or_else(|| "Could not determine application data directory".to_string())
}

// ============================================================================
// Error Sanitization
// ============================================================================

/// Sanitize error messages to prevent information leakage
///
/// Removes or obscures:
/// - File system paths
/// - Internal error details that could reveal system structure
pub fn sanitize_error(error: &str) -> String {
    // Replace home directory paths with ~
    let home_dir = dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut sanitized = error.to_string();

    // Replace home directory with ~
    if !home_dir.is_empty() {
        sanitized = sanitized.replace(&home_dir, "~");
    }

    // Replace common path patterns
    sanitized = sanitized
        .replace("/Users/", "~/")
        .replace("/home/", "~/");

    sanitized
}

/// Create a safe error message for external consumers
#[allow(dead_code)]
pub fn safe_error(context: &str, _internal_error: &str) -> String {
    // Only return the context, not the internal error details
    format!("Operation failed: {}", context)
}

// ============================================================================
// Input Validation
// ============================================================================

/// Maximum length constants
pub const MAX_NAME_LENGTH: usize = 256;
pub const MAX_DESCRIPTION_LENGTH: usize = 2048;
pub const MAX_PATH_LENGTH: usize = 4096;
pub const MAX_COMMAND_LENGTH: usize = 8192;
pub const MAX_TIMEOUT_MS: u64 = 60 * 60 * 1000; // 1 hour

/// Validate and sanitize path inputs to prevent path traversal attacks
pub fn validate_path(path: &str) -> Result<PathBuf, String> {
    // Check length
    if path.len() > MAX_PATH_LENGTH {
        return Err("Path exceeds maximum length".to_string());
    }

    let path_buf = PathBuf::from(path);

    // Must be absolute path
    if !path_buf.is_absolute() {
        return Err("Path must be absolute".to_string());
    }

    // Check for path traversal patterns
    let path_str = path_buf.to_string_lossy();
    if path_str.contains("..") {
        return Err("Path contains invalid traversal pattern".to_string());
    }

    // Canonicalize to resolve symlinks and validate existence
    let canonical = path_buf
        .canonicalize()
        .map_err(|_| "Path does not exist or is not accessible".to_string())?;

    // Restrict to user home directory for safety
    if let Some(home) = dirs::home_dir() {
        if !canonical.starts_with(&home) {
            return Err("Path must be within user home directory".to_string());
        }
    }

    Ok(canonical)
}

/// Validate path without requiring it to exist (for new paths)
#[allow(dead_code)]
pub fn validate_path_format(path: &str) -> Result<(), String> {
    // Check length
    if path.len() > MAX_PATH_LENGTH {
        return Err("Path exceeds maximum length".to_string());
    }

    let path_buf = PathBuf::from(path);

    // Must be absolute path
    if !path_buf.is_absolute() {
        return Err("Path must be absolute".to_string());
    }

    // Check for path traversal patterns
    let path_str = path_buf.to_string_lossy();
    if path_str.contains("..") {
        return Err("Path contains invalid traversal pattern".to_string());
    }

    Ok(())
}

/// Dangerous command patterns that should be blocked
const DANGEROUS_PATTERNS: &[&str] = &[
    "rm -rf /",
    "rm -rf ~",
    "> /dev/",
    ">> /dev/",
    "chmod 777 /",
    "mkfs.",
    "dd if=",
    ":(){:|:&};:", // Fork bomb
];

/// Validate command string to prevent command injection
pub fn validate_command(command: &str) -> Result<(), String> {
    // Length check
    if command.len() > MAX_COMMAND_LENGTH {
        return Err(format!(
            "Command exceeds maximum length of {} characters",
            MAX_COMMAND_LENGTH
        ));
    }

    // Empty check
    if command.trim().is_empty() {
        return Err("Command cannot be empty".to_string());
    }

    // Check for dangerous patterns
    let cmd_lower = command.to_lowercase();
    for pattern in DANGEROUS_PATTERNS {
        if cmd_lower.contains(pattern) {
            return Err("Command contains potentially dangerous pattern".to_string());
        }
    }

    Ok(())
}

/// Validate string field length
pub fn validate_string_length(field_name: &str, value: &str, max_len: usize) -> Result<(), String> {
    if value.len() > max_len {
        return Err(format!(
            "{} exceeds maximum length ({} > {})",
            field_name,
            value.len(),
            max_len
        ));
    }
    if value.trim().is_empty() {
        return Err(format!("{} cannot be empty", field_name));
    }
    Ok(())
}

/// Validate timeout value
pub fn validate_timeout(timeout_ms: u64) -> Result<(), String> {
    if timeout_ms > MAX_TIMEOUT_MS {
        return Err(format!(
            "Timeout exceeds maximum of {} ms (1 hour)",
            MAX_TIMEOUT_MS
        ));
    }
    Ok(())
}

// ============================================================================
// Output Sanitization (Secret Redaction)
// ============================================================================

/// Patterns that might indicate secrets in command output
const SECRET_PATTERNS: &[(&str, &str)] = &[
    ("ghp_", "[GITHUB_TOKEN]"),      // GitHub personal access token
    ("gho_", "[GITHUB_OAUTH]"),      // GitHub OAuth token
    ("github_pat_", "[GITHUB_PAT]"), // GitHub PAT
    ("sk-", "[API_KEY]"),            // OpenAI/Anthropic keys
    ("pk_live_", "[STRIPE_KEY]"),    // Stripe live key
    ("sk_live_", "[STRIPE_KEY]"),    // Stripe secret key
    ("AKIA", "[AWS_KEY]"),           // AWS access key
    ("xoxb-", "[SLACK_TOKEN]"),      // Slack bot token
    ("xoxp-", "[SLACK_TOKEN]"),      // Slack user token
];

/// Keys that indicate sensitive data in JSON parameters
const SENSITIVE_KEYS: &[&str] = &[
    "api_key",
    "apikey",
    "secret",
    "password",
    "token",
    "auth",
    "authorization",
    "bearer",
    "credential",
    "private_key",
    "access_token",
    "refresh_token",
];

/// Redaction placeholder for sensitive values
const REDACTED: &str = "***REDACTED***";

/// Sanitize sensitive data in JSON parameters before logging
///
/// Recursively processes JSON values and redacts any fields
/// whose keys match sensitive patterns (case-insensitive).
pub fn sanitize_sensitive(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut sanitized = serde_json::Map::new();
            for (key, val) in map {
                let key_lower = key.to_lowercase();
                let is_sensitive = SENSITIVE_KEYS
                    .iter()
                    .any(|&pattern| key_lower.contains(pattern));

                if is_sensitive {
                    sanitized.insert(key.clone(), serde_json::Value::String(REDACTED.to_string()));
                } else {
                    sanitized.insert(key.clone(), sanitize_sensitive(val));
                }
            }
            serde_json::Value::Object(sanitized)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(sanitize_sensitive).collect())
        }
        // Primitive values pass through unchanged
        _ => value.clone(),
    }
}

/// Sanitize command output to remove potential secrets
pub fn sanitize_output(output: &str) -> String {
    let mut result = output.to_string();

    // Replace known secret patterns
    for (pattern, replacement) in SECRET_PATTERNS {
        if result.contains(pattern) {
            // Find and redact the entire token (until whitespace or end)
            let mut new_result = String::new();
            let mut chars = result.chars().peekable();
            let pattern_chars: Vec<char> = pattern.chars().collect();

            while let Some(c) = chars.next() {
                // Check if we're at the start of a pattern
                let mut matched = true;
                let mut pattern_match = String::from(c);

                for &pc in pattern_chars.iter().skip(1) {
                    if let Some(&next) = chars.peek() {
                        if next == pc {
                            pattern_match.push(chars.next().unwrap());
                        } else {
                            matched = false;
                            break;
                        }
                    } else {
                        matched = false;
                        break;
                    }
                }

                if matched && pattern_match == *pattern {
                    // Skip until whitespace or end
                    while let Some(&next) = chars.peek() {
                        if next.is_whitespace() || next == '"' || next == '\'' {
                            break;
                        }
                        chars.next();
                    }
                    new_result.push_str(replacement);
                } else {
                    new_result.push_str(&pattern_match);
                }
            }
            result = new_result;
        }
    }

    // Also redact common patterns like "password=xxx" or "api_key: xxx"
    let lines: Vec<&str> = result.lines().collect();
    let mut sanitized_lines = Vec::new();

    for line in lines {
        let lower = line.to_lowercase();
        if lower.contains("password") || lower.contains("secret") || lower.contains("api_key") || lower.contains("apikey") || lower.contains("token=") {
            // Check if it looks like a key=value pair
            if line.contains('=') || line.contains(':') {
                sanitized_lines.push("[SENSITIVE_LINE_REDACTED]");
                continue;
            }
        }
        sanitized_lines.push(line);
    }

    sanitized_lines.join("\n")
}

// ============================================================================
// Rate Limiting
// ============================================================================

/// Default rate limit: 60 requests per minute
pub const DEFAULT_RATE_LIMIT: u64 = 60;
pub const DEFAULT_WINDOW_SECS: u64 = 60;

/// Simple rate limiter using sliding window
pub struct RateLimiter {
    max_requests: u64,
    window_secs: u64,
    request_count: AtomicU64,
    window_start: AtomicU64,
}

impl RateLimiter {
    pub fn new(max_requests: u64, window_secs: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            max_requests,
            window_secs,
            request_count: AtomicU64::new(0),
            window_start: AtomicU64::new(now),
        }
    }

    /// Check if request is allowed and increment counter
    pub fn check_and_increment(&self) -> Result<(), String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let window_start = self.window_start.load(Ordering::Relaxed);

        // Reset window if expired
        if now - window_start >= self.window_secs {
            self.window_start.store(now, Ordering::Relaxed);
            self.request_count.store(1, Ordering::Relaxed);
            return Ok(());
        }

        let count = self.request_count.fetch_add(1, Ordering::Relaxed);
        if count >= self.max_requests {
            return Err(format!(
                "Rate limit exceeded: {} requests per {} seconds",
                self.max_requests, self.window_secs
            ));
        }

        Ok(())
    }

    /// Get current request count
    #[allow(dead_code)]
    pub fn current_count(&self) -> u64 {
        self.request_count.load(Ordering::Relaxed)
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(DEFAULT_RATE_LIMIT, DEFAULT_WINDOW_SECS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_path_traversal() {
        assert!(validate_path("/tmp/../etc/passwd").is_err());
    }

    #[test]
    fn test_validate_command_dangerous() {
        assert!(validate_command("rm -rf /").is_err());
        assert!(validate_command("echo hello").is_ok());
    }

    #[test]
    fn test_sanitize_output_github_token() {
        let output = "token: ghp_abc123xyz";
        let sanitized = sanitize_output(output);
        assert!(sanitized.contains("[GITHUB_TOKEN]"));
        assert!(!sanitized.contains("ghp_abc123xyz"));
    }

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(2, 60);
        assert!(limiter.check_and_increment().is_ok());
        assert!(limiter.check_and_increment().is_ok());
        assert!(limiter.check_and_increment().is_err());
    }

    #[test]
    fn test_sanitize_sensitive_object() {
        let input = serde_json::json!({
            "name": "my-script",
            "api_key": "sk-secret-key-12345",
            "password": "super-secret",
            "normal_field": "visible"
        });
        let sanitized = sanitize_sensitive(&input);
        assert_eq!(sanitized["name"], "my-script");
        assert_eq!(sanitized["api_key"], "***REDACTED***");
        assert_eq!(sanitized["password"], "***REDACTED***");
        assert_eq!(sanitized["normal_field"], "visible");
    }

    #[test]
    fn test_sanitize_sensitive_nested() {
        let input = serde_json::json!({
            "config": {
                "auth_token": "bearer-abc123",
                "url": "https://example.com"
            }
        });
        let sanitized = sanitize_sensitive(&input);
        assert_eq!(sanitized["config"]["auth_token"], "***REDACTED***");
        assert_eq!(sanitized["config"]["url"], "https://example.com");
    }

    #[test]
    fn test_sanitize_sensitive_array() {
        let input = serde_json::json!([
            {"name": "item1", "secret": "hidden"},
            {"name": "item2", "value": "visible"}
        ]);
        let sanitized = sanitize_sensitive(&input);
        assert_eq!(sanitized[0]["secret"], "***REDACTED***");
        assert_eq!(sanitized[1]["value"], "visible");
    }
}
