// Snapshot Diff Service
// Compares two snapshots and generates detailed diff information

use std::collections::HashMap;

use crate::models::snapshot::{
    DependencyChange, DependencyChangeType, DiffSummary, ExecutionSnapshot, PostinstallChange,
    SnapshotDependency, SnapshotDiff,
};
use crate::repositories::SnapshotRepository;
use crate::services::security_guardian::patterns::{analyze_dependency_changes, PatternAnalysisResult};
use crate::utils::database::Database;

/// Service for comparing execution snapshots
pub struct SnapshotDiffService {
    db: Database,
}

impl SnapshotDiffService {
    /// Create a new SnapshotDiffService
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Compare two snapshots and generate a diff
    pub fn compare_snapshots(
        &self,
        snapshot_a_id: &str,
        snapshot_b_id: &str,
    ) -> Result<SnapshotDiff, String> {
        let repo = SnapshotRepository::new(self.db.clone());

        // Check for cached diff first
        if let Some(cached) = repo.get_cached_diff(snapshot_a_id, snapshot_b_id)? {
            return Ok(cached);
        }

        // Get both snapshots with dependencies
        let snapshot_a = repo
            .get_snapshot_with_dependencies(snapshot_a_id)?
            .ok_or_else(|| format!("Snapshot {} not found", snapshot_a_id))?;

        let snapshot_b = repo
            .get_snapshot_with_dependencies(snapshot_b_id)?
            .ok_or_else(|| format!("Snapshot {} not found", snapshot_b_id))?;

        // Build the diff
        let diff = self.build_diff(&snapshot_a.snapshot, &snapshot_a.dependencies, &snapshot_b.snapshot, &snapshot_b.dependencies);

        // Cache the result
        repo.cache_diff(&diff)?;

        Ok(diff)
    }

    /// Build diff between two snapshots
    fn build_diff(
        &self,
        snapshot_a: &ExecutionSnapshot,
        deps_a: &[SnapshotDependency],
        snapshot_b: &ExecutionSnapshot,
        deps_b: &[SnapshotDependency],
    ) -> SnapshotDiff {
        // Build dependency maps
        let deps_a_map: HashMap<&str, &SnapshotDependency> =
            deps_a.iter().map(|d| (d.name.as_str(), d)).collect();
        let deps_b_map: HashMap<&str, &SnapshotDependency> =
            deps_b.iter().map(|d| (d.name.as_str(), d)).collect();

        let mut dependency_changes = Vec::new();
        let mut postinstall_changes = Vec::new();

        let mut added_count = 0;
        let mut removed_count = 0;
        let mut updated_count = 0;
        let mut unchanged_count = 0;
        let mut postinstall_added = 0;
        let mut postinstall_removed = 0;
        let mut postinstall_changed = 0;

        // Check for removed and changed dependencies
        for (name, dep_a) in &deps_a_map {
            if let Some(dep_b) = deps_b_map.get(name) {
                // Dependency exists in both
                if dep_a.version != dep_b.version {
                    updated_count += 1;

                    let postinstall_changed_flag =
                        dep_a.has_postinstall != dep_b.has_postinstall ||
                        dep_a.postinstall_script != dep_b.postinstall_script;

                    if postinstall_changed_flag {
                        if !dep_a.has_postinstall && dep_b.has_postinstall {
                            postinstall_added += 1;
                        } else if dep_a.has_postinstall && !dep_b.has_postinstall {
                            postinstall_removed += 1;
                        } else {
                            postinstall_changed += 1;
                        }

                        postinstall_changes.push(PostinstallChange {
                            package_name: name.to_string(),
                            change_type: DependencyChangeType::Updated,
                            old_script: dep_a.postinstall_script.clone(),
                            new_script: dep_b.postinstall_script.clone(),
                        });
                    }

                    dependency_changes.push(DependencyChange {
                        name: name.to_string(),
                        change_type: DependencyChangeType::Updated,
                        old_version: Some(dep_a.version.clone()),
                        new_version: Some(dep_b.version.clone()),
                        is_direct: dep_b.is_direct,
                        is_dev: dep_b.is_dev,
                        postinstall_changed: postinstall_changed_flag,
                        old_postinstall: dep_a.postinstall_script.clone(),
                        new_postinstall: dep_b.postinstall_script.clone(),
                    });
                } else {
                    unchanged_count += 1;
                }
            } else {
                // Dependency was removed
                removed_count += 1;

                if dep_a.has_postinstall {
                    postinstall_removed += 1;
                    postinstall_changes.push(PostinstallChange {
                        package_name: name.to_string(),
                        change_type: DependencyChangeType::Removed,
                        old_script: dep_a.postinstall_script.clone(),
                        new_script: None,
                    });
                }

                dependency_changes.push(DependencyChange {
                    name: name.to_string(),
                    change_type: DependencyChangeType::Removed,
                    old_version: Some(dep_a.version.clone()),
                    new_version: None,
                    is_direct: dep_a.is_direct,
                    is_dev: dep_a.is_dev,
                    postinstall_changed: dep_a.has_postinstall,
                    old_postinstall: dep_a.postinstall_script.clone(),
                    new_postinstall: None,
                });
            }
        }

        // Check for added dependencies
        for (name, dep_b) in &deps_b_map {
            if !deps_a_map.contains_key(name) {
                added_count += 1;

                if dep_b.has_postinstall {
                    postinstall_added += 1;
                    postinstall_changes.push(PostinstallChange {
                        package_name: name.to_string(),
                        change_type: DependencyChangeType::Added,
                        old_script: None,
                        new_script: dep_b.postinstall_script.clone(),
                    });
                }

                dependency_changes.push(DependencyChange {
                    name: name.to_string(),
                    change_type: DependencyChangeType::Added,
                    old_version: None,
                    new_version: Some(dep_b.version.clone()),
                    is_direct: dep_b.is_direct,
                    is_dev: dep_b.is_dev,
                    postinstall_changed: dep_b.has_postinstall,
                    old_postinstall: None,
                    new_postinstall: dep_b.postinstall_script.clone(),
                });
            }
        }

        // Sort changes by name
        dependency_changes.sort_by(|a, b| a.name.cmp(&b.name));
        postinstall_changes.sort_by(|a, b| a.package_name.cmp(&b.package_name));

        // Calculate security score change
        let security_score_change = match (snapshot_a.security_score, snapshot_b.security_score) {
            (Some(a), Some(b)) => Some(b - a),
            _ => None,
        };

        SnapshotDiff {
            snapshot_a_id: snapshot_a.id.clone(),
            snapshot_b_id: snapshot_b.id.clone(),
            summary: DiffSummary {
                added_count,
                removed_count,
                updated_count,
                unchanged_count,
                postinstall_added,
                postinstall_removed,
                postinstall_changed,
                security_score_change,
            },
            dependency_changes,
            postinstall_changes,
            lockfile_type_changed: snapshot_a.lockfile_type != snapshot_b.lockfile_type,
            old_lockfile_type: snapshot_a.lockfile_type.clone(),
            new_lockfile_type: snapshot_b.lockfile_type.clone(),
        }
    }

