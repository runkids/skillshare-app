// Lockfile Validation Engine
// Provides configurable rule-based validation for npm ecosystem lockfiles

use chrono::Utc;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::models::security_insight::InsightSeverity;
use crate::models::snapshot::SnapshotDependency;

// ============================================================================
// Configuration Models
// ============================================================================

/// Validation strictness level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ValidationStrictness {
    /// Only critical issues (blocked packages, insecure protocols)
    Relaxed,
    /// Default security checks
    #[default]
    Standard,
    /// All validations, warnings as errors
    Strict,
}

impl ValidationStrictness {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Relaxed => "relaxed",
            Self::Standard => "standard",
            Self::Strict => "strict",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "relaxed" => Some(Self::Relaxed),
            "standard" => Some(Self::Standard),
            "strict" => Some(Self::Strict),
            _ => None,
        }
    }
}

/// Individual validation rules that can be enabled/disabled
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationRuleSet {
    pub require_integrity: bool,
    pub require_https_resolved: bool,
    pub check_allowed_registries: bool,
    pub check_blocked_packages: bool,
    pub check_manifest_consistency: bool,
    pub enhanced_typosquatting: bool,
}

impl Default for ValidationRuleSet {
    fn default() -> Self {
        Self {
            require_integrity: true,
            require_https_resolved: true,
            check_allowed_registries: false,
            check_blocked_packages: true,
            check_manifest_consistency: true,
            enhanced_typosquatting: false,
        }
    }
}

/// Entry in the blocked packages list
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockedPackageEntry {
    pub name: String,
    pub reason: String,
    pub blocked_at: String,
}

impl BlockedPackageEntry {
    pub fn new(name: String, reason: String) -> Self {
        Self {
            name,
            reason,
            blocked_at: Utc::now().to_rfc3339(),
        }
    }
}

/// Complete lockfile validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockfileValidationConfig {
    pub enabled: bool,
    pub strictness: ValidationStrictness,
    pub rules: ValidationRuleSet,
    pub allowed_registries: Vec<String>,
    pub blocked_packages: Vec<BlockedPackageEntry>,
}

impl Default for LockfileValidationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            strictness: ValidationStrictness::Standard,
            rules: ValidationRuleSet::default(),
            allowed_registries: DEFAULT_ALLOWED_REGISTRIES
                .iter()
                .map(|s| s.to_string())
                .collect(),
            blocked_packages: Vec::new(),
        }
    }
}

// ============================================================================
// Validation Result Models
// ============================================================================

/// A single validation failure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationFailure {
    pub rule_id: String,
    pub package_name: String,
    pub message: String,
    pub severity: InsightSeverity,
    pub remediation: Option<String>,
}

/// A validation warning (less severe than failure)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationWarning {
    pub rule_id: String,
    pub package_name: Option<String>,
    pub message: String,
}

/// Summary statistics for validation result
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ValidationSummary {
    pub total_packages: i32,
    pub packages_with_issues: i32,
    pub critical_failures: i32,
    pub high_failures: i32,
    pub medium_failures: i32,
    pub warnings: i32,
    pub validation_passed: bool,
}

/// Complete validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResult {
    pub snapshot_id: String,
    pub passed_rules: Vec<String>,
    pub failures: Vec<ValidationFailure>,
    pub warnings: Vec<ValidationWarning>,
    pub summary: ValidationSummary,
}

impl ValidationResult {
    pub fn new(snapshot_id: String) -> Self {
        Self {
            snapshot_id,
            passed_rules: Vec::new(),
            failures: Vec::new(),
            warnings: Vec::new(),
            summary: ValidationSummary::default(),
        }
    }

    pub fn compute_summary(&mut self) {
        let packages_with_issues: HashSet<_> = self
            .failures
            .iter()
            .map(|f| &f.package_name)
            .collect();

        self.summary.packages_with_issues = packages_with_issues.len() as i32;
        self.summary.critical_failures = self
            .failures
            .iter()
            .filter(|f| f.severity == InsightSeverity::Critical)
            .count() as i32;
        self.summary.high_failures = self
            .failures
            .iter()
            .filter(|f| f.severity == InsightSeverity::High)
            .count() as i32;
        self.summary.medium_failures = self
            .failures
            .iter()
            .filter(|f| f.severity == InsightSeverity::Medium)
            .count() as i32;
        self.summary.warnings = self.warnings.len() as i32;
        self.summary.validation_passed = self.summary.critical_failures == 0
            && self.summary.high_failures == 0;
    }
}

