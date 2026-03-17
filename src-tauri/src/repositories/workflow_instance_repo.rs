// Workflow Instance Repository
// SQLite CRUD for `workflow_instances` and `phase_history` tables.

use crate::local_models::workflow_phase::WorkflowInstance;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

/// A row from the `phase_history` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseHistoryRow {
    pub id: i64,
    pub instance_id: String,
    pub from_phase: Option<String>,
    pub to_phase: String,
    pub gate_result: Option<String>,
    pub action_results: Option<String>,
    pub transitioned_at: String,
}

/// Insert a new workflow instance.
pub fn create_instance(conn: &Connection, instance: &WorkflowInstance) -> Result<(), String> {
    conn.execute(
        r#"
        INSERT INTO workflow_instances (id, spec_id, workflow_id, current_phase, started_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![
            instance.id,
            instance.spec_id,
            instance.workflow_id,
            instance.current_phase,
            instance.started_at,
            instance.updated_at,
        ],
    )
    .map_err(|e| format!("Failed to create workflow instance: {e}"))?;
    Ok(())
}

/// Get the workflow instance for a given spec (at most one per spec).
pub fn get_instance_by_spec(
    conn: &Connection,
    spec_id: &str,
) -> Result<Option<WorkflowInstance>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, spec_id, workflow_id, current_phase, started_at, updated_at
            FROM workflow_instances
            WHERE spec_id = ?1
            "#,
        )
        .map_err(|e| format!("Failed to prepare get_instance_by_spec: {e}"))?;

    let mut rows = stmt
        .query_map(params![spec_id], |row| {
            Ok(WorkflowInstance {
                id: row.get(0)?,
                spec_id: row.get(1)?,
                workflow_id: row.get(2)?,
                current_phase: row.get(3)?,
                started_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| format!("Failed to query workflow instance: {e}"))?;

    match rows.next() {
        Some(row) => Ok(Some(
            row.map_err(|e| format!("Failed to read workflow instance row: {e}"))?,
        )),
        None => Ok(None),
    }
}

/// Update the current phase of a workflow instance.
pub fn update_instance_phase(
    conn: &Connection,
    instance_id: &str,
    new_phase: &str,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let updated = conn
        .execute(
            r#"
            UPDATE workflow_instances
            SET current_phase = ?1, updated_at = ?2
            WHERE id = ?3
            "#,
            params![new_phase, now, instance_id],
        )
        .map_err(|e| format!("Failed to update workflow instance phase: {e}"))?;

    if updated == 0 {
        return Err(format!("Workflow instance not found: {instance_id}"));
    }
    Ok(())
}

/// Record a phase transition in the history table.
pub fn insert_phase_history(
    conn: &Connection,
    instance_id: &str,
    from_phase: Option<&str>,
    to_phase: &str,
    gate_result: Option<&str>,
    action_results: Option<&str>,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        r#"
        INSERT INTO phase_history (instance_id, from_phase, to_phase, gate_result, action_results, transitioned_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![instance_id, from_phase, to_phase, gate_result, action_results, now],
    )
    .map_err(|e| format!("Failed to insert phase history: {e}"))?;
    Ok(())
}

/// Get the full phase transition history for a workflow instance.
pub fn get_phase_history(
    conn: &Connection,
    instance_id: &str,
) -> Result<Vec<PhaseHistoryRow>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, instance_id, from_phase, to_phase, gate_result, action_results, transitioned_at
            FROM phase_history
            WHERE instance_id = ?1
            ORDER BY id ASC
            "#,
        )
        .map_err(|e| format!("Failed to prepare get_phase_history: {e}"))?;

    let rows = stmt
        .query_map(params![instance_id], |row| {
            Ok(PhaseHistoryRow {
                id: row.get(0)?,
                instance_id: row.get(1)?,
                from_phase: row.get(2)?,
                to_phase: row.get(3)?,
                gate_result: row.get(4)?,
                action_results: row.get(5)?,
                transitioned_at: row.get(6)?,
            })
        })
        .map_err(|e| format!("Failed to query phase history: {e}"))?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| format!("Failed to read phase history row: {e}"))?);
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::schema::run_migrations;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        run_migrations(&conn).expect("run migrations");
        conn
    }

    fn sample_instance() -> WorkflowInstance {
        WorkflowInstance {
            id: "wfi-001".to_string(),
            spec_id: "spec-2026-01-01-test-0000".to_string(),
            workflow_id: "basic-sdd".to_string(),
            current_phase: "discuss".to_string(),
            started_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    /// Insert a dummy spec row so the FK constraint is satisfied.
    fn insert_dummy_spec(conn: &Connection, spec_id: &str) {
        conn.execute(
            r#"
            INSERT INTO specs (id, schema_id, title, status, file_path, created_at, updated_at)
            VALUES (?1, 'test', 'Test', 'draft', '.specforge/specs/test.md', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')
            "#,
            params![spec_id],
        )
        .expect("insert dummy spec");
    }

    #[test]
    fn test_create_and_get_instance() {
        let conn = setup_db();
        let inst = sample_instance();
        insert_dummy_spec(&conn, &inst.spec_id);

        create_instance(&conn, &inst).expect("create");

        let fetched = get_instance_by_spec(&conn, &inst.spec_id)
            .expect("get")
            .expect("should exist");
        assert_eq!(fetched.id, "wfi-001");
        assert_eq!(fetched.current_phase, "discuss");
        assert_eq!(fetched.workflow_id, "basic-sdd");
    }

    #[test]
    fn test_get_instance_nonexistent() {
        let conn = setup_db();
        let result = get_instance_by_spec(&conn, "no-such-spec").expect("get");
        assert!(result.is_none());
    }

    #[test]
    fn test_update_instance_phase() {
        let conn = setup_db();
        let inst = sample_instance();
        insert_dummy_spec(&conn, &inst.spec_id);
        create_instance(&conn, &inst).expect("create");

        update_instance_phase(&conn, "wfi-001", "specify").expect("update");

        let fetched = get_instance_by_spec(&conn, &inst.spec_id)
            .expect("get")
            .expect("should exist");
        assert_eq!(fetched.current_phase, "specify");
        // updated_at should have changed
        assert_ne!(fetched.updated_at, inst.updated_at);
    }

    #[test]
    fn test_update_nonexistent_instance() {
        let conn = setup_db();
        let result = update_instance_phase(&conn, "no-such-id", "specify");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_phase_history() {
        let conn = setup_db();
        let inst = sample_instance();
        insert_dummy_spec(&conn, &inst.spec_id);
        create_instance(&conn, &inst).expect("create");

        // Record two transitions
        insert_phase_history(
            &conn,
            "wfi-001",
            None,
            "discuss",
            None,
            None,
        )
        .expect("insert initial");

        insert_phase_history(
            &conn,
            "wfi-001",
            Some("discuss"),
            "specify",
            Some("passed"),
            Some("[\"notify\"]"),
        )
        .expect("insert transition");

        let history = get_phase_history(&conn, "wfi-001").expect("get history");
        assert_eq!(history.len(), 2);

        assert!(history[0].from_phase.is_none());
        assert_eq!(history[0].to_phase, "discuss");

        assert_eq!(history[1].from_phase, Some("discuss".to_string()));
        assert_eq!(history[1].to_phase, "specify");
        assert_eq!(history[1].gate_result, Some("passed".to_string()));
    }

    #[test]
    fn test_phase_history_empty() {
        let conn = setup_db();
        let history = get_phase_history(&conn, "nonexistent").expect("get");
        assert!(history.is_empty());
    }
}
