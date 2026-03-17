// Snapshot Repository
// Handles all database operations for execution snapshots and security insights

use rusqlite::params;

use crate::models::security_insight::{InsightSeverity, InsightType, InsightSummary, SecurityInsight};
use crate::models::snapshot::{
    ExecutionSnapshot, LockfileState, LockfileType, SnapshotDependency, SnapshotDiff,
    SnapshotFilter, SnapshotListItem, SnapshotStatus, SnapshotWithDependencies,
    TimeMachineSettings, TriggerSource,
};
use crate::utils::database::Database;

/// Repository for snapshot data access
pub struct SnapshotRepository {
    db: Database,
}

impl SnapshotRepository {
    /// Create a new SnapshotRepository
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    // =========================================================================
    // Execution Snapshots
    // =========================================================================

    /// Create a new execution snapshot
    pub fn create_snapshot(&self, snapshot: &ExecutionSnapshot) -> Result<(), String> {
        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO execution_snapshots (
                    id, project_path, status, trigger_source,
                    lockfile_type, lockfile_hash, dependency_tree_hash, package_json_hash,
                    total_dependencies, direct_dependencies, dev_dependencies,
                    security_score, postinstall_count, storage_path, compressed_size,
                    error_message, created_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
                "#,
                params![
                    snapshot.id,
                    snapshot.project_path,
                    snapshot.status.as_str(),
                    snapshot.trigger_source.as_str(),
                    snapshot.lockfile_type.as_ref().map(|t| t.as_str()),
                    snapshot.lockfile_hash,
                    snapshot.dependency_tree_hash,
                    snapshot.package_json_hash,
                    snapshot.total_dependencies,
                    snapshot.direct_dependencies,
                    snapshot.dev_dependencies,
                    snapshot.security_score,
                    snapshot.postinstall_count,
                    snapshot.storage_path,
                    snapshot.compressed_size,
                    snapshot.error_message,
                    snapshot.created_at,
                ],
            )
            .map_err(|e| format!("Failed to create snapshot: {}", e))?;

            Ok(())
        })
    }

    /// Update an existing snapshot
    pub fn update_snapshot(&self, snapshot: &ExecutionSnapshot) -> Result<(), String> {
        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                UPDATE execution_snapshots SET
                    status = ?2,
                    lockfile_type = ?3,
                    lockfile_hash = ?4,
                    dependency_tree_hash = ?5,
                    package_json_hash = ?6,
                    total_dependencies = ?7,
                    direct_dependencies = ?8,
                    dev_dependencies = ?9,
                    security_score = ?10,
                    postinstall_count = ?11,
                    storage_path = ?12,
                    compressed_size = ?13,
                    error_message = ?14
                WHERE id = ?1
                "#,
                params![
                    snapshot.id,
                    snapshot.status.as_str(),
                    snapshot.lockfile_type.as_ref().map(|t| t.as_str()),
                    snapshot.lockfile_hash,
                    snapshot.dependency_tree_hash,
                    snapshot.package_json_hash,
                    snapshot.total_dependencies,
                    snapshot.direct_dependencies,
                    snapshot.dev_dependencies,
                    snapshot.security_score,
                    snapshot.postinstall_count,
                    snapshot.storage_path,
                    snapshot.compressed_size,
                    snapshot.error_message,
                ],
            )
            .map_err(|e| format!("Failed to update snapshot: {}", e))?;

            Ok(())
        })
    }

    /// Get a snapshot by ID
    pub fn get_snapshot(&self, id: &str) -> Result<Option<ExecutionSnapshot>, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT id, project_path, status, trigger_source,
                       lockfile_type, lockfile_hash, dependency_tree_hash, package_json_hash,
                       total_dependencies, direct_dependencies, dev_dependencies,
                       security_score, postinstall_count, storage_path, compressed_size,
                       error_message, created_at
                FROM execution_snapshots
                WHERE id = ?1
                "#,
                params![id],
                |row| {
                    Ok(SnapshotRow {
                        id: row.get(0)?,
                        project_path: row.get(1)?,
                        status: row.get(2)?,
                        trigger_source: row.get(3)?,
                        lockfile_type: row.get(4)?,
                        lockfile_hash: row.get(5)?,
                        dependency_tree_hash: row.get(6)?,
                        package_json_hash: row.get(7)?,
                        total_dependencies: row.get(8)?,
                        direct_dependencies: row.get(9)?,
                        dev_dependencies: row.get(10)?,
                        security_score: row.get(11)?,
                        postinstall_count: row.get(12)?,
                        storage_path: row.get(13)?,
                        compressed_size: row.get(14)?,
                        error_message: row.get(15)?,
                        created_at: row.get(16)?,
                    })
                },
            );

            match result {
                Ok(row) => Ok(Some(row.into_snapshot())),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get snapshot: {}", e)),
            }
        })
    }

    /// Get a snapshot with all its dependencies
    pub fn get_snapshot_with_dependencies(
        &self,
        id: &str,
    ) -> Result<Option<SnapshotWithDependencies>, String> {
        let snapshot = self.get_snapshot(id)?;
        match snapshot {
            Some(snapshot) => {
                let dependencies = self.list_dependencies(&snapshot.id)?;
                Ok(Some(SnapshotWithDependencies {
                    snapshot,
                    dependencies,
                }))
            }
            None => Ok(None),
        }
    }

    /// List snapshots with optional filters
    pub fn list_snapshots(&self, filter: &SnapshotFilter) -> Result<Vec<SnapshotListItem>, String> {
        self.db.with_connection(|conn| {
            let mut sql = String::from(
                r#"
                SELECT id, project_path, status, trigger_source, lockfile_type,
                       total_dependencies, security_score, postinstall_count, created_at
                FROM execution_snapshots
                WHERE 1=1
                "#,
            );

            let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(ref project_path) = filter.project_path {
                sql.push_str(" AND project_path = ?");
                params_vec.push(Box::new(project_path.clone()));
            }

            if let Some(ref trigger_source) = filter.trigger_source {
                sql.push_str(" AND trigger_source = ?");
                params_vec.push(Box::new(trigger_source.as_str().to_string()));
            }

            if let Some(ref status) = filter.status {
                sql.push_str(" AND status = ?");
                params_vec.push(Box::new(status.as_str().to_string()));
            }

            if let Some(ref from_date) = filter.from_date {
                sql.push_str(" AND created_at >= ?");
                params_vec.push(Box::new(from_date.clone()));
            }

            if let Some(ref to_date) = filter.to_date {
                sql.push_str(" AND created_at <= ?");
                params_vec.push(Box::new(to_date.clone()));
            }

            sql.push_str(" ORDER BY created_at DESC");

            if let Some(limit) = filter.limit {
                sql.push_str(&format!(" LIMIT {}", limit));
            }

            if let Some(offset) = filter.offset {
                sql.push_str(&format!(" OFFSET {}", offset));
            }

            let params_refs: Vec<&dyn rusqlite::ToSql> =
                params_vec.iter().map(|p| p.as_ref()).collect();

            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map(params_refs.as_slice(), |row| {
                    let status_str: String = row.get(2)?;
                    let trigger_source_str: String = row.get(3)?;
                    let lockfile_type_str: Option<String> = row.get(4)?;

                    Ok(SnapshotListItem {
                        id: row.get(0)?,
                        project_path: row.get(1)?,
                        status: SnapshotStatus::from_str(&status_str)
                            .unwrap_or(SnapshotStatus::Failed),
                        trigger_source: TriggerSource::from_str(&trigger_source_str)
                            .unwrap_or(TriggerSource::Manual),
                        lockfile_type: lockfile_type_str.and_then(|s| LockfileType::from_str(&s)),
                        total_dependencies: row.get(5)?,
                        security_score: row.get(6)?,
                        postinstall_count: row.get(7)?,
                        created_at: row.get(8)?,
                    })
                })
                .map_err(|e| format!("Failed to query snapshots: {}", e))?;

            let mut snapshots = Vec::new();
            for row in rows {
                snapshots.push(row.map_err(|e| format!("Failed to read row: {}", e))?);
            }

            Ok(snapshots)
        })
    }

    /// Get the latest snapshot for a project
    pub fn get_latest_snapshot(&self, project_path: &str) -> Result<Option<ExecutionSnapshot>, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT id, project_path, status, trigger_source,
                       lockfile_type, lockfile_hash, dependency_tree_hash, package_json_hash,
                       total_dependencies, direct_dependencies, dev_dependencies,
                       security_score, postinstall_count, storage_path, compressed_size,
                       error_message, created_at
                FROM execution_snapshots
                WHERE project_path = ?1 AND status = 'completed'
                ORDER BY created_at DESC
                LIMIT 1
                "#,
                params![project_path],
                |row| {
                    Ok(SnapshotRow {
                        id: row.get(0)?,
                        project_path: row.get(1)?,
                        status: row.get(2)?,
                        trigger_source: row.get(3)?,
                        lockfile_type: row.get(4)?,
                        lockfile_hash: row.get(5)?,
                        dependency_tree_hash: row.get(6)?,
                        package_json_hash: row.get(7)?,
                        total_dependencies: row.get(8)?,
                        direct_dependencies: row.get(9)?,
                        dev_dependencies: row.get(10)?,
                        security_score: row.get(11)?,
                        postinstall_count: row.get(12)?,
                        storage_path: row.get(13)?,
                        compressed_size: row.get(14)?,
                        error_message: row.get(15)?,
                        created_at: row.get(16)?,
                    })
                },
            );

            match result {
                Ok(row) => Ok(Some(row.into_snapshot())),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get latest snapshot: {}", e)),
            }
        })
    }

    /// Delete a snapshot by ID
    pub fn delete_snapshot(&self, id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let rows_affected = conn
                .execute("DELETE FROM execution_snapshots WHERE id = ?1", params![id])
                .map_err(|e| format!("Failed to delete snapshot: {}", e))?;

            Ok(rows_affected > 0)
        })
    }

    /// Delete old snapshots (keep last N per project)
    pub fn prune_snapshots(&self, keep_per_project: usize) -> Result<usize, String> {
        self.db.with_connection(|conn| {
            let rows_affected = conn
                .execute(
                    r#"
                    DELETE FROM execution_snapshots
                    WHERE id NOT IN (
                        SELECT id FROM (
                            SELECT id, ROW_NUMBER() OVER (
                                PARTITION BY project_path
                                ORDER BY created_at DESC
                            ) as rn
                            FROM execution_snapshots
                        )
                        WHERE rn <= ?1
                    )
                    "#,
                    params![keep_per_project as i64],
                )
                .map_err(|e| format!("Failed to prune snapshots: {}", e))?;

            Ok(rows_affected)
        })
    }

    // =========================================================================
    // Snapshot Dependencies
    // =========================================================================

    /// Add dependencies to a snapshot
    pub fn add_dependencies(&self, dependencies: &[SnapshotDependency]) -> Result<(), String> {
        if dependencies.is_empty() {
            return Ok(());
        }

        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    INSERT INTO snapshot_dependencies (
                        snapshot_id, name, version, is_direct, is_dev,
                        has_postinstall, postinstall_script, integrity_hash, resolved_url
                    )
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            for dep in dependencies {
                stmt.execute(params![
                    dep.snapshot_id,
                    dep.name,
                    dep.version,
                    dep.is_direct as i32,
                    dep.is_dev as i32,
                    dep.has_postinstall as i32,
                    dep.postinstall_script,
                    dep.integrity_hash,
                    dep.resolved_url,
                ])
                .map_err(|e| format!("Failed to insert dependency: {}", e))?;
            }

            Ok(())
        })
    }

    /// List dependencies for a snapshot
    pub fn list_dependencies(&self, snapshot_id: &str) -> Result<Vec<SnapshotDependency>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, snapshot_id, name, version, is_direct, is_dev,
                           has_postinstall, postinstall_script, integrity_hash, resolved_url
                    FROM snapshot_dependencies
                    WHERE snapshot_id = ?1
                    ORDER BY name
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map(params![snapshot_id], |row| {
                    Ok(SnapshotDependency {
                        id: row.get(0)?,
                        snapshot_id: row.get(1)?,
                        name: row.get(2)?,
                        version: row.get(3)?,
                        is_direct: row.get::<_, i32>(4)? != 0,
                        is_dev: row.get::<_, i32>(5)? != 0,
                        has_postinstall: row.get::<_, i32>(6)? != 0,
                        postinstall_script: row.get(7)?,
                        integrity_hash: row.get(8)?,
                        resolved_url: row.get(9)?,
                    })
                })
                .map_err(|e| format!("Failed to query dependencies: {}", e))?;

            let mut dependencies = Vec::new();
            for row in rows {
                dependencies.push(row.map_err(|e| format!("Failed to read row: {}", e))?);
            }

            Ok(dependencies)
        })
    }

    /// List dependencies with postinstall scripts
    pub fn list_postinstall_dependencies(
        &self,
        snapshot_id: &str,
    ) -> Result<Vec<SnapshotDependency>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, snapshot_id, name, version, is_direct, is_dev,
                           has_postinstall, postinstall_script, integrity_hash, resolved_url
                    FROM snapshot_dependencies
                    WHERE snapshot_id = ?1 AND has_postinstall = 1
                    ORDER BY name
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map(params![snapshot_id], |row| {
                    Ok(SnapshotDependency {
                        id: row.get(0)?,
                        snapshot_id: row.get(1)?,
                        name: row.get(2)?,
                        version: row.get(3)?,
                        is_direct: row.get::<_, i32>(4)? != 0,
                        is_dev: row.get::<_, i32>(5)? != 0,
                        has_postinstall: row.get::<_, i32>(6)? != 0,
                        postinstall_script: row.get(7)?,
                        integrity_hash: row.get(8)?,
                        resolved_url: row.get(9)?,
                    })
                })
                .map_err(|e| format!("Failed to query postinstall dependencies: {}", e))?;

            let mut dependencies = Vec::new();
            for row in rows {
                dependencies.push(row.map_err(|e| format!("Failed to read row: {}", e))?);
            }

            Ok(dependencies)
        })
    }

    /// Search dependencies using FTS
    pub fn search_dependencies(
        &self,
        snapshot_id: &str,
        query: &str,
    ) -> Result<Vec<SnapshotDependency>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT d.id, d.snapshot_id, d.name, d.version, d.is_direct, d.is_dev,
                           d.has_postinstall, d.postinstall_script, d.integrity_hash, d.resolved_url
                    FROM snapshot_dependencies d
                    JOIN snapshot_dependencies_fts fts ON d.id = fts.rowid
                    WHERE d.snapshot_id = ?1 AND snapshot_dependencies_fts MATCH ?2
                    ORDER BY rank
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map(params![snapshot_id, query], |row| {
                    Ok(SnapshotDependency {
                        id: row.get(0)?,
                        snapshot_id: row.get(1)?,
                        name: row.get(2)?,
                        version: row.get(3)?,
                        is_direct: row.get::<_, i32>(4)? != 0,
                        is_dev: row.get::<_, i32>(5)? != 0,
                        has_postinstall: row.get::<_, i32>(6)? != 0,
                        postinstall_script: row.get(7)?,
                        integrity_hash: row.get(8)?,
                        resolved_url: row.get(9)?,
                    })
                })
                .map_err(|e| format!("Failed to search dependencies: {}", e))?;

            let mut dependencies = Vec::new();
            for row in rows {
                dependencies.push(row.map_err(|e| format!("Failed to read row: {}", e))?);
            }

            Ok(dependencies)
        })
    }

    // =========================================================================
    // Security Insights
    // =========================================================================

    /// Create a security insight
    pub fn create_insight(&self, insight: &SecurityInsight) -> Result<(), String> {
        let metadata_json = insight
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(m).unwrap_or_default());

        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO security_insights (
                    id, snapshot_id, insight_type, severity, title, description,
                    package_name, previous_value, current_value, recommendation,
                    metadata, is_dismissed, created_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                "#,
                params![
                    insight.id,
                    insight.snapshot_id,
                    insight.insight_type.as_str(),
                    insight.severity.as_str(),
                    insight.title,
                    insight.description,
                    insight.package_name,
                    insight.previous_value,
                    insight.current_value,
                    insight.recommendation,
                    metadata_json,
                    insight.is_dismissed as i32,
                    insight.created_at,
                ],
            )
            .map_err(|e| format!("Failed to create insight: {}", e))?;

            Ok(())
        })
    }

    /// List insights for a snapshot
    pub fn list_insights(&self, snapshot_id: &str) -> Result<Vec<SecurityInsight>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, snapshot_id, insight_type, severity, title, description,
                           package_name, previous_value, current_value, recommendation,
                           metadata, is_dismissed, created_at
                    FROM security_insights
                    WHERE snapshot_id = ?1
                    ORDER BY
                        CASE severity
                            WHEN 'critical' THEN 1
                            WHEN 'high' THEN 2
                            WHEN 'medium' THEN 3
                            WHEN 'low' THEN 4
                            WHEN 'info' THEN 5
                        END,
                        created_at DESC
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map(params![snapshot_id], |row| {
                    let insight_type_str: String = row.get(2)?;
                    let severity_str: String = row.get(3)?;
                    let metadata_str: Option<String> = row.get(10)?;

                    Ok(InsightRow {
                        id: row.get(0)?,
                        snapshot_id: row.get(1)?,
                        insight_type: insight_type_str,
                        severity: severity_str,
                        title: row.get(4)?,
                        description: row.get(5)?,
                        package_name: row.get(6)?,
                        previous_value: row.get(7)?,
                        current_value: row.get(8)?,
                        recommendation: row.get(9)?,
                        metadata: metadata_str,
                        is_dismissed: row.get::<_, i32>(11)? != 0,
                        created_at: row.get(12)?,
                    })
                })
                .map_err(|e| format!("Failed to query insights: {}", e))?;

            let mut insights = Vec::new();
            for row in rows {
                let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                insights.push(row.into_insight());
            }

            Ok(insights)
        })
    }

    /// Get insight summary for a snapshot
    pub fn get_insight_summary(&self, snapshot_id: &str) -> Result<InsightSummary, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT
                        COUNT(*) as total,
                        SUM(CASE WHEN severity = 'critical' THEN 1 ELSE 0 END) as critical,
                        SUM(CASE WHEN severity = 'high' THEN 1 ELSE 0 END) as high,
                        SUM(CASE WHEN severity = 'medium' THEN 1 ELSE 0 END) as medium,
                        SUM(CASE WHEN severity = 'low' THEN 1 ELSE 0 END) as low,
                        SUM(CASE WHEN severity = 'info' THEN 1 ELSE 0 END) as info,
                        SUM(CASE WHEN is_dismissed = 1 THEN 1 ELSE 0 END) as dismissed
                    FROM security_insights
                    WHERE snapshot_id = ?1
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let summary = stmt
                .query_row(params![snapshot_id], |row| {
                    Ok(InsightSummary {
                        total: row.get(0)?,
                        critical: row.get(1)?,
                        high: row.get(2)?,
                        medium: row.get(3)?,
                        low: row.get(4)?,
                        info: row.get(5)?,
                        dismissed: row.get(6)?,
                    })
                })
                .map_err(|e| format!("Failed to get insight summary: {}", e))?;

            Ok(summary)
        })
    }

    /// Dismiss an insight
    pub fn dismiss_insight(&self, id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let rows_affected = conn
                .execute(
                    "UPDATE security_insights SET is_dismissed = 1 WHERE id = ?1",
                    params![id],
                )
                .map_err(|e| format!("Failed to dismiss insight: {}", e))?;

            Ok(rows_affected > 0)
        })
    }

    // =========================================================================
    // Diff Cache
    // =========================================================================

    /// Get cached diff between two snapshots
    pub fn get_cached_diff(
        &self,
        snapshot_a_id: &str,
        snapshot_b_id: &str,
    ) -> Result<Option<SnapshotDiff>, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT diff_data
                FROM snapshot_diff_cache
                WHERE snapshot_a_id = ?1 AND snapshot_b_id = ?2
                "#,
                params![snapshot_a_id, snapshot_b_id],
                |row| {
                    let diff_data: String = row.get(0)?;
                    Ok(diff_data)
                },
            );

            match result {
                Ok(json) => {
                    let diff: SnapshotDiff = serde_json::from_str(&json)
                        .map_err(|e| format!("Failed to parse cached diff: {}", e))?;
                    Ok(Some(diff))
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get cached diff: {}", e)),
            }
        })
    }

    /// Cache a diff between two snapshots
    pub fn cache_diff(&self, diff: &SnapshotDiff) -> Result<(), String> {
        let diff_data = serde_json::to_string(diff)
            .map_err(|e| format!("Failed to serialize diff: {}", e))?;

        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT OR REPLACE INTO snapshot_diff_cache (
                    id, snapshot_a_id, snapshot_b_id, diff_data, created_at
                )
                VALUES (?1, ?2, ?3, ?4, datetime('now'))
                "#,
                params![
                    format!("{}_{}", diff.snapshot_a_id, diff.snapshot_b_id),
                    diff.snapshot_a_id,
                    diff.snapshot_b_id,
                    diff_data,
                ],
            )
            .map_err(|e| format!("Failed to cache diff: {}", e))?;

            Ok(())
        })
    }

    /// Clear old diff cache entries
    pub fn clear_old_cache(&self, days: i32) -> Result<usize, String> {
        self.db.with_connection(|conn| {
            let rows_affected = conn
                .execute(
                    r#"
                    DELETE FROM snapshot_diff_cache
                    WHERE created_at < datetime('now', ?1)
                    "#,
                    params![format!("-{} days", days)],
                )
                .map_err(|e| format!("Failed to clear old cache: {}", e))?;

            Ok(rows_affected)
        })
    }

    // =========================================================================
    // Snapshot by Hash (for deduplication)
    // =========================================================================

    /// Get a snapshot by project path and lockfile hash (to avoid duplicates)
    pub fn get_snapshot_by_hash(
        &self,
        project_path: &str,
        lockfile_hash: &str,
    ) -> Result<Option<ExecutionSnapshot>, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT id, project_path, status, trigger_source,
                       lockfile_type, lockfile_hash, dependency_tree_hash, package_json_hash,
                       total_dependencies, direct_dependencies, dev_dependencies,
                       security_score, postinstall_count, storage_path, compressed_size,
                       error_message, created_at
                FROM execution_snapshots
                WHERE project_path = ?1 AND lockfile_hash = ?2 AND status = 'completed'
                ORDER BY created_at DESC
                LIMIT 1
                "#,
                params![project_path, lockfile_hash],
                |row| {
                    Ok(SnapshotRow {
                        id: row.get(0)?,
                        project_path: row.get(1)?,
                        status: row.get(2)?,
                        trigger_source: row.get(3)?,
                        lockfile_type: row.get(4)?,
                        lockfile_hash: row.get(5)?,
                        dependency_tree_hash: row.get(6)?,
                        package_json_hash: row.get(7)?,
                        total_dependencies: row.get(8)?,
                        direct_dependencies: row.get(9)?,
                        dev_dependencies: row.get(10)?,
                        security_score: row.get(11)?,
                        postinstall_count: row.get(12)?,
                        storage_path: row.get(13)?,
                        compressed_size: row.get(14)?,
                        error_message: row.get(15)?,
                        created_at: row.get(16)?,
                    })
                },
            );

            match result {
                Ok(row) => Ok(Some(row.into_snapshot())),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get snapshot by hash: {}", e)),
            }
        })
    }

    // =========================================================================
    // Lockfile State Management
    // =========================================================================

    /// Get the current lockfile state for a project
    pub fn get_lockfile_state(&self, project_path: &str) -> Result<Option<LockfileState>, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT project_path, lockfile_type, lockfile_hash, last_snapshot_id, updated_at
                FROM project_lockfile_state
                WHERE project_path = ?1
                "#,
                params![project_path],
                |row| {
                    let lockfile_type_str: Option<String> = row.get(1)?;
                    Ok(LockfileState {
                        project_path: row.get(0)?,
                        lockfile_type: lockfile_type_str.and_then(|s| LockfileType::from_str(&s)),
                        lockfile_hash: row.get(2)?,
                        last_snapshot_id: row.get(3)?,
                        updated_at: row.get(4)?,
                    })
                },
            );

            match result {
                Ok(state) => Ok(Some(state)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get lockfile state: {}", e)),
            }
        })
    }

    /// Update or insert lockfile state for a project
    pub fn update_lockfile_state(&self, state: &LockfileState) -> Result<(), String> {
        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO project_lockfile_state (
                    project_path, lockfile_type, lockfile_hash, last_snapshot_id, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(project_path) DO UPDATE SET
                    lockfile_type = excluded.lockfile_type,
                    lockfile_hash = excluded.lockfile_hash,
                    last_snapshot_id = excluded.last_snapshot_id,
                    updated_at = excluded.updated_at
                "#,
                params![
                    state.project_path,
                    state.lockfile_type.as_ref().map(|t| t.as_str()),
                    state.lockfile_hash,
                    state.last_snapshot_id,
                    state.updated_at,
                ],
            )
            .map_err(|e| format!("Failed to update lockfile state: {}", e))?;

            Ok(())
        })
    }

    /// Delete lockfile state for a project
    pub fn delete_lockfile_state(&self, project_path: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let rows_affected = conn
                .execute(
                    "DELETE FROM project_lockfile_state WHERE project_path = ?1",
                    params![project_path],
                )
                .map_err(|e| format!("Failed to delete lockfile state: {}", e))?;

            Ok(rows_affected > 0)
        })
    }

    // =========================================================================
    // Time Machine Settings
    // =========================================================================

    /// Get Time Machine settings
    pub fn get_time_machine_settings(&self) -> Result<TimeMachineSettings, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT auto_watch_enabled, debounce_ms, updated_at
                FROM time_machine_settings
                WHERE id = 1
                "#,
                [],
                |row| {
                    Ok(TimeMachineSettings {
                        auto_watch_enabled: row.get::<_, i32>(0)? != 0,
                        debounce_ms: row.get(1)?,
                        updated_at: row.get(2)?,
                    })
                },
            );

            match result {
                Ok(settings) => Ok(settings),
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    // Return default settings if not found
                    Ok(TimeMachineSettings::default())
                }
                Err(e) => Err(format!("Failed to get Time Machine settings: {}", e)),
            }
        })
    }

    /// Update Time Machine settings
    pub fn update_time_machine_settings(&self, settings: &TimeMachineSettings) -> Result<(), String> {
        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO time_machine_settings (id, auto_watch_enabled, debounce_ms, updated_at)
                VALUES (1, ?1, ?2, ?3)
                ON CONFLICT(id) DO UPDATE SET
                    auto_watch_enabled = excluded.auto_watch_enabled,
                    debounce_ms = excluded.debounce_ms,
                    updated_at = excluded.updated_at
                "#,
                params![
                    settings.auto_watch_enabled as i32,
                    settings.debounce_ms,
                    settings.updated_at,
                ],
            )
            .map_err(|e| format!("Failed to update Time Machine settings: {}", e))?;

            Ok(())
        })
    }
}

