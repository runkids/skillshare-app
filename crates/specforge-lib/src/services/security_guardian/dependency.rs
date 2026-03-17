// Security Guardian - Dependency Integrity Service
// Checks current dependencies against reference snapshot for drift detection

use std::collections::HashMap;
use std::path::Path;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

use crate::models::snapshot::{
    DependencyChange, DependencyChangeType, ExecutionSnapshot, LockfileType, SnapshotDependency,
};
use crate::repositories::SnapshotRepository;
use crate::utils::database::Database;

use super::patterns::{check_typosquatting, PatternAlert, PatternAlertType, AlertSeverity};

// =============================================================================
// Types
// =============================================================================

/// Result of a dependency integrity check
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityCheckResult {
    pub has_drift: bool,
    pub reference_snapshot_id: Option<String>,
    pub reference_snapshot_date: Option<String>,
    pub current_lockfile_hash: Option<String>,
    pub reference_lockfile_hash: Option<String>,
    pub lockfile_matches: bool,
    pub dependency_changes: Vec<DependencyChange>,
    pub postinstall_alerts: Vec<PostinstallAlert>,
    pub typosquatting_alerts: Vec<PatternAlert>,
    pub summary: IntegrityCheckSummary,
}

/// Summary of integrity check
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityCheckSummary {
    pub total_changes: usize,
    pub added_count: usize,
    pub removed_count: usize,
    pub updated_count: usize,
    pub postinstall_changes: usize,
    pub typosquatting_suspects: usize,
    pub risk_level: RiskLevel,
}

/// Risk level from integrity check
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

/// Alert for postinstall script changes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostinstallAlert {
    pub package_name: String,
    pub version: String,
    pub change_type: PostinstallChangeType,
    pub old_script: Option<String>,
    pub new_script: Option<String>,
    pub script_hash: Option<String>,
}

/// Type of postinstall change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostinstallChangeType {
    Added,
    Removed,
    Changed,
    Unchanged,
}

/// Current dependency state from project
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentDependencyState {
    pub lockfile_type: Option<LockfileType>,
    pub lockfile_hash: Option<String>,
    pub dependencies: Vec<CurrentDependency>,
    pub total_count: usize,
}

/// A dependency from current state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentDependency {
    pub name: String,
    pub version: String,
    pub is_direct: bool,
    pub is_dev: bool,
    pub has_postinstall: bool,
    pub postinstall_script: Option<String>,
}

// =============================================================================
// Service
// =============================================================================

/// Service for checking dependency integrity
pub struct DependencyIntegrityService {
    db: Database,
}

impl DependencyIntegrityService {
    /// Create a new DependencyIntegrityService
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Check dependency integrity for a project against its last successful snapshot
    pub fn check_integrity(
        &self,
        project_path: &str,
    ) -> Result<IntegrityCheckResult, String> {
        // Get reference snapshot (last successful snapshot)
        let reference = self.get_reference_snapshot(project_path)?;

        // Scan current dependencies from lockfile
        let current_state = self.scan_current_dependencies(project_path)?;

        match reference {
            Some((snapshot, ref_deps)) => {
                // Compare current state against reference
                self.compare_against_reference(&current_state, &snapshot, &ref_deps)
            }
            None => {
                // No reference snapshot - just scan for typosquatting
                self.check_without_reference(&current_state)
            }
        }
    }

    /// Get the reference snapshot for a project
    fn get_reference_snapshot(
        &self,
        project_path: &str,
    ) -> Result<Option<(ExecutionSnapshot, Vec<SnapshotDependency>)>, String> {
        let repo = SnapshotRepository::new(self.db.clone());

        // Build filter for latest successful snapshot
        let filter = crate::models::snapshot::SnapshotFilter {
            project_path: Some(project_path.to_string()),
            status: Some(crate::models::snapshot::SnapshotStatus::Completed),
            limit: Some(1),
            ..Default::default()
        };

        let snapshots = repo.list_snapshots(&filter)?;

        if let Some(snapshot_item) = snapshots.first() {
            if let Some(snapshot_with_deps) = repo.get_snapshot_with_dependencies(&snapshot_item.id)? {
                return Ok(Some((snapshot_with_deps.snapshot, snapshot_with_deps.dependencies)));
            }
        }

        Ok(None)
    }