    /// Generate AI-friendly prompt for diff analysis
    pub fn generate_ai_prompt(&self, diff: &SnapshotDiff) -> String {
        let mut prompt = String::from("Analyze this dependency change between two workflow executions:\n\n");

        prompt.push_str(&format!(
            "Summary:\n- Added: {} packages\n- Removed: {} packages\n- Updated: {} packages\n- Unchanged: {} packages\n\n",
            diff.summary.added_count,
            diff.summary.removed_count,
            diff.summary.updated_count,
            diff.summary.unchanged_count
        ));

        if diff.summary.postinstall_added > 0 || diff.summary.postinstall_removed > 0 || diff.summary.postinstall_changed > 0 {
            prompt.push_str(&format!(
                "Postinstall Script Changes:\n- Added: {}\n- Removed: {}\n- Changed: {}\n\n",
                diff.summary.postinstall_added,
                diff.summary.postinstall_removed,
                diff.summary.postinstall_changed
            ));
        }

        if let Some(score_change) = diff.summary.security_score_change {
            prompt.push_str(&format!(
                "Security Score Change: {:+}\n\n",
                score_change
            ));
        }

        if diff.lockfile_type_changed {
            prompt.push_str(&format!(
                "Package Manager Changed: {:?} -> {:?}\n\n",
                diff.old_lockfile_type, diff.new_lockfile_type
            ));
        }

        // List significant changes
        let significant_changes: Vec<_> = diff
            .dependency_changes
            .iter()
            .filter(|c| c.change_type != DependencyChangeType::Unchanged)
            .take(20)
            .collect();

        if !significant_changes.is_empty() {
            prompt.push_str("Notable Changes:\n");
            for change in significant_changes {
                match change.change_type {
                    DependencyChangeType::Added => {
                        prompt.push_str(&format!(
                            "- + {} @ {}\n",
                            change.name,
                            change.new_version.as_deref().unwrap_or("unknown")
                        ));
                    }
                    DependencyChangeType::Removed => {
                        prompt.push_str(&format!(
                            "- - {} @ {}\n",
                            change.name,
                            change.old_version.as_deref().unwrap_or("unknown")
                        ));
                    }
                    DependencyChangeType::Updated => {
                        prompt.push_str(&format!(
                            "- ^ {} {} -> {}\n",
                            change.name,
                            change.old_version.as_deref().unwrap_or("unknown"),
                            change.new_version.as_deref().unwrap_or("unknown")
                        ));
                    }
                    DependencyChangeType::Unchanged => {}
                }

                if change.postinstall_changed {
                    prompt.push_str("    ⚠️ Postinstall script changed\n");
                }
            }
        }

        prompt.push_str("\nPlease analyze these changes and highlight any potential security concerns, breaking changes, or recommendations.");

        prompt
    }

    /// Get snapshots for comparison (latest N for a project)
    pub fn get_comparison_candidates(
        &self,
        project_path: &str,
        limit: i32,
    ) -> Result<Vec<ExecutionSnapshot>, String> {
        let repo = SnapshotRepository::new(self.db.clone());

        let filter = crate::models::snapshot::SnapshotFilter {
            project_path: Some(project_path.to_string()),
            status: Some(crate::models::snapshot::SnapshotStatus::Completed),
            limit: Some(limit),
            ..Default::default()
        };

        let list_items = repo.list_snapshots(&filter)?;

        // Get full snapshots for each item
        let mut snapshots = Vec::new();
        for item in list_items {
            if let Some(snapshot) = repo.get_snapshot(&item.id)? {
                snapshots.push(snapshot);
            }
        }

        Ok(snapshots)
    }

    /// Perform offline pattern-based security analysis on a diff
    /// This provides security insights without requiring AI/cloud access
    pub fn analyze_patterns(&self, diff: &SnapshotDiff) -> PatternAnalysisResult {
        analyze_dependency_changes(&diff.dependency_changes)
    }

    /// Compare snapshots and include pattern analysis
    pub fn compare_with_patterns(
        &self,
        snapshot_a_id: &str,
        snapshot_b_id: &str,
    ) -> Result<(SnapshotDiff, PatternAnalysisResult), String> {
        let diff = self.compare_snapshots(snapshot_a_id, snapshot_b_id)?;
        let patterns = self.analyze_patterns(&diff);
        Ok((diff, patterns))
    }
}
