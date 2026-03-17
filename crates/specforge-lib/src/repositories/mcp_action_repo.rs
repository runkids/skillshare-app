// MCP Action Repository
// Handles all database operations for MCP actions, permissions, and execution history
// @see specs/021-mcp-actions/data-model.md

use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

use crate::models::mcp_action::{
    ActionFilter, ExecutionFilter, ExecutionStatus, MCPAction, MCPActionExecution,
    MCPActionPermission, MCPActionType, PermissionLevel,
};
use crate::utils::database::Database;

/// Repository for MCP action data access
pub struct MCPActionRepository {
    db: Database,
}

impl MCPActionRepository {
    /// Create a new MCPActionRepository
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    // ============================================================================
    // Actions CRUD
    // ============================================================================

    /// List all actions with optional filtering
    pub fn list_actions(&self, filter: &ActionFilter) -> Result<Vec<MCPAction>, String> {
        self.db.with_connection(|conn| {
            let mut sql = String::from(
                r#"
                SELECT id, action_type, name, description, config, project_id, is_enabled, created_at, updated_at
                FROM mcp_actions
                WHERE 1=1
                "#,
            );

            let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(project_id) = &filter.project_id {
                sql.push_str(" AND (project_id = ? OR project_id IS NULL)");
                params_vec.push(Box::new(project_id.clone()));
            }

            if let Some(action_type) = &filter.action_type {
                sql.push_str(" AND action_type = ?");
                params_vec.push(Box::new(action_type.to_string()));
            }

            if let Some(is_enabled) = filter.is_enabled {
                sql.push_str(" AND is_enabled = ?");
                params_vec.push(Box::new(is_enabled as i32));
            }

            sql.push_str(" ORDER BY updated_at DESC");

            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let params_refs: Vec<&dyn rusqlite::ToSql> =
                params_vec.iter().map(|p| p.as_ref()).collect();

            let rows = stmt
                .query_map(params_refs.as_slice(), |row| {
                    Ok(ActionRow {
                        id: row.get(0)?,
                        action_type: row.get(1)?,
                        name: row.get(2)?,
                        description: row.get(3)?,
                        config: row.get(4)?,
                        project_id: row.get(5)?,
                        is_enabled: row.get(6)?,
                        created_at: row.get(7)?,
                        updated_at: row.get(8)?,
                    })
                })
                .map_err(|e| format!("Failed to query actions: {}", e))?;

            let mut actions = Vec::new();
            for row in rows {
                let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                actions.push(row.into_action()?);
            }

            Ok(actions)
        })
    }

