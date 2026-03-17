// Snapshot Capture Service
// Captures execution snapshots including lockfile parsing and dependency analysis

use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

use crate::models::security_insight::{InsightType, SecurityInsight};
use crate::models::snapshot::{
    CreateSnapshotRequest, ExecutionSnapshot, LockfileType, PostinstallEntry, SecurityContext,
    SnapshotDependency, SnapshotStatus, TriggerSource, TyposquattingAlert,
};
use crate::repositories::{LockfileValidationRepository, SnapshotRepository};
use crate::services::snapshot::storage::SnapshotStorage;
use crate::services::snapshot::validation::{ValidationEngine, ValidationFailure};
use crate::utils::database::Database;

/// Service for capturing execution snapshots
pub struct SnapshotCaptureService {
    storage: SnapshotStorage,
    db: Database,
}

impl SnapshotCaptureService {
    /// Create a new SnapshotCaptureService
    pub fn new(storage: SnapshotStorage, db: Database) -> Self {
        Self { storage, db }
    }

    /// Capture a snapshot for a project
    pub fn capture_snapshot(
        &self,
        request: &CreateSnapshotRequest,
    ) -> Result<ExecutionSnapshot, String> {
        let snapshot_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        // Create initial snapshot record
        let mut snapshot = ExecutionSnapshot {
            id: snapshot_id.clone(),
            project_path: request.project_path.clone(),
            status: SnapshotStatus::Capturing,
            trigger_source: request.trigger_source.clone(),
            lockfile_type: None,
            lockfile_hash: None,
            dependency_tree_hash: None,
            package_json_hash: None,
            total_dependencies: 0,
            direct_dependencies: 0,
            dev_dependencies: 0,
            security_score: None,
            postinstall_count: 0,
            storage_path: None,
            compressed_size: None,
            error_message: None,
            created_at: now,
        };

        let repo = SnapshotRepository::new(self.db.clone());
        repo.create_snapshot(&snapshot)?;

        // Capture the snapshot data
        match self.capture_snapshot_data(&mut snapshot) {
            Ok((dependencies, package_json)) => {
                snapshot.status = SnapshotStatus::Completed;
                repo.update_snapshot(&snapshot)?;

                // Store dependencies
                repo.add_dependencies(&dependencies)?;

                // Run lockfile validation if enabled
                self.run_validation_and_store_insights(&snapshot.id, &dependencies, package_json.as_ref())?;

                Ok(snapshot)
            }
            Err(e) => {
                snapshot.status = SnapshotStatus::Failed;
                snapshot.error_message = Some(e.clone());
                repo.update_snapshot(&snapshot)?;
                Err(e)
            }
        }
    }

    /// Capture a snapshot triggered by lockfile change
    pub fn capture_lockfile_change_snapshot(
        &self,
        project_path: &str,
    ) -> Result<ExecutionSnapshot, String> {
        let request = CreateSnapshotRequest {
            project_path: project_path.to_string(),
            trigger_source: TriggerSource::LockfileChange,
        };
        self.capture_snapshot(&request)
    }

    /// Capture a snapshot triggered manually
    pub fn capture_manual_snapshot(
        &self,
        project_path: &str,
    ) -> Result<ExecutionSnapshot, String> {
        let request = CreateSnapshotRequest {
            project_path: project_path.to_string(),
            trigger_source: TriggerSource::Manual,
        };
        self.capture_snapshot(&request)
    }