// ============================================================================
// Constants
// ============================================================================

/// Default allowed registries
pub const DEFAULT_ALLOWED_REGISTRIES: &[&str] = &[
    "registry.npmjs.org",
    "npm.pkg.github.com",
    "registry.yarnpkg.com",
];

/// Top npm packages for typosquatting detection
/// This is a subset; full list should be loaded from external file
pub const POPULAR_PACKAGES: &[&str] = &[
    // Build tools
    "webpack", "vite", "rollup", "parcel", "esbuild", "swc", "turbo",
    // Frameworks
    "react", "vue", "angular", "svelte", "next", "nuxt", "gatsby", "remix",
    // React ecosystem
    "react-dom", "react-router", "react-router-dom", "redux", "zustand", "jotai", "recoil",
    // Vue ecosystem
    "vue-router", "vuex", "pinia",
    // Utilities
    "lodash", "underscore", "ramda", "date-fns", "moment", "dayjs", "luxon",
    // HTTP clients
    "axios", "node-fetch", "got", "ky", "superagent",
    // Testing
    "jest", "mocha", "chai", "vitest", "cypress", "playwright", "puppeteer",
    // Linting/formatting
    "eslint", "prettier", "stylelint", "biome",
    // TypeScript
    "typescript", "ts-node", "tsup", "tslib",
    // Node.js
    "express", "fastify", "koa", "hapi", "nest", "hono",
    // Database
    "mongoose", "sequelize", "prisma", "typeorm", "drizzle-orm", "knex",
    // Validation
    "zod", "yup", "joi", "ajv", "class-validator",
    // CSS
    "tailwindcss", "sass", "less", "postcss", "autoprefixer",
    // CLI tools
    "commander", "yargs", "inquirer", "chalk", "ora", "picocolors",
    // Other popular
    "uuid", "nanoid", "dotenv", "cross-env", "concurrently", "nodemon",
    "socket.io", "ws", "graphql", "apollo-server", "trpc",
];

// ============================================================================
// Validation Engine
// ============================================================================

/// Main validation engine
pub struct ValidationEngine {
    config: LockfileValidationConfig,
    popular_packages: HashSet<String>,
    blocked_packages: HashSet<String>,
}

impl ValidationEngine {
    /// Create a new validation engine with the given config
    pub fn new(config: LockfileValidationConfig) -> Self {
        let popular_packages: HashSet<String> = POPULAR_PACKAGES
            .iter()
            .map(|s| s.to_string())
            .collect();

        let blocked_packages: HashSet<String> = config
            .blocked_packages
            .iter()
            .map(|b| b.name.clone())
            .collect();

        Self {
            config,
            popular_packages,
            blocked_packages,
        }
    }

