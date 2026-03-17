// Snapshot Replay Service
// Enables safe replay of historical executions with dependency verification

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

use crate::models::snapshot::{ExecutionSnapshot, LockfileType, SnapshotDependency};
use crate::repositories::SnapshotRepository;
use crate::services::snapshot::storage::SnapshotStorage;
use crate::utils::database::Database;

// =============================================================================
// Types
// =============================================================================

/// Result of preparing a replay
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayPreparation {
    pub snapshot_id: String,
    pub project_path: String,
    pub ready_to_replay: bool,
    pub has_mismatch: bool,
    pub mismatch_details: Option<ReplayMismatch>,
    pub available_options: Vec<ReplayOption>,
}

/// Details about dependency mismatch
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayMismatch {
    pub lockfile_changed: bool,
    pub current_lockfile_hash: Option<String>,
    pub snapshot_lockfile_hash: Option<String>,
    pub dependency_tree_changed: bool,
    pub current_tree_hash: Option<String>,
    pub snapshot_tree_hash: Option<String>,
    pub added_packages: Vec<String>,
    pub removed_packages: Vec<String>,
    pub changed_packages: Vec<PackageVersionChange>,
}

/// Package version change info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageVersionChange {
    pub name: String,
    pub snapshot_version: String,
    pub current_version: String,
}

/// Available options when mismatch is detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayOption {
    /// Abort the replay
    Abort,
    /// View the diff between snapshot and current state
    ViewDiff,
    /// Restore lockfile from snapshot
    RestoreLockfile,
    /// Proceed with current dependencies
    ProceedWithCurrent,
}

/// Result of executing a replay
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayResult {
    pub success: bool,
    pub execution_id: Option<String>,
    pub is_verified_replay: bool,
    pub lockfile_restored: bool,
    pub error: Option<String>,
}

/// Request to execute a replay
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteReplayRequest {
    pub snapshot_id: String,
    pub option: ReplayOption,
    pub force: bool,
}

// =============================================================================
// Service
// =============================================================================

/// Service for replaying historical executions
pub struct SnapshotReplayService {
    storage: SnapshotStorage,
    db: Database,
}

impl SnapshotReplayService {
    /// Create a new SnapshotReplayService
    pub fn new(storage: SnapshotStorage, db: Database) -> Self {
        Self { storage, db }
    }

    /// Prepare a replay by verifying dependencies
    pub fn prepare_replay(&self, snapshot_id: &str) -> Result<ReplayPreparation, String> {
        let repo = SnapshotRepository::new(self.db.clone());

        // Get snapshot with dependencies
        let snapshot_with_deps = repo
            .get_snapshot_with_dependencies(snapshot_id)?
            .ok_or_else(|| format!("Snapshot not found: {}", snapshot_id))?;

        let snapshot = &snapshot_with_deps.snapshot;
        let snapshot_deps = &snapshot_with_deps.dependencies;

        // Get current state
        let project_path = Path::new(&snapshot.project_path);
        if !project_path.exists() {
            return Err(format!("Project path does not exist: {}", snapshot.project_path));
        }

        // Detect and compare lockfile
        let mismatch = self.detect_mismatch(snapshot, snapshot_deps, project_path)?;
        let has_mismatch = mismatch.is_some();

        // Determine available options
        let available_options = if has_mismatch {
            vec![
                ReplayOption::Abort,
                ReplayOption::ViewDiff,
                ReplayOption::RestoreLockfile,
                ReplayOption::ProceedWithCurrent,
            ]
        } else {
            vec![ReplayOption::Abort, ReplayOption::ProceedWithCurrent]
        };

        Ok(ReplayPreparation {
            snapshot_id: snapshot_id.to_string(),
            project_path: snapshot.project_path.clone(),
            ready_to_replay: !has_mismatch,
            has_mismatch,
            mismatch_details: mismatch,
            available_options,
        })
    }