    /// Capture snapshot data from the project
    /// Returns (dependencies, package_json) for validation
    fn capture_snapshot_data(
        &self,
        snapshot: &mut ExecutionSnapshot,
    ) -> Result<(Vec<SnapshotDependency>, Option<serde_json::Value>), String> {
        let project_path = Path::new(&snapshot.project_path);

        // Detect lockfile type and read lockfile
        let (lockfile_type, lockfile_content) = self.detect_and_read_lockfile(project_path)?;
        snapshot.lockfile_type = Some(lockfile_type.clone());

        // Compute lockfile hash
        snapshot.lockfile_hash = Some(self.compute_hash(&lockfile_content));

        // Store compressed lockfile
        let lockfile_name = lockfile_type.lockfile_name();
        let (_, compressed_size) = self
            .storage
            .store_lockfile(&snapshot.id, lockfile_name, &lockfile_content)?;
        snapshot.compressed_size = Some(compressed_size as i64);
        snapshot.storage_path = Some(self.storage.get_snapshot_path(&snapshot.id).to_string_lossy().to_string());

        // Read and store package.json
        let mut package_json: Option<serde_json::Value> = None;
        let package_json_path = project_path.join("package.json");
        if package_json_path.exists() {
            let package_json_content = fs::read(&package_json_path)
                .map_err(|e| format!("Failed to read package.json: {}", e))?;
            snapshot.package_json_hash = Some(self.compute_hash(&package_json_content));
            self.storage.store_package_json(&snapshot.id, &package_json_content)?;
            // Parse package.json for validation
            package_json = serde_json::from_slice(&package_json_content).ok();
        }

        // Parse lockfile and extract dependencies
        let dependencies = self.parse_lockfile(&lockfile_type, &lockfile_content, &snapshot.id)?;

        // Compute dependency statistics
        snapshot.total_dependencies = dependencies.len() as i32;
        snapshot.direct_dependencies = dependencies.iter().filter(|d| d.is_direct).count() as i32;
        snapshot.dev_dependencies = dependencies.iter().filter(|d| d.is_dev).count() as i32;
        snapshot.postinstall_count = dependencies.iter().filter(|d| d.has_postinstall).count() as i32;

        // Compute dependency tree hash
        let tree_hash = self.compute_dependency_tree_hash(&dependencies);
        snapshot.dependency_tree_hash = Some(tree_hash);

        // Compute security score (simplified)
        snapshot.security_score = Some(self.compute_security_score(&dependencies));

        Ok((dependencies, package_json))
    }

    /// Run lockfile validation and store insights
    fn run_validation_and_store_insights(
        &self,
        snapshot_id: &str,
        dependencies: &[SnapshotDependency],
        package_json: Option<&serde_json::Value>,
    ) -> Result<(), String> {
        // Load validation config
        let validation_repo = LockfileValidationRepository::new(self.db.clone());
        let config = validation_repo.get_config()?;

        // Skip if validation is disabled
        if !config.enabled {
            log::debug!("[SnapshotCapture] Lockfile validation is disabled, skipping");
            return Ok(());
        }

        log::info!(
            "[SnapshotCapture] Running lockfile validation with strictness: {}",
            config.strictness.as_str()
        );

        // Run validation
        let engine = ValidationEngine::new(config);
        let result = engine.validate(snapshot_id, dependencies, package_json);

        log::info!(
            "[SnapshotCapture] Validation complete: {} failures, {} warnings",
            result.failures.len(),
            result.warnings.len()
        );

        // Convert failures to security insights and store them
        let repo = SnapshotRepository::new(self.db.clone());
        let now = chrono::Utc::now().to_rfc3339();

        for failure in result.failures {
            let insight = Self::failure_to_insight(snapshot_id, &failure, &now);
            if let Err(e) = repo.create_insight(&insight) {
                log::warn!("[SnapshotCapture] Failed to store insight: {}", e);
            }
        }

        Ok(())
    }

    /// Convert a validation failure to a security insight
    fn failure_to_insight(
        snapshot_id: &str,
        failure: &ValidationFailure,
        created_at: &str,
    ) -> SecurityInsight {
        let insight_type = match failure.rule_id.as_str() {
            "require-integrity" => InsightType::MissingIntegrity,
            "require-https-resolved" => InsightType::InsecureProtocol,
            "check-allowed-registries" => InsightType::UnexpectedRegistry,
            "check-blocked-packages" => InsightType::BlockedPackage,
            "check-manifest-consistency" => InsightType::ManifestMismatch,
            "enhanced-typosquatting" => {
                // Determine specific type based on message
                if failure.message.contains("scope confusion") {
                    InsightType::ScopeConfusion
                } else if failure.message.contains("lookalike") {
                    InsightType::HomoglyphSuspect
                } else {
                    InsightType::TyposquattingSuspect
                }
            }
            _ => InsightType::TyposquattingSuspect,
        };

        SecurityInsight {
            id: uuid::Uuid::new_v4().to_string(),
            snapshot_id: snapshot_id.to_string(),
            insight_type,
            severity: failure.severity.clone(),
            title: format!("{}: {}", failure.rule_id, failure.package_name),
            description: failure.message.clone(),
            package_name: Some(failure.package_name.clone()),
            previous_value: None,
            current_value: None,
            recommendation: failure.remediation.clone(),
            metadata: None,
            is_dismissed: false,
            created_at: created_at.to_string(),
        }
    }

