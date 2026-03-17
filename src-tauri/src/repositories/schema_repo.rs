// Schema Repository
// SQLite CRUD for the `schemas` table (registry of known schema definitions)

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

/// Row struct mapping directly to the `schemas` SQLite table columns.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaRow {
    pub id: String,
    pub name: String,
    pub display_name: Option<String>,
    pub file_path: String,
    pub fields_json: Option<String>,
    pub updated_at: String,
}

/// INSERT OR REPLACE a schema row.
pub fn upsert_schema(
    conn: &Connection,
    name: &str,
    display_name: Option<&str>,
    file_path: &str,
    fields_json: &str,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        r#"
        INSERT OR REPLACE INTO schemas (id, name, display_name, file_path, fields_json, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![name, name, display_name, file_path, fields_json, now],
    )
    .map_err(|e| format!("Failed to upsert schema: {}", e))?;
    Ok(())
}

/// List all schema rows.
pub fn list_schemas(conn: &Connection) -> Result<Vec<SchemaRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, display_name, file_path, fields_json, updated_at FROM schemas ORDER BY name",
        )
        .map_err(|e| format!("Failed to prepare list_schemas: {}", e))?;

    let rows = stmt
        .query_map([], row_mapper)
        .map_err(|e| format!("Failed to query schemas: {}", e))?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| format!("Failed to read schema row: {}", e))?);
    }
    Ok(result)
}

/// Get a single schema row by name (which is used as the primary key/id).
pub fn get_schema(conn: &Connection, name: &str) -> Result<Option<SchemaRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, display_name, file_path, fields_json, updated_at FROM schemas WHERE name = ?1",
        )
        .map_err(|e| format!("Failed to prepare get_schema: {}", e))?;

    let mut rows = stmt
        .query_map(params![name], row_mapper)
        .map_err(|e| format!("Failed to query schema: {}", e))?;

    match rows.next() {
        Some(row) => Ok(Some(
            row.map_err(|e| format!("Failed to read schema row: {}", e))?,
        )),
        None => Ok(None),
    }
}

/// Map a rusqlite row to a SchemaRow.
fn row_mapper(row: &rusqlite::Row) -> rusqlite::Result<SchemaRow> {
    Ok(SchemaRow {
        id: row.get(0)?,
        name: row.get(1)?,
        display_name: row.get(2)?,
        file_path: row.get(3)?,
        fields_json: row.get(4)?,
        updated_at: row.get(5)?,
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

    #[test]
    fn test_upsert_and_get() {
        let conn = setup_db();
        upsert_schema(
            &conn,
            "change-request",
            Some("Change Request"),
            ".specforge/schemas/change-request.schema.yaml",
            r#"{"title":"string","priority":["high","medium","low"]}"#,
        )
        .expect("upsert");

        let row = get_schema(&conn, "change-request")
            .expect("get")
            .expect("should exist");
        assert_eq!(row.name, "change-request");
        assert_eq!(row.display_name, Some("Change Request".to_string()));
        assert!(row.fields_json.is_some());
    }

    #[test]
    fn test_get_nonexistent() {
        let conn = setup_db();
        let result = get_schema(&conn, "nope").expect("get");
        assert!(result.is_none());
    }

    #[test]
    fn test_upsert_replaces() {
        let conn = setup_db();
        upsert_schema(
            &conn,
            "spec",
            Some("Spec"),
            ".specforge/schemas/spec.schema.yaml",
            r#"{"title":"string"}"#,
        )
        .expect("upsert 1");

        upsert_schema(
            &conn,
            "spec",
            Some("Updated Spec"),
            ".specforge/schemas/spec.schema.yaml",
            r#"{"title":"string","status":["draft","active"]}"#,
        )
        .expect("upsert 2");

        let all = list_schemas(&conn).expect("list");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].display_name, Some("Updated Spec".to_string()));
    }

    #[test]
    fn test_list_schemas() {
        let conn = setup_db();

        upsert_schema(
            &conn,
            "spec",
            Some("Spec"),
            ".specforge/schemas/spec.schema.yaml",
            "{}",
        )
        .expect("upsert spec");

        upsert_schema(
            &conn,
            "task",
            Some("Task"),
            ".specforge/schemas/task.schema.yaml",
            "{}",
        )
        .expect("upsert task");

        let all = list_schemas(&conn).expect("list");
        assert_eq!(all.len(), 2);
        // Ordered by name
        assert_eq!(all[0].name, "spec");
        assert_eq!(all[1].name, "task");
    }
}
