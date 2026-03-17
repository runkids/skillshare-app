// Security Repository
// Handles all database operations for security scans

use rusqlite::params;

use crate::models::security::{SecurityScanData, VulnScanResult};
use crate::models::PackageManager;
use crate::utils::database::Database;

/// Repository for security scan data access
pub struct SecurityRepository {
    db: Database,
}

impl SecurityRepository {
    /// Create a new SecurityRepository
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Get security scan data for a project
    pub fn get(&self, project_id: &str) -> Result<Option<SecurityScanData>, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT package_manager, last_scan, scan_history, snooze_until
                FROM security_scans
                WHERE project_id = ?1
                "#,
                params![project_id],
                |row| {
                    let package_manager: String = row.get(0)?;
                    let last_scan: Option<String> = row.get(1)?;
                    let scan_history: String = row.get(2)?;
                    let snooze_until: Option<String> = row.get(3)?;
                    Ok((package_manager, last_scan, scan_history, snooze_until))
                },
            );

            match result {
                Ok((package_manager, last_scan, scan_history_json, snooze_until)) => {
                    let pm = string_to_package_manager(&package_manager);

                    // Parse last_scan JSON
                    let last_scan_data: Option<VulnScanResult> = last_scan
                        .as_ref()
                        .and_then(|json| serde_json::from_str(json).ok());

                    // Parse scan_history JSON
                    let scan_history: Vec<VulnScanResult> =
                        serde_json::from_str(&scan_history_json).unwrap_or_default();

                    Ok(Some(SecurityScanData {
                        project_id: project_id.to_string(),
                        package_manager: pm,
                        last_scan: last_scan_data,
                        scan_history,
                        snooze_until,
                    }))
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get security scan: {}", e)),
            }
        })
    }

    /// Save security scan data for a project
    pub fn save(&self, project_id: &str, data: &SecurityScanData) -> Result<(), String> {
        let package_manager = package_manager_to_string(&data.package_manager);

        let last_scan_json = data
            .last_scan
            .as_ref()
            .map(|scan| serde_json::to_string(scan).ok())
            .flatten();

        let scan_history_json = serde_json::to_string(&data.scan_history)
            .map_err(|e| format!("Failed to serialize scan_history: {}", e))?;

        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT OR REPLACE INTO security_scans
                (project_id, package_manager, last_scan, scan_history, snooze_until)
                VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                params![
                    project_id,
                    package_manager,
                    last_scan_json,
                    scan_history_json,
                    data.snooze_until,
                ],
            )
            .map_err(|e| format!("Failed to save security scan: {}", e))?;

            Ok(())
        })
    }

    /// List all security scans (returns HashMap keyed by project_id)
    pub fn list_all(&self) -> Result<std::collections::HashMap<String, SecurityScanData>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT project_id, package_manager, last_scan, scan_history, snooze_until
                    FROM security_scans
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map([], |row| {
                    let project_id: String = row.get(0)?;
                    let package_manager: String = row.get(1)?;
                    let last_scan: Option<String> = row.get(2)?;
                    let scan_history: String = row.get(3)?;
                    let snooze_until: Option<String> = row.get(4)?;
                    Ok((project_id, package_manager, last_scan, scan_history, snooze_until))
                })
                .map_err(|e| format!("Failed to query security scans: {}", e))?;

            let mut result = std::collections::HashMap::new();
            for row in rows {
                let (project_id, package_manager, last_scan, scan_history_json, snooze_until) =
                    row.map_err(|e| format!("Failed to read row: {}", e))?;

                let pm = string_to_package_manager(&package_manager);
                let last_scan_data: Option<VulnScanResult> = last_scan
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok());
                let scan_history: Vec<VulnScanResult> =
                    serde_json::from_str(&scan_history_json).unwrap_or_default();

                result.insert(
                    project_id.clone(),
                    SecurityScanData {
                        project_id,
                        package_manager: pm,
                        last_scan: last_scan_data,
                        scan_history,
                        snooze_until,
                    },
                );
            }

            Ok(result)
        })
    }

    /// Delete security scan data for a project
    pub fn delete(&self, project_id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let rows_affected = conn
                .execute(
                    "DELETE FROM security_scans WHERE project_id = ?1",
                    params![project_id],
                )
                .map_err(|e| format!("Failed to delete security scan: {}", e))?;

            Ok(rows_affected > 0)
        })
    }

    /// Update snooze until date
    pub fn set_snooze_until(
        &self,
        project_id: &str,
        snooze_until: Option<&str>,
    ) -> Result<(), String> {
        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE security_scans SET snooze_until = ?1 WHERE project_id = ?2",
                params![snooze_until, project_id],
            )
            .map_err(|e| format!("Failed to update snooze_until: {}", e))?;

            Ok(())
        })
    }
}

/// Convert string to PackageManager enum
fn string_to_package_manager(s: &str) -> PackageManager {
    match s.to_lowercase().as_str() {
        "npm" => PackageManager::Npm,
        "yarn" => PackageManager::Yarn,
        "pnpm" => PackageManager::Pnpm,
        "bun" => PackageManager::Bun,
        _ => PackageManager::Unknown,
    }
}

/// Convert PackageManager enum to string
fn package_manager_to_string(pm: &PackageManager) -> &'static str {
    match pm {
        PackageManager::Npm => "npm",
        PackageManager::Yarn => "yarn",
        PackageManager::Pnpm => "pnpm",
        PackageManager::Bun => "bun",
        PackageManager::Unknown => "unknown",
    }
}