    /// Run all enabled validations on dependencies
    pub fn validate(
        &self,
        snapshot_id: &str,
        dependencies: &[SnapshotDependency],
        package_json: Option<&serde_json::Value>,
    ) -> ValidationResult {
        let mut result = ValidationResult::new(snapshot_id.to_string());
        result.summary.total_packages = dependencies.len() as i32;

        if !self.config.enabled {
            result.summary.validation_passed = true;
            return result;
        }

        // Run each enabled rule
        if self.config.rules.require_integrity {
            let failures = self.check_integrity(dependencies);
            if failures.is_empty() {
                result.passed_rules.push("require-integrity".to_string());
            }
            result.failures.extend(failures);
        }

        if self.config.rules.require_https_resolved {
            let failures = self.check_https_resolved(dependencies);
            if failures.is_empty() {
                result.passed_rules.push("require-https-resolved".to_string());
            }
            result.failures.extend(failures);
        }

        if self.config.rules.check_allowed_registries {
            let failures = self.check_allowed_registries(dependencies);
            if failures.is_empty() {
                result.passed_rules.push("check-allowed-registries".to_string());
            }
            result.failures.extend(failures);
        }

        if self.config.rules.check_blocked_packages {
            let failures = self.check_blocked_packages(dependencies);
            if failures.is_empty() {
                result.passed_rules.push("check-blocked-packages".to_string());
            }
            result.failures.extend(failures);
        }

        if self.config.rules.check_manifest_consistency {
            if let Some(pkg_json) = package_json {
                let failures = self.check_manifest_consistency(dependencies, pkg_json);
                if failures.is_empty() {
                    result.passed_rules.push("check-manifest-consistency".to_string());
                }
                result.failures.extend(failures);
            }
        }

        if self.config.rules.enhanced_typosquatting {
            let failures = self.check_typosquatting_enhanced(dependencies);
            if failures.is_empty() {
                result.passed_rules.push("enhanced-typosquatting".to_string());
            }
            result.failures.extend(failures);
        }

        // Apply strictness level
        match self.config.strictness {
            ValidationStrictness::Relaxed => {
                // Only keep critical and high severity issues
                result.failures.retain(|f| {
                    f.severity == InsightSeverity::Critical || f.severity == InsightSeverity::High
                });
                result.warnings.clear();
            }
            ValidationStrictness::Strict => {
                // Elevate warnings to failures
                for warning in result.warnings.drain(..) {
                    result.failures.push(ValidationFailure {
                        rule_id: warning.rule_id,
                        package_name: warning.package_name.unwrap_or_default(),
                        message: warning.message,
                        severity: InsightSeverity::Medium,
                        remediation: None,
                    });
                }
            }
            ValidationStrictness::Standard => {
                // Keep as-is
            }
        }

        result.compute_summary();
        result
    }

