// Spec Repository
// SQLite CRUD for the `specs` table (index/cache — file on disk is source of truth)

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

/// Row struct mapping directly to the `specs` SQLite table columns.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecRow {
    pub id: String,
    pub schema_id: String,
    pub title: String,
    pub status: String,
    pub workflow_id: Option<String>,
    pub workflow_phase: Option<String>,
    pub file_path: String,
    pub fields_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Insert a spec row into the SQLite index.
pub fn insert_spec(conn: &Connection, row: &SpecRow) -> Result<(), String> {
    conn.execute(
        r#"
        INSERT INTO specs (id, schema_id, title, status, workflow_id, workflow_phase, file_path, fields_json, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#,
        params![
            row.id,
            row.schema_id,
            row.title,
            row.status,
            row.workflow_id,
            row.workflow_phase,
            row.file_path,
            row.fields_json,
            row.created_at,
            row.updated_at,
        ],
    )
    .map_err(|e| format!("Failed to insert spec: {}", e))?;
    Ok(())
}

/// Get a single spec row by ID.
pub fn get_spec(conn: &Connection, id: &str) -> Result<Option<SpecRow>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, schema_id, title, status, workflow_id, workflow_phase, file_path, fields_json, created_at, updated_at
            FROM specs
            WHERE id = ?1
            "#,
        )
        .map_err(|e| format!("Failed to prepare get_spec: {}", e))?;

    let mut rows = stmt
        .query_map(params![id], row_mapper)
        .map_err(|e| format!("Failed to query spec: {}", e))?;

    match rows.next() {
        Some(row) => Ok(Some(row.map_err(|e| format!("Failed to read spec row: {}", e))?)),
        None => Ok(None),
    }
}

/// List specs with optional filters on status and workflow_phase.
pub fn list_specs(
    conn: &Connection,
    status: Option<&str>,
    workflow_phase: Option<&str>,
) -> Result<Vec<SpecRow>, String> {
    let mut sql = String::from(
        "SELECT id, schema_id, title, status, workflow_id, workflow_phase, file_path, fields_json, created_at, updated_at FROM specs",
    );
    let mut conditions: Vec<String> = Vec::new();

    if status.is_some() {
        conditions.push("status = ?1".to_string());
    }
    if workflow_phase.is_some() {
        let idx = if status.is_some() { 2 } else { 1 };
        conditions.push(format!("workflow_phase = ?{}", idx));
    }

    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }

    sql.push_str(" ORDER BY updated_at DESC");

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to prepare list_specs: {}", e))?;

    let rows = match (status, workflow_phase) {
        (Some(s), Some(wp)) => stmt
            .query_map(params![s, wp], row_mapper)
            .map_err(|e| format!("Failed to query specs: {}", e))?,
        (Some(s), None) => stmt
            .query_map(params![s], row_mapper)
            .map_err(|e| format!("Failed to query specs: {}", e))?,
        (None, Some(wp)) => stmt
            .query_map(params![wp], row_mapper)
            .map_err(|e| format!("Failed to query specs: {}", e))?,
        (None, None) => stmt
            .query_map([], row_mapper)
            .map_err(|e| format!("Failed to query specs: {}", e))?,
    };

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| format!("Failed to read spec row: {}", e))?);
    }
    Ok(result)
}

/// Update an existing spec row.
pub fn update_spec(conn: &Connection, row: &SpecRow) -> Result<(), String> {
    let updated = conn
        .execute(
            r#"
            UPDATE specs
            SET schema_id = ?1, title = ?2, status = ?3, workflow_id = ?4, workflow_phase = ?5,
                file_path = ?6, fields_json = ?7, updated_at = ?8
            WHERE id = ?9
            "#,
            params![
                row.schema_id,
                row.title,
                row.status,
                row.workflow_id,
                row.workflow_phase,
                row.file_path,
                row.fields_json,
                row.updated_at,
                row.id,
            ],
        )
        .map_err(|e| format!("Failed to update spec: {}", e))?;

    if updated == 0 {
        return Err(format!("Spec not found: {}", row.id));
    }
    Ok(())
}

/// Delete a spec row by ID.
pub fn delete_spec(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM specs WHERE id = ?1", params![id])
        .map_err(|e| format!("Failed to delete spec: {}", e))?;
    Ok(())
}

