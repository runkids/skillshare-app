// Monorepo Tool Support Models
// Feature: 008-monorepo-support

use serde::{Deserialize, Serialize};

/// Monorepo tool type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MonorepoToolType {
    Nx,
    Turbo,
    Workspaces,
    Lerna,
    Unknown,
}

impl Default for MonorepoToolType {
    fn default() -> Self {
        MonorepoToolType::Unknown
    }
}

/// Information about a detected monorepo tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonorepoToolInfo {
    #[serde(rename = "type")]
    pub tool_type: MonorepoToolType,
    pub version: Option<String>,
    pub config_path: String,
    pub is_available: bool,
}

/// Dependency graph structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyGraph {
    pub nodes: Vec<DependencyNode>,
    pub edges: Vec<DependencyEdge>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycles: Option<Vec<Vec<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affected_nodes: Option<Vec<String>>,
}

/// A node in the dependency graph
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyNode {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub root: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    pub scripts_count: u32,
}

/// An edge in the dependency graph
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyEdge {
    pub source: String,
    pub target: String,
    #[serde(rename = "type")]
    pub edge_type: String,
}

/// Nx target information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxTarget {
    pub name: String,
    pub projects: Vec<String>,
    pub cached: bool,
}

/// Turborepo pipeline information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurboPipeline {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    pub cache: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Vec<String>>,
}

/// Turborepo cache status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurboCacheStatus {
    pub total_size: String,
    pub hit_rate: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_cleared: Option<String>,
}

/// Batch execution result for a single package
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchExecutionResult {
    pub package_name: String,
    pub success: bool,
    pub exit_code: i32,
    pub duration: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for detect_monorepo_tools command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectMonorepoToolsResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<MonorepoToolInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<MonorepoToolType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for get_dependency_graph command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDependencyGraphResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<DependencyGraph>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for get_nx_targets command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNxTargetsResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<NxTarget>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for get_turbo_pipelines command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTurboPipelinesResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipelines: Option<Vec<TurboPipeline>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for run_nx_command command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunNxCommandResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for run_turbo_command command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunTurboCommandResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for get_turbo_cache_status command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTurboCacheStatusResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<TurboCacheStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for clear_turbo_cache command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearTurboCacheResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Nx cache status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxCacheStatus {
    pub total_size: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries: Option<u32>,
}

/// Response for get_nx_cache_status command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNxCacheStatusResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<NxCacheStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for clear_nx_cache command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearNxCacheResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for run_batch_scripts command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunBatchScriptsResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Event payload for batch progress
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchProgressPayload {
    pub execution_id: String,
    pub total: u32,
    pub completed: u32,
    pub running: Vec<String>,
    pub results: Vec<BatchExecutionResult>,
}

/// Event payload for batch completed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchCompletedPayload {
    pub execution_id: String,
    pub success: bool,
    pub results: Vec<BatchExecutionResult>,
    pub duration: u64,
}
