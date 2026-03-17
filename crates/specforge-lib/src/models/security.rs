// Security audit data models
// Represents security scan results for Node.js packages

use serde::{Deserialize, Serialize};

// Re-export PackageManager from project module for consistency
pub use super::project::PackageManager;

/// Vulnerability severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Moderate,
    Low,
    Info,
}

impl Default for Severity {
    fn default() -> Self {
        Severity::Info
    }
}

/// Security scan status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ScanStatus {
    Pending,
    Running,
    Success,
    Failed,
}

impl Default for ScanStatus {
    fn default() -> Self {
        ScanStatus::Pending
    }
}

/// Error codes for scan failures
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScanErrorCode {
    CliNotFound,
    NoLockfile,
    NoNodeModules,
    NetworkError,
    ParseError,
    Timeout,
    Unknown,
}

/// Vulnerability count summary by severity
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VulnSummary {
    pub total: u32,
    pub critical: u32,
    pub high: u32,
    pub moderate: u32,
    pub low: u32,
    pub info: u32,
}

impl VulnSummary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_vulnerabilities(vulns: &[VulnItem]) -> Self {
        let mut summary = Self::new();
        for vuln in vulns {
            summary.total += 1;
            match vuln.severity {
                Severity::Critical => summary.critical += 1,
                Severity::High => summary.high += 1,
                Severity::Moderate => summary.moderate += 1,
                Severity::Low => summary.low += 1,
                Severity::Info => summary.info += 1,
            }
        }
        summary
    }
}

/// Dependency count statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DependencyCount {
    pub prod: u32,
    pub dev: u32,
    pub optional: u32,
    pub peer: u32,
    pub total: u32,
}

/// CVSS (Common Vulnerability Scoring System) information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CvssInfo {
    /// CVSS score (0.0 - 10.0)
    pub score: f32,
    /// CVSS vector string
    pub vector: String,
}

/// Fix information for a vulnerability
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixInfo {
    /// Package that needs to be updated
    pub package: String,
    /// Suggested version to update to
    pub version: String,
    /// Whether this is a major version update
    pub is_major_update: bool,
}

/// Scan error details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanError {
    /// Error code
    pub code: ScanErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Additional error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    /// Suggested solution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl ScanError {
    pub fn cli_not_found(pm: &str) -> Self {
        Self {
            code: ScanErrorCode::CliNotFound,
            message: format!("{} is not installed", pm),
            details: None,
            suggestion: Some(format!("Please install {} and try again", pm)),
        }
    }

    pub fn no_lockfile(pm: &str) -> Self {
        Self {
            code: ScanErrorCode::NoLockfile,
            message: "Lock file not found".to_string(),
            details: None,
            suggestion: Some(format!("Please run {} install to create a lock file", pm)),
        }
    }

    pub fn no_node_modules() -> Self {
        Self {
            code: ScanErrorCode::NoNodeModules,
            message: "node_modules directory not found".to_string(),
            details: None,
            suggestion: Some(
                "Please run npm/pnpm/yarn install to install dependencies".to_string(),
            ),
        }
    }

    pub fn network_error(details: Option<String>) -> Self {
        Self {
            code: ScanErrorCode::NetworkError,
            message: "Cannot connect to registry".to_string(),
            details,
            suggestion: Some("Please check your network connection and try again".to_string()),
        }
    }

    pub fn parse_error(details: Option<String>) -> Self {
        Self {
            code: ScanErrorCode::ParseError,
            message: "Failed to parse audit output".to_string(),
            details,
            suggestion: Some(
                "Please try scanning again, or check your package manager version".to_string(),
            ),
        }
    }

    pub fn timeout() -> Self {
        Self {
            code: ScanErrorCode::Timeout,
            message: "Scan timed out".to_string(),
            details: None,
            suggestion: Some("The project may be too large. Please try again later".to_string()),
        }
    }

    pub fn unknown(message: String) -> Self {
        Self {
            code: ScanErrorCode::Unknown,
            message,
            details: None,
            suggestion: None,
        }
    }
}