/// INSERT OR REPLACE — upsert a spec row.
pub fn upsert_spec(conn: &Connection, row: &SpecRow) -> Result<(), String> {
    conn.execute(
        r#"
        INSERT OR REPLACE INTO specs (id, schema_id, title, status, workflow_id, workflow_phase, file_path, fields_json, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#,
        params![
            row.id,
            row.schema_id,
            row.title,
            row.status,
            row.workflow_id,
            row.workflow_phase,
            row.file_path,
            row.fields_json,
            row.created_at,
            row.updated_at,
        ],
    )
    .map_err(|e| format!("Failed to upsert spec: {}", e))?;
    Ok(())
}

/// Map a rusqlite row to a SpecRow.
fn row_mapper(row: &rusqlite::Row) -> rusqlite::Result<SpecRow> {
    Ok(SpecRow {
        id: row.get(0)?,
        schema_id: row.get(1)?,
        title: row.get(2)?,
        status: row.get(3)?,
        workflow_id: row.get(4)?,
        workflow_phase: row.get(5)?,
        file_path: row.get(6)?,
        fields_json: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::schema::run_migrations;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        run_migrations(&conn).expect("run migrations");
        conn
    }

    fn sample_row() -> SpecRow {
        SpecRow {
            id: "spec-2026-03-17-test-abcd".to_string(),
            schema_id: "change-request".to_string(),
            title: "Add OAuth2".to_string(),
            status: "draft".to_string(),
            workflow_id: Some("default".to_string()),
            workflow_phase: Some("discuss".to_string()),
            file_path: ".specforge/specs/spec-2026-03-17-test-abcd.md".to_string(),
            fields_json: Some(r#"{"priority":"high"}"#.to_string()),
            created_at: "2026-03-17T10:00:00Z".to_string(),
            updated_at: "2026-03-17T10:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_insert_and_get() {
        let conn = setup_db();
        let row = sample_row();

        insert_spec(&conn, &row).expect("insert");

        let fetched = get_spec(&conn, &row.id).expect("get").expect("should exist");
        assert_eq!(fetched.id, row.id);
        assert_eq!(fetched.title, "Add OAuth2");
        assert_eq!(fetched.schema_id, "change-request");
        assert_eq!(fetched.status, "draft");
        assert_eq!(fetched.workflow_id, Some("default".to_string()));
        assert_eq!(fetched.workflow_phase, Some("discuss".to_string()));
    }

    #[test]
    fn test_get_nonexistent() {
        let conn = setup_db();
        let result = get_spec(&conn, "does-not-exist").expect("get");
        assert!(result.is_none());
    }

    #[test]
    fn test_list_specs_no_filter() {
        let conn = setup_db();

        let mut row1 = sample_row();
        row1.id = "spec-001".to_string();
        row1.file_path = ".specforge/specs/spec-001.md".to_string();
        insert_spec(&conn, &row1).expect("insert 1");

        let mut row2 = sample_row();
        row2.id = "spec-002".to_string();
        row2.status = "active".to_string();
        row2.file_path = ".specforge/specs/spec-002.md".to_string();
        insert_spec(&conn, &row2).expect("insert 2");

        let all = list_specs(&conn, None, None).expect("list");
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_list_specs_filter_status() {
        let conn = setup_db();

        let mut row1 = sample_row();
        row1.id = "spec-001".to_string();
        row1.file_path = ".specforge/specs/spec-001.md".to_string();
        insert_spec(&conn, &row1).expect("insert 1");

        let mut row2 = sample_row();
        row2.id = "spec-002".to_string();
        row2.status = "active".to_string();
        row2.file_path = ".specforge/specs/spec-002.md".to_string();
        insert_spec(&conn, &row2).expect("insert 2");

        let drafts = list_specs(&conn, Some("draft"), None).expect("list drafts");
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].id, "spec-001");

        let active = list_specs(&conn, Some("active"), None).expect("list active");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "spec-002");
    }

    #[test]
    fn test_list_specs_filter_workflow_phase() {
        let conn = setup_db();

        let mut row1 = sample_row();
        row1.id = "spec-001".to_string();
        row1.workflow_phase = Some("discuss".to_string());
        row1.file_path = ".specforge/specs/spec-001.md".to_string();
        insert_spec(&conn, &row1).expect("insert");

        let mut row2 = sample_row();
        row2.id = "spec-002".to_string();
        row2.workflow_phase = Some("implement".to_string());
        row2.file_path = ".specforge/specs/spec-002.md".to_string();
        insert_spec(&conn, &row2).expect("insert");

        let discuss = list_specs(&conn, None, Some("discuss")).expect("list");
        assert_eq!(discuss.len(), 1);
        assert_eq!(discuss[0].id, "spec-001");
    }

    #[test]
    fn test_update_spec() {
        let conn = setup_db();
        let mut row = sample_row();
        insert_spec(&conn, &row).expect("insert");

        row.title = "Updated Title".to_string();
        row.status = "active".to_string();
        row.updated_at = "2026-03-17T12:00:00Z".to_string();
        update_spec(&conn, &row).expect("update");

        let fetched = get_spec(&conn, &row.id).expect("get").expect("should exist");
        assert_eq!(fetched.title, "Updated Title");
        assert_eq!(fetched.status, "active");
    }

    #[test]
    fn test_update_nonexistent() {
        let conn = setup_db();
        let row = sample_row();
        let result = update_spec(&conn, &row);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Spec not found"));
    }

    #[test]
    fn test_delete_spec() {
        let conn = setup_db();
        let row = sample_row();
        insert_spec(&conn, &row).expect("insert");

        delete_spec(&conn, &row.id).expect("delete");

        let fetched = get_spec(&conn, &row.id).expect("get");
        assert!(fetched.is_none());
    }

    #[test]
    fn test_upsert_spec_insert() {
        let conn = setup_db();
        let row = sample_row();
        upsert_spec(&conn, &row).expect("upsert insert");

        let fetched = get_spec(&conn, &row.id).expect("get").expect("should exist");
        assert_eq!(fetched.title, "Add OAuth2");
    }

    #[test]
    fn test_upsert_spec_replace() {
        let conn = setup_db();
        let mut row = sample_row();
        upsert_spec(&conn, &row).expect("upsert insert");

        row.title = "Replaced Title".to_string();
        row.updated_at = "2026-03-17T12:00:00Z".to_string();
        upsert_spec(&conn, &row).expect("upsert replace");

        let fetched = get_spec(&conn, &row.id).expect("get").expect("should exist");
        assert_eq!(fetched.title, "Replaced Title");

        // Should still be only one row
        let all = list_specs(&conn, None, None).expect("list");
        assert_eq!(all.len(), 1);
    }
}