/// Internal row structure for snapshots
struct SnapshotRow {
    id: String,
    project_path: String,
    status: String,
    trigger_source: String,
    lockfile_type: Option<String>,
    lockfile_hash: Option<String>,
    dependency_tree_hash: Option<String>,
    package_json_hash: Option<String>,
    total_dependencies: i32,
    direct_dependencies: i32,
    dev_dependencies: i32,
    security_score: Option<i32>,
    postinstall_count: i32,
    storage_path: Option<String>,
    compressed_size: Option<i64>,
    error_message: Option<String>,
    created_at: String,
}

impl SnapshotRow {
    fn into_snapshot(self) -> ExecutionSnapshot {
        ExecutionSnapshot {
            id: self.id,
            project_path: self.project_path,
            status: SnapshotStatus::from_str(&self.status).unwrap_or(SnapshotStatus::Failed),
            trigger_source: TriggerSource::from_str(&self.trigger_source)
                .unwrap_or(TriggerSource::Manual),
            lockfile_type: self.lockfile_type.and_then(|s| LockfileType::from_str(&s)),
            lockfile_hash: self.lockfile_hash,
            dependency_tree_hash: self.dependency_tree_hash,
            package_json_hash: self.package_json_hash,
            total_dependencies: self.total_dependencies,
            direct_dependencies: self.direct_dependencies,
            dev_dependencies: self.dev_dependencies,
            security_score: self.security_score,
            postinstall_count: self.postinstall_count,
            storage_path: self.storage_path,
            compressed_size: self.compressed_size,
            error_message: self.error_message,
            created_at: self.created_at,
        }
    }
}

/// Internal row structure for insights
struct InsightRow {
    id: String,
    snapshot_id: String,
    insight_type: String,
    severity: String,
    title: String,
    description: String,
    package_name: Option<String>,
    previous_value: Option<String>,
    current_value: Option<String>,
    recommendation: Option<String>,
    metadata: Option<String>,
    is_dismissed: bool,
    created_at: String,
}

impl InsightRow {
    fn into_insight(self) -> SecurityInsight {
        SecurityInsight {
            id: self.id,
            snapshot_id: self.snapshot_id,
            insight_type: InsightType::from_str(&self.insight_type)
                .unwrap_or(InsightType::NewDependency),
            severity: InsightSeverity::from_str(&self.severity).unwrap_or(InsightSeverity::Info),
            title: self.title,
            description: self.description,
            package_name: self.package_name,
            previous_value: self.previous_value,
            current_value: self.current_value,
            recommendation: self.recommendation,
            metadata: self.metadata.and_then(|s| serde_json::from_str(&s).ok()),
            is_dismissed: self.is_dismissed,
            created_at: self.created_at,
        }
    }
}