    /// Detect mismatch between snapshot and current state
    fn detect_mismatch(
        &self,
        snapshot: &ExecutionSnapshot,
        snapshot_deps: &[SnapshotDependency],
        project_path: &Path,
    ) -> Result<Option<ReplayMismatch>, String> {
        // Try to read current lockfile
        let lockfile_type = snapshot.lockfile_type.clone().unwrap_or(LockfileType::Npm);
        let lockfile_name = lockfile_type.lockfile_name();
        let lockfile_path = project_path.join(lockfile_name);

        let current_lockfile_hash = if lockfile_path.exists() {
            let content = fs::read(&lockfile_path)
                .map_err(|e| format!("Failed to read lockfile: {}", e))?;
            Some(compute_hash(&content))
        } else {
            None
        };

        // Check if lockfile changed
        let lockfile_changed = match (&current_lockfile_hash, &snapshot.lockfile_hash) {
            (Some(current), Some(snapshot_hash)) => current != snapshot_hash,
            (None, Some(_)) => true,  // Lockfile deleted
            (Some(_), None) => true,  // Snapshot didn't have lockfile
            (None, None) => false,    // Neither has lockfile
        };

        // If lockfile matches, no mismatch
        if !lockfile_changed {
            return Ok(None);
        }

        // Parse current dependencies for detailed comparison
        let current_deps = self.parse_current_dependencies(project_path, &lockfile_type)?;

        // Compare dependencies
        let snapshot_dep_map: std::collections::HashMap<&str, &SnapshotDependency> = snapshot_deps
            .iter()
            .map(|d| (d.name.as_str(), d))
            .collect();

        let current_dep_map: std::collections::HashMap<&str, &String> = current_deps
            .iter()
            .map(|(name, version)| (name.as_str(), version))
            .collect();

        let mut added_packages = Vec::new();
        let mut removed_packages = Vec::new();
        let mut changed_packages = Vec::new();

        // Find removed and changed
        for (name, snapshot_dep) in &snapshot_dep_map {
            match current_dep_map.get(name) {
                Some(current_version) => {
                    if **current_version != snapshot_dep.version {
                        changed_packages.push(PackageVersionChange {
                            name: name.to_string(),
                            snapshot_version: snapshot_dep.version.clone(),
                            current_version: current_version.to_string(),
                        });
                    }
                }
                None => {
                    removed_packages.push(name.to_string());
                }
            }
        }

        // Find added
        for name in current_dep_map.keys() {
            if !snapshot_dep_map.contains_key(name) {
                added_packages.push(name.to_string());
            }
        }

        // Sort for consistent output
        added_packages.sort();
        removed_packages.sort();
        changed_packages.sort_by(|a, b| a.name.cmp(&b.name));

        let dependency_tree_changed = !added_packages.is_empty()
            || !removed_packages.is_empty()
            || !changed_packages.is_empty();

        // Compute current tree hash for comparison
        let current_tree_hash = if !current_deps.is_empty() {
            let mut items: Vec<_> = current_deps.iter().collect();
            items.sort_by(|a, b| a.0.cmp(&b.0));
            let tree_str: String = items.iter().map(|(n, v)| format!("{}@{}", n, v)).collect::<Vec<_>>().join(",");
            Some(compute_hash(tree_str.as_bytes()))
        } else {
            None
        };

        Ok(Some(ReplayMismatch {
            lockfile_changed,
            current_lockfile_hash,
            snapshot_lockfile_hash: snapshot.lockfile_hash.clone(),
            dependency_tree_changed,
            current_tree_hash,
            snapshot_tree_hash: snapshot.dependency_tree_hash.clone(),
            added_packages,
            removed_packages,
            changed_packages,
        }))
    }

    /// Parse current dependencies from lockfile (simplified)
    fn parse_current_dependencies(
        &self,
        project_path: &Path,
        lockfile_type: &LockfileType,
    ) -> Result<Vec<(String, String)>, String> {
        let lockfile_path = project_path.join(lockfile_type.lockfile_name());

        if !lockfile_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&lockfile_path)
            .map_err(|e| format!("Failed to read lockfile: {}", e))?;

