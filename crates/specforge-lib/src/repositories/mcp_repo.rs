// MCP Repository
// Handles all database operations for MCP server configuration and logs

use chrono::{DateTime, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::models::mcp::{DevServerMode, MCPEncryptedSecrets, MCPPermissionMode, MCPServerConfig};
use crate::utils::database::Database;

/// MCP request log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpLogEntry {
    pub id: Option<i64>,
    pub timestamp: DateTime<Utc>,
    pub tool: String,
    pub arguments: serde_json::Value,
    pub result: String,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub source: Option<String>,
}

/// Repository for MCP configuration data access
pub struct MCPRepository {
    db: Database,
}

impl MCPRepository {
    /// Create a new MCPRepository
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Get MCP server configuration
    pub fn get_config(&self) -> Result<MCPServerConfig, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT is_enabled, permission_mode, dev_server_mode, allowed_tools, log_requests, encrypted_secrets
                FROM mcp_config
                WHERE id = 1
                "#,
                [],
                |row| {
                    Ok(MCPConfigRow {
                        is_enabled: row.get(0)?,
                        permission_mode: row.get(1)?,
                        dev_server_mode: row.get(2)?,
                        allowed_tools: row.get(3)?,
                        log_requests: row.get(4)?,
                        encrypted_secrets: row.get(5)?,
                    })
                },
            );

            match result {
                Ok(row) => row.into_config(),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(MCPServerConfig::default()),
                Err(e) => Err(format!("Failed to get MCP config: {}", e)),
            }
        })
    }

    /// Save MCP server configuration
    pub fn save_config(&self, config: &MCPServerConfig) -> Result<(), String> {
        let allowed_tools_json = serde_json::to_string(&config.allowed_tools)
            .map_err(|e| format!("Failed to serialize allowed_tools: {}", e))?;

        let encrypted_secrets_json = serde_json::to_string(&config.encrypted_secrets)
            .map_err(|e| format!("Failed to serialize encrypted_secrets: {}", e))?;

        let permission_mode_str = match config.permission_mode {
            MCPPermissionMode::ReadOnly => "read_only",
            MCPPermissionMode::ExecuteWithConfirm => "execute_with_confirm",
            MCPPermissionMode::FullAccess => "full_access",
        };

        let dev_server_mode_str = match config.dev_server_mode {
            DevServerMode::McpManaged => "mcp_managed",
            DevServerMode::UiIntegrated => "ui_integrated",
            DevServerMode::RejectWithHint => "reject_with_hint",
        };

        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT OR REPLACE INTO mcp_config
                (id, is_enabled, permission_mode, dev_server_mode, allowed_tools, log_requests, encrypted_secrets)
                VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![
                    config.is_enabled as i32,
                    permission_mode_str,
                    dev_server_mode_str,
                    allowed_tools_json,
                    config.log_requests as i32,
                    encrypted_secrets_json,
                ],
            )
            .map_err(|e| format!("Failed to save MCP config: {}", e))?;

            Ok(())
        })
    }

    /// Update MCP enabled state
    pub fn set_enabled(&self, enabled: bool) -> Result<(), String> {
        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE mcp_config SET is_enabled = ?1 WHERE id = 1",
                params![enabled as i32],
            )
            .map_err(|e| format!("Failed to update MCP enabled state: {}", e))?;

            Ok(())
        })
    }

    /// Update MCP permission mode
    pub fn set_permission_mode(&self, mode: MCPPermissionMode) -> Result<(), String> {
        let mode_str = match mode {
            MCPPermissionMode::ReadOnly => "read_only",
            MCPPermissionMode::ExecuteWithConfirm => "execute_with_confirm",
            MCPPermissionMode::FullAccess => "full_access",
        };

        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE mcp_config SET permission_mode = ?1 WHERE id = 1",
                params![mode_str],
            )
            .map_err(|e| format!("Failed to update MCP permission mode: {}", e))?;

            Ok(())
        })
    }

    /// Update log requests setting
    pub fn set_log_requests(&self, enabled: bool) -> Result<(), String> {
        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE mcp_config SET log_requests = ?1 WHERE id = 1",
                params![enabled as i32],
            )
            .map_err(|e| format!("Failed to update log_requests: {}", e))?;

            Ok(())
        })
    }

    // ============================================================================
    // MCP Logs
    // ============================================================================

    /// Insert a new MCP log entry
    pub fn insert_log(&self, entry: &McpLogEntry) -> Result<i64, String> {
        let arguments_json = serde_json::to_string(&entry.arguments)
            .map_err(|e| format!("Failed to serialize arguments: {}", e))?;

        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO mcp_logs (timestamp, tool, arguments, result, duration_ms, error, source)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                params![
                    entry.timestamp.to_rfc3339(),
                    entry.tool,
                    arguments_json,
                    entry.result,
                    entry.duration_ms as i64,
                    entry.error,
                    entry.source.as_deref().unwrap_or("mcp_server"),
                ],
            )
            .map_err(|e| format!("Failed to insert MCP log: {}", e))?;

            Ok(conn.last_insert_rowid())
        })
    }

    /// Update an existing MCP log entry's status (for background processes)
    pub fn update_log_status(
        &self,
        log_id: i64,
        result: &str,
        duration_ms: u64,
        error: Option<&str>,
    ) -> Result<(), String> {
        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                UPDATE mcp_logs
                SET result = ?1, duration_ms = ?2, error = ?3
                WHERE id = ?4
                "#,
                params![result, duration_ms as i64, error, log_id],
            )
            .map_err(|e| format!("Failed to update MCP log: {}", e))?;

            Ok(())
        })
    }

    /// Get MCP logs with optional limit
    pub fn get_logs(&self, limit: Option<usize>) -> Result<Vec<McpLogEntry>, String> {
        let limit = limit.unwrap_or(100) as i64;

        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, timestamp, tool, arguments, result, duration_ms, error, source
                    FROM mcp_logs
                    ORDER BY timestamp DESC
                    LIMIT ?1
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map(params![limit], |row| {
                    Ok(McpLogRow {
                        id: row.get(0)?,
                        timestamp: row.get(1)?,
                        tool: row.get(2)?,
                        arguments: row.get(3)?,
                        result: row.get(4)?,
                        duration_ms: row.get(5)?,
                        error: row.get(6)?,
                        source: row.get(7)?,
                    })
                })
                .map_err(|e| format!("Failed to query MCP logs: {}", e))?;

            let mut entries = Vec::new();
            for row in rows {
                let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                entries.push(row.into_entry()?);
            }

            Ok(entries)
        })
    }

    /// Get total count of MCP logs
    pub fn get_log_count(&self) -> Result<usize, String> {
        self.db.with_connection(|conn| {
            conn.query_row("SELECT COUNT(*) FROM mcp_logs", [], |row| row.get(0))
                .map_err(|e| format!("Failed to count MCP logs: {}", e))
        })
    }

    /// Clear all MCP logs
    pub fn clear_logs(&self) -> Result<(), String> {
        self.db.with_connection(|conn| {
            conn.execute("DELETE FROM mcp_logs", [])
                .map_err(|e| format!("Failed to clear MCP logs: {}", e))?;
            Ok(())
        })
    }

    /// Delete old logs (keep only the most recent N entries)
    pub fn prune_logs(&self, keep_count: usize) -> Result<usize, String> {
        self.db.with_connection(|conn| {
            let deleted = conn
                .execute(
                    r#"
                    DELETE FROM mcp_logs
                    WHERE id NOT IN (
                        SELECT id FROM mcp_logs
                        ORDER BY timestamp DESC
                        LIMIT ?1
                    )
                    "#,
                    params![keep_count as i64],
                )
                .map_err(|e| format!("Failed to prune MCP logs: {}", e))?;

            Ok(deleted)
        })
    }
}

