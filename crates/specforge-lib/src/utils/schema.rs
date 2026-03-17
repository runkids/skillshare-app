// SQLite Schema Definitions — SpecForge v1 (clean rewrite)
// No migrations: single CREATE TABLE pass for all tables.

use rusqlite::{Connection, params};

/// Current schema version
pub const CURRENT_VERSION: i32 = 1;

/// The complete v1 schema — every table the app needs.
const SCHEMA_V1: &str = r#"
    -- Settings (key-value store)
    CREATE TABLE IF NOT EXISTS settings (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );

    -- Schema version tracking
    CREATE TABLE IF NOT EXISTS schema_version (
        version INTEGER NOT NULL
    );

    -- Workflow definitions
    CREATE TABLE IF NOT EXISTS workflows (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        nodes TEXT NOT NULL DEFAULT '[]',
        webhook TEXT,
        incoming_webhook TEXT,
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at TEXT NOT NULL DEFAULT (datetime('now')),
        last_executed_at TEXT
    );

    -- Running executions (ephemeral workflow engine state)
    CREATE TABLE IF NOT EXISTS running_executions (
        workflow_id TEXT PRIMARY KEY,
        current_node_index INTEGER NOT NULL DEFAULT 0,
        status TEXT NOT NULL DEFAULT 'idle',
        started_at TEXT,
        node_outputs TEXT NOT NULL DEFAULT '{}',
        execution_id TEXT
    );

    -- Execution history
    CREATE TABLE IF NOT EXISTS execution_history (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        workflow_id TEXT NOT NULL,
        status TEXT NOT NULL,
        started_at TEXT NOT NULL,
        finished_at TEXT,
        duration_ms INTEGER,
        node_count INTEGER DEFAULT 0,
        completed_node_count INTEGER DEFAULT 0,
        error_message TEXT,
        output TEXT,
        triggered_by TEXT DEFAULT 'manual'
    );

    -- MCP configuration (singleton row)
    CREATE TABLE IF NOT EXISTS mcp_config (
        id INTEGER PRIMARY KEY CHECK (id = 1),
        is_enabled INTEGER NOT NULL DEFAULT 0,
        permission_mode TEXT NOT NULL DEFAULT 'read_only',
        allowed_tools TEXT
    );

    -- MCP request logs
    CREATE TABLE IF NOT EXISTS mcp_logs (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        timestamp TEXT NOT NULL DEFAULT (datetime('now')),
        tool_name TEXT NOT NULL,
        input TEXT,
        output TEXT,
        duration_ms INTEGER,
        status TEXT NOT NULL DEFAULT 'success'
    );

    -- Notifications
    CREATE TABLE IF NOT EXISTS notifications (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        title TEXT NOT NULL,
        message TEXT NOT NULL,
        type TEXT NOT NULL DEFAULT 'info',
        is_read INTEGER NOT NULL DEFAULT 0,
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        related_workflow_id TEXT,
        related_execution_id TEXT
    );

    -- Spec index (cache — file on disk is source of truth)
    CREATE TABLE IF NOT EXISTS specs (
        id TEXT PRIMARY KEY,
        schema_id TEXT NOT NULL,
        title TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'draft',
        workflow_id TEXT,
        workflow_phase TEXT,
        file_path TEXT NOT NULL,
        fields_json TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );

    -- Schema registry
    CREATE TABLE IF NOT EXISTS schemas (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        display_name TEXT,
        file_path TEXT NOT NULL,
        fields_json TEXT,
        updated_at TEXT NOT NULL
    );

    -- Workflow instances (per-spec execution state)
    CREATE TABLE IF NOT EXISTS workflow_instances (
        id TEXT PRIMARY KEY,
        spec_id TEXT NOT NULL REFERENCES specs(id),
        workflow_id TEXT NOT NULL,
        current_phase TEXT NOT NULL,
        started_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );

    -- Phase transition history
    CREATE TABLE IF NOT EXISTS phase_history (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        instance_id TEXT NOT NULL REFERENCES workflow_instances(id),
        from_phase TEXT,
        to_phase TEXT NOT NULL,
        gate_result TEXT,
        action_results TEXT,
        transitioned_at TEXT NOT NULL
    );

    -- Spec reviews
    CREATE TABLE IF NOT EXISTS spec_reviews (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        spec_id TEXT NOT NULL REFERENCES specs(id),
        phase TEXT NOT NULL,
        reviewer TEXT NOT NULL,
        approved INTEGER NOT NULL,
        comment TEXT,
        created_at TEXT NOT NULL
    );

    -- Agent run tracking
    CREATE TABLE IF NOT EXISTS agent_runs (
        id TEXT PRIMARY KEY,
        spec_id TEXT NOT NULL REFERENCES specs(id),
        phase TEXT NOT NULL,
        prompt TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'pending',
        pid INTEGER,
        started_at TEXT NOT NULL,
        finished_at TEXT,
        error TEXT
    );

    ---------------------------------------------------------------
    -- Indexes
    ---------------------------------------------------------------
    CREATE INDEX IF NOT EXISTS idx_specs_schema ON specs(schema_id);
    CREATE INDEX IF NOT EXISTS idx_specs_status ON specs(status);
    CREATE INDEX IF NOT EXISTS idx_specs_workflow_phase ON specs(workflow_phase);
    CREATE INDEX IF NOT EXISTS idx_workflow_instances_spec ON workflow_instances(spec_id);
    CREATE INDEX IF NOT EXISTS idx_phase_history_instance ON phase_history(instance_id);
    CREATE INDEX IF NOT EXISTS idx_spec_reviews_spec ON spec_reviews(spec_id);
    CREATE INDEX IF NOT EXISTS idx_agent_runs_spec ON agent_runs(spec_id);
    CREATE INDEX IF NOT EXISTS idx_agent_runs_status ON agent_runs(status);
    CREATE INDEX IF NOT EXISTS idx_execution_history_workflow ON execution_history(workflow_id);
    CREATE INDEX IF NOT EXISTS idx_notifications_read ON notifications(is_read);
