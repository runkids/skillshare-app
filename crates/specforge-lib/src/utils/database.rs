// SQLite Database Connection Management
// Provides thread-safe database access for both Tauri App and MCP Server

use rusqlite::{Connection, Result as SqliteResult};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use super::schema;
use super::shared_store::APP_IDENTIFIER;

/// Database file name
#[cfg(debug_assertions)]
pub const DATABASE_FILE: &str = "specforge-dev.db";

#[cfg(not(debug_assertions))]
pub const DATABASE_FILE: &str = "specforge.db";

/// Thread-safe database wrapper
/// Uses Arc<Mutex<Connection>> for concurrent access from multiple threads
#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    path: PathBuf,
}

impl Database {
    /// Create a new database connection
    /// Automatically enables WAL mode and runs migrations
    pub fn new(path: PathBuf) -> Result<Self, String> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create database directory: {}", e))?;
        }

        let conn = Connection::open(&path)
            .map_err(|e| format!("Failed to open database: {}", e))?;

        // Configure SQLite for optimal concurrent access
        conn.execute_batch(
            r#"
            PRAGMA journal_mode=WAL;
            PRAGMA busy_timeout=5000;
            PRAGMA synchronous=NORMAL;
            PRAGMA foreign_keys=ON;
            PRAGMA cache_size=-64000;
            "#,
        )
        .map_err(|e| format!("Failed to configure database: {}", e))?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            path,
        };

        // Run migrations
        db.run_migrations()?;

        Ok(db)
    }

    /// Get database file path
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Get a lock on the connection for executing queries
    pub fn lock(&self) -> Result<MutexGuard<'_, Connection>, String> {
        self.conn
            .lock()
            .map_err(|e| format!("Failed to acquire database lock: {}", e))
    }

    /// Run all pending migrations
    fn run_migrations(&self) -> Result<(), String> {
        let conn = self.lock()?;
        schema::run_migrations(&conn)
    }

    /// Execute a function with the database connection
    /// The closure should return Result<T, String> with errors already converted
    pub fn with_connection<T, F>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&Connection) -> Result<T, String>,
    {
        let conn = self.lock()?;
        f(&conn)
    }

    /// Execute a function with the database connection (raw SQLite result)
    /// For operations that want to use rusqlite's error type directly
    pub fn with_connection_raw<T, F>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&Connection) -> SqliteResult<T>,
    {
        let conn = self.lock()?;
        f(&conn).map_err(|e| format!("Database error: {}", e))
    }

    /// Execute a function with a transaction
    /// Automatically commits on success, rolls back on error
    pub fn with_transaction<T, F>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&Connection) -> Result<T, String>,
    {
        let mut conn = self.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("Failed to start transaction: {}", e))?;

        match f(&tx) {
            Ok(result) => {
                tx.commit()
                    .map_err(|e| format!("Failed to commit transaction: {}", e))?;
                Ok(result)
            }
            Err(e) => {
                // Transaction will automatically rollback when dropped
                Err(e)
            }
        }
    }

    /// Check if the database file exists
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Get the current schema version
    pub fn schema_version(&self) -> Result<i32, String> {
        self.with_connection_raw(|conn| {
            conn.query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0),
            )
        })
        .or_else(|_| Ok(0))
    }
}

/// Get the default database path
pub fn get_database_path() -> Result<PathBuf, String> {
    dirs::data_dir()
        .map(|p| p.join(APP_IDENTIFIER).join(DATABASE_FILE))
        .ok_or_else(|| "Could not determine application data directory".to_string())
}

/// Open the default database
pub fn open_default_database() -> Result<Database, String> {
    let path = get_database_path()?;
    Database::new(path)
}

/// Database instance for MCP Server (standalone binary)
/// Uses the same path as the Tauri app for shared access
pub fn open_mcp_database() -> Result<Database, String> {
    open_default_database()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_database_creation() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::new(path.clone()).unwrap();

        assert!(path.exists());
        assert!(db.schema_version().unwrap() >= 0);
    }

    #[test]
    fn test_wal_mode() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::new(path).unwrap();

        let mode: String = db
            .with_connection_raw(|conn| conn.query_row("PRAGMA journal_mode", [], |row| row.get(0)))
            .unwrap();

        assert_eq!(mode.to_lowercase(), "wal");
    }

    #[test]
    fn test_transaction() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::new(path).unwrap();

        // Transaction that succeeds
        let result = db.with_transaction(|conn| {
            conn.execute(
                "CREATE TABLE test (id INTEGER PRIMARY KEY)",
                [],
            )
            .map_err(|e| e.to_string())?;
            Ok(42)
        });
        assert_eq!(result.unwrap(), 42);

        // Verify table exists
        let exists: i32 = db
            .with_connection_raw(|conn| {
                conn.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='test'",
                    [],
                    |row| row.get(0),
                )
            })
            .unwrap();
        assert_eq!(exists, 1);
    }
}