/// Internal row structure for mapping database rows
struct MCPConfigRow {
    is_enabled: i32,
    permission_mode: String,
    dev_server_mode: Option<String>,
    allowed_tools: String,
    log_requests: i32,
    encrypted_secrets: Option<String>,
}

impl MCPConfigRow {
    fn into_config(self) -> Result<MCPServerConfig, String> {
        let permission_mode = match self.permission_mode.as_str() {
            "read_only" => MCPPermissionMode::ReadOnly,
            "execute_with_confirm" => MCPPermissionMode::ExecuteWithConfirm,
            "full_access" => MCPPermissionMode::FullAccess,
            _ => MCPPermissionMode::ReadOnly,
        };

        let dev_server_mode = match self.dev_server_mode.as_deref() {
            Some("ui_integrated") => DevServerMode::UiIntegrated,
            Some("reject_with_hint") => DevServerMode::RejectWithHint,
            _ => DevServerMode::McpManaged,
        };

        let allowed_tools: Vec<String> = serde_json::from_str(&self.allowed_tools)
            .unwrap_or_default();

        let encrypted_secrets: MCPEncryptedSecrets = self
            .encrypted_secrets
            .as_ref()
            .map(|json| serde_json::from_str(json).ok())
            .flatten()
            .unwrap_or_default();

        Ok(MCPServerConfig {
            is_enabled: self.is_enabled != 0,
            permission_mode,
            dev_server_mode,
            allowed_tools,
            log_requests: self.log_requests != 0,
            encrypted_secrets,
        })
    }
}

/// Internal row structure for MCP log entries
struct McpLogRow {
    id: i64,
    timestamp: String,
    tool: String,
    arguments: String,
    result: String,
    duration_ms: i64,
    error: Option<String>,
    source: Option<String>,
}

impl McpLogRow {
    fn into_entry(self) -> Result<McpLogEntry, String> {
        let timestamp = DateTime::parse_from_rfc3339(&self.timestamp)
            .map_err(|e| format!("Failed to parse timestamp: {}", e))?
            .with_timezone(&Utc);

        let arguments: serde_json::Value = serde_json::from_str(&self.arguments)
            .unwrap_or(serde_json::json!({}));

        Ok(McpLogEntry {
            id: Some(self.id),
            timestamp,
            tool: self.tool,
            arguments,
            result: self.result,
            duration_ms: self.duration_ms as u64,
            error: self.error,
            source: self.source,
        })
    }
}
