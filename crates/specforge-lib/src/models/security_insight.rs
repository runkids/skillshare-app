// Security Insight Models
// Tracks security-related observations from snapshot analysis

use serde::{Deserialize, Serialize};

/// Type of security insight
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InsightType {
    // Existing types
    NewDependency,
    RemovedDependency,
    VersionChange,
    PostinstallAdded,
    PostinstallRemoved,
    PostinstallChanged,
    IntegrityMismatch,
    TyposquattingSuspect,
    FrequentUpdater,
    SuspiciousScript,
    // Lockfile validation types (v7)
    InsecureProtocol,     // git:// or http:// resolved URL
    UnexpectedRegistry,   // Package from non-whitelisted registry
    ManifestMismatch,     // Lockfile doesn't match package.json
    BlockedPackage,       // Package on blocked list
    MissingIntegrity,     // No integrity hash present
    ScopeConfusion,       // @scope/pkg vs scope-pkg typosquatting
    HomoglyphSuspect,     // Visually similar characters in name
}

impl InsightType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NewDependency => "new_dependency",
            Self::RemovedDependency => "removed_dependency",
            Self::VersionChange => "version_change",
            Self::PostinstallAdded => "postinstall_added",
            Self::PostinstallRemoved => "postinstall_removed",
            Self::PostinstallChanged => "postinstall_changed",
            Self::IntegrityMismatch => "integrity_mismatch",
            Self::TyposquattingSuspect => "typosquatting_suspect",
            Self::FrequentUpdater => "frequent_updater",
            Self::SuspiciousScript => "suspicious_script",
            // Lockfile validation types (v7)
            Self::InsecureProtocol => "insecure_protocol",
            Self::UnexpectedRegistry => "unexpected_registry",
            Self::ManifestMismatch => "manifest_mismatch",
            Self::BlockedPackage => "blocked_package",
            Self::MissingIntegrity => "missing_integrity",
            Self::ScopeConfusion => "scope_confusion",
            Self::HomoglyphSuspect => "homoglyph_suspect",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "new_dependency" => Some(Self::NewDependency),
            "removed_dependency" => Some(Self::RemovedDependency),
            "version_change" => Some(Self::VersionChange),
            "postinstall_added" => Some(Self::PostinstallAdded),
            "postinstall_removed" => Some(Self::PostinstallRemoved),
            "postinstall_changed" => Some(Self::PostinstallChanged),
            "integrity_mismatch" => Some(Self::IntegrityMismatch),
            "typosquatting_suspect" => Some(Self::TyposquattingSuspect),
            "frequent_updater" => Some(Self::FrequentUpdater),
            "suspicious_script" => Some(Self::SuspiciousScript),
            // Lockfile validation types (v7)
            "insecure_protocol" => Some(Self::InsecureProtocol),
            "unexpected_registry" => Some(Self::UnexpectedRegistry),
            "manifest_mismatch" => Some(Self::ManifestMismatch),
            "blocked_package" => Some(Self::BlockedPackage),
            "missing_integrity" => Some(Self::MissingIntegrity),
            "scope_confusion" => Some(Self::ScopeConfusion),
            "homoglyph_suspect" => Some(Self::HomoglyphSuspect),
            _ => None,
        }
    }
}

/// Severity level for security insights
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum InsightSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl InsightSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "info" => Some(Self::Info),
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "critical" => Some(Self::Critical),
            _ => None,
        }
    }
}

/// A security insight generated from snapshot analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityInsight {
    pub id: String,
    pub snapshot_id: String,
    pub insight_type: InsightType,
    pub severity: InsightSeverity,
    pub title: String,
    pub description: String,
    pub package_name: Option<String>,
    pub previous_value: Option<String>,
    pub current_value: Option<String>,
    pub recommendation: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub is_dismissed: bool,
    pub created_at: String,
}

/// Security insight summary for a snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightSummary {
    pub total: i32,
    pub critical: i32,
    pub high: i32,
    pub medium: i32,
    pub low: i32,
    pub info: i32,
    pub dismissed: i32,
}

impl Default for InsightSummary {
    fn default() -> Self {
        Self {
            total: 0,
            critical: 0,
            high: 0,
            medium: 0,
            low: 0,
            info: 0,
            dismissed: 0,
        }
    }
}

/// Frequent updater detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrequentUpdater {
    pub package_name: String,
    pub update_count: i32,
    pub time_span_days: i32,
    pub versions: Vec<String>,
}

/// Dependency health assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyHealth {
    pub package_name: String,
    pub version: String,
    pub health_score: i32,
    pub factors: Vec<HealthFactor>,
}

/// Health factor contributing to overall health score
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthFactor {
    pub name: String,
    pub score: i32,
    pub max_score: i32,
    pub description: String,
}

/// Create insight request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInsightRequest {
    pub snapshot_id: String,
    pub insight_type: InsightType,
    pub severity: InsightSeverity,
    pub title: String,
    pub description: String,
    pub package_name: Option<String>,
    pub previous_value: Option<String>,
    pub current_value: Option<String>,
    pub recommendation: Option<String>,
    pub metadata: Option<serde_json::Value>,
}