"#;

/// Run all pending migrations using the Database wrapper.
pub fn migrate(db: &super::database::Database) -> Result<(), String> {
    db.with_connection(|conn| run_migrations(conn))
}

/// Ensure the schema is at the current version.
///
/// * Fresh database  → create all tables, stamp version 1.
/// * Version 1       → nothing to do.
/// * Any other version → drop everything, recreate (no real users yet).
pub fn run_migrations(conn: &Connection) -> Result<(), String> {
    // Ensure schema_version table exists so we can read the current version.
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL)",
        [],
    )
    .map_err(|e| format!("Failed to create schema_version table: {}", e))?;

    let current_version: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if current_version == CURRENT_VERSION {
        return Ok(()); // Already up-to-date
    }

    // Any version other than 0 (fresh) or CURRENT_VERSION means stale data.
    // Since no real users exist, nuke and recreate.
    if current_version != 0 {
        log::info!(
            "Schema version {} != {}; dropping and recreating all tables",
            current_version,
            CURRENT_VERSION,
        );
        drop_all_tables(conn)?;
    }

    log::info!("Creating SpecForge v1 schema");
    conn.execute_batch(SCHEMA_V1)
        .map_err(|e| format!("Failed to create schema: {}", e))?;

    // Stamp the version
    conn.execute(
        "INSERT INTO schema_version (version) VALUES (?1)",
        params![CURRENT_VERSION],
    )
    .map_err(|e| format!("Failed to record schema version: {}", e))?;

    log::info!("Schema v{} ready", CURRENT_VERSION);
    Ok(())
}

/// Get the current schema version (0 if no version row exists).
pub fn get_version(conn: &Connection) -> Result<i32, String> {
    conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )
    .map_err(|e| format!("Failed to get schema version: {}", e))
}

/// Check whether a table exists in the database.
pub fn table_exists(conn: &Connection, table_name: &str) -> Result<bool, String> {
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            params![table_name],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to check table existence: {}", e))?;
    Ok(count > 0)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Drop every user table (and index) so we can recreate from scratch.
fn drop_all_tables(conn: &Connection) -> Result<(), String> {
    let tables: Vec<String> = {
        let mut stmt = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type='table' \
                 AND name NOT LIKE 'sqlite_%'",
            )
            .map_err(|e| format!("Failed to list tables: {}", e))?;

        let rows = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| format!("Failed to query tables: {}", e))?;

        rows.filter_map(|r| r.ok()).collect()
    };

    for table in &tables {
        conn.execute_batch(&format!("DROP TABLE IF EXISTS [{}]", table))
            .map_err(|e| format!("Failed to drop table {}: {}", table, e))?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fresh_schema() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        assert_eq!(get_version(&conn).unwrap(), CURRENT_VERSION);

        // Core tables
        assert!(table_exists(&conn, "settings").unwrap());
        assert!(table_exists(&conn, "schema_version").unwrap());
        assert!(table_exists(&conn, "workflows").unwrap());
        assert!(table_exists(&conn, "running_executions").unwrap());
        assert!(table_exists(&conn, "execution_history").unwrap());
        assert!(table_exists(&conn, "mcp_config").unwrap());
        assert!(table_exists(&conn, "mcp_logs").unwrap());
        assert!(table_exists(&conn, "notifications").unwrap());

        // SpecForge tables
        assert!(table_exists(&conn, "specs").unwrap());
        assert!(table_exists(&conn, "schemas").unwrap());
        assert!(table_exists(&conn, "workflow_instances").unwrap());
        assert!(table_exists(&conn, "phase_history").unwrap());
        assert!(table_exists(&conn, "spec_reviews").unwrap());
        assert!(table_exists(&conn, "agent_runs").unwrap());

        // Removed tables should NOT exist
        assert!(!table_exists(&conn, "projects").unwrap());
        assert!(!table_exists(&conn, "ai_providers").unwrap());
        assert!(!table_exists(&conn, "deploy_accounts").unwrap());
    }

    #[test]
    fn test_idempotent_migrations() {
        let conn = Connection::open_in_memory().unwrap();

        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();

        assert_eq!(get_version(&conn).unwrap(), CURRENT_VERSION);
    }

    #[test]
    fn test_stale_version_recreates() {
        let conn = Connection::open_in_memory().unwrap();

        // Simulate a stale v99 database
        conn.execute_batch("CREATE TABLE schema_version (version INTEGER NOT NULL)")
            .unwrap();
        conn.execute("INSERT INTO schema_version (version) VALUES (99)", [])
            .unwrap();
        conn.execute_batch("CREATE TABLE projects (id TEXT PRIMARY KEY)")
            .unwrap();

        // Migration should drop and recreate
        run_migrations(&conn).unwrap();

        assert_eq!(get_version(&conn).unwrap(), CURRENT_VERSION);
        assert!(!table_exists(&conn, "projects").unwrap());
        assert!(table_exists(&conn, "specs").unwrap());
    }
}