    /// Detect lockfile type and read content
    fn detect_and_read_lockfile(&self, project_path: &Path) -> Result<(LockfileType, Vec<u8>), String> {
        log::info!(
            "[SnapshotCapture] Detecting lockfile in: {}",
            project_path.display()
        );

        // Try each lockfile type in order of preference
        let lockfile_checks = [
            (LockfileType::Pnpm, "pnpm-lock.yaml"),
            (LockfileType::Npm, "package-lock.json"),
            (LockfileType::Yarn, "yarn.lock"),
            (LockfileType::Bun, "bun.lockb"),
        ];

        for (lockfile_type, filename) in lockfile_checks {
            let path = project_path.join(filename);
            log::debug!(
                "[SnapshotCapture] Checking {} - exists: {}",
                path.display(),
                path.exists()
            );
            if path.exists() {
                log::info!(
                    "[SnapshotCapture] Found {} lockfile: {}",
                    lockfile_type.as_str(),
                    path.display()
                );
                let content = fs::read(&path)
                    .map_err(|e| format!("Failed to read {}: {}", filename, e))?;
                return Ok((lockfile_type, content));
            }
        }

        log::error!(
            "[SnapshotCapture] No lockfile found in: {}",
            project_path.display()
        );
        Err(format!("No lockfile found in project: {}", project_path.display()))
    }

    /// Parse lockfile and extract dependencies
    fn parse_lockfile(
        &self,
        lockfile_type: &LockfileType,
        content: &[u8],
        snapshot_id: &str,
    ) -> Result<Vec<SnapshotDependency>, String> {
        match lockfile_type {
            LockfileType::Npm => self.parse_npm_lockfile(content, snapshot_id),
            LockfileType::Pnpm => self.parse_pnpm_lockfile(content, snapshot_id),
            LockfileType::Yarn => self.parse_yarn_lockfile(content, snapshot_id),
            LockfileType::Bun => self.parse_bun_lockfile(content, snapshot_id),
        }
    }

    /// Parse npm package-lock.json (v3 format)
    fn parse_npm_lockfile(
        &self,
        content: &[u8],
        snapshot_id: &str,
    ) -> Result<Vec<SnapshotDependency>, String> {
        let lockfile: serde_json::Value = serde_json::from_slice(content)
            .map_err(|e| format!("Failed to parse package-lock.json: {}", e))?;

        let mut dependencies = Vec::new();

        // npm v3 format uses "packages" key
        if let Some(packages) = lockfile.get("packages").and_then(|p| p.as_object()) {
            for (path, pkg) in packages {
                // Skip the root package (empty path)
                if path.is_empty() {
                    continue;
                }

                // Extract package name from path (node_modules/name or node_modules/@scope/name)
                let name = path
                    .strip_prefix("node_modules/")
                    .unwrap_or(path)
                    .to_string();

                let version = pkg
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let is_dev = pkg.get("dev").and_then(|v| v.as_bool()).unwrap_or(false);

                // Check for postinstall script
                let has_postinstall = pkg
                    .get("hasInstallScript")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let integrity_hash = pkg
                    .get("integrity")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let resolved_url = pkg
                    .get("resolved")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                dependencies.push(SnapshotDependency {
                    id: None,
                    snapshot_id: snapshot_id.to_string(),
                    name,
                    version,
                    is_direct: !path.contains("/node_modules/"),
                    is_dev,
                    has_postinstall,
                    postinstall_script: None, // Would need to read from node_modules
                    integrity_hash,
                    resolved_url,
                });
            }
        }

        Ok(dependencies)
    }