    /// Check for packages missing integrity hashes
    fn check_integrity(&self, deps: &[SnapshotDependency]) -> Vec<ValidationFailure> {
        deps.par_iter()
            .filter_map(|dep| {
                if dep.integrity_hash.is_none() {
                    Some(ValidationFailure {
                        rule_id: "require-integrity".to_string(),
                        package_name: dep.name.clone(),
                        message: format!("Package '{}' is missing integrity hash", dep.name),
                        severity: InsightSeverity::Medium,
                        remediation: Some("Regenerate lockfile with 'npm install' or 'pnpm install'".to_string()),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check for insecure protocols (git://, http://)
    fn check_https_resolved(&self, deps: &[SnapshotDependency]) -> Vec<ValidationFailure> {
        deps.par_iter()
            .filter_map(|dep| {
                let url = dep.resolved_url.as_ref()?;

                if url.starts_with("git://") {
                    Some(ValidationFailure {
                        rule_id: "require-https-resolved".to_string(),
                        package_name: dep.name.clone(),
                        message: format!("Package '{}' uses insecure git:// protocol", dep.name),
                        severity: InsightSeverity::High,
                        remediation: Some("Use git+https:// or https:// instead".to_string()),
                    })
                } else if url.starts_with("http://") && !url.contains("localhost") && !url.contains("127.0.0.1") {
                    Some(ValidationFailure {
                        rule_id: "require-https-resolved".to_string(),
                        package_name: dep.name.clone(),
                        message: format!("Package '{}' resolves over unencrypted HTTP", dep.name),
                        severity: InsightSeverity::Medium,
                        remediation: Some("Use HTTPS registry URL".to_string()),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check for packages from non-whitelisted registries
    fn check_allowed_registries(&self, deps: &[SnapshotDependency]) -> Vec<ValidationFailure> {
        let allowed: HashSet<&str> = self.config.allowed_registries.iter().map(|s| s.as_str()).collect();

        deps.par_iter()
            .filter_map(|dep| {
                let url = dep.resolved_url.as_ref()?;
                let host = extract_registry_host(url)?;

                if !allowed.contains(host.as_str()) {
                    Some(ValidationFailure {
                        rule_id: "check-allowed-registries".to_string(),
                        package_name: dep.name.clone(),
                        message: format!("Package '{}' from unexpected registry: {}", dep.name, host),
                        severity: InsightSeverity::Medium,
                        remediation: Some(format!("Add '{}' to allowed registries or verify package source", host)),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check for blocked packages
    fn check_blocked_packages(&self, deps: &[SnapshotDependency]) -> Vec<ValidationFailure> {
        deps.par_iter()
            .filter_map(|dep| {
                if self.blocked_packages.contains(&dep.name) {
                    let reason = self.config.blocked_packages
                        .iter()
                        .find(|b| b.name == dep.name)
                        .map(|b| b.reason.clone())
                        .unwrap_or_else(|| "Package is on block list".to_string());

                    Some(ValidationFailure {
                        rule_id: "check-blocked-packages".to_string(),
                        package_name: dep.name.clone(),
                        message: format!("Blocked package '{}': {}", dep.name, reason),
                        severity: InsightSeverity::Critical,
                        remediation: Some("Remove this package from your dependencies".to_string()),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check manifest consistency between package.json and lockfile
    fn check_manifest_consistency(
        &self,
        deps: &[SnapshotDependency],
        package_json: &serde_json::Value,
    ) -> Vec<ValidationFailure> {
        let mut failures = Vec::new();

        // Get declared dependencies from package.json
        let mut declared_deps: HashSet<String> = HashSet::new();
        if let Some(dependencies) = package_json.get("dependencies").and_then(|d| d.as_object()) {
            declared_deps.extend(dependencies.keys().cloned());
        }
        if let Some(dev_deps) = package_json.get("devDependencies").and_then(|d| d.as_object()) {
            declared_deps.extend(dev_deps.keys().cloned());
        }

        // Get direct dependencies from lockfile
        let lockfile_direct: HashSet<_> = deps
            .iter()
            .filter(|d| d.is_direct)
            .map(|d| d.name.clone())
            .collect();

        // Check for missing lockfile entries
        for declared in &declared_deps {
            if !lockfile_direct.contains(declared) && !deps.iter().any(|d| &d.name == declared) {
                failures.push(ValidationFailure {
                    rule_id: "check-manifest-consistency".to_string(),
                    package_name: declared.clone(),
                    message: format!("Package '{}' in package.json but not in lockfile", declared),
                    severity: InsightSeverity::High,
                    remediation: Some("Run 'npm install' to regenerate lockfile".to_string()),
                });
            }
        }

        failures
    }

    /// Enhanced typosquatting detection with scope confusion and homoglyphs
    fn check_typosquatting_enhanced(&self, deps: &[SnapshotDependency]) -> Vec<ValidationFailure> {
        deps.par_iter()
            .filter_map(|dep| {
                // Skip if it's a popular package
                if self.popular_packages.contains(&dep.name) {
                    return None;
                }

                // Check Levenshtein distance
                for popular in &self.popular_packages {
                    let distance = strsim::levenshtein(&dep.name, popular);
                    if distance > 0 && distance <= 2 {
                        return Some(ValidationFailure {
                            rule_id: "enhanced-typosquatting".to_string(),
                            package_name: dep.name.clone(),
                            message: format!(
                                "Package '{}' is similar to popular package '{}' (distance: {})",
                                dep.name, popular, distance
                            ),
                            severity: InsightSeverity::High,
                            remediation: Some(format!("Verify you intended to install '{}' and not '{}'", dep.name, popular)),
                        });
                    }
                }

                // Check scope confusion
                if let Some(variant) = check_scope_confusion(&dep.name) {
                    if self.popular_packages.contains(&variant) {
                        return Some(ValidationFailure {
                            rule_id: "enhanced-typosquatting".to_string(),
                            package_name: dep.name.clone(),
                            message: format!(
                                "Package '{}' may be a scope confusion attack targeting '{}'",
                                dep.name, variant
                            ),
                            severity: InsightSeverity::High,
                            remediation: Some(format!("Verify package scope: did you mean '{}'?", variant)),
                        });
                    }
                }

                // Check homoglyphs
                if let Some(normalized) = normalize_homoglyphs(&dep.name) {
                    if normalized != dep.name && self.popular_packages.contains(&normalized) {
                        return Some(ValidationFailure {
                            rule_id: "enhanced-typosquatting".to_string(),
                            package_name: dep.name.clone(),
                            message: format!(
                                "Package '{}' contains lookalike characters, similar to '{}'",
                                dep.name, normalized
                            ),
                            severity: InsightSeverity::High,
                            remediation: Some(format!("Verify package name: did you mean '{}'?", normalized)),
                        });
                    }
                }

                None
            })
            .collect()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract registry host from resolved URL
pub fn extract_registry_host(resolved_url: &str) -> Option<String> {
    // Handle git+https:// prefix
    let url_str = resolved_url
        .strip_prefix("git+")
        .unwrap_or(resolved_url);

    url::Url::parse(url_str)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
}

/// Check for scope confusion (@scope/pkg vs scope-pkg)
pub fn check_scope_confusion(name: &str) -> Option<String> {
    if name.starts_with('@') {
        // @scope/pkg -> scope-pkg
        let without_at = &name[1..];
        if let Some(slash_pos) = without_at.find('/') {
            let scope = &without_at[..slash_pos];
            let pkg = &without_at[slash_pos + 1..];
            return Some(format!("{}-{}", scope, pkg));
        }
    } else if name.contains('-') {
        // scope-pkg -> @scope/pkg
        if let Some(dash_pos) = name.find('-') {
            let scope = &name[..dash_pos];
            let pkg = &name[dash_pos + 1..];
            return Some(format!("@{}/{}", scope, pkg));
        }
    }
    None
}

/// Normalize homoglyphs for lookalike detection
pub fn normalize_homoglyphs(name: &str) -> Option<String> {
    let mut normalized = name.to_lowercase();
    let original = normalized.clone();

    // Common lookalike replacements
    normalized = normalized.replace('0', "o");
    normalized = normalized.replace('1', "l");
    normalized = normalized.replace("rn", "m");
    normalized = normalized.replace("vv", "w");
    normalized = normalized.replace("cl", "d");
    normalized = normalized.replace("nn", "m");

    if normalized != original {
        Some(normalized)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_registry_host() {
        assert_eq!(
            extract_registry_host("https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz"),
            Some("registry.npmjs.org".to_string())
        );
        assert_eq!(
            extract_registry_host("git+https://github.com/user/repo.git"),
            Some("github.com".to_string())
        );
        assert_eq!(
            extract_registry_host("git://github.com/user/repo.git"),
            Some("github.com".to_string())
        );
    }

    #[test]
    fn test_check_scope_confusion() {
        assert_eq!(
            check_scope_confusion("@types/react"),
            Some("types-react".to_string())
        );
        assert_eq!(
            check_scope_confusion("types-react"),
            Some("@types/react".to_string())
        );
        assert_eq!(check_scope_confusion("lodash"), None);
    }

    #[test]
    fn test_normalize_homoglyphs() {
        assert_eq!(
            normalize_homoglyphs("l0dash"),
            Some("lodash".to_string())
        );
        assert_eq!(
            normalize_homoglyphs("rnornent"),
            Some("moment".to_string())
        );
        assert_eq!(normalize_homoglyphs("lodash"), None);
    }

    #[test]
    fn test_validation_engine_blocked_packages() {
        let mut config = LockfileValidationConfig::default();
        config.enabled = true;
        config.blocked_packages.push(BlockedPackageEntry::new(
            "malicious-pkg".to_string(),
            "Known malware".to_string(),
        ));

        let engine = ValidationEngine::new(config);
        let deps = vec![
            SnapshotDependency {
                id: None,
                snapshot_id: "test".to_string(),
                name: "malicious-pkg".to_string(),
                version: "1.0.0".to_string(),
                is_direct: true,
                is_dev: false,
                has_postinstall: false,
                postinstall_script: None,
                integrity_hash: Some("sha512-...".to_string()),
                resolved_url: Some("https://registry.npmjs.org/malicious-pkg".to_string()),
            },
        ];

        let result = engine.validate("test-snapshot", &deps, None);
        assert_eq!(result.failures.len(), 1);
        assert_eq!(result.failures[0].severity, InsightSeverity::Critical);
    }

    #[test]
    fn test_validation_engine_insecure_protocol() {
        let mut config = LockfileValidationConfig::default();
        config.enabled = true;

        let engine = ValidationEngine::new(config);
        let deps = vec![
            SnapshotDependency {
                id: None,
                snapshot_id: "test".to_string(),
                name: "some-pkg".to_string(),
                version: "1.0.0".to_string(),
                is_direct: true,
                is_dev: false,
                has_postinstall: false,
                postinstall_script: None,
                integrity_hash: Some("sha512-...".to_string()),
                resolved_url: Some("git://github.com/user/repo.git".to_string()),
            },
        ];

        let result = engine.validate("test-snapshot", &deps, None);
        assert!(result.failures.iter().any(|f| f.rule_id == "require-https-resolved"));
    }
}
