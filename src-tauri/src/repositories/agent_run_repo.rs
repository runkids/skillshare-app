// Agent Run Repository
// SQLite CRUD for the `agent_runs` table — tracks AI agent subprocess lifecycle.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

/// A row from the `agent_runs` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRun {
    pub id: String,
    pub spec_id: String,
    pub phase: String,
    pub prompt: String,
    pub status: String,
    pub pid: Option<u32>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub error: Option<String>,
}

/// Insert a new agent run.
pub fn insert_agent_run(conn: &Connection, run: &AgentRun) -> Result<(), String> {
    conn.execute(
        r#"
        INSERT INTO agent_runs (id, spec_id, phase, prompt, status, pid, started_at, finished_at, error)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#,
        params![
            run.id,
            run.spec_id,
            run.phase,
            run.prompt,
            run.status,
            run.pid,
            run.started_at,
            run.finished_at,
            run.error,
        ],
    )
    .map_err(|e| format!("Failed to insert agent run: {e}"))?;
    Ok(())
}

/// Get an agent run by ID.
pub fn get_agent_run(conn: &Connection, id: &str) -> Result<Option<AgentRun>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, spec_id, phase, prompt, status, pid, started_at, finished_at, error
            FROM agent_runs
            WHERE id = ?1
            "#,
        )
        .map_err(|e| format!("Failed to prepare get_agent_run: {e}"))?;

    let mut rows = stmt
        .query_map(params![id], row_to_agent_run)
        .map_err(|e| format!("Failed to query agent run: {e}"))?;

    match rows.next() {
        Some(row) => Ok(Some(
            row.map_err(|e| format!("Failed to read agent run row: {e}"))?,
        )),
        None => Ok(None),
    }
}

/// Get all agent runs with status "running".
pub fn get_running_agents(conn: &Connection) -> Result<Vec<AgentRun>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, spec_id, phase, prompt, status, pid, started_at, finished_at, error
            FROM agent_runs
            WHERE status = 'running'
            ORDER BY started_at DESC
            "#,
        )
        .map_err(|e| format!("Failed to prepare get_running_agents: {e}"))?;

    let rows = stmt
        .query_map([], row_to_agent_run)
        .map_err(|e| format!("Failed to query running agents: {e}"))?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| format!("Failed to read agent run row: {e}"))?);
    }
    Ok(result)
}

/// Get all agent runs for a specific spec.
pub fn get_agents_for_spec(conn: &Connection, spec_id: &str) -> Result<Vec<AgentRun>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, spec_id, phase, prompt, status, pid, started_at, finished_at, error
            FROM agent_runs
            WHERE spec_id = ?1
            ORDER BY started_at DESC
            "#,
        )
        .map_err(|e| format!("Failed to prepare get_agents_for_spec: {e}"))?;

    let rows = stmt
        .query_map(params![spec_id], row_to_agent_run)
        .map_err(|e| format!("Failed to query agents for spec: {e}"))?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| format!("Failed to read agent run row: {e}"))?);
    }
    Ok(result)
}

/// Update agent status and optionally set error message.
pub fn update_agent_status(
    conn: &Connection,
    id: &str,
    status: &str,
    error: Option<&str>,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let updated = conn
        .execute(
            r#"
            UPDATE agent_runs
            SET status = ?1, finished_at = ?2, error = ?3
            WHERE id = ?4
            "#,
            params![status, now, error, id],
        )
        .map_err(|e| format!("Failed to update agent status: {e}"))?;

    if updated == 0 {
        return Err(format!("Agent run not found: {id}"));
    }
    Ok(())
}

/// Map a SQLite row to an AgentRun struct.
fn row_to_agent_run(row: &rusqlite::Row) -> rusqlite::Result<AgentRun> {
    Ok(AgentRun {
        id: row.get(0)?,
        spec_id: row.get(1)?,
        phase: row.get(2)?,
        prompt: row.get(3)?,
        status: row.get(4)?,
        pid: row.get(5)?,
        started_at: row.get(6)?,
        finished_at: row.get(7)?,
        error: row.get(8)?,
    })
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

    /// Insert a dummy spec row so FK constraints are satisfied.
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

    fn sample_run() -> AgentRun {
        AgentRun {
            id: "run-001".to_string(),
            spec_id: "spec-2026-01-01-test-0000".to_string(),
            phase: "implement".to_string(),
            prompt: "Implement the feature described in the spec".to_string(),
            status: "running".to_string(),
            pid: Some(12345),
            started_at: "2026-01-01T00:00:00Z".to_string(),
            finished_at: None,
            error: None,
        }
    }

    #[test]
    fn test_insert_and_get() {
        let conn = setup_db();
        let run = sample_run();
        insert_dummy_spec(&conn, &run.spec_id);

        insert_agent_run(&conn, &run).expect("insert");

        let fetched = get_agent_run(&conn, "run-001")
            .expect("get")
            .expect("should exist");
        assert_eq!(fetched.id, "run-001");
        assert_eq!(fetched.spec_id, "spec-2026-01-01-test-0000");
        assert_eq!(fetched.phase, "implement");
        assert_eq!(fetched.status, "running");
        assert_eq!(fetched.pid, Some(12345));
    }

    #[test]
    fn test_get_nonexistent() {
        let conn = setup_db();
        let result = get_agent_run(&conn, "no-such-id").expect("get");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_running_agents() {
        let conn = setup_db();
        let run = sample_run();
        insert_dummy_spec(&conn, &run.spec_id);
        insert_agent_run(&conn, &run).expect("insert");

        let running = get_running_agents(&conn).expect("get running");
        assert_eq!(running.len(), 1);
        assert_eq!(running[0].id, "run-001");
    }

    #[test]
    fn test_get_agents_for_spec() {
        let conn = setup_db();
        let run = sample_run();
        insert_dummy_spec(&conn, &run.spec_id);
        insert_agent_run(&conn, &run).expect("insert");

        let agents = get_agents_for_spec(&conn, &run.spec_id).expect("get for spec");
        assert_eq!(agents.len(), 1);

        let empty = get_agents_for_spec(&conn, "other-spec").expect("get for other");
        assert!(empty.is_empty());
    }

    #[test]
    fn test_update_status() {
        let conn = setup_db();
        let run = sample_run();
        insert_dummy_spec(&conn, &run.spec_id);
        insert_agent_run(&conn, &run).expect("insert");

        update_agent_status(&conn, "run-001", "completed", None).expect("update");

        let fetched = get_agent_run(&conn, "run-001")
            .expect("get")
            .expect("should exist");
        assert_eq!(fetched.status, "completed");
        assert!(fetched.finished_at.is_some());
        assert!(fetched.error.is_none());
    }

    #[test]
    fn test_update_status_with_error() {
        let conn = setup_db();
        let run = sample_run();
        insert_dummy_spec(&conn, &run.spec_id);
        insert_agent_run(&conn, &run).expect("insert");

        update_agent_status(
            &conn,
            "run-001",
            "failed",
            Some("Process exited with code 1"),
        )
        .expect("update");

        let fetched = get_agent_run(&conn, "run-001")
            .expect("get")
            .expect("should exist");
        assert_eq!(fetched.status, "failed");
        assert_eq!(
            fetched.error,
            Some("Process exited with code 1".to_string())
        );
    }

    #[test]
    fn test_update_nonexistent() {
        let conn = setup_db();
        let result = update_agent_status(&conn, "no-such-id", "completed", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }
}