    /// Scan current dependencies from project lockfile
    fn scan_current_dependencies(&self, project_path: &str) -> Result<CurrentDependencyState, String> {
        let path = Path::new(project_path);

        // Try to find and parse lockfile
        let (lockfile_type, lockfile_hash, dependencies) = if path.join("package-lock.json").exists() {
            self.parse_npm_lockfile(&path.join("package-lock.json"))?
        } else if path.join("pnpm-lock.yaml").exists() {
            self.parse_pnpm_lockfile(&path.join("pnpm-lock.yaml"))?
        } else if path.join("yarn.lock").exists() {
            // Basic yarn support - just hash the lockfile
            let content = std::fs::read_to_string(path.join("yarn.lock"))
                .map_err(|e| format!("Failed to read yarn.lock: {}", e))?;
            let hash = compute_hash(&content);
            (Some(LockfileType::Yarn), Some(hash), Vec::new())
        } else {
            return Err("No supported lockfile found (package-lock.json, pnpm-lock.yaml, yarn.lock)".to_string());
        };

        let total_count = dependencies.len();

        Ok(CurrentDependencyState {
            lockfile_type,
            lockfile_hash,
            dependencies,
            total_count,
        })
    }

    /// Parse npm package-lock.json
    fn parse_npm_lockfile(
        &self,
        lockfile_path: &Path,
    ) -> Result<(Option<LockfileType>, Option<String>, Vec<CurrentDependency>), String> {
        let content = std::fs::read_to_string(lockfile_path)
            .map_err(|e| format!("Failed to read package-lock.json: {}", e))?;

        let hash = compute_hash(&content);

        let lockfile: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse package-lock.json: {}", e))?;

        let mut dependencies = Vec::new();

        // Parse packages (npm v3 format)
        if let Some(packages) = lockfile.get("packages").and_then(|p| p.as_object()) {
            for (path, pkg_data) in packages {
                // Skip root package
                if path.is_empty() {
                    continue;
                }

                // Extract package name from path (e.g., "node_modules/lodash" -> "lodash")
                let name = path.trim_start_matches("node_modules/").to_string();

                // Skip nested dependencies for now (they have multiple node_modules in path)
                if name.contains("node_modules/") {
                    continue;
                }

                let version = pkg_data.get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let is_dev = pkg_data.get("dev").and_then(|v| v.as_bool()).unwrap_or(false);

                dependencies.push(CurrentDependency {
                    name,
                    version,
                    is_direct: true, // Simplified - all direct for now
                    is_dev,
                    has_postinstall: false, // Will be filled by postinstall scan
                    postinstall_script: None,
                });
            }
        }

        Ok((Some(LockfileType::Npm), Some(hash), dependencies))
    }

    /// Parse pnpm pnpm-lock.yaml
    fn parse_pnpm_lockfile(
        &self,
        lockfile_path: &Path,
    ) -> Result<(Option<LockfileType>, Option<String>, Vec<CurrentDependency>), String> {
        let content = std::fs::read_to_string(lockfile_path)
            .map_err(|e| format!("Failed to read pnpm-lock.yaml: {}", e))?;

        let hash = compute_hash(&content);

        let lockfile: serde_yaml::Value = serde_yaml::from_str(&content)
            .map_err(|e| format!("Failed to parse pnpm-lock.yaml: {}", e))?;

        let mut dependencies = Vec::new();

        // Parse importers.'.'.dependencies and devDependencies (pnpm v9 format)
        if let Some(importers) = lockfile.get("importers") {
            if let Some(root) = importers.get(".") {
                // Direct dependencies
                if let Some(deps) = root.get("dependencies").and_then(|d| d.as_mapping()) {
                    for (name, info) in deps {
                        if let (Some(name), Some(info)) = (name.as_str(), info.as_mapping()) {
                            let version = info.get("version")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();

                            dependencies.push(CurrentDependency {
                                name: name.to_string(),
                                version,
                                is_direct: true,
                                is_dev: false,
                                has_postinstall: false,
                                postinstall_script: None,
                            });
                        }
                    }
                }

                // Dev dependencies
                if let Some(deps) = root.get("devDependencies").and_then(|d| d.as_mapping()) {
                    for (name, info) in deps {
                        if let (Some(name), Some(info)) = (name.as_str(), info.as_mapping()) {
                            let version = info.get("version")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();

                            dependencies.push(CurrentDependency {
                                name: name.to_string(),
                                version,
                                is_direct: true,
                                is_dev: true,
                                has_postinstall: false,
                                postinstall_script: None,
                            });
                        }
                    }
                }
            }
        }

        Ok((Some(LockfileType::Pnpm), Some(hash), dependencies))
    }