/// Individual vulnerability item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VulnItem {
    /// Advisory ID (numeric or GHSA ID)
    pub id: String,
    /// Affected package name
    pub package_name: String,
    /// Currently installed version
    pub installed_version: String,
    /// Vulnerability severity
    pub severity: Severity,
    /// Vulnerability title
    pub title: String,
    /// Detailed description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Remediation recommendation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<String>,
    /// Advisory URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advisory_url: Option<String>,
    /// CVE identifiers
    #[serde(default)]
    pub cves: Vec<String>,
    /// CWE identifiers
    #[serde(default)]
    pub cwes: Vec<String>,
    /// CVSS scoring information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cvss: Option<CvssInfo>,
    /// Vulnerable version range
    pub vulnerable_versions: String,
    /// Patched version range
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patched_versions: Option<String>,
    /// Dependency paths (multiple paths possible)
    pub paths: Vec<Vec<String>>,
    /// Whether this is a direct dependency
    pub is_direct: bool,
    /// Whether a fix is available
    pub fix_available: bool,
    /// Fix details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_info: Option<FixInfo>,
    /// Workspace packages affected by this vulnerability (for monorepos)
    #[serde(default)]
    pub workspace_packages: Vec<String>,
}

/// Security summary for a single workspace package in a monorepo
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceVulnSummary {
    /// Workspace package name
    pub package_name: String,
    /// Workspace relative path
    pub relative_path: String,
    /// Vulnerability counts for this workspace
    pub summary: VulnSummary,
    /// IDs of vulnerabilities affecting this workspace
    pub vulnerability_ids: Vec<String>,
}

/// Complete scan result for a single scan operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VulnScanResult {
    /// Unique scan ID (UUID)
    pub id: String,
    /// Associated project ID
    pub project_id: String,
    /// Scan timestamp (ISO 8601)
    pub scanned_at: String,
    /// Scan status
    pub status: ScanStatus,
    /// Package manager used
    pub package_manager: PackageManager,
    /// Package manager version
    pub package_manager_version: String,
    /// Vulnerability summary by severity
    pub summary: VulnSummary,
    /// List of vulnerabilities
    pub vulnerabilities: Vec<VulnItem>,
    /// Dependency statistics
    pub dependency_count: DependencyCount,
    /// Error information (only when status is failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ScanError>,
    /// Per-workspace vulnerability summaries (for monorepos)
    #[serde(default)]
    pub workspace_summaries: Vec<WorkspaceVulnSummary>,
}

impl VulnScanResult {
    pub fn new(project_id: String, package_manager: PackageManager) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            project_id,
            scanned_at: chrono::Utc::now().to_rfc3339(),
            status: ScanStatus::Running,
            package_manager,
            package_manager_version: String::new(),
            summary: VulnSummary::default(),
            vulnerabilities: Vec::new(),
            dependency_count: DependencyCount::default(),
            error: None,
            workspace_summaries: Vec::new(),
        }
    }

    pub fn success(
        mut self,
        vulnerabilities: Vec<VulnItem>,
        dependency_count: DependencyCount,
    ) -> Self {
        self.status = ScanStatus::Success;
        self.summary = VulnSummary::from_vulnerabilities(&vulnerabilities);
        self.vulnerabilities = vulnerabilities;
        self.dependency_count = dependency_count;
        self
    }

    pub fn failed(mut self, error: ScanError) -> Self {
        self.status = ScanStatus::Failed;
        self.error = Some(error);
        self
    }
}

/// Security scan data for a project (stored in Store)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityScanData {
    /// Associated project ID
    pub project_id: String,
    /// Detected package manager
    pub package_manager: PackageManager,
    /// Most recent scan result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_scan: Option<VulnScanResult>,
    /// Scan history (up to 10 entries)
    #[serde(default)]
    pub scan_history: Vec<VulnScanResult>,
    /// Snooze reminder until this timestamp (ISO 8601)
    /// When set, scan reminders won't be shown until this time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snooze_until: Option<String>,
}

impl SecurityScanData {
    pub fn new(project_id: String, package_manager: PackageManager) -> Self {
        Self {
            project_id,
            package_manager,
            last_scan: None,
            scan_history: Vec::new(),
            snooze_until: None,
        }
    }

    /// Update with new scan result, maintaining history limit
    pub fn update_scan(&mut self, result: VulnScanResult) {
        // Move current last_scan to history
        if let Some(prev_scan) = self.last_scan.take() {
            self.scan_history.insert(0, prev_scan);
            // Keep only 10 entries in history
            if self.scan_history.len() > 10 {
                self.scan_history.truncate(10);
            }
        }
        self.last_scan = Some(result);
    }
}

/// Summary information for dashboard display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityScanSummary {
    pub project_id: String,
    pub project_name: String,
    pub project_path: String,
    pub package_manager: PackageManager,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_scanned_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<VulnSummary>,
    pub status: ScanStatus,
}
