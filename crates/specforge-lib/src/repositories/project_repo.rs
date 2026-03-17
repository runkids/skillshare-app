// Project Repository
// Handles all database operations for projects

use rusqlite::params;
use std::collections::HashMap;

use crate::models::{PackageManager, Project};
use crate::utils::database::Database;

/// Repository for project data access
pub struct ProjectRepository {
    db: Database,
}

impl ProjectRepository {
    /// Create a new ProjectRepository
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// List all projects
    pub fn list(&self) -> Result<Vec<Project>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, name, path, version, description, is_monorepo,
                           package_manager, scripts, worktree_sessions,
                           created_at, last_opened_at,
                           monorepo_tool, framework, ui_framework
                    FROM projects
                    ORDER BY last_opened_at DESC
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(ProjectRow {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        path: row.get(2)?,
                        version: row.get(3)?,
                        description: row.get(4)?,
                        is_monorepo: row.get(5)?,
                        package_manager: row.get(6)?,
                        scripts: row.get(7)?,
                        worktree_sessions: row.get(8)?,
                        created_at: row.get(9)?,
                        last_opened_at: row.get(10)?,
                        monorepo_tool: row.get(11)?,
                        framework: row.get(12)?,
                        ui_framework: row.get(13)?,
                    })
                })
                .map_err(|e| format!("Failed to query projects: {}", e))?;

            let mut projects = Vec::new();
            for row in rows {
                let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                projects.push(row.into_project()?);
            }

            Ok(projects)
        })
    }

    /// Get a project by ID
    pub fn get(&self, id: &str) -> Result<Option<Project>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, name, path, version, description, is_monorepo,
                           package_manager, scripts, worktree_sessions,
                           created_at, last_opened_at,
                           monorepo_tool, framework, ui_framework
                    FROM projects
                    WHERE id = ?1
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let result = stmt
                .query_row(params![id], |row| {
                    Ok(ProjectRow {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        path: row.get(2)?,
                        version: row.get(3)?,
                        description: row.get(4)?,
                        is_monorepo: row.get(5)?,
                        package_manager: row.get(6)?,
                        scripts: row.get(7)?,
                        worktree_sessions: row.get(8)?,
                        created_at: row.get(9)?,
                        last_opened_at: row.get(10)?,
                        monorepo_tool: row.get(11)?,
                        framework: row.get(12)?,
                        ui_framework: row.get(13)?,
                    })
                });

            match result {
                Ok(row) => Ok(Some(row.into_project()?)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to query project: {}", e)),
            }
        })
    }

    /// Get a project by path
    pub fn get_by_path(&self, path: &str) -> Result<Option<Project>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, name, path, version, description, is_monorepo,
                           package_manager, scripts, worktree_sessions,
                           created_at, last_opened_at,
                           monorepo_tool, framework, ui_framework
                    FROM projects
                    WHERE path = ?1
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let result = stmt
                .query_row(params![path], |row| {
                    Ok(ProjectRow {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        path: row.get(2)?,
                        version: row.get(3)?,
                        description: row.get(4)?,
                        is_monorepo: row.get(5)?,
                        package_manager: row.get(6)?,
                        scripts: row.get(7)?,
                        worktree_sessions: row.get(8)?,
                        created_at: row.get(9)?,
                        last_opened_at: row.get(10)?,
                        monorepo_tool: row.get(11)?,
                        framework: row.get(12)?,
                        ui_framework: row.get(13)?,
                    })
                });

            match result {
                Ok(row) => Ok(Some(row.into_project()?)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to query project: {}", e)),
            }
        })
    }

    /// Save a project (insert or update)
    /// IMPORTANT: Uses ON CONFLICT DO UPDATE instead of INSERT OR REPLACE
    /// to avoid triggering ON DELETE CASCADE on dependent tables like deployment_configs.
    /// INSERT OR REPLACE internally does DELETE + INSERT which triggers cascades.
    pub fn save(&self, project: &Project) -> Result<(), String> {
        let scripts_json = serde_json::to_string(&project.scripts)
            .map_err(|e| format!("Failed to serialize scripts: {}", e))?;

        let worktree_sessions_json = serde_json::to_string(&project.worktree_sessions)
            .map_err(|e| format!("Failed to serialize worktree_sessions: {}", e))?;

        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO projects
                (id, name, path, version, description, is_monorepo, package_manager,
                 scripts, worktree_sessions, created_at, last_opened_at,
                 monorepo_tool, framework, ui_framework)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    path = excluded.path,
                    version = excluded.version,
                    description = excluded.description,
                    is_monorepo = excluded.is_monorepo,
                    package_manager = excluded.package_manager,
                    scripts = excluded.scripts,
                    worktree_sessions = excluded.worktree_sessions,
                    last_opened_at = excluded.last_opened_at,
                    monorepo_tool = excluded.monorepo_tool,
                    framework = excluded.framework,
                    ui_framework = excluded.ui_framework
                "#,
                params![
                    project.id,
                    project.name,
                    project.path,
                    project.version,
                    project.description,
                    project.is_monorepo as i32,
                    format!("{:?}", project.package_manager).to_lowercase(),
                    scripts_json,
                    worktree_sessions_json,
                    project.created_at,
                    project.last_opened_at,
                    project.monorepo_tool,
                    project.framework,
                    project.ui_framework,
                ],
            )
            .map_err(|e| format!("Failed to save project: {}", e))?;

            Ok(())
        })
    }

    /// Delete a project by ID
    pub fn delete(&self, id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let rows_affected = conn
                .execute("DELETE FROM projects WHERE id = ?1", params![id])
                .map_err(|e| format!("Failed to delete project: {}", e))?;

            Ok(rows_affected > 0)
        })
    }

    /// Update last opened time
    pub fn update_last_opened(&self, id: &str, timestamp: &str) -> Result<(), String> {
        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE projects SET last_opened_at = ?1 WHERE id = ?2",
                params![timestamp, id],
            )
            .map_err(|e| format!("Failed to update last_opened_at: {}", e))?;

            Ok(())
        })
    }

    /// Check if a project with the given path exists
    pub fn exists_by_path(&self, path: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let count: i32 = conn
                .query_row(
                    "SELECT COUNT(*) FROM projects WHERE path = ?1",
                    params![path],
                    |row| row.get(0),
                )
                .map_err(|e| format!("Failed to check project existence: {}", e))?;

            Ok(count > 0)
        })
    }
}