    /// Get a single action by ID
    pub fn get_action(&self, id: &str) -> Result<Option<MCPAction>, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT id, action_type, name, description, config, project_id, is_enabled, created_at, updated_at
                FROM mcp_actions
                WHERE id = ?1
                "#,
                params![id],
                |row| {
                    Ok(ActionRow {
                        id: row.get(0)?,
                        action_type: row.get(1)?,
                        name: row.get(2)?,
                        description: row.get(3)?,
                        config: row.get(4)?,
                        project_id: row.get(5)?,
                        is_enabled: row.get(6)?,
                        created_at: row.get(7)?,
                        updated_at: row.get(8)?,
                    })
                },
            );

            match result {
                Ok(row) => Ok(Some(row.into_action()?)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get action: {}", e)),
            }
        })
    }

    /// Save an action (insert or update using UPSERT pattern)
    pub fn save_action(&self, action: &MCPAction) -> Result<(), String> {
        let config_json = serde_json::to_string(&action.config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO mcp_actions (id, action_type, name, description, config, project_id, is_enabled, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT(id) DO UPDATE SET
                    action_type = excluded.action_type,
                    name = excluded.name,
                    description = excluded.description,
                    config = excluded.config,
                    project_id = excluded.project_id,
                    is_enabled = excluded.is_enabled,
                    updated_at = excluded.updated_at
                "#,
                params![
                    action.id,
                    action.action_type.to_string(),
                    action.name,
                    action.description,
                    config_json,
                    action.project_id,
                    action.is_enabled as i32,
                    action.created_at,
                    action.updated_at,
                ],
            )
            .map_err(|e| format!("Failed to save action: {}", e))?;

            Ok(())
        })
    }

    /// Delete an action by ID
    pub fn delete_action(&self, id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let deleted = conn
                .execute("DELETE FROM mcp_actions WHERE id = ?1", params![id])
                .map_err(|e| format!("Failed to delete action: {}", e))?;

            Ok(deleted > 0)
        })
    }

    // ============================================================================
    // Permissions CRUD
    // ============================================================================

    /// Get permission level for an action (with fallback logic)
    /// Priority: action_id match > action_type match > global default
    pub fn get_permission(&self, action_id: Option<&str>, action_type: &MCPActionType) -> Result<PermissionLevel, String> {
        self.db.with_connection(|conn| {
            // 1. Try exact action match
            if let Some(id) = action_id {
                let result = conn.query_row(
                    "SELECT permission_level FROM mcp_action_permissions WHERE action_id = ?1",
                    params![id],
                    |row| row.get::<_, String>(0),
                );

                if let Ok(level) = result {
                    return level.parse::<PermissionLevel>();
                }
            }

            // 2. Try action type match
            let result = conn.query_row(
                "SELECT permission_level FROM mcp_action_permissions WHERE action_id IS NULL AND action_type = ?1",
                params![action_type.to_string()],
                |row| row.get::<_, String>(0),
            );

            if let Ok(level) = result {
                return level.parse::<PermissionLevel>();
            }

            // 3. Try global default (both NULL)
            let result = conn.query_row(
                "SELECT permission_level FROM mcp_action_permissions WHERE action_id IS NULL AND action_type IS NULL",
                [],
                |row| row.get::<_, String>(0),
            );

            if let Ok(level) = result {
                return level.parse::<PermissionLevel>();
            }

            // 4. Return default
            Ok(PermissionLevel::RequireConfirm)
        })
    }

    /// Save a permission rule
    pub fn save_permission(&self, permission: &MCPActionPermission) -> Result<(), String> {
        let action_type_str = permission.action_type.as_ref().map(|t| t.to_string());

        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO mcp_action_permissions (id, action_id, action_type, permission_level, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(id) DO UPDATE SET
                    action_id = excluded.action_id,
                    action_type = excluded.action_type,
                    permission_level = excluded.permission_level
                "#,
                params![
                    permission.id,
                    permission.action_id,
                    action_type_str,
                    permission.permission_level.to_string(),
                    permission.created_at,
                ],
            )
            .map_err(|e| format!("Failed to save permission: {}", e))?;

            Ok(())
        })
    }

    /// List all permissions
    pub fn list_permissions(&self) -> Result<Vec<MCPActionPermission>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, action_id, action_type, permission_level, created_at
                    FROM mcp_action_permissions
                    ORDER BY
                        CASE WHEN action_id IS NOT NULL THEN 0 ELSE 1 END,
                        CASE WHEN action_type IS NOT NULL THEN 0 ELSE 1 END,
                        created_at DESC
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(PermissionRow {
                        id: row.get(0)?,
                        action_id: row.get(1)?,
                        action_type: row.get(2)?,
                        permission_level: row.get(3)?,
                        created_at: row.get(4)?,
                    })
                })
                .map_err(|e| format!("Failed to query permissions: {}", e))?;

            let mut permissions = Vec::new();
            for row in rows {
                let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                permissions.push(row.into_permission()?);
            }

            Ok(permissions)
        })
    }

    /// Delete a permission by ID
    pub fn delete_permission(&self, id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let deleted = conn
                .execute("DELETE FROM mcp_action_permissions WHERE id = ?1", params![id])
                .map_err(|e| format!("Failed to delete permission: {}", e))?;

            Ok(deleted > 0)
        })
    }

    // ============================================================================
    // Executions CRUD
    // ============================================================================

    /// Save a new execution record
    pub fn save_execution(&self, execution: &MCPActionExecution) -> Result<(), String> {
        let parameters_json = execution
            .parameters
            .as_ref()
            .map(|p| serde_json::to_string(p))
            .transpose()
            .map_err(|e| format!("Failed to serialize parameters: {}", e))?;

        let result_json = execution
            .result
            .as_ref()
            .map(|r| serde_json::to_string(r))
            .transpose()
            .map_err(|e| format!("Failed to serialize result: {}", e))?;

        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO mcp_action_executions
                (id, action_id, action_type, action_name, source_client, parameters, status, result, error_message, started_at, completed_at, duration_ms)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                ON CONFLICT(id) DO UPDATE SET
                    status = excluded.status,
                    result = excluded.result,
                    error_message = excluded.error_message,
                    completed_at = excluded.completed_at,
                    duration_ms = excluded.duration_ms
                "#,
                params![
                    execution.id,
                    execution.action_id,
                    execution.action_type.to_string(),
                    execution.action_name,
                    execution.source_client,
                    parameters_json,
                    execution.status.to_string(),
                    result_json,
                    execution.error_message,
                    execution.started_at,
                    execution.completed_at,
                    execution.duration_ms,
                ],
            )
            .map_err(|e| format!("Failed to save execution: {}", e))?;

            Ok(())
        })
    }

    /// Get a single execution by ID
    pub fn get_execution(&self, id: &str) -> Result<Option<MCPActionExecution>, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT id, action_id, action_type, action_name, source_client, parameters, status, result, error_message, started_at, completed_at, duration_ms
                FROM mcp_action_executions
                WHERE id = ?1
                "#,
                params![id],
                |row| {
                    Ok(ExecutionRow {
                        id: row.get(0)?,
                        action_id: row.get(1)?,
                        action_type: row.get(2)?,
                        action_name: row.get(3)?,
                        source_client: row.get(4)?,
                        parameters: row.get(5)?,
                        status: row.get(6)?,
                        result: row.get(7)?,
                        error_message: row.get(8)?,
                        started_at: row.get(9)?,
                        completed_at: row.get(10)?,
                        duration_ms: row.get(11)?,
                    })
                },
            );

            match result {
                Ok(row) => Ok(Some(row.into_execution()?)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get execution: {}", e)),
            }
        })
    }

    /// List executions with optional filtering
    pub fn list_executions(&self, filter: &ExecutionFilter) -> Result<Vec<MCPActionExecution>, String> {
        self.db.with_connection(|conn| {
            let mut sql = String::from(
                r#"
                SELECT id, action_id, action_type, action_name, source_client, parameters, status, result, error_message, started_at, completed_at, duration_ms
                FROM mcp_action_executions
                WHERE 1=1
                "#,
            );

            let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(action_id) = &filter.action_id {
                sql.push_str(" AND action_id = ?");
                params_vec.push(Box::new(action_id.clone()));
            }

            if let Some(action_type) = &filter.action_type {
                sql.push_str(" AND action_type = ?");
                params_vec.push(Box::new(action_type.to_string()));
            }

            if let Some(status) = &filter.status {
                sql.push_str(" AND status = ?");
                params_vec.push(Box::new(status.to_string()));
            }

            sql.push_str(" ORDER BY started_at DESC LIMIT ?");
            params_vec.push(Box::new(filter.limit as i64));

            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let params_refs: Vec<&dyn rusqlite::ToSql> =
                params_vec.iter().map(|p| p.as_ref()).collect();

            let rows = stmt
                .query_map(params_refs.as_slice(), |row| {
                    Ok(ExecutionRow {
                        id: row.get(0)?,
                        action_id: row.get(1)?,
                        action_type: row.get(2)?,
                        action_name: row.get(3)?,
                        source_client: row.get(4)?,
                        parameters: row.get(5)?,
                        status: row.get(6)?,
                        result: row.get(7)?,
                        error_message: row.get(8)?,
                        started_at: row.get(9)?,
                        completed_at: row.get(10)?,
                        duration_ms: row.get(11)?,
                    })
                })
                .map_err(|e| format!("Failed to query executions: {}", e))?;

            let mut executions = Vec::new();
            for row in rows {
                let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                executions.push(row.into_execution()?);
            }

            Ok(executions)
        })
    }

    /// Update execution status
    pub fn update_execution_status(
        &self,
        id: &str,
        status: ExecutionStatus,
        result: Option<serde_json::Value>,
        error_message: Option<String>,
    ) -> Result<bool, String> {
        let now = Utc::now().to_rfc3339();
        let result_json = result
            .map(|r| serde_json::to_string(&r))
            .transpose()
            .map_err(|e| format!("Failed to serialize result: {}", e))?;

        self.db.with_connection(|conn| {
            // Get started_at for duration calculation
            let started_at: Option<String> = conn
                .query_row(
                    "SELECT started_at FROM mcp_action_executions WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .ok();

            let duration_ms = started_at.and_then(|start| {
                chrono::DateTime::parse_from_rfc3339(&start)
                    .ok()
                    .map(|start_dt| {
                        let now_dt = Utc::now();
                        (now_dt - start_dt.with_timezone(&Utc)).num_milliseconds()
                    })
            });

            let updated = conn
                .execute(
                    r#"
                    UPDATE mcp_action_executions
                    SET status = ?1, result = ?2, error_message = ?3, completed_at = ?4, duration_ms = ?5
                    WHERE id = ?6
                    "#,
                    params![
                        status.to_string(),
                        result_json,
                        error_message,
                        now,
                        duration_ms,
                        id,
                    ],
                )
                .map_err(|e| format!("Failed to update execution status: {}", e))?;

            Ok(updated > 0)
        })
    }

    /// Get pending confirmation executions
    pub fn get_pending_confirmations(&self) -> Result<Vec<MCPActionExecution>, String> {
        let filter = ExecutionFilter {
            status: Some(ExecutionStatus::PendingConfirm),
            limit: 100,
            ..Default::default()
        };
        self.list_executions(&filter)
    }

    /// Cleanup old executions (retention policy)
    pub fn cleanup_old_executions(&self, keep_count: usize, max_age_days: i64) -> Result<usize, String> {
        self.db.with_connection(|conn| {
            let cutoff_date = (Utc::now() - chrono::Duration::days(max_age_days)).to_rfc3339();

            // Delete old entries beyond retention count or age
            let deleted = conn
                .execute(
                    r#"
                    DELETE FROM mcp_action_executions
                    WHERE id NOT IN (
                        SELECT id FROM mcp_action_executions
                        ORDER BY started_at DESC
                        LIMIT ?1
                    )
                    OR started_at < ?2
                    "#,
                    params![keep_count as i64, cutoff_date],
                )
                .map_err(|e| format!("Failed to cleanup executions: {}", e))?;

            Ok(deleted)
        })
    }

    /// Get total count of executions
    pub fn get_execution_count(&self) -> Result<usize, String> {
        self.db.with_connection(|conn| {
            conn.query_row("SELECT COUNT(*) FROM mcp_action_executions", [], |row| {
                row.get(0)
            })
            .map_err(|e| format!("Failed to count executions: {}", e))
        })
    }

    // ============================================================================
    // Helper: Create new action with generated ID
    // ============================================================================

    /// Create a new action with generated UUID
    pub fn create_action(
        &self,
        action_type: MCPActionType,
        name: String,
        description: Option<String>,
        config: serde_json::Value,
        project_id: Option<String>,
    ) -> Result<MCPAction, String> {
        let now = Utc::now().to_rfc3339();
        let action = MCPAction {
            id: Uuid::new_v4().to_string(),
            action_type,
            name,
            description,
            config,
            project_id,
            is_enabled: true,
            created_at: now.clone(),
            updated_at: now,
        };

        self.save_action(&action)?;
        Ok(action)
    }

    /// Create a new execution record with generated UUID
    pub fn create_execution(
        &self,
        action_id: Option<String>,
        action_type: MCPActionType,
        action_name: String,
        source_client: Option<String>,
        parameters: Option<serde_json::Value>,
        status: ExecutionStatus,
    ) -> Result<MCPActionExecution, String> {
        let now = Utc::now().to_rfc3339();
        let execution = MCPActionExecution {
            id: Uuid::new_v4().to_string(),
            action_id,
            action_type,
            action_name,
            source_client,
            parameters,
            status,
            result: None,
            error_message: None,
            started_at: now,
            completed_at: None,
            duration_ms: None,
        };

        self.save_execution(&execution)?;
        Ok(execution)
    }
}