    /// Compare current state against reference snapshot
    fn compare_against_reference(
        &self,
        current: &CurrentDependencyState,
        reference: &ExecutionSnapshot,
        ref_deps: &[SnapshotDependency],
    ) -> Result<IntegrityCheckResult, String> {
        // Build reference dependency map
        let ref_map: HashMap<&str, &SnapshotDependency> = ref_deps
            .iter()
            .map(|d| (d.name.as_str(), d))
            .collect();

        // Build current dependency map
        let current_map: HashMap<&str, &CurrentDependency> = current.dependencies
            .iter()
            .map(|d| (d.name.as_str(), d))
            .collect();

        let mut dependency_changes = Vec::new();
        let mut postinstall_alerts = Vec::new();
        let mut typosquatting_alerts = Vec::new();

        let mut added_count = 0;
        let mut removed_count = 0;
        let mut updated_count = 0;
        let mut postinstall_changes = 0;

        // Check for removed and changed dependencies
        for (name, ref_dep) in &ref_map {
            if let Some(cur_dep) = current_map.get(name) {
                // Exists in both - check for changes
                if ref_dep.version != cur_dep.version {
                    updated_count += 1;

                    dependency_changes.push(DependencyChange {
                        name: name.to_string(),
                        change_type: DependencyChangeType::Updated,
                        old_version: Some(ref_dep.version.clone()),
                        new_version: Some(cur_dep.version.clone()),
                        is_direct: cur_dep.is_direct,
                        is_dev: cur_dep.is_dev,
                        postinstall_changed: ref_dep.has_postinstall != cur_dep.has_postinstall,
                        old_postinstall: ref_dep.postinstall_script.clone(),
                        new_postinstall: cur_dep.postinstall_script.clone(),
                    });

                    // Check for postinstall changes
                    if ref_dep.has_postinstall != cur_dep.has_postinstall ||
                       ref_dep.postinstall_script != cur_dep.postinstall_script {
                        postinstall_changes += 1;
                        postinstall_alerts.push(PostinstallAlert {
                            package_name: name.to_string(),
                            version: cur_dep.version.clone(),
                            change_type: if !ref_dep.has_postinstall && cur_dep.has_postinstall {
                                PostinstallChangeType::Added
                            } else if ref_dep.has_postinstall && !cur_dep.has_postinstall {
                                PostinstallChangeType::Removed
                            } else {
                                PostinstallChangeType::Changed
                            },
                            old_script: ref_dep.postinstall_script.clone(),
                            new_script: cur_dep.postinstall_script.clone(),
                            script_hash: cur_dep.postinstall_script.as_ref().map(|s| compute_hash(s)),
                        });
                    }
                }
            } else {
                // Dependency was removed
                removed_count += 1;

                dependency_changes.push(DependencyChange {
                    name: name.to_string(),
                    change_type: DependencyChangeType::Removed,
                    old_version: Some(ref_dep.version.clone()),
                    new_version: None,
                    is_direct: ref_dep.is_direct,
                    is_dev: ref_dep.is_dev,
                    postinstall_changed: ref_dep.has_postinstall,
                    old_postinstall: ref_dep.postinstall_script.clone(),
                    new_postinstall: None,
                });
            }
        }

        // Check for added dependencies
        for (name, cur_dep) in &current_map {
            if !ref_map.contains_key(name) {
                added_count += 1;

                dependency_changes.push(DependencyChange {
                    name: name.to_string(),
                    change_type: DependencyChangeType::Added,
                    old_version: None,
                    new_version: Some(cur_dep.version.clone()),
                    is_direct: cur_dep.is_direct,
                    is_dev: cur_dep.is_dev,
                    postinstall_changed: cur_dep.has_postinstall,
                    old_postinstall: None,
                    new_postinstall: cur_dep.postinstall_script.clone(),
                });

                // Check for typosquatting on new packages
                let typo_result = check_typosquatting(name, 2);
                if typo_result.is_suspicious {
                    typosquatting_alerts.push(PatternAlert {
                        alert_type: PatternAlertType::Typosquatting,
                        severity: AlertSeverity::High,
                        package_name: name.to_string(),
                        title: format!("Potential typosquatting: {}", name),
                        description: format!(
                            "New package '{}' is similar to popular package '{}'. Verify this is intentional.",
                            name,
                            typo_result.similar_to.as_deref().unwrap_or("unknown")
                        ),
                        recommendation: Some("Review the package source before proceeding".to_string()),
                    });
                }

                // Alert for new postinstall scripts
                if cur_dep.has_postinstall {
                    postinstall_changes += 1;
                    postinstall_alerts.push(PostinstallAlert {
                        package_name: name.to_string(),
                        version: cur_dep.version.clone(),
                        change_type: PostinstallChangeType::Added,
                        old_script: None,
                        new_script: cur_dep.postinstall_script.clone(),
                        script_hash: cur_dep.postinstall_script.as_ref().map(|s| compute_hash(s)),
                    });
                }
            }
        }

        // Sort changes
        dependency_changes.sort_by(|a, b| a.name.cmp(&b.name));

        // Check lockfile hash match
        let lockfile_matches = current.lockfile_hash == reference.lockfile_hash;

        // Calculate risk level
        let total_changes = added_count + removed_count + updated_count;
        let risk_level = calculate_risk_level(
            total_changes,
            postinstall_changes,
            typosquatting_alerts.len(),
        );

        let typosquatting_count = typosquatting_alerts.len();

        Ok(IntegrityCheckResult {
            has_drift: total_changes > 0 || !lockfile_matches,
            reference_snapshot_id: Some(reference.id.clone()),
            reference_snapshot_date: Some(reference.created_at.clone()),
            current_lockfile_hash: current.lockfile_hash.clone(),
            reference_lockfile_hash: reference.lockfile_hash.clone(),
            lockfile_matches,
            dependency_changes,
            postinstall_alerts,
            typosquatting_alerts,
            summary: IntegrityCheckSummary {
                total_changes,
                added_count,
                removed_count,
                updated_count,
                postinstall_changes,
                typosquatting_suspects: typosquatting_count,
                risk_level,
            },
        })
    }