        match lockfile_type {
            LockfileType::Npm => self.parse_npm_lockfile_deps(&content),
            LockfileType::Pnpm => self.parse_pnpm_lockfile_deps(&content),
            LockfileType::Yarn => Ok(Vec::new()), // Simplified - yarn parsing is complex
            LockfileType::Bun => Ok(Vec::new()),  // Simplified - bun uses binary format
        }
    }

    /// Parse npm package-lock.json dependencies
    fn parse_npm_lockfile_deps(&self, content: &str) -> Result<Vec<(String, String)>, String> {
        let lockfile: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| format!("Failed to parse package-lock.json: {}", e))?;

        let mut deps = Vec::new();

        if let Some(packages) = lockfile.get("packages").and_then(|p| p.as_object()) {
            for (path, pkg_data) in packages {
                if path.is_empty() {
                    continue;
                }
                let name = path.trim_start_matches("node_modules/").to_string();
                if name.contains("node_modules/") {
                    continue;
                }
                if let Some(version) = pkg_data.get("version").and_then(|v| v.as_str()) {
                    deps.push((name, version.to_string()));
                }
            }
        }

        Ok(deps)
    }

    /// Parse pnpm-lock.yaml dependencies
    fn parse_pnpm_lockfile_deps(&self, content: &str) -> Result<Vec<(String, String)>, String> {
        let lockfile: serde_yaml::Value = serde_yaml::from_str(content)
            .map_err(|e| format!("Failed to parse pnpm-lock.yaml: {}", e))?;

        let mut deps = Vec::new();

        if let Some(importers) = lockfile.get("importers") {
            if let Some(root) = importers.get(".") {
                // Direct dependencies
                if let Some(dependencies) = root.get("dependencies").and_then(|d| d.as_mapping()) {
                    for (name, info) in dependencies {
                        if let (Some(name), Some(info)) = (name.as_str(), info.as_mapping()) {
                            if let Some(version) = info.get("version").and_then(|v| v.as_str()) {
                                deps.push((name.to_string(), version.to_string()));
                            }
                        }
                    }
                }
                // Dev dependencies
                if let Some(dev_deps) = root.get("devDependencies").and_then(|d| d.as_mapping()) {
                    for (name, info) in dev_deps {
                        if let (Some(name), Some(info)) = (name.as_str(), info.as_mapping()) {
                            if let Some(version) = info.get("version").and_then(|v| v.as_str()) {
                                deps.push((name.to_string(), version.to_string()));
                            }
                        }
                    }
                }
            }
        }

        Ok(deps)
    }

    /// Restore lockfile from snapshot
    pub fn restore_lockfile(&self, snapshot_id: &str) -> Result<bool, String> {
        let repo = SnapshotRepository::new(self.db.clone());

        // Get snapshot
        let snapshot = repo
            .get_snapshot(snapshot_id)?
            .ok_or_else(|| format!("Snapshot not found: {}", snapshot_id))?;

        let lockfile_type = snapshot.lockfile_type.clone().unwrap_or(LockfileType::Npm);
        let lockfile_name = lockfile_type.lockfile_name();

        // Read lockfile from storage
        let lockfile_content = self.storage.read_lockfile(snapshot_id, lockfile_name)?;

        // Write to project
        let project_path = Path::new(&snapshot.project_path);
        let lockfile_path = project_path.join(lockfile_name);

        // Backup current lockfile
        if lockfile_path.exists() {
            let backup_path = lockfile_path.with_extension(format!(
                "{}.backup.{}",
                lockfile_path.extension().unwrap_or_default().to_string_lossy(),
                chrono::Utc::now().format("%Y%m%d_%H%M%S")
            ));
            fs::copy(&lockfile_path, &backup_path)
                .map_err(|e| format!("Failed to backup lockfile: {}", e))?;
        }

        // Write restored lockfile
        fs::write(&lockfile_path, &lockfile_content)
            .map_err(|e| format!("Failed to write lockfile: {}", e))?;

        Ok(true)
    }

    /// Execute a replay with the specified option
    pub fn execute_replay(&self, request: &ExecuteReplayRequest) -> Result<ReplayResult, String> {
        match request.option {
            ReplayOption::Abort => {
                Ok(ReplayResult {
                    success: false,
                    execution_id: None,
                    is_verified_replay: false,
                    lockfile_restored: false,
                    error: Some("Replay aborted by user".to_string()),
                })
            }
            ReplayOption::ViewDiff => {
                // This should be handled by the frontend - return info
                Ok(ReplayResult {
                    success: false,
                    execution_id: None,
                    is_verified_replay: false,
                    lockfile_restored: false,
                    error: Some("Use compare_snapshots to view diff".to_string()),
                })
            }
            ReplayOption::RestoreLockfile => {
                // Restore lockfile first
                self.restore_lockfile(&request.snapshot_id)?;

                // Now ready for verified replay
                Ok(ReplayResult {
                    success: true,
                    execution_id: None,
                    is_verified_replay: true,
                    lockfile_restored: true,
                    error: None,
                })
            }
            ReplayOption::ProceedWithCurrent => {
                // Check if we should warn about mismatch
                let prep = self.prepare_replay(&request.snapshot_id)?;

                Ok(ReplayResult {
                    success: true,
                    execution_id: None,
                    is_verified_replay: !prep.has_mismatch,
                    lockfile_restored: false,
                    error: None,
                })
            }
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Compute SHA-256 hash of content
fn compute_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash() {
        let hash = compute_hash(b"test content");
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_replay_option_serialization() {
        let option = ReplayOption::RestoreLockfile;
        let json = serde_json::to_string(&option).unwrap();
        assert_eq!(json, "\"restore_lockfile\"");
    }
}