    /// Parse pnpm-lock.yaml
    fn parse_pnpm_lockfile(
        &self,
        content: &[u8],
        snapshot_id: &str,
    ) -> Result<Vec<SnapshotDependency>, String> {
        let content_str = String::from_utf8_lossy(content);
        let lockfile: serde_yaml::Value = serde_yaml::from_str(&content_str)
            .map_err(|e| format!("Failed to parse pnpm-lock.yaml: {}", e))?;

        let mut dependencies = Vec::new();

        // pnpm uses "packages" key
        if let Some(packages) = lockfile.get("packages").and_then(|p| p.as_mapping()) {
            for (path, pkg) in packages {
                let path_str = path.as_str().unwrap_or_default();

                // Parse package name and version from path
                // Format: /name@version or /@scope/name@version
                let (name, version) = self.parse_pnpm_package_path(path_str);

                let is_dev = pkg
                    .get("dev")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // Check for scripts
                let has_postinstall = pkg
                    .get("hasBin")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                    || pkg.get("scripts").is_some();

                let integrity_hash = pkg
                    .get("resolution")
                    .and_then(|r| r.get("integrity"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                dependencies.push(SnapshotDependency {
                    id: None,
                    snapshot_id: snapshot_id.to_string(),
                    name,
                    version,
                    is_direct: false, // Would need to check importers
                    is_dev,
                    has_postinstall,
                    postinstall_script: None,
                    integrity_hash,
                    resolved_url: None,
                });
            }
        }

        Ok(dependencies)
    }

    /// Parse pnpm package path to extract name and version
    fn parse_pnpm_package_path(&self, path: &str) -> (String, String) {
        // Remove leading slash
        let path = path.strip_prefix('/').unwrap_or(path);

        // Find the last @ that separates name from version
        if let Some(at_pos) = path.rfind('@') {
            if at_pos > 0 {
                let name = &path[..at_pos];
                let version = &path[at_pos + 1..];
                // Handle additional path components after version
                let version = version.split('/').next().unwrap_or(version);
                return (name.to_string(), version.to_string());
            }
        }

        (path.to_string(), "unknown".to_string())
    }

    /// Parse yarn.lock (v1 format - simplified)
    fn parse_yarn_lockfile(
        &self,
        content: &[u8],
        snapshot_id: &str,
    ) -> Result<Vec<SnapshotDependency>, String> {
        let content_str = String::from_utf8_lossy(content);
        let mut dependencies = Vec::new();
        let mut current_name = String::new();
        let mut current_version = String::new();
        let mut current_integrity = Option::<String>::None;

        for line in content_str.lines() {
            let line = line.trim();

            // Package declaration line
            if !line.starts_with(' ') && !line.starts_with('#') && line.ends_with(':') {
                // Parse package name from line like "package-name@version:"
                let spec = line.trim_end_matches(':');
                if let Some(at_pos) = spec.find('@') {
                    // Handle scoped packages (@scope/name)
                    if spec.starts_with('@') {
                        if let Some(second_at) = spec[1..].find('@') {
                            current_name = spec[..second_at + 1].to_string();
                        }
                    } else {
                        current_name = spec[..at_pos].to_string();
                    }
                }
            }

            // Version line
            if line.starts_with("version ") {
                current_version = line
                    .strip_prefix("version ")
                    .unwrap_or("")
                    .trim_matches('"')
                    .to_string();
            }

            // Integrity line
            if line.starts_with("integrity ") {
                current_integrity = Some(
                    line.strip_prefix("integrity ")
                        .unwrap_or("")
                        .trim_matches('"')
                        .to_string(),
                );
            }

            // Empty line indicates end of entry
            if line.is_empty() && !current_name.is_empty() && !current_version.is_empty() {
                dependencies.push(SnapshotDependency {
                    id: None,
                    snapshot_id: snapshot_id.to_string(),
                    name: current_name.clone(),
                    version: current_version.clone(),
                    is_direct: false,
                    is_dev: false,
                    has_postinstall: false,
                    postinstall_script: None,
                    integrity_hash: current_integrity.take(),
                    resolved_url: None,
                });
                current_name.clear();
                current_version.clear();
            }
        }

        Ok(dependencies)
    }

    /// Parse bun.lockb (binary format - returns empty for now)
    fn parse_bun_lockfile(
        &self,
        _content: &[u8],
        _snapshot_id: &str,
    ) -> Result<Vec<SnapshotDependency>, String> {
        // Bun lockfile is binary format, would need special handling
        // For now, return empty list
        log::warn!("Bun lockfile parsing not implemented, returning empty dependency list");
        Ok(Vec::new())
    }

    /// Compute SHA-256 hash of data
    fn compute_hash(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// Compute hash of dependency tree for change detection
    fn compute_dependency_tree_hash(&self, dependencies: &[SnapshotDependency]) -> String {
        let mut sorted_deps: Vec<_> = dependencies
            .iter()
            .map(|d| format!("{}@{}", d.name, d.version))
            .collect();
        sorted_deps.sort();

        let combined = sorted_deps.join("\n");
        self.compute_hash(combined.as_bytes())
    }

    /// Compute a security score based on dependencies (0-100)
    fn compute_security_score(&self, dependencies: &[SnapshotDependency]) -> i32 {
        let total = dependencies.len() as f64;
        if total == 0.0 {
            return 100;
        }

        let postinstall_count = dependencies.iter().filter(|d| d.has_postinstall).count() as f64;
        let without_integrity = dependencies.iter().filter(|d| d.integrity_hash.is_none()).count() as f64;

        // Simple scoring formula
        // - Postinstall scripts are risky (-2 points per script, max -30)
        // - Missing integrity hashes are concerning (-1 point per, max -20)
        let postinstall_penalty = (postinstall_count * 2.0).min(30.0);
        let integrity_penalty = (without_integrity / total * 20.0).min(20.0);

        let score = 100.0 - postinstall_penalty - integrity_penalty;
        score.max(0.0) as i32
    }

    /// Detect postinstall scripts in node_modules (parallel scanning)
    pub fn scan_postinstall_scripts(&self, project_path: &Path) -> Vec<PostinstallEntry> {
        let node_modules = project_path.join("node_modules");
        if !node_modules.exists() {
            return Vec::new();
        }

        // Get all package.json files in node_modules
        let package_jsons: Vec<_> = self.find_package_jsons(&node_modules);

        // Parallel scan for postinstall scripts
        package_jsons
            .par_iter()
            .filter_map(|path| self.extract_postinstall_script(path))
            .collect()
    }

    /// Find all package.json files in node_modules
    fn find_package_jsons(&self, node_modules: &Path) -> Vec<std::path::PathBuf> {
        let mut results = Vec::new();

        if let Ok(entries) = fs::read_dir(node_modules) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    let pkg_json = path.join("package.json");
                    if pkg_json.exists() {
                        results.push(pkg_json);
                    }

                    // Handle scoped packages
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name.starts_with('@') {
                        if let Ok(scoped_entries) = fs::read_dir(&path) {
                            for scoped_entry in scoped_entries.filter_map(|e| e.ok()) {
                                let scoped_pkg = scoped_entry.path().join("package.json");
                                if scoped_pkg.exists() {
                                    results.push(scoped_pkg);
                                }
                            }
                        }
                    }
                }
            }
        }

        results
    }