/// Internal row structure for mapping database rows
struct ProjectRow {
    id: String,
    name: String,
    path: String,
    version: String,
    description: Option<String>,
    is_monorepo: i32,
    package_manager: String,
    scripts: Option<String>,
    worktree_sessions: Option<String>,
    created_at: String,
    last_opened_at: String,
    monorepo_tool: Option<String>,
    framework: Option<String>,
    ui_framework: Option<String>,
}

impl ProjectRow {
    fn into_project(self) -> Result<Project, String> {
        let scripts: HashMap<String, String> = if let Some(json) = &self.scripts {
            serde_json::from_str(json).unwrap_or_default()
        } else {
            HashMap::new()
        };

        let worktree_sessions = if let Some(json) = &self.worktree_sessions {
            serde_json::from_str(json).unwrap_or_default()
        } else {
            Vec::new()
        };

        let package_manager = match self.package_manager.as_str() {
            "npm" => PackageManager::Npm,
            "yarn" => PackageManager::Yarn,
            "pnpm" => PackageManager::Pnpm,
            "bun" => PackageManager::Bun,
            _ => PackageManager::Unknown,
        };

        Ok(Project {
            id: self.id,
            name: self.name,
            path: self.path,
            version: self.version,
            description: self.description,
            is_monorepo: self.is_monorepo != 0,
            monorepo_tool: self.monorepo_tool,
            framework: self.framework,
            ui_framework: self.ui_framework,
            package_manager,
            scripts,
            worktree_sessions,
            created_at: self.created_at,
            last_opened_at: self.last_opened_at,
        })
    }
}
