// Version management models
// Implements 006-node-package-manager feature

use crate::models::PackageManager;
use serde::{Deserialize, Serialize};

/// Version requirement source from package.json
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum VersionSource {
    /// From package.json volta field
    Volta,
    /// From package.json packageManager field
    PackageManager,
    /// From package.json engines field
    Engines,
    /// No version requirement found
    None,
}

/// Volta configuration from package.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoltaConfig {
    pub node: Option<String>,
    pub npm: Option<String>,
    pub yarn: Option<String>,
    pub pnpm: Option<String>,
}

/// Engines configuration from package.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnginesConfig {
    pub node: Option<String>,
    pub npm: Option<String>,
    pub yarn: Option<String>,
    pub pnpm: Option<String>,
}

/// Project version requirement (unified format)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionRequirement {
    /// Node.js version requirement (exact version or range)
    pub node: Option<String>,
    /// Package manager name
    pub package_manager_name: Option<PackageManager>,
    /// Package manager version (exact version)
    pub package_manager_version: Option<String>,
    /// Primary source of the version requirement (for backward compatibility)
    pub source: VersionSource,
    /// Source of Node.js version requirement
    pub node_source: Option<VersionSource>,
    /// Source of package manager version requirement
    pub package_manager_source: Option<VersionSource>,
}

impl Default for VersionRequirement {
    fn default() -> Self {
        Self {
            node: None,
            package_manager_name: None,
            package_manager_version: None,
            source: VersionSource::None,
            node_source: None,
            package_manager_source: None,
        }
    }
}

/// Tool installation status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    /// Whether the tool is available
    pub available: bool,
    /// Tool version
    pub version: Option<String>,
    /// Tool installation path
    pub path: Option<String>,
}

impl Default for ToolStatus {
    fn default() -> Self {
        Self {
            available: false,
            version: None,
            path: None,
        }
    }
}

/// System environment information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemEnvironment {
    /// Current Node.js version
    pub node_version: Option<String>,
    /// Current npm version
    pub npm_version: Option<String>,
    /// Current yarn version
    pub yarn_version: Option<String>,
    /// Current pnpm version
    pub pnpm_version: Option<String>,
    /// Volta installation status
    pub volta: ToolStatus,
    /// Corepack status
    pub corepack: ToolStatus,
}

impl Default for SystemEnvironment {
    fn default() -> Self {
        Self {
            node_version: None,
            npm_version: None,
            yarn_version: None,
            pnpm_version: None,
            volta: ToolStatus::default(),
            corepack: ToolStatus::default(),
        }
    }
}

/// Single compatibility check result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityItem {
    /// Whether compatible
    pub is_compatible: bool,
    /// Current version
    pub current: Option<String>,
    /// Required version
    pub required: Option<String>,
    /// Name (e.g., "node", "pnpm", "yarn", "npm")
    pub name: Option<String>,
    /// Incompatibility message
    pub message: Option<String>,
}

impl Default for CompatibilityItem {
    fn default() -> Self {
        Self {
            is_compatible: true,
            current: None,
            required: None,
            name: None,
            message: None,
        }
    }
}

/// Available version management tool
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum VersionTool {
    Volta,
    Corepack,
}

/// Recommended action when version mismatch
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum RecommendedAction {
    /// Execute directly, versions are compatible
    Execute,
    /// Use Volta to auto-switch
    UseVolta,
    /// Use Corepack to handle package manager
    UseCorepack,
    /// Show warning and let user decide
    WarnAndAsk,
}

/// Volta/Corepack conflict information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VoltaCorepackConflict {
    /// Whether a conflict is detected
    pub has_conflict: bool,
    /// Affected tools (yarn, pnpm, etc.)
    pub affected_tools: Vec<String>,
    /// Human-readable description of the conflict
    pub description: Option<String>,
    /// Suggested fix command
    pub fix_command: Option<String>,
}

/// Version compatibility check result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionCompatibility {
    /// Overall compatibility
    pub is_compatible: bool,
    /// Node.js compatibility
    pub node: CompatibilityItem,
    /// Package manager compatibility
    pub package_manager: CompatibilityItem,
    /// Available version management tools
    pub available_tools: Vec<VersionTool>,
    /// Recommended action
    pub recommended_action: RecommendedAction,
    /// Volta/Corepack conflict warning (if any)
    #[serde(default)]
    pub volta_corepack_conflict: Option<VoltaCorepackConflict>,
}

impl Default for VersionCompatibility {
    fn default() -> Self {
        Self {
            is_compatible: true,
            node: CompatibilityItem::default(),
            package_manager: CompatibilityItem::default(),
            available_tools: vec![],
            recommended_action: RecommendedAction::Execute,
            volta_corepack_conflict: None,
        }
    }
}