    /// Extract postinstall script from a package.json
    fn extract_postinstall_script(&self, package_json_path: &Path) -> Option<PostinstallEntry> {
        let content = fs::read_to_string(package_json_path).ok()?;
        let pkg: serde_json::Value = serde_json::from_str(&content).ok()?;

        let scripts = pkg.get("scripts")?.as_object()?;

        // Check for various install hooks
        let install_scripts = ["postinstall", "install", "preinstall"];
        for script_name in install_scripts {
            if let Some(script) = scripts.get(script_name).and_then(|s| s.as_str()) {
                let name = pkg.get("name")?.as_str()?.to_string();
                let version = pkg
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                return Some(PostinstallEntry {
                    package_name: name,
                    version,
                    script: script.to_string(),
                    script_hash: self.compute_hash(script.as_bytes()),
                });
            }
        }

        None
    }

    /// Check for typosquatting suspects using Levenshtein distance
    pub fn check_typosquatting(&self, dependencies: &[SnapshotDependency]) -> Vec<TyposquattingAlert> {
        // Popular packages to check against
        let popular_packages = [
            "lodash", "express", "react", "vue", "angular", "axios", "moment",
            "webpack", "babel", "eslint", "prettier", "typescript", "jest",
            "mocha", "chai", "underscore", "jquery", "bootstrap", "next",
            "gatsby", "nuxt", "electron", "socket.io", "mongoose", "sequelize",
        ];

        let dep_names: Vec<&str> = dependencies.iter().map(|d| d.name.as_str()).collect();

        dep_names
            .par_iter()
            .filter_map(|name| {
                for popular in &popular_packages {
                    if name == popular {
                        continue;
                    }

                    let distance = strsim::levenshtein(name, popular);
                    if distance > 0 && distance <= 2 {
                        let confidence = 1.0 - (distance as f64 / popular.len().max(name.len()) as f64);
                        if confidence >= 0.7 {
                            return Some(TyposquattingAlert {
                                package_name: name.to_string(),
                                similar_to: popular.to_string(),
                                distance: distance as u32,
                                confidence,
                            });
                        }
                    }
                }
                None
            })
            .collect()
    }

    /// Build security context for a snapshot
    pub fn build_security_context(
        &self,
        project_path: &Path,
        dependencies: &[SnapshotDependency],
    ) -> SecurityContext {
        let postinstall_scripts = self.scan_postinstall_scripts(project_path);
        let typosquatting_suspects = self.check_typosquatting(dependencies);

        SecurityContext {
            postinstall_scripts,
            typosquatting_suspects,
            integrity_issues: Vec::new(), // Would need additional analysis
        }
    }
}
