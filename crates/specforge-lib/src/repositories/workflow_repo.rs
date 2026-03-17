// Workflow Repository
// Handles all database operations for workflows

use rusqlite::params;

use crate::models::{IncomingWebhookConfig, WebhookConfig, Workflow, WorkflowNode};
use crate::utils::database::Database;

/// Repository for workflow data access
pub struct WorkflowRepository {
    db: Database,
}

impl WorkflowRepository {
    /// Create a new WorkflowRepository
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// List all workflows
    pub fn list(&self) -> Result<Vec<Workflow>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, name, description, project_id, nodes, webhook,
                           incoming_webhook, created_at, updated_at, last_executed_at
                    FROM workflows
                    ORDER BY updated_at DESC
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(WorkflowRow {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        description: row.get(2)?,
                        project_id: row.get(3)?,
                        nodes: row.get(4)?,
                        webhook: row.get(5)?,
                        incoming_webhook: row.get(6)?,
                        created_at: row.get(7)?,
                        updated_at: row.get(8)?,
                        last_executed_at: row.get(9)?,
                    })
                })
                .map_err(|e| format!("Failed to query workflows: {}", e))?;

            let mut workflows = Vec::new();
            for row in rows {
                let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                workflows.push(row.into_workflow()?);
            }

            Ok(workflows)
        })
    }

    /// List workflows by project ID
    pub fn list_by_project(&self, project_id: &str) -> Result<Vec<Workflow>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, name, description, project_id, nodes, webhook,
                           incoming_webhook, created_at, updated_at, last_executed_at
                    FROM workflows
                    WHERE project_id = ?1
                    ORDER BY updated_at DESC
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map(params![project_id], |row| {
                    Ok(WorkflowRow {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        description: row.get(2)?,
                        project_id: row.get(3)?,
                        nodes: row.get(4)?,
                        webhook: row.get(5)?,
                        incoming_webhook: row.get(6)?,
                        created_at: row.get(7)?,
                        updated_at: row.get(8)?,
                        last_executed_at: row.get(9)?,
                    })
                })
                .map_err(|e| format!("Failed to query workflows: {}", e))?;

            let mut workflows = Vec::new();
            for row in rows {
                let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                workflows.push(row.into_workflow()?);
            }

            Ok(workflows)
        })
    }

    /// Get a workflow by ID
    pub fn get(&self, id: &str) -> Result<Option<Workflow>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, name, description, project_id, nodes, webhook,
                           incoming_webhook, created_at, updated_at, last_executed_at
                    FROM workflows
                    WHERE id = ?1
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let result = stmt.query_row(params![id], |row| {
                Ok(WorkflowRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    project_id: row.get(3)?,
                    nodes: row.get(4)?,
                    webhook: row.get(5)?,
                    incoming_webhook: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                    last_executed_at: row.get(9)?,
                })
            });

            match result {
                Ok(row) => Ok(Some(row.into_workflow()?)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to query workflow: {}", e)),
            }
        })
    }

    /// Save a workflow (insert or update)
    pub fn save(&self, workflow: &Workflow) -> Result<(), String> {
        // DEBUG: Track ALL save operations
        println!("=== [WorkflowRepository::save] CALLED ===");
        println!("[WorkflowRepository::save] id={}, name={}, project_id={:?}",
            workflow.id, workflow.name, workflow.project_id);
        // Print backtrace to find caller
        println!("[WorkflowRepository::save] Backtrace:");
        let bt = std::backtrace::Backtrace::capture();
        println!("{}", bt);

        let nodes_json = serde_json::to_string(&workflow.nodes)
            .map_err(|e| format!("Failed to serialize nodes: {}", e))?;

        let webhook_json = workflow
            .webhook
            .as_ref()
            .map(|w| serde_json::to_string(w).ok())
            .flatten();

        let incoming_webhook_json = workflow
            .incoming_webhook
            .as_ref()
            .map(|w| serde_json::to_string(w).ok())
            .flatten();

        // IMPORTANT: Uses ON CONFLICT DO UPDATE instead of INSERT OR REPLACE
        // to avoid triggering ON DELETE CASCADE on webhook_tokens table.
        // INSERT OR REPLACE internally does DELETE + INSERT which triggers cascades.
        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO workflows
                (id, name, description, project_id, nodes, webhook, incoming_webhook,
                 created_at, updated_at, last_executed_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    description = excluded.description,
                    project_id = excluded.project_id,
                    nodes = excluded.nodes,
                    webhook = excluded.webhook,
                    incoming_webhook = excluded.incoming_webhook,
                    updated_at = excluded.updated_at,
                    last_executed_at = excluded.last_executed_at
                "#,
                params![
                    workflow.id,
                    workflow.name,
                    workflow.description,
                    workflow.project_id,
                    nodes_json,
                    webhook_json,
                    incoming_webhook_json,
                    workflow.created_at,
                    workflow.updated_at,
                    workflow.last_executed_at,
                ],
            )
            .map_err(|e| format!("Failed to save workflow: {}", e))?;

            // DEBUG: Verify what was actually saved
            let saved: Option<String> = conn
                .query_row(
                    "SELECT project_id FROM workflows WHERE id = ?1",
                    [&workflow.id],
                    |row| row.get(0),
                )
                .ok();
            println!("[WorkflowRepository::save] VERIFIED in DB: project_id={:?}", saved);

            Ok(())
        })
    }

    /// Delete a workflow by ID
    pub fn delete(&self, id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let rows_affected = conn
                .execute("DELETE FROM workflows WHERE id = ?1", params![id])
                .map_err(|e| format!("Failed to delete workflow: {}", e))?;

            Ok(rows_affected > 0)
        })
    }

    /// Update last executed time
    pub fn update_last_executed(&self, id: &str, timestamp: &str) -> Result<(), String> {
        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE workflows SET last_executed_at = ?1 WHERE id = ?2",
                params![timestamp, id],
            )
            .map_err(|e| format!("Failed to update last_executed_at: {}", e))?;

            Ok(())
        })
    }

    /// Update workflow updated_at timestamp
    pub fn touch(&self, id: &str, timestamp: &str) -> Result<(), String> {
        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE workflows SET updated_at = ?1 WHERE id = ?2",
                params![timestamp, id],
            )
            .map_err(|e| format!("Failed to update updated_at: {}", e))?;

            Ok(())
        })
    }

    // =========================================================================
    // Webhook Tokens (Encrypted)
    // =========================================================================

    /// Store an encrypted webhook token for a workflow
    pub fn store_webhook_token(
        &self,
        workflow_id: &str,
        ciphertext: &str,
        nonce: &str,
    ) -> Result<(), String> {
        println!(
            "[store_webhook_token] Attempting to store token for workflow {}, cipher_len={}, nonce_len={}",
            workflow_id,
            ciphertext.len(),
            nonce.len()
        );
        self.db.with_connection(|conn| {
            let rows_affected = conn.execute(
                r#"
                INSERT OR REPLACE INTO webhook_tokens
                (workflow_id, ciphertext, nonce, updated_at)
                VALUES (?1, ?2, ?3, datetime('now'))
                "#,
                params![workflow_id, ciphertext, nonce],
            )
            .map_err(|e| format!("Failed to store webhook token: {}", e))?;

            println!(
                "[store_webhook_token] SQL executed, rows_affected={}",
                rows_affected
            );

            // Force WAL checkpoint to ensure data is persisted to disk
            // This prevents data loss if the app exits before automatic checkpoint
            let checkpoint_result: Result<(i32, i32, i32), _> = conn.query_row(
                "PRAGMA wal_checkpoint(PASSIVE)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            );
            match checkpoint_result {
                Ok((busy, log, checkpointed)) => {
                    println!(
                        "[store_webhook_token] WAL checkpoint: busy={}, log={}, checkpointed={}",
                        busy, log, checkpointed
                    );
                }
                Err(e) => {
                    println!("[store_webhook_token] WAL checkpoint error: {}", e);
                }
            }

            // Verify the insert
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM webhook_tokens WHERE workflow_id = ?1",
                    params![workflow_id],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            println!(
                "[store_webhook_token] Verification: count for workflow {} = {}",
                workflow_id, count
            );

            Ok(())
        })
    }

    /// Get encrypted webhook token data for a workflow
    /// Returns (ciphertext, nonce) if found
    pub fn get_webhook_token(&self, workflow_id: &str) -> Result<Option<(String, String)>, String> {
        println!(
            "[get_webhook_token] Looking up token for workflow {}",
            workflow_id
        );
        self.db.with_connection(|conn| {
            // Debug: Count all rows in webhook_tokens
            let total_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM webhook_tokens", [], |row| row.get(0))
                .unwrap_or(-1);
            println!(
                "[get_webhook_token] Total rows in webhook_tokens table: {}",
                total_count
            );

            // Debug: List all workflow_ids in webhook_tokens
            if total_count > 0 {
                let mut stmt = conn
                    .prepare("SELECT workflow_id FROM webhook_tokens")
                    .ok();
                if let Some(ref mut s) = stmt {
                    let ids: Vec<String> = s
                        .query_map([], |row| row.get(0))
                        .ok()
                        .map(|rows| rows.filter_map(|r| r.ok()).collect())
                        .unwrap_or_default();
                    println!("[get_webhook_token] Workflow IDs in table: {:?}", ids);
                }
            }

            let result = conn.query_row(
                "SELECT ciphertext, nonce FROM webhook_tokens WHERE workflow_id = ?1",
                params![workflow_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            );

            match result {
                Ok(data) => {
                    println!(
                        "[get_webhook_token] Found token for workflow {}, cipher_len={}, nonce_len={}",
                        workflow_id,
                        data.0.len(),
                        data.1.len()
                    );
                    Ok(Some(data))
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    println!(
                        "[get_webhook_token] No rows returned for workflow {}",
                        workflow_id
                    );
                    Ok(None)
                }
                Err(e) => {
                    println!(
                        "[get_webhook_token] Error querying for workflow {}: {}",
                        workflow_id, e
                    );
                    Err(format!("Failed to get webhook token: {}", e))
                }
            }
        })
    }

    /// Delete encrypted webhook token for a workflow
    pub fn delete_webhook_token(&self, workflow_id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let rows_affected = conn
                .execute(
                    "DELETE FROM webhook_tokens WHERE workflow_id = ?1",
                    params![workflow_id],
                )
                .map_err(|e| format!("Failed to delete webhook token: {}", e))?;

            Ok(rows_affected > 0)
        })
    }

    /// Check if encrypted webhook token exists for a workflow
    pub fn has_webhook_token(&self, workflow_id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM webhook_tokens WHERE workflow_id = ?1",
                    params![workflow_id],
                    |row| row.get(0),
                )
                .map_err(|e| format!("Failed to check webhook token: {}", e))?;

            Ok(count > 0)
        })
    }
}

/// Internal row structure for mapping database rows
struct WorkflowRow {
    id: String,
    name: String,
    description: Option<String>,
    project_id: Option<String>,
    nodes: String,
    webhook: Option<String>,
    incoming_webhook: Option<String>,
    created_at: String,
    updated_at: String,
    last_executed_at: Option<String>,
}

impl WorkflowRow {
    fn into_workflow(self) -> Result<Workflow, String> {
        let nodes: Vec<WorkflowNode> = serde_json::from_str(&self.nodes)
            .map_err(|e| format!("Failed to parse nodes: {}", e))?;

        let webhook: Option<WebhookConfig> = self
            .webhook
            .as_ref()
            .map(|json| serde_json::from_str(json).ok())
            .flatten();

        let incoming_webhook: Option<IncomingWebhookConfig> = self
            .incoming_webhook
            .as_ref()
            .map(|json| serde_json::from_str(json).ok())
            .flatten();

        Ok(Workflow {
            id: self.id,
            name: self.name,
            description: self.description,
            project_id: self.project_id,
            nodes,
            created_at: self.created_at,
            updated_at: self.updated_at,
            last_executed_at: self.last_executed_at,
            webhook,
            incoming_webhook,
        })
    }
}