    /// Check without reference (just scan for issues)
    fn check_without_reference(
        &self,
        current: &CurrentDependencyState,
    ) -> Result<IntegrityCheckResult, String> {
        let mut typosquatting_alerts = Vec::new();

        // Check all packages for typosquatting
        for dep in &current.dependencies {
            let typo_result = check_typosquatting(&dep.name, 2);
            if typo_result.is_suspicious {
                typosquatting_alerts.push(PatternAlert {
                    alert_type: PatternAlertType::Typosquatting,
                    severity: AlertSeverity::High,
                    package_name: dep.name.clone(),
                    title: format!("Potential typosquatting: {}", dep.name),
                    description: format!(
                        "Package '{}' is similar to popular package '{}'. Verify this is intentional.",
                        dep.name,
                        typo_result.similar_to.as_deref().unwrap_or("unknown")
                    ),
                    recommendation: Some("Review the package source".to_string()),
                });
            }
        }

        let typosquatting_count = typosquatting_alerts.len();

        let risk_level = if typosquatting_count == 0 {
            RiskLevel::None
        } else if typosquatting_count > 3 {
            RiskLevel::High
        } else {
            RiskLevel::Medium
        };

        Ok(IntegrityCheckResult {
            has_drift: false,
            reference_snapshot_id: None,
            reference_snapshot_date: None,
            current_lockfile_hash: current.lockfile_hash.clone(),
            reference_lockfile_hash: None,
            lockfile_matches: true,
            dependency_changes: Vec::new(),
            postinstall_alerts: Vec::new(),
            typosquatting_alerts,
            summary: IntegrityCheckSummary {
                total_changes: 0,
                added_count: 0,
                removed_count: 0,
                updated_count: 0,
                postinstall_changes: 0,
                typosquatting_suspects: typosquatting_count,
                risk_level,
            },
        })
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Compute SHA-256 hash of content
fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Calculate risk level based on changes
fn calculate_risk_level(
    total_changes: usize,
    postinstall_changes: usize,
    typosquatting_count: usize,
) -> RiskLevel {
    // Critical if typosquatting detected
    if typosquatting_count > 0 {
        return RiskLevel::Critical;
    }

    // High if postinstall scripts changed
    if postinstall_changes > 0 {
        return RiskLevel::High;
    }

    // Medium if significant changes
    if total_changes > 10 {
        return RiskLevel::Medium;
    }

    // Low if some changes
    if total_changes > 0 {
        return RiskLevel::Low;
    }

    RiskLevel::None
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash() {
        let hash = compute_hash("test content");
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex chars
    }

    #[test]
    fn test_risk_level_calculation() {
        assert_eq!(calculate_risk_level(0, 0, 0), RiskLevel::None);
        assert_eq!(calculate_risk_level(5, 0, 0), RiskLevel::Low);
        assert_eq!(calculate_risk_level(15, 0, 0), RiskLevel::Medium);
        assert_eq!(calculate_risk_level(5, 1, 0), RiskLevel::High);
        assert_eq!(calculate_risk_level(0, 0, 1), RiskLevel::Critical);
    }
}