// ============================================================================
// Internal Row Types
// ============================================================================

struct ActionRow {
    id: String,
    action_type: String,
    name: String,
    description: Option<String>,
    config: String,
    project_id: Option<String>,
    is_enabled: i32,
    created_at: String,
    updated_at: String,
}

impl ActionRow {
    fn into_action(self) -> Result<MCPAction, String> {
        let action_type = self.action_type.parse::<MCPActionType>()?;
        let config: serde_json::Value = serde_json::from_str(&self.config)
            .map_err(|e| format!("Failed to parse config: {}", e))?;

        Ok(MCPAction {
            id: self.id,
            action_type,
            name: self.name,
            description: self.description,
            config,
            project_id: self.project_id,
            is_enabled: self.is_enabled != 0,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

struct PermissionRow {
    id: String,
    action_id: Option<String>,
    action_type: Option<String>,
    permission_level: String,
    created_at: String,
}

impl PermissionRow {
    fn into_permission(self) -> Result<MCPActionPermission, String> {
        let action_type = self
            .action_type
            .map(|t| t.parse::<MCPActionType>())
            .transpose()?;

        let permission_level = self.permission_level.parse::<PermissionLevel>()?;

        Ok(MCPActionPermission {
            id: self.id,
            action_id: self.action_id,
            action_type,
            permission_level,
            created_at: self.created_at,
        })
    }
}

struct ExecutionRow {
    id: String,
    action_id: Option<String>,
    action_type: String,
    action_name: String,
    source_client: Option<String>,
    parameters: Option<String>,
    status: String,
    result: Option<String>,
    error_message: Option<String>,
    started_at: String,
    completed_at: Option<String>,
    duration_ms: Option<i64>,
}

impl ExecutionRow {
    fn into_execution(self) -> Result<MCPActionExecution, String> {
        let action_type = self.action_type.parse::<MCPActionType>()?;
        let status = self.status.parse::<ExecutionStatus>()?;

        let parameters = self
            .parameters
            .map(|p| serde_json::from_str(&p))
            .transpose()
            .map_err(|e| format!("Failed to parse parameters: {}", e))?;

        let result = self
            .result
            .map(|r| serde_json::from_str(&r))
            .transpose()
            .map_err(|e| format!("Failed to parse result: {}", e))?;

        Ok(MCPActionExecution {
            id: self.id,
            action_id: self.action_id,
            action_type,
            action_name: self.action_name,
            source_client: self.source_client,
            parameters,
            status,
            result,
            error_message: self.error_message,
            started_at: self.started_at,
            completed_at: self.completed_at,
            duration_ms: self.duration_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::database::Database;
    use crate::utils::schema::run_migrations;
    use rusqlite::Connection;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn setup_test_db() -> Database {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::new(db_path).unwrap();
        db.with_connection(|conn| run_migrations(conn)).unwrap();
        db
    }

    #[test]
    fn test_action_crud() {
        let db = setup_test_db();
        let repo = MCPActionRepository::new(db);

        // Create
        let action = repo
            .create_action(
                MCPActionType::Script,
                "Test Script".to_string(),
                Some("A test script".to_string()),
                serde_json::json!({"command": "npm test"}),
                None,
            )
            .unwrap();

        assert!(!action.id.is_empty());
        assert_eq!(action.name, "Test Script");

        // Read
        let fetched = repo.get_action(&action.id).unwrap().unwrap();
        assert_eq!(fetched.name, "Test Script");

        // List
        let actions = repo.list_actions(&ActionFilter::default()).unwrap();
        assert_eq!(actions.len(), 1);

        // Delete
        let deleted = repo.delete_action(&action.id).unwrap();
        assert!(deleted);

        let fetched = repo.get_action(&action.id).unwrap();
        assert!(fetched.is_none());
    }

    #[test]
    fn test_permission_fallback() {
        let db = setup_test_db();
        let repo = MCPActionRepository::new(db);

        // Default should be RequireConfirm
        let level = repo.get_permission(None, &MCPActionType::Script).unwrap();
        assert_eq!(level, PermissionLevel::RequireConfirm);
    }

    #[test]
    fn test_execution_status_update() {
        let db = setup_test_db();
        let repo = MCPActionRepository::new(db);

        // Create execution
        let execution = repo
            .create_execution(
                None,
                MCPActionType::Script,
                "npm test".to_string(),
                Some("claude-code".to_string()),
                None,
                ExecutionStatus::Running,
            )
            .unwrap();

        // Update status
        repo.update_execution_status(
            &execution.id,
            ExecutionStatus::Completed,
            Some(serde_json::json!({"exit_code": 0})),
            None,
        )
        .unwrap();

        // Verify
        let updated = repo.get_execution(&execution.id).unwrap().unwrap();
        assert_eq!(updated.status, ExecutionStatus::Completed);
        assert!(updated.completed_at.is_some());
        assert!(updated.duration_ms.is_some());
    }
}
