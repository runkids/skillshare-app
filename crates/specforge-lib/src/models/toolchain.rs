// Toolchain models for Node.js toolchain conflict detection
// Feature: 017-toolchain-conflict-detection

use serde::{Deserialize, Serialize};

/// Toolchain execution strategy selection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolchainStrategy {
    /// Use Volta for both Node.js and package manager
    /// Command: volta run --node <ver> --<pm> <ver> <cmd>
    VoltaPriority,
    /// Use Corepack for package manager (direct execution)
    /// Command: <pm> <cmd> (let Corepack handle it)
    CorepackPriority,
    /// Volta for Node.js, Corepack for package manager
    /// Command: volta run --node <ver> <pm> <cmd>
    Hybrid,
    /// No special handling, use system defaults
    /// Command: <pm> <cmd>
    SystemDefault,
}

impl Default for ToolchainStrategy {
    fn default() -> Self {
        ToolchainStrategy::SystemDefault
    }
}

/// Toolchain conflict type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolchainConflictType {
    /// No conflict detected
    None,
    /// package.json has both volta and packageManager fields
    DualConfig {
        volta_node: Option<String>,
        volta_pm: Option<String>,
        package_manager: String,
    },
    /// Corepack shim overwrites Volta shim
    ShimOverwrite {
        affected_tools: Vec<String>,
        fix_command: String,
    },
    /// Volta not installed but project has volta config
    VoltaMissing,
    /// Corepack not enabled but project has packageManager config
    CorepackDisabled,
}

/// Parsed package manager information from packageManager field
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParsedPackageManager {
    pub name: String,
    pub version: String,
    pub hash: Option<String>,
}

/// Volta configuration from package.json (for toolchain detection)
/// Named differently from version::VoltaConfig to avoid conflict
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoltaToolchainConfig {
    pub node: Option<String>,
    pub npm: Option<String>,
    pub yarn: Option<String>,
    pub pnpm: Option<String>,
}

/// Project toolchain configuration parsed from package.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainConfig {
    /// Volta configuration (from package.json.volta)
    pub volta: Option<VoltaToolchainConfig>,
    /// packageManager field value (e.g., "pnpm@9.15.0+sha512.xxx")
    pub package_manager: Option<String>,
    /// Parsed package manager information
    pub parsed_package_manager: Option<ParsedPackageManager>,
}

/// Toolchain conflict detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainConflictResult {
    /// Whether a conflict was detected
    pub has_conflict: bool,
    /// Type of conflict
    pub conflict_type: ToolchainConflictType,
    /// Human-readable conflict description
    pub description: Option<String>,
    /// Suggested strategy options
    pub suggested_strategies: Vec<ToolchainStrategy>,
    /// Recommended default strategy
    pub recommended_strategy: ToolchainStrategy,
}

/// Project toolchain preference stored in settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectPreference {
    /// Project path (unique identifier)
    pub project_path: String,
    /// Selected strategy
    pub strategy: ToolchainStrategy,
    /// Whether to remember this choice
    pub remember: bool,
    /// Last updated timestamp (ISO 8601)
    pub updated_at: String,
}

/// Toolchain error codes for humanization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ToolchainErrorCode {
    VersionNotFound,
    NetworkError,
    CorepackDisabled,
    PmNotInstalled,
    Unknown,
}

/// Humanized toolchain error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainError {
    /// Error code for categorization
    pub code: ToolchainErrorCode,
    /// User-friendly message
    pub message: String,
    /// Suggested solution
    pub suggestion: Option<String>,
    /// Command to run for fix
    pub command: Option<String>,
}

/// Environment diagnostics information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentDiagnostics {
    /// Volta status
    pub volta: VoltaInfo,
    /// Corepack status
    pub corepack: CorepackInfo,
    /// System Node.js
    pub system_node: SystemNodeInfo,
    /// Installed package managers
    pub package_managers: PackageManagersInfo,
    /// PATH order analysis
    pub path_analysis: PathAnalysis,
}

/// Volta environment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoltaInfo {
    pub available: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub shim_path: Option<String>,
}

/// Corepack environment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorepackInfo {
    pub available: bool,
    pub enabled: bool,
    pub version: Option<String>,
    pub path: Option<String>,
}

/// System Node.js information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemNodeInfo {
    pub version: Option<String>,
    pub path: Option<String>,
}

/// Package managers information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManagersInfo {
    pub npm: Option<ToolVersionInfo>,
    pub pnpm: Option<ToolVersionInfo>,
    pub yarn: Option<ToolVersionInfo>,
}

/// Tool version and path information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolVersionInfo {
    pub version: String,
    pub path: String,
}

/// PATH order analysis for shim priority detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathAnalysis {
    pub volta_first: bool,
    pub corepack_first: bool,
    pub order: Vec<String>,
}

/// Build command result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildCommandResult {
    /// The command to execute
    pub command: String,
    /// Command arguments
    pub args: Vec<String>,
    /// Strategy that was used
    pub strategy_used: ToolchainStrategy,
    /// Whether Volta wrapping is active
    pub using_volta: bool,
    /// Whether Corepack is handling package manager
    pub using_corepack: bool,
}
