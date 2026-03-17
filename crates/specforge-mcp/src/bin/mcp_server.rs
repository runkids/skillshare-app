// SpecForge MCP Server
// Provides MCP (Model Context Protocol) tools for AI assistants like Claude Code
//
// Run with: cargo run --bin specforge-mcp
// Or install: cargo install --path . --bin specforge-mcp
//
// This server uses SQLite database for data storage with WAL mode for concurrent access.

// MCP server modules (extracted for maintainability)
mod mcp;
use mcp::{
    // Types
    types::*,
    // Security
    ToolCategory, get_tool_category, is_tool_allowed,
    // State
    RATE_LIMITER, TOOL_RATE_LIMITERS, ACTION_SEMAPHORE,
    // Templates
    get_builtin_templates,
    // Store (database access and local types)
    read_store_data, write_store_data, log_request, open_database, get_database_path,
    Project, Workflow, WorkflowNode, CustomStepTemplate,
    // Background process management
    BackgroundProcessStatus, BACKGROUND_PROCESS_MANAGER, CLEANUP_INTERVAL_SECS,
    // Instance management (smart multi-instance support)
    InstanceManager,
};

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use tokio::time::timeout as tokio_timeout;

use chrono::Utc;
use rmcp::{
    ErrorData as McpError,
    ServerHandler,
    handler::server::tool::{ToolCallContext, ToolRouter},
    handler::server::wrapper::Parameters,
    model::*,
    service::RequestContext,
    tool, tool_router,
};
use tokio::io::{stdin, stdout};
#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};
use uuid::Uuid;

// Import SQLite database and repositories
use specforge_lib::utils::database::Database;
use specforge_lib::repositories::{
    MCPActionRepository,
    // New repositories for enhanced MCP tools
    AIRepository, AIConversationRepository, NotificationRepository,
    SecurityRepository, DeployRepository, SnapshotRepository,
};

// Import MCP action models and services
use specforge_lib::models::mcp_action::{
    MCPActionType, PermissionLevel, ExecutionStatus, ActionFilter, ExecutionFilter,
};
use specforge_lib::services::mcp_action::create_executor;
use rusqlite::params;

// Import shared store utilities (for validation, etc.)
use specforge_lib::utils::shared_store::{
    // Error handling
    sanitize_error,
    // Input validation
    validate_path, validate_command, validate_string_length, validate_timeout,
    MAX_NAME_LENGTH, MAX_DESCRIPTION_LENGTH,
    // Output sanitization
    sanitize_output,
};

// Import MCP types from models
use specforge_lib::models::mcp::{MCPServerConfig, DevServerMode};

// Import snapshot services for Time Machine
use specforge_lib::services::snapshot::{
    SnapshotStorage, SnapshotDiffService, SnapshotReplayService, SnapshotSearchService,
    SnapshotCaptureService,
};

// Import replay types from service module
use specforge_lib::services::snapshot::replay::{ReplayOption, ExecuteReplayRequest};

// Import search types from service module
use specforge_lib::services::snapshot::search::{SnapshotSearchCriteria, ExportFormat};

// Import security guardian services
use specforge_lib::services::security_guardian::{
    DependencyIntegrityService, SecurityInsightsService,
};

// Import snapshot models
use specforge_lib::models::snapshot::SnapshotFilter;

// Import path_resolver for proper command execution on macOS GUI apps
use specforge_lib::utils::path_resolver;

// Rate limiters, semaphore, and security are now imported from mcp::{state, security}
// Background process management is now imported from mcp::background
// ============================================================================
// MCP Server Implementation
// ============================================================================

#[derive(Clone)]
pub struct SpecForgeMcp {
    /// Tool router for handling tool calls
    tool_router: ToolRouter<Self>,
}

impl SpecForgeMcp {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    /// Execute a git command and return the output
    ///
    /// Uses path_resolver for proper environment setup on macOS GUI apps:
    /// - Sets correct PATH to find git
    /// - Sets SSH_AUTH_SOCK for SSH key authentication
    fn git_command(cwd: &str, args: &[&str]) -> Result<String, String> {
        // Use path_resolver::create_command for proper PATH and SSH_AUTH_SOCK setup
        let mut cmd = path_resolver::create_command("git");
        let output = cmd
            .args(args)
            .current_dir(cwd)
            .output()
            .map_err(|e| format!("Failed to execute git: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    /// Check if a path is a git repository
    fn is_git_repo(path: &str) -> bool {
        Self::git_command(path, &["rev-parse", "--git-dir"]).is_ok()
    }

    /// Get the current branch name
    fn get_current_branch(path: &str) -> Option<String> {
        Self::git_command(path, &["rev-parse", "--abbrev-ref", "HEAD"])
            .ok()
            .map(|s| s.trim().to_string())
    }

    /// Get the remote URL
    fn get_remote_url(path: &str) -> Option<String> {
        Self::git_command(path, &["remote", "get-url", "origin"])
            .ok()
            .map(|s| s.trim().to_string())
    }

    /// Execute a shell command with timeout enforcement
    ///
    /// Uses path_resolver for proper environment setup on macOS GUI apps:
    /// - Sets correct PATH including Volta, Homebrew, Cargo paths
    /// - Sets HOME, SSH_AUTH_SOCK, VOLTA_HOME environment variables
    /// - Sets terminal/encoding environment (TERM, LANG, FORCE_COLOR)
    ///
    /// Security features:
    /// - Default timeout: 5 minutes (300,000 ms)
    /// - Maximum timeout: 1 hour (from validation)
    /// - Returns error if command exceeds timeout
    async fn shell_command_async(cwd: &str, command: &str, timeout_ms: Option<u64>) -> Result<(i32, String, String), String> {
        // Default timeout: 5 minutes, max is enforced by validate_timeout (1 hour)
        let timeout_duration = Duration::from_millis(timeout_ms.unwrap_or(300_000));

        // Use path_resolver::create_async_command for proper environment setup
        // This ensures the command has access to Volta, Homebrew, and other tools
        let mut cmd = path_resolver::create_async_command("sh");
        cmd.arg("-c")
            .arg(command)
            .current_dir(cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Spawn the child process
        let child = cmd.spawn()
            .map_err(|e| format!("Failed to spawn command: {}", e))?;

        // Wait for output with timeout
        let result = tokio_timeout(timeout_duration, child.wait_with_output()).await;

        match result {
            Ok(output_result) => {
                let output = output_result
                    .map_err(|e| format!("Failed to execute command: {}", e))?;

                let exit_code = output.status.code().unwrap_or(-1);
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                Ok((exit_code, stdout, stderr))
            }
            Err(_) => {
                // Timeout occurred
                Err(format!(
                    "Command execution timed out after {} seconds. The process has been terminated.",
                    timeout_duration.as_secs()
                ))
            }
        }
    }

    /// Check if a script is a dev server script (long-running process)
    ///
    /// Dev server scripts typically include:
    /// - Script names: dev, start, serve, watch, preview, etc.
    /// - Commands containing: vite, next dev, webpack serve, nuxt dev, etc.
    fn is_dev_server_script(script_name: &str, script_command: &str) -> bool {
        // Common dev server script names
        let dev_script_names = [
            "dev", "start", "serve", "watch", "preview",
            "dev:server", "start:dev", "serve:dev",
        ];

        // Check script name
        let name_lower = script_name.to_lowercase();
        if dev_script_names.iter().any(|&n| name_lower == n || name_lower.starts_with(&format!("{}:", n))) {
            return true;
        }

        // Common dev server command patterns
        let dev_command_patterns = [
            "vite", "next dev", "next start", "nuxt dev", "nuxt start",
            "webpack serve", "webpack-dev-server", "parcel watch",
            "react-scripts start", "vue-cli-service serve",
            "nodemon", "ts-node-dev", "tsx watch",
            "astro dev", "remix dev",
        ];

        let cmd_lower = script_command.to_lowercase();
        dev_command_patterns.iter().any(|&p| cmd_lower.contains(p))
    }

}

// Implement tools using the tool_router macro
#[tool_router]
impl SpecForgeMcp {
    // ========================================================================
    // Existing Git Tools
    // ========================================================================

    /// List all registered projects in SpecForge
    #[tool(description = "List all registered projects in SpecForge with detailed info including project type, package manager, and workflow count.")]
    async fn list_projects(
        &self,
        Parameters(params): Parameters<GetProjectsParams>,
    ) -> Result<CallToolResult, McpError> {
        let store_data = read_store_data()
            .map_err(|e| McpError::internal_error(e, None))?;

        let mut projects: Vec<&Project> = store_data.projects.iter().collect();

        // Filter by query if specified
        if let Some(ref query) = params.query {
            let query_lower = query.to_lowercase();
            projects.retain(|p| {
                p.name.to_lowercase().contains(&query_lower) ||
                p.path.to_lowercase().contains(&query_lower) ||
                p.description.as_ref().map(|d| d.to_lowercase().contains(&query_lower)).unwrap_or(false)
            });
        }

        // Sort by name
        projects.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        // Build detailed project list
        let detailed_projects: Vec<ProjectListItem> = projects.iter().map(|p| {
            let path_buf = PathBuf::from(&p.path);
            let (package_manager, _, _) = Self::read_package_json(&path_buf);
            let project_type = Self::detect_project_type(&path_buf);
            let current_branch = Self::get_current_branch(&p.path);
            let workflow_count = store_data.workflows.iter()
                .filter(|w| w.project_id.as_ref() == Some(&p.id))
                .count();

            ProjectListItem {
                id: p.id.clone(),
                name: p.name.clone(),
                path: p.path.clone(),
                description: p.description.clone(),
                project_type,
                package_manager,
                current_branch,
                workflow_count,
            }
        }).collect();

        let response = serde_json::json!({
            "projects": detailed_projects,
            "total": detailed_projects.len()
        });
        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get information about a project at the specified path
    #[tool(description = "Get detailed information about a project including ID, scripts, package manager, workflows, and git info")]
    async fn get_project(
        &self,
        Parameters(params): Parameters<GetProjectParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = params.path;

        let path_buf = PathBuf::from(&path);
        if !path_buf.exists() {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Project path does not exist: {}", path)
            )]));
        }

        if !Self::is_git_repo(&path) {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Not a git repository: {}", path)
            )]));
        }

        // Try to find project in store to get ID and description
        let store_data = read_store_data().ok();
        let registered_project = store_data.as_ref().and_then(|data| {
            data.projects.iter().find(|p| p.path == path)
        });

        // Get associated workflows for this project
        let workflows = registered_project.and_then(|p| {
            store_data.as_ref().map(|data| {
                data.workflows.iter()
                    .filter(|w| w.project_id.as_ref() == Some(&p.id))
                    .map(|w| WorkflowRef {
                        id: w.id.clone(),
                        name: w.name.clone(),
                    })
                    .collect::<Vec<_>>()
            })
        }).filter(|v: &Vec<WorkflowRef>| !v.is_empty());

        // Detect package manager and read package.json
        let (package_manager, scripts, node_version_from_pkg) = Self::read_package_json(&path_buf);

        // Detect project type
        let project_type = Self::detect_project_type(&path_buf);

        // Get node version from various sources
        let node_version = Self::get_node_version(&path_buf).or(node_version_from_pkg);

        let project = ProjectInfo {
            id: registered_project.map(|p| p.id.clone()),
            path: path.clone(),
            name: registered_project
                .map(|p| p.name.clone())
                .unwrap_or_else(|| {
                    path_buf
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                }),
            description: registered_project.and_then(|p| p.description.clone()),
            git_remote: Self::get_remote_url(&path),
            current_branch: Self::get_current_branch(&path),
            package_manager,
            scripts,
            project_type,
            node_version,
            workflows,
        };

        let json = serde_json::to_string_pretty(&project)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Read package.json and extract scripts, detect package manager
    fn read_package_json(path: &PathBuf) -> (Option<String>, Option<HashMap<String, String>>, Option<String>) {
        let package_json_path = path.join("package.json");

        // Detect package manager from lockfile
        let package_manager = if path.join("pnpm-lock.yaml").exists() {
            Some("pnpm".to_string())
        } else if path.join("yarn.lock").exists() {
            Some("yarn".to_string())
        } else if path.join("bun.lockb").exists() || path.join("bun.lock").exists() {
            Some("bun".to_string())
        } else if path.join("package-lock.json").exists() {
            Some("npm".to_string())
        } else if package_json_path.exists() {
            Some("npm".to_string()) // Default to npm if package.json exists
        } else {
            None
        };

        if !package_json_path.exists() {
            return (package_manager, None, None);
        }

        // Read and parse package.json
        let content = match std::fs::read_to_string(&package_json_path) {
            Ok(c) => c,
            Err(_) => return (package_manager, None, None),
        };

        let pkg: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => return (package_manager, None, None),
        };

        // Extract scripts
        let scripts = pkg.get("scripts").and_then(|s| {
            s.as_object().map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|val| (k.clone(), val.to_string())))
                    .collect::<HashMap<String, String>>()
            })
        }).filter(|s| !s.is_empty());

        // Extract node version from engines
        let node_version = pkg.get("engines")
            .and_then(|e| e.get("node"))
            .and_then(|n| n.as_str())
            .map(|s| s.to_string());

        (package_manager, scripts, node_version)
    }

    /// Detect project type based on files present
    fn detect_project_type(path: &PathBuf) -> Option<String> {
        // Check for various project indicators
        if path.join("Cargo.toml").exists() {
            return Some("rust".to_string());
        }
        if path.join("go.mod").exists() {
            return Some("go".to_string());
        }
        if path.join("requirements.txt").exists() || path.join("pyproject.toml").exists() || path.join("setup.py").exists() {
            return Some("python".to_string());
        }
        if path.join("Gemfile").exists() {
            return Some("ruby".to_string());
        }
        if path.join("pom.xml").exists() || path.join("build.gradle").exists() || path.join("build.gradle.kts").exists() {
            return Some("java".to_string());
        }
        if path.join("Package.swift").exists() {
            return Some("swift".to_string());
        }
        if path.join("pubspec.yaml").exists() {
            return Some("dart".to_string());
        }

        // Check for Node.js project types
        if path.join("package.json").exists() {
            // Check for specific frameworks
            if let Ok(content) = std::fs::read_to_string(path.join("package.json")) {
                if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
                    let deps = pkg.get("dependencies").and_then(|d| d.as_object());
                    let dev_deps = pkg.get("devDependencies").and_then(|d| d.as_object());

                    // Check dependencies for framework detection
                    let has_dep = |name: &str| {
                        deps.map(|d| d.contains_key(name)).unwrap_or(false) ||
                        dev_deps.map(|d| d.contains_key(name)).unwrap_or(false)
                    };

                    if has_dep("next") {
                        return Some("nextjs".to_string());
                    }
                    if has_dep("nuxt") {
                        return Some("nuxt".to_string());
                    }
                    if has_dep("@tauri-apps/api") || path.join("src-tauri").exists() {
                        return Some("tauri".to_string());
                    }
                    if has_dep("electron") {
                        return Some("electron".to_string());
                    }
                    if has_dep("react-native") || has_dep("expo") {
                        return Some("react-native".to_string());
                    }
                    if has_dep("vue") {
                        return Some("vue".to_string());
                    }
                    if has_dep("react") {
                        return Some("react".to_string());
                    }
                    if has_dep("svelte") {
                        return Some("svelte".to_string());
                    }
                    if has_dep("@angular/core") {
                        return Some("angular".to_string());
                    }
                    if has_dep("express") || has_dep("fastify") || has_dep("koa") || has_dep("hono") {
                        return Some("node-server".to_string());
                    }
                }
            }
            return Some("node".to_string());
        }

        None
    }

    /// Get Node.js version from .nvmrc, .node-version, or volta config
    fn get_node_version(path: &PathBuf) -> Option<String> {
        // Check .nvmrc
        if let Ok(version) = std::fs::read_to_string(path.join(".nvmrc")) {
            let v = version.trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }

        // Check .node-version
        if let Ok(version) = std::fs::read_to_string(path.join(".node-version")) {
            let v = version.trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }

        // Check volta in package.json
        if let Ok(content) = std::fs::read_to_string(path.join("package.json")) {
            if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(version) = pkg.get("volta")
                    .and_then(|v| v.get("node"))
                    .and_then(|n| n.as_str())
                {
                    return Some(version.to_string());
                }
            }
        }

        None
    }

    /// List all git worktrees for a project
    #[tool(description = "List all git worktrees for a project, showing path, branch, and whether it's the main worktree")]
    async fn list_worktrees(
        &self,
        Parameters(params): Parameters<ListWorktreesParams>,
    ) -> Result<CallToolResult, McpError> {
        let project_path = params.project_path;

        if !PathBuf::from(&project_path).exists() {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Project path does not exist: {}", project_path)
            )]));
        }

        if !Self::is_git_repo(&project_path) {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Not a git repository: {}", project_path)
            )]));
        }

        let output = Self::git_command(&project_path, &["worktree", "list", "--porcelain"])
            .map_err(|e| McpError::internal_error(e, None))?;

        let mut worktrees = Vec::new();
        let mut current_worktree: Option<WorktreeInfo> = None;

        for line in output.lines() {
            if line.starts_with("worktree ") {
                if let Some(wt) = current_worktree.take() {
                    worktrees.push(wt);
                }
                let path = line.strip_prefix("worktree ").unwrap_or("").to_string();
                current_worktree = Some(WorktreeInfo {
                    path,
                    branch: String::new(),
                    is_main: false,
                    is_bare: false,
                });
            } else if line.starts_with("branch ") {
                if let Some(ref mut wt) = current_worktree {
                    wt.branch = line
                        .strip_prefix("branch refs/heads/")
                        .unwrap_or(line.strip_prefix("branch ").unwrap_or(""))
                        .to_string();
                }
            } else if line == "bare" {
                if let Some(ref mut wt) = current_worktree {
                    wt.is_bare = true;
                }
            }
        }

        if let Some(wt) = current_worktree {
            worktrees.push(wt);
        }

        if let Some(first) = worktrees.iter_mut().find(|w| !w.is_bare) {
            first.is_main = true;
        }

        let response = serde_json::json!({ "worktrees": worktrees });
        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get git status for a specific worktree or project
    #[tool(description = "Get git status including current branch, ahead/behind counts, staged files, modified files, and untracked files")]
    async fn get_worktree_status(
        &self,
        Parameters(params): Parameters<GetWorktreeStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let worktree_path = params.worktree_path;

        if !PathBuf::from(&worktree_path).exists() {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Path does not exist: {}", worktree_path)
            )]));
        }

        if !Self::is_git_repo(&worktree_path) {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Not a git repository: {}", worktree_path)
            )]));
        }

        let branch = Self::get_current_branch(&worktree_path)
            .unwrap_or_else(|| "HEAD".to_string());

        let (ahead, behind) = Self::git_command(&worktree_path, &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
            .ok()
            .and_then(|s| {
                let parts: Vec<&str> = s.trim().split_whitespace().collect();
                if parts.len() == 2 {
                    Some((
                        parts[0].parse().unwrap_or(0),
                        parts[1].parse().unwrap_or(0),
                    ))
                } else {
                    None
                }
            })
            .unwrap_or((0, 0));

        let status_output = Self::git_command(&worktree_path, &["status", "--porcelain"])
            .unwrap_or_default();

        let mut staged = Vec::new();
        let mut modified = Vec::new();
        let mut untracked = Vec::new();

        for line in status_output.lines() {
            if line.len() < 3 {
                continue;
            }
            let index_status = line.chars().next().unwrap_or(' ');
            let worktree_status = line.chars().nth(1).unwrap_or(' ');
            let file_path = line[3..].to_string();

            if index_status != ' ' && index_status != '?' {
                staged.push(file_path.clone());
            }
            if worktree_status == 'M' {
                modified.push(file_path.clone());
            }
            if index_status == '?' {
                untracked.push(file_path);
            }
        }

        let status = GitStatusInfo {
            branch,
            ahead,
            behind,
            staged,
            modified,
            untracked,
        };

        let json = serde_json::to_string_pretty(&status)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get the staged changes diff for commit message generation
    #[tool(description = "Get the staged changes diff. Useful for generating commit messages. Returns the diff content along with statistics.")]
    async fn get_git_diff(
        &self,
        Parameters(params): Parameters<GetGitDiffParams>,
    ) -> Result<CallToolResult, McpError> {
        let worktree_path = params.worktree_path;

        if !PathBuf::from(&worktree_path).exists() {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Path does not exist: {}", worktree_path)
            )]));
        }

        if !Self::is_git_repo(&worktree_path) {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Not a git repository: {}", worktree_path)
            )]));
        }

        let diff = Self::git_command(&worktree_path, &["diff", "--cached"])
            .unwrap_or_default();

        if diff.is_empty() {
            return Ok(CallToolResult::error(vec![Content::text(
                "No staged changes. Please stage files first with 'git add'."
            )]));
        }

        let stats = Self::git_command(&worktree_path, &["diff", "--cached", "--stat"])
            .unwrap_or_default();

        let mut files_changed = 0;
        let mut insertions = 0;
        let mut deletions = 0;

        for line in stats.lines() {
            if line.contains("files changed") || line.contains("file changed") {
                for part in line.split(',') {
                    let part = part.trim();
                    if part.contains("file") {
                        files_changed = part.split_whitespace().next()
                            .and_then(|n| n.parse().ok())
                            .unwrap_or(0);
                    } else if part.contains("insertion") {
                        insertions = part.split_whitespace().next()
                            .and_then(|n| n.parse().ok())
                            .unwrap_or(0);
                    } else if part.contains("deletion") {
                        deletions = part.split_whitespace().next()
                            .and_then(|n| n.parse().ok())
                            .unwrap_or(0);
                    }
                }
            }
        }

        let diff_info = DiffInfo {
            diff,
            files_changed,
            insertions,
            deletions,
        };

        let json = serde_json::to_string_pretty(&diff_info)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // New Workflow Tools
    // ========================================================================

    /// List all workflows, optionally filtered by project
    #[tool(description = "List all workflows in SpecForge. Optionally filter by project_id. Returns workflow summaries including step count.")]
    async fn list_workflows(
        &self,
        Parameters(params): Parameters<ListWorkflowsParams>,
    ) -> Result<CallToolResult, McpError> {
        let store_data = read_store_data()
            .map_err(|e| McpError::internal_error(e, None))?;

        let mut workflows: Vec<WorkflowSummary> = store_data.workflows.iter()
            .filter(|w| {
                if let Some(ref project_id) = params.project_id {
                    w.project_id.as_ref() == Some(project_id)
                } else {
                    true
                }
            })
            .map(|w| WorkflowSummary {
                id: w.id.clone(),
                name: w.name.clone(),
                description: w.description.clone(),
                project_id: w.project_id.clone(),
                step_count: w.nodes.len(),
                created_at: w.created_at.clone(),
                updated_at: w.updated_at.clone(),
                last_executed_at: w.last_executed_at.clone(),
            })
            .collect();

        workflows.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        let response = serde_json::json!({ "workflows": workflows });
        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get detailed information about a specific workflow
    #[tool(description = "Get detailed information about a workflow including all its steps/nodes. Returns the full workflow structure.")]
    async fn get_workflow(
        &self,
        Parameters(params): Parameters<GetWorkflowParams>,
    ) -> Result<CallToolResult, McpError> {
        let store_data = read_store_data()
            .map_err(|e| McpError::internal_error(e, None))?;

        let workflow = store_data.workflows.iter()
            .find(|w| w.id == params.workflow_id);

        match workflow {
            Some(w) => {
                let json = serde_json::to_string_pretty(w)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            None => {
                Ok(CallToolResult::error(vec![Content::text(
                    format!("Workflow not found: {}", params.workflow_id)
                )]))
            }
        }
    }

    /// Create a new workflow
    #[tool(description = "Create a new workflow with the specified name. Optionally associate it with a project and add a description.")]
    async fn create_workflow(
        &self,
        Parameters(params): Parameters<CreateWorkflowParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut store_data = read_store_data()
            .map_err(|e| McpError::internal_error(e, None))?;

        let now = chrono::Utc::now().to_rfc3339();
        let workflow_id = format!("wf-{}", Uuid::new_v4());

        let workflow = Workflow {
            id: workflow_id.clone(),
            name: params.name.clone(),
            description: params.description,
            project_id: params.project_id,
            nodes: Vec::new(),
            created_at: now.clone(),
            updated_at: now.clone(),
            last_executed_at: None,
            webhook: None,
            incoming_webhook: None,
        };

        store_data.workflows.push(workflow);

        write_store_data(&store_data)
            .map_err(|e| McpError::internal_error(e, None))?;

        let response = CreateWorkflowResponse {
            workflow_id,
            name: params.name,
            created_at: now,
        };

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Create a workflow with steps atomically
    #[tool(description = "Create a new workflow with steps in a single atomic operation. This is the recommended method - it prevents sync issues that can occur when using separate create_workflow and add_workflow_steps calls. Maximum 10 steps.

USAGE:
- Provide workflow name (required) and optional description/project_id
- Steps array contains the steps to create (1-10 steps)
- Steps execute in array order (index 0 = first step)

EXAMPLE:
{
  \"name\": \"Build and Deploy\",
  \"description\": \"CI/CD pipeline\",
  \"steps\": [
    { \"name\": \"Install\", \"command\": \"npm ci\" },
    { \"name\": \"Build\", \"command\": \"npm run build\" },
    { \"name\": \"Test\", \"command\": \"npm test\" }
  ]
}

RETURNS: Created workflow ID and step details. Use workflow_id with run_workflow to execute.")]
    async fn create_workflow_with_steps(
        &self,
        Parameters(params): Parameters<CreateWorkflowWithStepsParams>,
    ) -> Result<CallToolResult, McpError> {
        const MAX_BATCH_SIZE: usize = 10;

        // Validate workflow name
        if params.name.trim().is_empty() {
            return Ok(CallToolResult::error(vec![Content::text(
                "Workflow name cannot be empty"
            )]));
        }
        if let Err(e) = validate_string_length("name", &params.name, MAX_NAME_LENGTH) {
            return Ok(CallToolResult::error(vec![Content::text(e)]));
        }

        // Validate steps
        if params.steps.is_empty() {
            return Ok(CallToolResult::error(vec![Content::text(
                "Steps array cannot be empty. Provide at least 1 step."
            )]));
        }
        if params.steps.len() > MAX_BATCH_SIZE {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Too many steps: {} (max {})", params.steps.len(), MAX_BATCH_SIZE)
            )]));
        }

        // Validate all steps upfront
        let mut step_names: Vec<&str> = Vec::new();
        for (i, step) in params.steps.iter().enumerate() {
            if step.name.trim().is_empty() {
                return Ok(CallToolResult::error(vec![Content::text(
                    format!("Step {} has empty name", i + 1)
                )]));
            }
            if step_names.contains(&step.name.as_str()) {
                return Ok(CallToolResult::error(vec![Content::text(
                    format!("Duplicate step name '{}'", step.name)
                )]));
            }
            step_names.push(&step.name);

            if step.command.trim().is_empty() {
                return Ok(CallToolResult::error(vec![Content::text(
                    format!("Step {} '{}' has empty command", i + 1, step.name)
                )]));
            }
            if let Err(e) = validate_command(&step.command) {
                return Ok(CallToolResult::error(vec![Content::text(
                    format!("Step {} '{}': {}", i + 1, step.name, e)
                )]));
            }
        }

        let mut store_data = read_store_data()
            .map_err(|e| McpError::internal_error(e, None))?;

        let now = chrono::Utc::now().to_rfc3339();
        let workflow_id = format!("wf-{}", Uuid::new_v4());

        // Create workflow with all steps
        let mut nodes: Vec<WorkflowNode> = Vec::new();
        let mut created_steps: Vec<CreatedStepInfo> = Vec::new();

        for (i, step) in params.steps.iter().enumerate() {
            let node_id = format!("node-{}", Uuid::new_v4());
            let order = i as i32;

            let mut config = serde_json::json!({
                "command": step.command,
            });
            if let Some(cwd) = &step.cwd {
                config["cwd"] = serde_json::json!(cwd);
            }
            if let Some(timeout) = step.timeout {
                config["timeout"] = serde_json::json!(timeout);
            }

            nodes.push(WorkflowNode {
                id: node_id.clone(),
                node_type: "script".to_string(),
                name: step.name.clone(),
                config,
                order,
                position: None,
            });

            created_steps.push(CreatedStepInfo {
                node_id,
                name: step.name.clone(),
                order,
                command: step.command.clone(),
            });
        }

        let workflow = Workflow {
            id: workflow_id.clone(),
            name: params.name.clone(),
            description: params.description.clone(),
            project_id: params.project_id.clone(),
            nodes,
            created_at: now.clone(),
            updated_at: now.clone(),
            last_executed_at: None,
            webhook: None,
            incoming_webhook: None,
        };

        store_data.workflows.push(workflow);

        eprintln!("[MCP Debug] create_workflow_with_steps - Writing workflow '{}' with {} steps",
            params.name, created_steps.len());

        write_store_data(&store_data)
            .map_err(|e| {
                eprintln!("[MCP Debug] create_workflow_with_steps - Write FAILED: {}", e);
                McpError::internal_error(e, None)
            })?;

        eprintln!("[MCP Debug] create_workflow_with_steps - Write SUCCESS");

        let response = CreateWorkflowWithStepsResponse {
            success: true,
            workflow_id,
            workflow_name: params.name,
            description: params.description,
            project_id: params.project_id,
            created_steps,
            total_steps: params.steps.len(),
            created_at: now,
            message: format!("Workflow created with {} steps", params.steps.len()),
        };

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Add a step to an existing workflow
    #[tool(description = "Add a new step (script node) to an existing workflow. Specify the command to execute, optional working directory, and timeout.")]
    async fn add_workflow_step(
        &self,
        Parameters(params): Parameters<AddWorkflowStepParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut store_data = read_store_data()
            .map_err(|e| McpError::internal_error(e, None))?;

        let workflow = store_data.workflows.iter_mut()
            .find(|w| w.id == params.workflow_id);

        match workflow {
            Some(w) => {
                let now = chrono::Utc::now().to_rfc3339();
                let node_id = format!("node-{}", Uuid::new_v4());

                // Calculate order: use provided order or max + 1
                let order = params.order.unwrap_or_else(|| {
                    w.nodes.iter().map(|n| n.order).max().unwrap_or(-1) + 1
                });

                // Build config
                let mut config = serde_json::json!({
                    "command": params.command,
                });
                if let Some(cwd) = &params.cwd {
                    config["cwd"] = serde_json::json!(cwd);
                }
                if let Some(timeout) = params.timeout {
                    config["timeout"] = serde_json::json!(timeout);
                }

                let node = WorkflowNode {
                    id: node_id.clone(),
                    node_type: "script".to_string(),
                    name: params.name.clone(),
                    config,
                    order,
                    position: None,
                };

                w.nodes.push(node);
                w.updated_at = now.clone();

                eprintln!("[MCP Debug] add_workflow_step - Writing to store...");
                eprintln!("[MCP Debug] add_workflow_step - Workflow {} now has {} nodes", params.workflow_id, w.nodes.len());

                write_store_data(&store_data)
                    .map_err(|e| {
                        eprintln!("[MCP Debug] add_workflow_step - Write FAILED: {}", e);
                        McpError::internal_error(e, None)
                    })?;

                eprintln!("[MCP Debug] add_workflow_step - Write SUCCESS");

                let response = AddStepResponse {
                    node_id,
                    workflow_id: params.workflow_id,
                    name: params.name,
                    order,
                };

                let json = serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            None => {
                Ok(CallToolResult::error(vec![Content::text(
                    format!("Workflow not found: {}", params.workflow_id)
                )]))
            }
        }
    }

    /// Add multiple steps to a workflow atomically (batch operation)
    #[tool(description = "Add multiple steps to a workflow in a single atomic operation. Use this for efficiently creating workflows with multiple steps. All steps are validated upfront and added together - if any validation fails, no steps are added. Maximum 10 steps per call.

USAGE:
- First get workflow_id from create_workflow or list_workflows
- Steps are executed in the order they appear in the array
- Each step requires: name (unique within batch) and command
- Optional per-step: cwd (working directory) and timeout (ms)

EXAMPLE:
{
  \"workflow_id\": \"workflow-abc123\",
  \"steps\": [
    { \"name\": \"Install dependencies\", \"command\": \"npm ci\" },
    { \"name\": \"Run linter\", \"command\": \"npm run lint\" },
    { \"name\": \"Run tests\", \"command\": \"npm test\" },
    { \"name\": \"Build project\", \"command\": \"npm run build\" }
  ]
}

LIMITS:
- Max 10 steps per call
- Command timeout default: 5 minutes (300000ms)

RETURNS: List of created step IDs and their assigned order positions.")]
    async fn add_workflow_steps(
        &self,
        Parameters(params): Parameters<AddWorkflowStepsParams>,
    ) -> Result<CallToolResult, McpError> {
        const MAX_BATCH_SIZE: usize = 10;

        // Validate batch size
        if params.steps.is_empty() {
            return Ok(CallToolResult::error(vec![Content::text(
                "Steps array cannot be empty. Provide at least 1 step to add."
            )]));
        }

        if params.steps.len() > MAX_BATCH_SIZE {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Too many steps: {} (max {}). Split into multiple calls.", params.steps.len(), MAX_BATCH_SIZE)
            )]));
        }

        // Validate all steps upfront
        let mut step_names: Vec<&str> = Vec::new();
        for (i, step) in params.steps.iter().enumerate() {
            // Check name
            if step.name.trim().is_empty() {
                return Ok(CallToolResult::error(vec![Content::text(
                    format!("Step {} has empty name. Each step requires a non-empty name.", i + 1)
                )]));
            }
            if let Err(e) = validate_string_length("name", &step.name, MAX_NAME_LENGTH) {
                return Ok(CallToolResult::error(vec![Content::text(
                    format!("Step {}: {}", i + 1, e)
                )]));
            }

            // Check for duplicate names within batch
            if step_names.contains(&step.name.as_str()) {
                return Ok(CallToolResult::error(vec![Content::text(
                    format!("Duplicate step name '{}' found. Each step must have a unique name within the batch.", step.name)
                )]));
            }
            step_names.push(&step.name);

            // Check command
            if step.command.trim().is_empty() {
                return Ok(CallToolResult::error(vec![Content::text(
                    format!("Step {} '{}' has empty command. Each step requires a non-empty command.", i + 1, step.name)
                )]));
            }
            if let Err(e) = validate_command(&step.command) {
                return Ok(CallToolResult::error(vec![Content::text(
                    format!("Step {} '{}': {}", i + 1, step.name, e)
                )]));
            }

            // Validate timeout if provided
            if let Some(timeout) = step.timeout {
                if let Err(e) = validate_timeout(timeout) {
                    return Ok(CallToolResult::error(vec![Content::text(
                        format!("Step {} '{}': {}", i + 1, step.name, e)
                    )]));
                }
            }
        }

        // All validation passed, now add the steps
        let mut store_data = read_store_data()
            .map_err(|e| McpError::internal_error(e, None))?;

        // Find workflow index first
        let workflow_idx = store_data.workflows.iter()
            .position(|w| w.id == params.workflow_id);

        let workflow_idx = match workflow_idx {
            Some(idx) => idx,
            None => {
                return Ok(CallToolResult::error(vec![Content::text(
                    format!("Workflow not found: {}. Use list_workflows to find valid IDs.", params.workflow_id)
                )]));
            }
        };

        let now = chrono::Utc::now().to_rfc3339();

        // Calculate starting order
        let start_order = store_data.workflows[workflow_idx].nodes.iter()
            .map(|n| n.order).max().unwrap_or(-1) + 1;

        let mut created_steps: Vec<CreatedStepInfo> = Vec::new();

        for (i, step) in params.steps.iter().enumerate() {
            let node_id = format!("node-{}", Uuid::new_v4());
            let order = start_order + i as i32;

            // Build config
            let mut config = serde_json::json!({
                "command": step.command,
            });
            if let Some(cwd) = &step.cwd {
                config["cwd"] = serde_json::json!(cwd);
            }
            if let Some(timeout) = step.timeout {
                config["timeout"] = serde_json::json!(timeout);
            }

            let node = WorkflowNode {
                id: node_id.clone(),
                node_type: "script".to_string(),
                name: step.name.clone(),
                config,
                order,
                position: None,
            };

            store_data.workflows[workflow_idx].nodes.push(node);

            created_steps.push(CreatedStepInfo {
                node_id,
                name: step.name.clone(),
                order,
                command: step.command.clone(),
            });
        }

        store_data.workflows[workflow_idx].updated_at = now;

        let total_nodes = store_data.workflows[workflow_idx].nodes.len();
        let steps_added = created_steps.len();

        eprintln!("[MCP Debug] add_workflow_steps - Writing to store...");
        eprintln!("[MCP Debug] add_workflow_steps - Workflow {} now has {} nodes (added {})",
            params.workflow_id, total_nodes, steps_added);

        write_store_data(&store_data)
            .map_err(|e| {
                eprintln!("[MCP Debug] add_workflow_steps - Write FAILED: {}", e);
                McpError::internal_error(e, None)
            })?;

        eprintln!("[MCP Debug] add_workflow_steps - Write SUCCESS");

        let response = AddWorkflowStepsResponse {
            success: true,
            workflow_id: params.workflow_id,
            created_steps,
            total_workflow_steps: total_nodes,
            message: format!("Successfully added {} steps to workflow", steps_added),
        };

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // Step Template Tools
    // ========================================================================

    /// List available step templates
    #[tool(description = "List available step templates for workflow steps. Includes built-in templates and custom templates. Filter by category or search query.")]
    async fn list_step_templates(
        &self,
        Parameters(params): Parameters<ListStepTemplatesParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut templates: Vec<StepTemplateInfo> = Vec::new();

        // Add built-in templates if requested
        if params.include_builtin {
            templates.extend(get_builtin_templates());
        }

        // Add custom templates from store
        let store_data = read_store_data()
            .map_err(|e| McpError::internal_error(e, None))?;

        for custom in store_data.custom_step_templates {
            templates.push(StepTemplateInfo {
                id: custom.id,
                name: custom.name,
                command: custom.command,
                category: custom.category,
                description: custom.description,
                is_custom: true,
            });
        }

        // Filter by category if specified
        if let Some(ref category) = params.category {
            templates.retain(|t| t.category.to_lowercase() == category.to_lowercase());
        }

        // Filter by query if specified
        if let Some(ref query) = params.query {
            let query_lower = query.to_lowercase();
            templates.retain(|t| {
                t.name.to_lowercase().contains(&query_lower) ||
                t.command.to_lowercase().contains(&query_lower) ||
                t.description.as_ref().map(|d| d.to_lowercase().contains(&query_lower)).unwrap_or(false)
            });
        }

        let response = serde_json::json!({ "templates": templates });
        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Create a custom step template
    #[tool(description = "Create a custom step template that can be reused across workflows. Templates are saved in SpecForge.")]
    async fn create_step_template(
        &self,
        Parameters(params): Parameters<CreateStepTemplateParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut store_data = read_store_data()
            .map_err(|e| McpError::internal_error(e, None))?;

        let now = chrono::Utc::now().to_rfc3339();
        let template_id = format!("custom-{}", Uuid::new_v4());

        let template = CustomStepTemplate {
            id: template_id.clone(),
            name: params.name.clone(),
            command: params.command,
            category: params.category.clone(),
            description: params.description,
            is_custom: true,
            created_at: now.clone(),
        };

        store_data.custom_step_templates.push(template);

        write_store_data(&store_data)
            .map_err(|e| McpError::internal_error(e, None))?;

        let response = CreateTemplateResponse {
            template_id,
            name: params.name,
            category: params.category,
            created_at: now,
        };

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // Workflow Execution Tool
    // ========================================================================

    /// Execute a workflow synchronously
    #[tool(description = "Execute a workflow synchronously and return the execution result. Runs all steps in order and stops on first failure.")]
    async fn run_workflow(
        &self,
        Parameters(params): Parameters<RunWorkflowParams>,
    ) -> Result<CallToolResult, McpError> {
        let store_data = read_store_data()
            .map_err(|e| McpError::internal_error(e, None))?;

        // Find workflow
        let workflow = store_data.workflows.iter()
            .find(|w| w.id == params.workflow_id);

        let workflow = match workflow {
            Some(w) => w.clone(),
            None => {
                return Ok(CallToolResult::error(vec![Content::text(
                    format!("Workflow not found: {}", params.workflow_id)
                )]));
            }
        };

        // Record execution start time
        let execution_id = format!("exec-{}", Uuid::new_v4());
        let started_at = Utc::now();

        // Determine working directory
        let cwd = if let Some(ref path) = params.project_path {
            path.clone()
        } else if let Some(ref project_id) = workflow.project_id {
            // Find project path
            store_data.projects.iter()
                .find(|p| p.id == *project_id)
                .map(|p| p.path.clone())
                .unwrap_or_else(|| std::env::current_dir().unwrap().to_string_lossy().to_string())
        } else {
            std::env::current_dir().unwrap().to_string_lossy().to_string()
        };

        // Sort nodes by order
        let mut nodes = workflow.nodes.clone();
        nodes.sort_by_key(|n| n.order);

        let total_steps = nodes.len();
        let mut steps_executed = 0;
        let mut failed_step: Option<FailedStepInfo> = None;
        let mut output_lines: Vec<String> = Vec::new();

        for node in &nodes {
            // Only execute script nodes
            if node.node_type != "script" {
                output_lines.push(format!("[SKIP] {}: Not a script node", node.name));
                continue;
            }

            // Get command from config
            let command = node.config.get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if command.is_empty() {
                output_lines.push(format!("[SKIP] {}: Empty command", node.name));
                continue;
            }

            // Get node-specific cwd or use workflow cwd
            let node_cwd = node.config.get("cwd")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| cwd.clone());

            let timeout = node.config.get("timeout")
                .and_then(|v| v.as_u64());

            output_lines.push(format!("[RUN] {}: {} (timeout: {}s)", node.name, command, timeout.unwrap_or(300_000) / 1000));

            // Use async shell command with timeout enforcement
            match Self::shell_command_async(&node_cwd, command, timeout).await {
                Ok((exit_code, stdout, stderr)) => {
                    steps_executed += 1;

                    // Sanitize output to redact sensitive content (API keys, tokens, etc.)
                    let sanitized_stdout = sanitize_output(&stdout);
                    let sanitized_stderr = sanitize_output(&stderr);

                    if exit_code == 0 {
                        output_lines.push(format!("[OK] {} completed successfully", node.name));
                        if !sanitized_stdout.trim().is_empty() {
                            // Add last 10 lines of stdout
                            let last_lines: Vec<&str> = sanitized_stdout.lines().rev().take(10).collect();
                            for line in last_lines.iter().rev() {
                                output_lines.push(format!("  > {}", line));
                            }
                        }
                    } else {
                        output_lines.push(format!("[FAIL] {} failed with exit code {}", node.name, exit_code));
                        if !sanitized_stderr.trim().is_empty() {
                            output_lines.push(format!("  Error: {}", sanitized_stderr.trim()));
                        }
                        failed_step = Some(FailedStepInfo {
                            node_id: node.id.clone(),
                            node_name: node.name.clone(),
                            exit_code,
                            error_message: sanitized_stderr.trim().to_string(),
                        });
                        break;
                    }
                }
                Err(e) => {
                    // Sanitize error message
                    let sanitized_error = sanitize_error(&e);
                    output_lines.push(format!("[ERROR] {}: {}", node.name, sanitized_error));
                    failed_step = Some(FailedStepInfo {
                        node_id: node.id.clone(),
                        node_name: node.name.clone(),
                        exit_code: -1,
                        error_message: sanitized_error,
                    });
                    break;
                }
            }
        }

        // Build output summary (last 50 lines)
        let output_summary = output_lines.iter()
            .rev()
            .take(50)
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");

        let status = if failed_step.is_some() {
            if steps_executed > 0 { "partial" } else { "failed" }
        } else {
            "completed"
        };

        // Record execution end time and duration
        let finished_at = Utc::now();
        let duration_ms = (finished_at - started_at).num_milliseconds() as u64;

        // Save execution history to database
        if let Err(e) = Self::save_execution_history(
            &execution_id,
            &workflow.id,
            &workflow.name,
            status,
            &started_at.to_rfc3339(),
            &finished_at.to_rfc3339(),
            duration_ms,
            total_steps,
            steps_executed,
            failed_step.as_ref().map(|f| f.error_message.clone()),
            &output_lines,
        ) {
            eprintln!("[MCP Server] Failed to save execution history: {}", e);
        }

        let response = RunWorkflowResponse {
            success: failed_step.is_none(),
            workflow_id: workflow.id,
            workflow_name: workflow.name,
            steps_executed,
            total_steps,
            status: status.to_string(),
            failed_step,
            output_summary,
        };

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // NPM Script Execution Tool
    // ========================================================================

    /// Execute an npm script from a project's package.json
    /// Supports volta and corepack for proper toolchain management
    #[tool(description = "Execute an npm/yarn/pnpm script from a project's package.json. Automatically detects and uses volta/corepack if configured. First use get_project to discover available scripts, then use this tool to run them.")]
    async fn run_npm_script(
        &self,
        Parameters(params): Parameters<RunNpmScriptParams>,
    ) -> Result<CallToolResult, McpError> {
        let project_path = PathBuf::from(&params.project_path);
        let package_json_path = project_path.join("package.json");

        // Check if package.json exists
        if !package_json_path.exists() {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("No package.json found at: {}", params.project_path)
            )]));
        }

        // Read and parse package.json to verify script exists and get toolchain config
        let package_json_content = std::fs::read_to_string(&package_json_path)
            .map_err(|e| McpError::internal_error(format!("Failed to read package.json: {}", e), None))?;

        let package_json: serde_json::Value = serde_json::from_str(&package_json_content)
            .map_err(|e| McpError::internal_error(format!("Failed to parse package.json: {}", e), None))?;

        // Check if the script exists
        let scripts = package_json.get("scripts")
            .and_then(|s| s.as_object());

        if let Some(scripts_obj) = scripts {
            if !scripts_obj.contains_key(&params.script_name) {
                let available_scripts: Vec<&String> = scripts_obj.keys().collect();
                return Ok(CallToolResult::error(vec![Content::text(
                    format!(
                        "Script '{}' not found in package.json. Available scripts: {}",
                        params.script_name,
                        available_scripts.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                    )
                )]));
            }
        } else {
            return Ok(CallToolResult::error(vec![Content::text(
                "No scripts defined in package.json"
            )]));
        }

        // Check dev_server_mode for background processes
        // Dev server scripts are long-running processes that typically include:
        // - Script names: dev, start, serve, watch, dev:server, etc.
        // - Commands containing: vite, next dev, webpack serve, etc.
        if params.run_in_background {
            let script_command = scripts
                .and_then(|s| s.get(&params.script_name))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let is_dev_server_script = Self::is_dev_server_script(&params.script_name, script_command);

            if is_dev_server_script {
                // Read config to check dev_server_mode
                if let Ok(store_data) = read_store_data() {
                    match store_data.mcp_config.dev_server_mode {
                        DevServerMode::RejectWithHint => {
                            return Ok(CallToolResult::error(vec![Content::text(
                                format!(
                                    "Dev server mode is set to 'reject_with_hint'. \
                                    Running dev servers via MCP is disabled. \
                                    Please use SpecForge UI to manage dev servers for better process visibility, \
                                    port tracking, and process management. \
                                    Script: '{}'",
                                    params.script_name
                                )
                            )]));
                        }
                        DevServerMode::UiIntegrated => {
                            // UI Integrated mode: Process will be tracked in SpecForge UI
                            // The background process manager will emit events for UI integration
                            eprintln!(
                                "[MCP Server] Starting dev server '{}' in UI integrated mode",
                                params.script_name
                            );
                        }
                        DevServerMode::McpManaged => {
                            // Default behavior: MCP manages the process independently
                        }
                    }
                }
            }
        }

        // Parse toolchain configuration from package.json
        let volta_config = package_json.get("volta");
        let package_manager_field = package_json.get("packageManager")
            .and_then(|v| v.as_str())
            .map(String::from);

        // Detect package manager from lock files or packageManager field
        let (package_manager, pm_version) = if let Some(ref pm_field) = package_manager_field {
            // Parse packageManager field (e.g., "pnpm@9.15.0+sha512.xxx")
            let parts: Vec<&str> = pm_field.split('@').collect();
            let pm_name = parts.first().unwrap_or(&"npm");
            let version = parts.get(1).map(|v| {
                // Remove hash if present (e.g., "9.15.0+sha512.xxx" -> "9.15.0")
                v.split('+').next().unwrap_or(v).to_string()
            });
            (pm_name.to_string(), version)
        } else if project_path.join("pnpm-lock.yaml").exists() {
            ("pnpm".to_string(), None)
        } else if project_path.join("yarn.lock").exists() {
            ("yarn".to_string(), None)
        } else if project_path.join("bun.lockb").exists() {
            ("bun".to_string(), None)
        } else {
            ("npm".to_string(), None)
        };

        // Check volta availability
        let home = path_resolver::get_home_dir();
        let volta_available = home.as_ref()
            .map(|h| std::path::Path::new(&format!("{}/.volta/bin/volta", h)).exists())
            .unwrap_or(false);

        // Determine toolchain strategy
        let use_volta = volta_available && volta_config.is_some();
        let use_corepack = package_manager_field.is_some();

        // Build the command based on toolchain strategy
        let (command, strategy_used) = if use_volta {
            // Use volta run for proper Node.js version management
            let volta_path = home.as_ref()
                .map(|h| format!("{}/.volta/bin/volta", h))
                .unwrap_or_else(|| "volta".to_string());

            let mut cmd_parts = vec![volta_path, "run".to_string()];

            // Add node version if specified in volta config
            if let Some(volta) = volta_config {
                if let Some(node_ver) = volta.get("node").and_then(|v| v.as_str()) {
                    cmd_parts.push("--node".to_string());
                    cmd_parts.push(node_ver.to_string());
                }

                // Add package manager version from volta config (unless using corepack)
                if !use_corepack {
                    if let Some(pnpm_ver) = volta.get("pnpm").and_then(|v| v.as_str()) {
                        cmd_parts.push("--pnpm".to_string());
                        cmd_parts.push(pnpm_ver.to_string());
                    } else if let Some(yarn_ver) = volta.get("yarn").and_then(|v| v.as_str()) {
                        cmd_parts.push("--yarn".to_string());
                        cmd_parts.push(yarn_ver.to_string());
                    } else if let Some(npm_ver) = volta.get("npm").and_then(|v| v.as_str()) {
                        cmd_parts.push("--npm".to_string());
                        cmd_parts.push(npm_ver.to_string());
                    }
                }
            }

            // Add the package manager run command
            cmd_parts.push(package_manager.clone());
            cmd_parts.push("run".to_string());
            cmd_parts.push(params.script_name.clone());

            // Add script arguments
            if let Some(ref args) = params.args {
                cmd_parts.push("--".to_string());
                cmd_parts.extend(args.iter().cloned());
            }

            let strategy = if use_corepack { "volta+corepack" } else { "volta" };
            (cmd_parts.join(" "), strategy.to_string())
        } else {
            // Direct execution - let PATH (with volta shims if available) handle it
            // Corepack will intercept if packageManager is set and corepack is enabled
            let mut cmd = format!("{} run {}", package_manager, params.script_name);

            if let Some(ref args) = params.args {
                cmd.push_str(&format!(" -- {}", args.join(" ")));
            }

            let strategy = if use_corepack { "corepack" } else { "system" };
            (cmd, strategy.to_string())
        };

        // Check if background mode is requested
        if params.run_in_background {
            // Log with "running" status before starting the process
            // This allows AI Activity to track the process lifecycle
            let arguments = serde_json::json!({
                "scriptName": params.script_name,
                "projectPath": params.project_path,
                "runInBackground": true,
            });
            let log_entry_id = log_request("run_npm_script", &arguments, "running", 0, None);

            // Background execution mode
            match BACKGROUND_PROCESS_MANAGER.start_process(
                params.script_name.clone(),
                params.project_path.clone(),
                command.clone(),
                params.success_pattern.clone(),
                params.success_timeout_ms,
                log_entry_id,
            ).await {
                Ok(process_info) => {
                    // Get initial output
                    let initial_output = BACKGROUND_PROCESS_MANAGER
                        .get_output(&process_info.id, 20)
                        .await
                        .map(|o| o.output_lines.join("\n"))
                        .unwrap_or_default();

                    let response = serde_json::json!({
                        "success": true,
                        "background": true,
                        "process_id": process_info.id,
                        "pid": process_info.pid,
                        "script_name": params.script_name,
                        "package_manager": package_manager,
                        "package_manager_version": pm_version,
                        "toolchain_strategy": strategy_used,
                        "status": process_info.status.to_string(),
                        "pattern_matched": process_info.pattern_matched,
                        "initial_output": sanitize_output(&initial_output),
                        "command": command,
                        "message": "Background process started. Use get_background_process_output to view output, stop_background_process to terminate."
                    });

                    let json = serde_json::to_string_pretty(&response)
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                    // Return error if pattern matching timed out or process failed early
                    if process_info.status == BackgroundProcessStatus::TimedOut
                        || process_info.status == BackgroundProcessStatus::Failed {
                        Ok(CallToolResult::error(vec![Content::text(json)]))
                    } else {
                        Ok(CallToolResult::success(vec![Content::text(json)]))
                    }
                }
                Err(e) => {
                    let sanitized_error = sanitize_error(&e);
                    Ok(CallToolResult::error(vec![Content::text(
                        format!("Failed to start background process '{}': {}", params.script_name, sanitized_error)
                    )]))
                }
            }
        } else {
            // Foreground execution mode (existing behavior)
            // Validate and apply timeout (default 5 min, max 1 hour)
            let timeout_ms = params.timeout_ms.map(|t| t.min(3_600_000)).unwrap_or(300_000);

            // Execute the command
            match Self::shell_command_async(&params.project_path, &command, Some(timeout_ms)).await {
                Ok((exit_code, stdout, stderr)) => {
                    // Sanitize outputs
                    let sanitized_stdout = sanitize_output(&stdout);
                    let sanitized_stderr = sanitize_output(&stderr);

                    let response = serde_json::json!({
                        "success": exit_code == 0,
                        "background": false,
                        "script_name": params.script_name,
                        "package_manager": package_manager,
                        "package_manager_version": pm_version,
                        "toolchain_strategy": strategy_used,
                        "volta_available": volta_available,
                        "volta_config": volta_config.is_some(),
                        "corepack_config": package_manager_field.is_some(),
                        "command": command,
                        "exit_code": exit_code,
                        "stdout": sanitized_stdout,
                        "stderr": sanitized_stderr,
                    });

                    let json = serde_json::to_string_pretty(&response)
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                    if exit_code == 0 {
                        Ok(CallToolResult::success(vec![Content::text(json)]))
                    } else {
                        Ok(CallToolResult::error(vec![Content::text(json)]))
                    }
                }
                Err(e) => {
                    let sanitized_error = sanitize_error(&e);
                    Ok(CallToolResult::error(vec![Content::text(
                        format!("Failed to execute script '{}': {}", params.script_name, sanitized_error)
                    )]))
                }
            }
        }
    }

    // ========================================================================
    // Package Manager Commands (install, update, add, remove, etc.)
    // ========================================================================

    /// Execute a package manager command (install, update, add, remove, etc.)
    #[tool(description = "Execute a package manager command (npm/yarn/pnpm). Supports: install, update, add, remove, ci, audit, outdated. Auto-detects package manager from lock files. Use for dependency management operations.")]
    async fn run_package_manager_command(
        &self,
        Parameters(params): Parameters<RunPackageManagerCommandParams>,
    ) -> Result<CallToolResult, McpError> {
        let project_path = PathBuf::from(&params.project_path);

        // Validate path
        if let Err(e) = validate_path(&params.project_path) {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Invalid project path: {}", e)
            )]));
        }

        // Check if directory exists
        if !project_path.exists() || !project_path.is_dir() {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Directory does not exist: {}", params.project_path)
            )]));
        }

        // Validate command
        let valid_commands = ["install", "i", "update", "up", "upgrade", "add", "remove", "rm", "uninstall", "ci", "audit", "outdated", "prune", "dedupe"];
        let command_lower = params.command.to_lowercase();
        if !valid_commands.contains(&command_lower.as_str()) {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Invalid command '{}'. Supported commands: {}", params.command, valid_commands.join(", "))
            )]));
        }

        // Check if packages are required for add/remove
        let needs_packages = matches!(command_lower.as_str(), "add" | "remove" | "rm" | "uninstall");
        if needs_packages && (params.packages.is_none() || params.packages.as_ref().map(|p| p.is_empty()).unwrap_or(true)) {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Command '{}' requires at least one package name", params.command)
            )]));
        }

        // Detect package manager from lock files
        let package_manager = if project_path.join("pnpm-lock.yaml").exists() {
            "pnpm"
        } else if project_path.join("yarn.lock").exists() {
            "yarn"
        } else if project_path.join("bun.lockb").exists() {
            "bun"
        } else {
            "npm"
        };

        // Build the command
        let mut cmd_parts: Vec<String> = vec![package_manager.to_string()];

        // Normalize command names
        let normalized_cmd = match command_lower.as_str() {
            "i" => "install",
            "up" | "upgrade" => "update",
            "rm" | "uninstall" => "remove",
            other => other,
        };
        cmd_parts.push(normalized_cmd.to_string());

        // Add packages if provided
        if let Some(packages) = &params.packages {
            for pkg in packages {
                // Validate package name (basic check)
                if pkg.is_empty() || pkg.contains("..") || pkg.contains(";") || pkg.contains("&") || pkg.contains("|") {
                    return Ok(CallToolResult::error(vec![Content::text(
                        format!("Invalid package name: {}", pkg)
                    )]));
                }
                cmd_parts.push(pkg.clone());
            }
        }

        // Add flags if provided
        if let Some(flags) = &params.flags {
            for flag in flags {
                // Validate flag (must start with -)
                if !flag.starts_with('-') {
                    return Ok(CallToolResult::error(vec![Content::text(
                        format!("Invalid flag '{}': flags must start with '-'", flag)
                    )]));
                }
                // Basic security check
                if flag.contains(";") || flag.contains("&") || flag.contains("|") || flag.contains("`") {
                    return Ok(CallToolResult::error(vec![Content::text(
                        format!("Invalid flag '{}': contains forbidden characters", flag)
                    )]));
                }
                cmd_parts.push(flag.clone());
            }
        }

        let command = cmd_parts.join(" ");

        // Validate and apply timeout (default 5 min, max 30 min for package operations)
        let timeout_ms = params.timeout_ms.map(|t| t.min(1_800_000)).unwrap_or(300_000);

        // Execute the command
        match Self::shell_command_async(&params.project_path, &command, Some(timeout_ms)).await {
            Ok((exit_code, stdout, stderr)) => {
                let sanitized_stdout = sanitize_output(&stdout);
                let sanitized_stderr = sanitize_output(&stderr);

                let response = serde_json::json!({
                    "success": exit_code == 0,
                    "command": normalized_cmd,
                    "package_manager": package_manager,
                    "full_command": command,
                    "exit_code": exit_code,
                    "stdout": sanitized_stdout,
                    "stderr": sanitized_stderr,
                    "packages": params.packages,
                    "flags": params.flags,
                });

                let json = serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                if exit_code == 0 {
                    Ok(CallToolResult::success(vec![Content::text(json)]))
                } else {
                    Ok(CallToolResult::error(vec![Content::text(json)]))
                }
            }
            Err(e) => {
                let sanitized_error = sanitize_error(&e);
                Ok(CallToolResult::error(vec![Content::text(
                    format!("Failed to execute '{}': {}", command, sanitized_error)
                )]))
            }
        }
    }

    /// Save execution history to database
    fn save_execution_history(
        execution_id: &str,
        workflow_id: &str,
        workflow_name: &str,
        status: &str,
        started_at: &str,
        finished_at: &str,
        duration_ms: u64,
        node_count: usize,
        completed_node_count: usize,
        error_message: Option<String>,
        output_lines: &[String],
    ) -> Result<(), String> {
        let db_path = get_database_path()?;
        let db = Database::new(db_path)?;

        // Convert output lines to JSON format matching WorkflowOutputLine interface
        // Frontend expects: { nodeId, nodeName, content, stream, timestamp }
        let output_json: Vec<serde_json::Value> = output_lines
            .iter()
            .map(|line| {
                serde_json::json!({
                    "nodeId": "mcp",
                    "nodeName": "MCP Execution",
                    "content": line,
                    "stream": "stdout",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
            })
            .collect();

        let output_str = serde_json::to_string(&output_json)
            .map_err(|e| format!("Failed to serialize output: {}", e))?;

        db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT OR REPLACE INTO execution_history
                (id, workflow_id, workflow_name, status, started_at, finished_at,
                 duration_ms, node_count, completed_node_count, error_message,
                 output, triggered_by)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                "#,
                params![
                    execution_id,
                    workflow_id,
                    workflow_name,
                    status,
                    started_at,
                    finished_at,
                    duration_ms as i64,
                    node_count as i32,
                    completed_node_count as i32,
                    error_message,
                    output_str,
                    "mcp", // triggered_by
                ],
            )
            .map_err(|e| format!("Failed to save execution history: {}", e))?;

            Ok(())
        })
    }

    // ========================================================================
    // MCP Action Tools
    // ========================================================================

    /// List available MCP actions (scripts, webhooks, workflows)
    #[tool(description = "List all available MCP actions that can be executed. Filter by type (script, webhook, workflow) or project.")]
    async fn list_actions(
        &self,
        Parameters(params): Parameters<ListActionsParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;
        let repo = MCPActionRepository::new(db);

        // Build filter
        let filter = ActionFilter {
            action_type: params.action_type.as_ref().and_then(|t| {
                match t.as_str() {
                    "script" => Some(MCPActionType::Script),
                    "webhook" => Some(MCPActionType::Webhook),
                    "workflow" => Some(MCPActionType::Workflow),
                    _ => None,
                }
            }),
            project_id: params.project_id,
            is_enabled: params.enabled_only,
        };

        let actions = repo.list_actions(&filter)
            .map_err(|e| McpError::internal_error(e, None))?;

        let response = serde_json::json!({
            "actions": actions,
            "total": actions.len()
        });

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get details of a specific MCP action
    #[tool(description = "Get detailed information about a specific MCP action by ID.")]
    async fn get_action(
        &self,
        Parameters(params): Parameters<GetActionParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;
        let repo = MCPActionRepository::new(db);

        let action = repo.get_action(&params.action_id)
            .map_err(|e| McpError::internal_error(e, None))?;

        match action {
            Some(action) => {
                let json = serde_json::to_string_pretty(&action)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            None => {
                Ok(CallToolResult::error(vec![Content::text(
                    format!("Action not found: {}", params.action_id)
                )]))
            }
        }
    }

    /// Execute a script action via MCP
    #[tool(description = "Execute a predefined script action. Requires user confirmation unless auto-approve is configured.")]
    async fn run_script(
        &self,
        Parameters(params): Parameters<RunScriptParams>,
    ) -> Result<CallToolResult, McpError> {
        let start = std::time::Instant::now();

        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;
        let repo = MCPActionRepository::new(db.clone());

        // Get the action
        let action = repo.get_action(&params.action_id)
            .map_err(|e| McpError::internal_error(e, None))?
            .ok_or_else(|| McpError::invalid_params(
                format!("Script action not found: {}", params.action_id),
                None
            ))?;

        // Verify it's a script action
        if action.action_type != MCPActionType::Script {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Action {} is not a script action", params.action_id)
            )]));
        }

        // Check if action is enabled
        if !action.is_enabled {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Script action {} is disabled", action.name)
            )]));
        }

        // Check permission
        let permission = repo.get_permission(Some(&params.action_id), &action.action_type)
            .map_err(|e| McpError::internal_error(e, None))?;

        if permission == PermissionLevel::Deny {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Permission denied for action: {}", action.name)
            )]));
        }

        // Create execution record
        let execution_id = Uuid::new_v4().to_string();
        let started_at = Utc::now().to_rfc3339();

        let mut execution = specforge_lib::models::mcp_action::MCPActionExecution {
            id: execution_id.clone(),
            action_id: Some(params.action_id.clone()),
            action_type: action.action_type.clone(),
            action_name: action.name.clone(),
            source_client: Some("mcp".to_string()),
            parameters: Some(serde_json::to_value(&params).unwrap_or_default()),
            status: ExecutionStatus::Running,
            result: None,
            error_message: None,
            started_at: started_at.clone(),
            completed_at: None,
            duration_ms: None,
        };

        repo.save_execution(&execution)
            .map_err(|e| McpError::internal_error(e, None))?;

        // Acquire semaphore permit for concurrency control
        let _permit = ACTION_SEMAPHORE.acquire().await
            .map_err(|e| McpError::internal_error(format!("Failed to acquire execution permit: {}", e), None))?;

        // Build execution parameters
        let mut exec_params = serde_json::json!({
            "config": action.config
        });

        // Apply overrides
        if let Some(cwd) = &params.cwd {
            exec_params["cwd"] = serde_json::Value::String(cwd.clone());
        }

        // Execute the script
        let executor = create_executor(MCPActionType::Script);
        let result = executor.execute(exec_params).await;

        let duration_ms = start.elapsed().as_millis() as i64;
        let completed_at = Utc::now().to_rfc3339();

        // Update execution record
        match result {
            Ok(result_value) => {
                execution.status = ExecutionStatus::Completed;
                execution.result = Some(result_value.clone());
                execution.completed_at = Some(completed_at);
                execution.duration_ms = Some(duration_ms);

                repo.save_execution(&execution)
                    .map_err(|e| McpError::internal_error(e, None))?;

                let response = serde_json::json!({
                    "success": true,
                    "executionId": execution_id,
                    "actionName": action.name,
                    "result": result_value
                });

                let json = serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(error) => {
                let sanitized_error = sanitize_error(&error);
                execution.status = ExecutionStatus::Failed;
                execution.error_message = Some(sanitized_error.clone());
                execution.completed_at = Some(completed_at);
                execution.duration_ms = Some(duration_ms);

                repo.save_execution(&execution)
                    .map_err(|e| McpError::internal_error(e, None))?;

                Ok(CallToolResult::error(vec![Content::text(
                    format!("Script execution failed: {}", sanitized_error)
                )]))
            }
        }
    }

    /// Trigger a webhook action via MCP
    #[tool(description = "Trigger a configured webhook action with optional variable substitution.")]
    async fn trigger_webhook(
        &self,
        Parameters(params): Parameters<TriggerWebhookParams>,
    ) -> Result<CallToolResult, McpError> {
        let start = std::time::Instant::now();

        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;
        let repo = MCPActionRepository::new(db.clone());

        // Get the action
        let action = repo.get_action(&params.action_id)
            .map_err(|e| McpError::internal_error(e, None))?
            .ok_or_else(|| McpError::invalid_params(
                format!("Webhook action not found: {}", params.action_id),
                None
            ))?;

        // Verify it's a webhook action
        if action.action_type != MCPActionType::Webhook {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Action {} is not a webhook action", params.action_id)
            )]));
        }

        // Check if action is enabled
        if !action.is_enabled {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Webhook action {} is disabled", action.name)
            )]));
        }

        // Check permission
        let permission = repo.get_permission(Some(&params.action_id), &action.action_type)
            .map_err(|e| McpError::internal_error(e, None))?;

        if permission == PermissionLevel::Deny {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Permission denied for action: {}", action.name)
            )]));
        }

        // Create execution record
        let execution_id = Uuid::new_v4().to_string();
        let started_at = Utc::now().to_rfc3339();

        let mut execution = specforge_lib::models::mcp_action::MCPActionExecution {
            id: execution_id.clone(),
            action_id: Some(params.action_id.clone()),
            action_type: action.action_type.clone(),
            action_name: action.name.clone(),
            source_client: Some("mcp".to_string()),
            parameters: Some(serde_json::to_value(&params).unwrap_or_default()),
            status: ExecutionStatus::Running,
            result: None,
            error_message: None,
            started_at: started_at.clone(),
            completed_at: None,
            duration_ms: None,
        };

        repo.save_execution(&execution)
            .map_err(|e| McpError::internal_error(e, None))?;

        // Acquire semaphore permit
        let _permit = ACTION_SEMAPHORE.acquire().await
            .map_err(|e| McpError::internal_error(format!("Failed to acquire execution permit: {}", e), None))?;

        // Build execution parameters
        let mut exec_params = serde_json::json!({
            "config": action.config
        });

        if let Some(vars) = &params.variables {
            exec_params["variables"] = serde_json::to_value(vars).unwrap_or_default();
        }
        if let Some(payload) = &params.payload {
            exec_params["payload"] = payload.clone();
        }

        // Execute the webhook
        let executor = create_executor(MCPActionType::Webhook);
        let result = executor.execute(exec_params).await;

        let duration_ms = start.elapsed().as_millis() as i64;
        let completed_at = Utc::now().to_rfc3339();

        // Update execution record
        match result {
            Ok(result_value) => {
                execution.status = ExecutionStatus::Completed;
                execution.result = Some(result_value.clone());
                execution.completed_at = Some(completed_at);
                execution.duration_ms = Some(duration_ms);

                repo.save_execution(&execution)
                    .map_err(|e| McpError::internal_error(e, None))?;

                let response = serde_json::json!({
                    "success": true,
                    "executionId": execution_id,
                    "actionName": action.name,
                    "result": result_value
                });

                let json = serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(error) => {
                let sanitized_error = sanitize_error(&error);
                execution.status = ExecutionStatus::Failed;
                execution.error_message = Some(sanitized_error.clone());
                execution.completed_at = Some(completed_at);
                execution.duration_ms = Some(duration_ms);

                repo.save_execution(&execution)
                    .map_err(|e| McpError::internal_error(e, None))?;

                Ok(CallToolResult::error(vec![Content::text(
                    format!("Webhook execution failed: {}", sanitized_error)
                )]))
            }
        }
    }

    /// Get the status of an action execution
    #[tool(description = "Get the current status and result of a running or completed action execution.")]
    async fn get_execution_status(
        &self,
        Parameters(params): Parameters<GetExecutionStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;
        let repo = MCPActionRepository::new(db);

        let execution = repo.get_execution(&params.execution_id)
            .map_err(|e| McpError::internal_error(e, None))?;

        match execution {
            Some(exec) => {
                let json = serde_json::to_string_pretty(&exec)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            None => {
                Ok(CallToolResult::error(vec![Content::text(
                    format!("Execution not found: {}", params.execution_id)
                )]))
            }
        }
    }

    /// List action execution history
    #[tool(description = "List recent action executions with optional filtering by action, type, or status.")]
    async fn list_action_executions(
        &self,
        Parameters(params): Parameters<ListActionExecutionsParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;
        let repo = MCPActionRepository::new(db);

        let filter = ExecutionFilter {
            action_id: params.action_id,
            action_type: params.action_type.as_ref().and_then(|t| {
                match t.as_str() {
                    "script" => Some(MCPActionType::Script),
                    "webhook" => Some(MCPActionType::Webhook),
                    "workflow" => Some(MCPActionType::Workflow),
                    _ => None,
                }
            }),
            status: params.status.as_ref().and_then(|s| {
                match s.as_str() {
                    "pending_confirm" => Some(ExecutionStatus::PendingConfirm),
                    "queued" => Some(ExecutionStatus::Queued),
                    "running" => Some(ExecutionStatus::Running),
                    "completed" => Some(ExecutionStatus::Completed),
                    "failed" => Some(ExecutionStatus::Failed),
                    "cancelled" => Some(ExecutionStatus::Cancelled),
                    "timed_out" => Some(ExecutionStatus::TimedOut),
                    _ => None,
                }
            }),
            limit: params.limit.map(|l| l as usize).unwrap_or(20),
        };

        let executions = repo.list_executions(&filter)
            .map_err(|e| McpError::internal_error(e, None))?;

        let response = serde_json::json!({
            "executions": executions,
            "total": executions.len()
        });

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get action permissions
    #[tool(description = "Get permission configuration for actions. Shows whether actions require confirmation, auto-approve, or are denied.")]
    async fn get_action_permissions(
        &self,
        Parameters(params): Parameters<GetActionPermissionsParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;
        let repo = MCPActionRepository::new(db);

        if let Some(action_id) = params.action_id {
            // Get the action to get its type
            let action = repo.get_action(&action_id)
                .map_err(|e| McpError::internal_error(e, None))?
                .ok_or_else(|| McpError::invalid_params(
                    format!("Action not found: {}", action_id),
                    None
                ))?;

            // Get permission for specific action
            let permission = repo.get_permission(Some(&action_id), &action.action_type)
                .map_err(|e| McpError::internal_error(e, None))?;

            let response = serde_json::json!({
                "actionId": action_id,
                "permissionLevel": permission.to_string()
            });

            let json = serde_json::to_string_pretty(&response)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            Ok(CallToolResult::success(vec![Content::text(json)]))
        } else {
            // List all permissions
            let permissions = repo.list_permissions()
                .map_err(|e| McpError::internal_error(e, None))?;

            let response = serde_json::json!({
                "permissions": permissions,
                "defaultLevel": "require_confirm"
            });

            let json = serde_json::to_string_pretty(&response)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            Ok(CallToolResult::success(vec![Content::text(json)]))
        }
    }

    // ========================================================================
    // Background Process Management Tools
    // ========================================================================

    /// Get output from a background process
    #[tool(description = "Get output from a background process started with run_npm_script (runInBackground: true). Returns the tail of stdout/stderr output.")]
    async fn get_background_process_output(
        &self,
        Parameters(params): Parameters<GetBackgroundProcessOutputParams>,
    ) -> Result<CallToolResult, McpError> {
        match BACKGROUND_PROCESS_MANAGER.get_output(&params.process_id, params.tail_lines).await {
            Ok(output) => {
                let json = serde_json::to_string_pretty(&output)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => {
                Ok(CallToolResult::error(vec![Content::text(e)]))
            }
        }
    }

    /// Stop a background process
    #[tool(description = "Stop/terminate a background process. Use force=true to send SIGKILL instead of SIGTERM.")]
    async fn stop_background_process(
        &self,
        Parameters(params): Parameters<StopBackgroundProcessParams>,
    ) -> Result<CallToolResult, McpError> {
        match BACKGROUND_PROCESS_MANAGER.stop_process(&params.process_id, params.force).await {
            Ok(()) => {
                let response = serde_json::json!({
                    "success": true,
                    "process_id": params.process_id,
                    "message": if params.force {
                        "Process killed (SIGKILL)"
                    } else {
                        "Process terminated (SIGTERM)"
                    }
                });
                let json = serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => {
                Ok(CallToolResult::error(vec![Content::text(e)]))
            }
        }
    }

    /// List all background processes
    #[tool(description = "List all background processes (running and recently completed).")]
    async fn list_background_processes(
        &self,
        #[allow(unused_variables)]
        Parameters(_params): Parameters<serde_json::Value>,
    ) -> Result<CallToolResult, McpError> {
        let processes = BACKGROUND_PROCESS_MANAGER.list_processes().await;

        let response = serde_json::json!({
            "processes": processes,
            "total": processes.len()
        });

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // Enhanced MCP Tools (New)
    // ========================================================================

    /// Get system environment information for troubleshooting
    #[tool(description = "Get system environment information including detected tools, versions, and paths. Useful for troubleshooting build/execution issues.")]
    async fn get_environment_info(
        &self,
        Parameters(params): Parameters<GetEnvironmentInfoParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut info = serde_json::json!({});

        // Get tool versions using path_resolver
        let node_version = path_resolver::create_command("node")
            .args(&["--version"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        let npm_version = path_resolver::create_command("npm")
            .args(&["--version"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        let pnpm_version = path_resolver::create_command("pnpm")
            .args(&["--version"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        let yarn_version = path_resolver::create_command("yarn")
            .args(&["--version"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        let git_version = path_resolver::create_command("git")
            .args(&["--version"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        let rust_version = path_resolver::create_command("rustc")
            .args(&["--version"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        // Check for tool managers
        let volta_installed = path_resolver::create_command("volta")
            .args(&["--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        let homebrew_installed = path_resolver::create_command("brew")
            .args(&["--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        info["nodeVersion"] = serde_json::json!(node_version);
        info["npmVersion"] = serde_json::json!(npm_version);
        info["pnpmVersion"] = serde_json::json!(pnpm_version);
        info["yarnVersion"] = serde_json::json!(yarn_version);
        info["gitVersion"] = serde_json::json!(git_version);
        info["rustVersion"] = serde_json::json!(rust_version);
        info["voltaInstalled"] = serde_json::json!(volta_installed);
        info["homebrewInstalled"] = serde_json::json!(homebrew_installed);

        // Include PATH if requested
        if params.include_paths {
            let path_env = std::env::var("PATH").unwrap_or_default();
            let paths: Vec<&str> = path_env.split(':').collect();
            info["pathEntries"] = serde_json::json!(paths);
        }

        // Check project-specific toolchain if provided
        if let Some(project_path) = params.project_path {
            let path = PathBuf::from(&project_path);
            if path.exists() {
                let node_version_file = Self::get_node_version(&path);
                info["projectToolchain"] = serde_json::json!({
                    "nodeVersionFile": node_version_file,
                    "path": project_path,
                });
            }
        }

        let json = serde_json::to_string_pretty(&info)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// List configured AI providers
    #[tool(description = "List all configured AI providers including their status, default model, and whether they are enabled. Use this to help users select or switch providers.")]
    async fn list_ai_providers(
        &self,
        Parameters(params): Parameters<ListAIProvidersParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;
        let repo = AIRepository::new(db);

        let providers = repo.list_providers()
            .map_err(|e| McpError::internal_error(e, None))?;

        let filtered: Vec<_> = if params.enabled_only {
            providers.into_iter().filter(|p| p.is_enabled).collect()
        } else {
            providers
        };

        // Find default provider
        let default_id = filtered.iter()
            .find(|p| p.is_default)
            .map(|p| p.id.clone());

        let provider_list: Vec<serde_json::Value> = filtered.iter().map(|p| {
            serde_json::json!({
                "id": p.id,
                "name": p.name,
                "provider": p.provider.to_string(),
                "model": p.model,
                "isDefault": p.is_default,
                "isEnabled": p.is_enabled,
            })
        }).collect();

        let response = serde_json::json!({
            "providers": provider_list,
            "defaultProviderId": default_id,
            "total": provider_list.len()
        });

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Check if files exist within a project
    #[tool(description = "Check if specified files or directories exist within a registered project. Use this to verify project structure before suggesting commands. IMPORTANT: Only works within registered projects for security.")]
    async fn check_file_exists(
        &self,
        Parameters(params): Parameters<CheckFileExistsParams>,
    ) -> Result<CallToolResult, McpError> {
        let base_path = PathBuf::from(&params.project_path);

        if !base_path.exists() {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Project path does not exist: {}", params.project_path)
            )]));
        }

        // Validate against registered projects
        let store_data = read_store_data()
            .map_err(|e| McpError::internal_error(e, None))?;

        let is_registered = store_data.projects.iter()
            .any(|p| p.path == params.project_path);

        if !is_registered {
            return Ok(CallToolResult::error(vec![Content::text(
                "Project path is not registered in SpecForge. Register the project first."
            )]));
        }

        let mut results = serde_json::Map::new();
        for relative_path in &params.paths {
            // Prevent path traversal
            if relative_path.contains("..") {
                results.insert(relative_path.clone(), serde_json::json!({
                    "exists": false,
                    "error": "Path traversal not allowed"
                }));
                continue;
            }

            let full_path = base_path.join(relative_path);
            if full_path.exists() {
                let is_file = full_path.is_file();
                let is_dir = full_path.is_dir();
                results.insert(relative_path.clone(), serde_json::json!({
                    "exists": true,
                    "isFile": is_file,
                    "isDirectory": is_dir
                }));
            } else {
                results.insert(relative_path.clone(), serde_json::json!({
                    "exists": false
                }));
            }
        }

        let response = serde_json::json!({
            "projectPath": params.project_path,
            "results": results
        });

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// List past AI conversations
    #[tool(description = "List past AI assistant conversations. Use this to help users find and resume previous conversations or to understand conversation history.")]
    async fn list_conversations(
        &self,
        Parameters(params): Parameters<ListConversationsParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;
        let repo = AIConversationRepository::new(db);

        let limit = params.limit.min(100);
        let response = repo.list_conversations(
            params.project_path.as_deref(),
            limit,
            0,
            "updated",
        ).map_err(|e| McpError::internal_error(e, None))?;

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get recent notifications
    #[tool(description = "Get recent notifications from SpecForge including workflow executions, security alerts, and deployment status. Use this to provide users with updates on recent activity.")]
    async fn get_notifications(
        &self,
        Parameters(params): Parameters<GetNotificationsParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;
        let repo = NotificationRepository::new(db);

        let limit = (params.limit as usize).min(100);
        let response = repo.get_recent(limit, 0)
            .map_err(|e| McpError::internal_error(e, None))?;

        // Filter by category if specified
        let filtered_notifications: Vec<_> = if let Some(ref category) = params.category {
            response.notifications.into_iter()
                .filter(|n| n.category == *category)
                .collect()
        } else if params.unread_only {
            response.notifications.into_iter()
                .filter(|n| !n.is_read)
                .collect()
        } else {
            response.notifications
        };

        let result = serde_json::json!({
            "notifications": filtered_notifications,
            "totalCount": response.total_count,
            "unreadCount": response.unread_count
        });

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Mark notifications as read
    #[tool(description = "Mark one or more notifications as read. Use this after presenting notifications to the user.")]
    async fn mark_notifications_read(
        &self,
        Parameters(params): Parameters<MarkNotificationsReadParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;
        let repo = NotificationRepository::new(db);

        let marked_count = if params.mark_all {
            repo.mark_all_as_read()
                .map_err(|e| McpError::internal_error(e, None))?
        } else if let Some(ids) = params.notification_ids {
            let mut count = 0u32;
            for id in ids {
                if repo.mark_as_read(&id).map_err(|e| McpError::internal_error(e, None))? {
                    count += 1;
                }
            }
            count
        } else {
            return Ok(CallToolResult::error(vec![Content::text(
                "Provide either notification_ids or set mark_all to true"
            )]));
        };

        let response = serde_json::json!({
            "success": true,
            "markedCount": marked_count
        });

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get security scan results for a project
    #[tool(description = "Get the latest security scan results for a project including vulnerability counts and severity levels. Use this to help users understand security status.")]
    async fn get_security_scan_results(
        &self,
        Parameters(params): Parameters<GetSecurityScanResultsParams>,
    ) -> Result<CallToolResult, McpError> {
        // Verify project exists
        let store_data = read_store_data()
            .map_err(|e| McpError::internal_error(e, None))?;

        let project = store_data.projects.iter()
            .find(|p| p.path == params.project_path);

        let project_id = match project {
            Some(p) => p.id.clone(),
            None => {
                return Ok(CallToolResult::error(vec![Content::text(
                    format!("Project not found for path: {}. Register the project first.", params.project_path)
                )]));
            }
        };

        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;
        let repo = SecurityRepository::new(db);

        let scan_data = repo.get(&project_id)
            .map_err(|e| McpError::internal_error(e, None))?;

        match scan_data {
            Some(data) => {
                let response = serde_json::json!({
                    "projectPath": params.project_path,
                    "packageManager": format!("{:?}", data.package_manager).to_lowercase(),
                    "lastScan": data.last_scan,
                    "snoozeUntil": data.snooze_until,
                    "scanHistory": data.scan_history.len()
                });

                let json = serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            None => {
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::json!({
                        "projectPath": params.project_path,
                        "message": "No security scans found for this project",
                        "lastScan": null
                    }).to_string()
                )]))
            }
        }
    }

    /// Run a security scan
    #[tool(description = "Run a security vulnerability scan (npm audit / yarn audit / pnpm audit) for a project. Returns vulnerability summary. May take 10-30 seconds.")]
    async fn run_security_scan(
        &self,
        Parameters(params): Parameters<RunSecurityScanParams>,
    ) -> Result<CallToolResult, McpError> {
        let project_path = PathBuf::from(&params.project_path);

        if !project_path.exists() {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Project path does not exist: {}", params.project_path)
            )]));
        }

        // Detect package manager using read_package_json
        let (pm, _, _) = Self::read_package_json(&project_path);
        let package_manager = pm.unwrap_or_else(|| "npm".to_string());

        // Build audit command
        let command = match package_manager.as_str() {
            "pnpm" => if params.fix { "pnpm audit --fix" } else { "pnpm audit --json" },
            "yarn" => if params.fix { "yarn audit --fix" } else { "yarn audit --json" },
            _ => if params.fix { "npm audit fix" } else { "npm audit --json" },
        };

        // Use shell_command_async with 30 second timeout
        let result = Self::shell_command_async(
            &params.project_path,
            command,
            Some(30_000),
        ).await;

        match result {
            Ok((exit_code, stdout, stderr)) => {
                let output = if !stdout.is_empty() { stdout } else { stderr };
                // Truncate output if too long (UTF-8 safe)
                let truncated_output = if output.len() > 10000 {
                    let truncate_at = output
                        .char_indices()
                        .take_while(|(i, _)| *i < 10000)
                        .last()
                        .map(|(i, c)| i + c.len_utf8())
                        .unwrap_or(output.len().min(10000));
                    format!("{}...[truncated]", &output[..truncate_at])
                } else {
                    output
                };
                let response = serde_json::json!({
                    "success": exit_code == 0,
                    "packageManager": package_manager,
                    "fixAttempted": params.fix,
                    "exitCode": exit_code,
                    "output": truncated_output,
                });

                let json = serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => {
                // Command failed to execute
                let response = serde_json::json!({
                    "success": false,
                    "packageManager": package_manager,
                    "error": sanitize_error(&e),
                });

                let json = serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
        }
    }

    // ========================================================================
    // Time Machine & Security Guardian Tools
    // ========================================================================

    /// Check dependency integrity against reference snapshot
    #[tool(description = "Check dependency integrity against a reference snapshot. Detects added, removed, or modified packages including postinstall script changes. Use this to verify dependencies haven't unexpectedly changed.")]
    async fn check_dependency_integrity(
        &self,
        Parameters(params): Parameters<CheckDependencyIntegrityParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;

        let service = DependencyIntegrityService::new(db);
        let result = service.check_integrity(&params.project_path)
            .map_err(|e| McpError::internal_error(e, None))?;

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get security insights for a project
    #[tool(description = "Get security insights and risk overview for a project. Returns risk score, typosquatting alerts, frequent updaters, and dependency health information.")]
    async fn get_security_insights(
        &self,
        Parameters(params): Parameters<GetSecurityInsightsParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;

        let service = SecurityInsightsService::new(db);
        let overview = service.get_project_overview(&params.project_path)
            .map_err(|e| McpError::internal_error(e, None))?;

        let json = serde_json::to_string_pretty(&overview)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// List execution snapshots
    #[tool(description = "List execution snapshots for a workflow or project. Snapshots capture the dependency state at execution time for comparison and replay.")]
    async fn list_execution_snapshots(
        &self,
        Parameters(params): Parameters<ListExecutionSnapshotsParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;

        let repo = SnapshotRepository::new(db);
        let filter = SnapshotFilter {
            project_path: params.project_path.clone(),
            trigger_source: None,
            status: None,
            from_date: None,
            to_date: None,
            limit: Some(params.limit),
            offset: None,
        };

        let snapshots = repo.list_snapshots(&filter)
            .map_err(|e| McpError::internal_error(e, None))?;

        let response = serde_json::json!({
            "snapshots": snapshots,
            "total": snapshots.len()
        });

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get snapshot details
    #[tool(description = "Get detailed information about a specific snapshot including lockfile info, dependency counts, and security score. Optionally include full dependency list.")]
    async fn get_snapshot_details(
        &self,
        Parameters(params): Parameters<GetSnapshotDetailsParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;

        let repo = SnapshotRepository::new(db);

        if params.include_dependencies {
            let snapshot = repo.get_snapshot_with_dependencies(&params.snapshot_id)
                .map_err(|e| McpError::internal_error(e, None))?;

            match snapshot {
                Some(s) => {
                    let json = serde_json::to_string_pretty(&s)
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                    Ok(CallToolResult::success(vec![Content::text(json)]))
                }
                None => {
                    Ok(CallToolResult::error(vec![Content::text(
                        format!("Snapshot not found: {}", params.snapshot_id)
                    )]))
                }
            }
        } else {
            let snapshot = repo.get_snapshot(&params.snapshot_id)
                .map_err(|e| McpError::internal_error(e, None))?;

            match snapshot {
                Some(s) => {
                    let json = serde_json::to_string_pretty(&s)
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                    Ok(CallToolResult::success(vec![Content::text(json)]))
                }
                None => {
                    Ok(CallToolResult::error(vec![Content::text(
                        format!("Snapshot not found: {}", params.snapshot_id)
                    )]))
                }
            }
        }
    }

    /// Compare two snapshots
    #[tool(description = "Compare two snapshots to see dependency changes between executions. Returns added, removed, and updated packages with version changes and postinstall script modifications.")]
    async fn compare_snapshots(
        &self,
        Parameters(params): Parameters<CompareSnapshotsParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;

        let diff_service = SnapshotDiffService::new(db);

        let diff = diff_service.compare_snapshots(&params.snapshot_a_id, &params.snapshot_b_id)
            .map_err(|e| McpError::internal_error(e, None))?;

        let json = serde_json::to_string_pretty(&diff)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Search snapshots
    #[tool(description = "Search snapshots by package name, version, or date range. Find when specific packages were added, removed, or updated across executions.")]
    async fn search_snapshots(
        &self,
        Parameters(params): Parameters<SearchSnapshotsParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;

        let service = SnapshotSearchService::new(db);
        let criteria = SnapshotSearchCriteria {
            package_name: params.package_name.clone(),
            package_version: params.package_version.clone(),
            project_path: params.project_path.clone(),
            from_date: params.from_date.clone(),
            to_date: params.to_date.clone(),
            has_postinstall: None,
            min_security_score: None,
            max_security_score: None,
            limit: Some(params.limit),
            offset: None,
        };

        let results = service.search(&criteria)
            .map_err(|e| McpError::internal_error(e, None))?;

        let json = serde_json::to_string_pretty(&results)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Replay execution from snapshot
    #[tool(description = "Replay a workflow execution from a snapshot. Can restore lockfile to match the snapshot state before re-running the workflow.")]
    async fn replay_execution(
        &self,
        Parameters(params): Parameters<ReplayExecutionParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;

        // Get storage base path for time machine
        let storage_base = dirs::data_dir()
            .map(|p| p.join("com.specforge.app").join("time-machine"))
            .ok_or_else(|| McpError::internal_error("Failed to get data directory", None))?;

        let storage = SnapshotStorage::new(storage_base);
        let service = SnapshotReplayService::new(storage, db);

        // Parse replay option
        let option = match params.option.as_str() {
            "abort" => ReplayOption::Abort,
            "view_diff" => ReplayOption::ViewDiff,
            "restore_lockfile" => ReplayOption::RestoreLockfile,
            "proceed_with_current" => ReplayOption::ProceedWithCurrent,
            _ => ReplayOption::Abort,
        };

        let request = ExecuteReplayRequest {
            snapshot_id: params.snapshot_id.clone(),
            option,
            force: params.force,
        };

        // First prepare the replay to check for mismatches
        let preparation = service.prepare_replay(&params.snapshot_id)
            .map_err(|e| McpError::internal_error(e, None))?;

        if !preparation.ready_to_replay && !params.force {
            // Return preparation info so user can see mismatches
            let json = serde_json::to_string_pretty(&preparation)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            return Ok(CallToolResult::success(vec![Content::text(json)]));
        }

        // Execute the replay
        let result = service.execute_replay(&request)
            .map_err(|e| McpError::internal_error(e, None))?;

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Export security report
    #[tool(description = "Generate and export a security audit report for a project. Supports JSON, Markdown, or HTML formats.")]
    async fn export_security_report(
        &self,
        Parameters(params): Parameters<ExportSecurityReportParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;

        let service = SnapshotSearchService::new(db);

        // Generate the audit report
        let report = service.generate_audit_report(&params.project_path)
            .map_err(|e| McpError::internal_error(e, None))?;

        // Parse format
        let format = match params.format.to_lowercase().as_str() {
            "json" => ExportFormat::Json,
            "markdown" | "md" => ExportFormat::Markdown,
            "html" => ExportFormat::Html,
            _ => ExportFormat::Markdown,
        };

        // Export the report
        let content = service.export_report(&report, format);

        Ok(CallToolResult::success(vec![Content::text(content)]))
    }

    /// Capture a manual snapshot
    #[tool(description = "Manually capture a Time Machine snapshot for a project. Captures current dependency state from lockfile.")]
    async fn capture_snapshot(
        &self,
        Parameters(params): Parameters<CaptureSnapshotParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;

        // Use the capture service to create a manual snapshot
        let storage_base = dirs::data_dir()
            .map(|p| p.join("com.specforge.app").join("time-machine"))
            .ok_or_else(|| McpError::internal_error("Failed to get data directory", None))?;

        let storage = SnapshotStorage::new(storage_base);
        let capture_service = SnapshotCaptureService::new(storage, db);

        let snapshot = capture_service.capture_manual_snapshot(&params.project_path)
            .map_err(|e| McpError::internal_error(e, None))?;

        let response = serde_json::json!({
            "success": true,
            "snapshot": {
                "id": snapshot.id,
                "projectPath": snapshot.project_path,
                "triggerSource": snapshot.trigger_source,
                "status": snapshot.status,
                "totalDependencies": snapshot.total_dependencies,
                "securityScore": snapshot.security_score,
                "createdAt": snapshot.created_at,
            },
            "message": format!("Snapshot captured successfully: {}", snapshot.id)
        });

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// List deployment history
    #[tool(description = "List deployment history for a project including status, platform, and timestamps. Use this to help users track deployment status and history.")]
    async fn list_deployments(
        &self,
        Parameters(params): Parameters<ListDeploymentsParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;
        let repo = DeployRepository::new(db);

        // Get project ID from path if provided
        let project_id = if let Some(ref path) = params.project_path {
            let store_data = read_store_data()
                .map_err(|e| McpError::internal_error(e, None))?;

            store_data.projects.iter()
                .find(|p| p.path == *path)
                .map(|p| p.id.clone())
        } else {
            None
        };

        let deployments = if let Some(ref id) = project_id {
            repo.list_deployments(id)
                .map_err(|e| McpError::internal_error(e, None))?
        } else {
            // If no project specified, return empty (or could list all)
            Vec::new()
        };

        // Apply filters
        let filtered: Vec<_> = deployments.into_iter()
            .filter(|d| {
                if let Some(ref platform) = params.platform {
                    let d_platform = format!("{:?}", d.platform).to_lowercase();
                    if !d_platform.contains(&platform.to_lowercase()) {
                        return false;
                    }
                }
                if let Some(ref status) = params.status {
                    let d_status = format!("{:?}", d.status).to_lowercase();
                    if !d_status.contains(&status.to_lowercase()) {
                        return false;
                    }
                }
                true
            })
            .take(params.limit as usize)
            .collect();

        let response = serde_json::json!({
            "deployments": filtered,
            "total": filtered.len()
        });

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get project dependencies
    #[tool(description = "Get project dependencies from package.json including versions and types. Use this to understand project requirements.")]
    async fn get_project_dependencies(
        &self,
        Parameters(params): Parameters<GetProjectDependenciesParams>,
    ) -> Result<CallToolResult, McpError> {
        let project_path = PathBuf::from(&params.project_path);
        let package_json_path = project_path.join("package.json");

        if !package_json_path.exists() {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("No package.json found at: {}", params.project_path)
            )]));
        }

        let content = std::fs::read_to_string(&package_json_path)
            .map_err(|e| McpError::internal_error(format!("Failed to read package.json: {}", e), None))?;

        let pkg: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| McpError::internal_error(format!("Failed to parse package.json: {}", e), None))?;

        let mut response = serde_json::json!({
            "projectPath": params.project_path,
        });

        if let Some(deps) = pkg.get("dependencies") {
            response["dependencies"] = deps.clone();
        }

        if params.include_dev {
            if let Some(dev_deps) = pkg.get("devDependencies") {
                response["devDependencies"] = dev_deps.clone();
            }
        }

        if params.include_peer {
            if let Some(peer_deps) = pkg.get("peerDependencies") {
                response["peerDependencies"] = peer_deps.clone();
            }
        }

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Update workflow properties
    #[tool(description = "Update a workflow's name or description. Use list_workflows first to get the workflow_id. Does NOT modify workflow steps.")]
    async fn update_workflow(
        &self,
        Parameters(params): Parameters<UpdateWorkflowParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut store_data = read_store_data()
            .map_err(|e| McpError::internal_error(e, None))?;

        // Find workflow index to avoid borrow issues
        let workflow_idx = store_data.workflows.iter()
            .position(|w| w.id == params.workflow_id);

        match workflow_idx {
            Some(idx) => {
                // Update workflow
                if let Some(name) = params.name.clone() {
                    store_data.workflows[idx].name = name;
                }
                if let Some(desc) = params.description.clone() {
                    store_data.workflows[idx].description = Some(desc);
                }
                let updated_at = chrono::Utc::now().to_rfc3339();
                store_data.workflows[idx].updated_at = updated_at.clone();

                // Get values for response before write
                let name = store_data.workflows[idx].name.clone();
                let description = store_data.workflows[idx].description.clone();

                write_store_data(&store_data)
                    .map_err(|e| McpError::internal_error(e, None))?;

                let response = serde_json::json!({
                    "success": true,
                    "workflowId": params.workflow_id,
                    "name": name,
                    "description": description,
                    "updatedAt": updated_at
                });

                let json = serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            None => {
                Ok(CallToolResult::error(vec![Content::text(
                    format!("Workflow not found: {}", params.workflow_id)
                )]))
            }
        }
    }

    /// Delete a step from a workflow
    #[tool(description = "Remove a step from a workflow. Use get_workflow first to see step IDs. This is a destructive operation.")]
    async fn delete_workflow_step(
        &self,
        Parameters(params): Parameters<DeleteWorkflowStepParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut store_data = read_store_data()
            .map_err(|e| McpError::internal_error(e, None))?;

        // Find workflow index to avoid borrow issues
        let workflow_idx = store_data.workflows.iter()
            .position(|w| w.id == params.workflow_id);

        match workflow_idx {
            Some(idx) => {
                let original_len = store_data.workflows[idx].nodes.len();
                store_data.workflows[idx].nodes.retain(|n| n.id != params.step_id);

                if store_data.workflows[idx].nodes.len() == original_len {
                    return Ok(CallToolResult::error(vec![Content::text(
                        format!("Step not found: {} in workflow {}", params.step_id, params.workflow_id)
                    )]));
                }

                store_data.workflows[idx].updated_at = chrono::Utc::now().to_rfc3339();

                // Get remaining steps count before write
                let remaining_steps = store_data.workflows[idx].nodes.len();

                write_store_data(&store_data)
                    .map_err(|e| McpError::internal_error(e, None))?;

                let response = serde_json::json!({
                    "success": true,
                    "workflowId": params.workflow_id,
                    "deletedStepId": params.step_id,
                    "remainingSteps": remaining_steps
                });

                let json = serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            None => {
                Ok(CallToolResult::error(vec![Content::text(
                    format!("Workflow not found: {}", params.workflow_id)
                )]))
            }
        }
    }

    /// Get detailed execution logs for a workflow run
    #[tool(description = "Get detailed execution logs for a specific workflow run including step-by-step output and timing. Use list_action_executions first to find execution IDs.")]
    async fn get_workflow_execution_details(
        &self,
        Parameters(params): Parameters<GetWorkflowExecutionDetailsParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = open_database()
            .map_err(|e| McpError::internal_error(e, None))?;
        let repo = MCPActionRepository::new(db);

        let execution = repo.get_execution(&params.execution_id)
            .map_err(|e| McpError::internal_error(e, None))?;

        match execution {
            Some(exec) => {
                let mut response = serde_json::json!({
                    "executionId": exec.id,
                    "actionId": exec.action_id,
                    "actionName": exec.action_name,
                    "actionType": exec.action_type.to_string(),
                    "status": exec.status.to_string(),
                    "startedAt": exec.started_at,
                    "completedAt": exec.completed_at,
                    "durationMs": exec.duration_ms,
                });

                if params.include_output {
                    if let Some(result) = exec.result {
                        let result_str = serde_json::to_string(&result).unwrap_or_default();
                        let truncated = if result_str.len() > params.truncate_output {
                            format!("{}...[truncated]", &result_str[..params.truncate_output])
                        } else {
                            result_str
                        };
                        response["result"] = serde_json::json!(truncated);
                    }
                }

                if let Some(err) = exec.error_message {
                    response["error"] = serde_json::json!(sanitize_error(&err));
                }

                let json = serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            None => {
                Ok(CallToolResult::error(vec![Content::text(
                    format!("Execution not found: {}", params.execution_id)
                )]))
            }
        }
    }

    /// Search for files within a project
    #[tool(description = "Search for files within a registered project by name pattern. Useful for finding configuration files or source files. Respects .gitignore patterns.")]
    async fn search_project_files(
        &self,
        Parameters(params): Parameters<SearchProjectFilesParams>,
    ) -> Result<CallToolResult, McpError> {
        let base_path = PathBuf::from(&params.project_path);

        if !base_path.exists() {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Project path does not exist: {}", params.project_path)
            )]));
        }

        // Verify project is registered
        let store_data = read_store_data()
            .map_err(|e| McpError::internal_error(e, None))?;

        let is_registered = store_data.projects.iter()
            .any(|p| p.path == params.project_path);

        if !is_registered {
            return Ok(CallToolResult::error(vec![Content::text(
                "Project path is not registered in SpecForge."
            )]));
        }

        // Use glob to find files
        let glob_pattern = format!("{}/{}", params.project_path, params.pattern);
        let matches: Vec<String> = glob::glob(&glob_pattern)
            .map_err(|e| McpError::internal_error(format!("Invalid pattern: {}", e), None))?
            .filter_map(|r| r.ok())
            .filter(|p| {
                if params.include_directories {
                    true
                } else {
                    p.is_file()
                }
            })
            .take(params.max_results)
            .filter_map(|p| p.strip_prefix(&base_path).ok().map(|r| r.to_string_lossy().to_string()))
            .collect();

        let response = serde_json::json!({
            "projectPath": params.project_path,
            "pattern": params.pattern,
            "matches": matches,
            "totalFound": matches.len()
        });

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Read file content from a project
    #[tool(description = "Read the content of a file within a registered project. Limited to common configuration and source files. SECURITY: Excludes sensitive files (.env, credentials).")]
    async fn read_project_file(
        &self,
        Parameters(params): Parameters<ReadProjectFileParams>,
    ) -> Result<CallToolResult, McpError> {
        // Prevent path traversal
        if params.file_path.contains("..") {
            return Ok(CallToolResult::error(vec![Content::text(
                "Path traversal not allowed"
            )]));
        }

        // Block sensitive files
        let blocklist = [".env", "credentials", "secrets", ".key", ".pem", ".p12"];
        let file_lower = params.file_path.to_lowercase();
        for blocked in blocklist {
            if file_lower.contains(blocked) {
                return Ok(CallToolResult::error(vec![Content::text(
                    format!("Access to sensitive file blocked: {}", params.file_path)
                )]));
            }
        }

        let base_path = PathBuf::from(&params.project_path);
        let full_path = base_path.join(&params.file_path);

        if !full_path.exists() {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("File not found: {}", params.file_path)
            )]));
        }

        if !full_path.is_file() {
            return Ok(CallToolResult::error(vec![Content::text(
                format!("Not a file: {}", params.file_path)
            )]));
        }

        // Verify project is registered
        let store_data = read_store_data()
            .map_err(|e| McpError::internal_error(e, None))?;

        let is_registered = store_data.projects.iter()
            .any(|p| p.path == params.project_path);

        if !is_registered {
            return Ok(CallToolResult::error(vec![Content::text(
                "Project path is not registered in SpecForge."
            )]));
        }

        // Check file size (max 1MB)
        let metadata = std::fs::metadata(&full_path)
            .map_err(|e| McpError::internal_error(format!("Failed to read metadata: {}", e), None))?;

        if metadata.len() > 1_000_000 {
            return Ok(CallToolResult::error(vec![Content::text(
                "File too large (max 1MB)"
            )]));
        }

        // Read file content
        let content = std::fs::read_to_string(&full_path)
            .map_err(|e| McpError::internal_error(format!("Failed to read file: {}", e), None))?;

        // Apply line limits
        let lines: Vec<&str> = content.lines().collect();
        let start_idx = (params.start_line.saturating_sub(1)).min(lines.len());
        let end_idx = (start_idx + params.max_lines).min(lines.len());
        let selected_lines: Vec<&str> = lines[start_idx..end_idx].to_vec();

        let response = serde_json::json!({
            "projectPath": params.project_path,
            "filePath": params.file_path,
            "content": selected_lines.join("\n"),
            "startLine": start_idx + 1,
            "endLine": end_idx,
            "totalLines": lines.len(),
            "hasMore": end_idx < lines.len()
        });

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

// Implement ServerHandler trait for the MCP server
impl ServerHandler for SpecForgeMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability::default()),
                ..Default::default()
            },
            server_info: Implementation {
                name: "specforge-mcp".to_string(),
                title: Some("SpecForge MCP Server".to_string()),
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some("SpecForge MCP Server provides tools for managing Git projects, worktrees, workflows, and step templates.".to_string()),
        }
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        async move {
            Ok(ListToolsResult {
                tools: self.tool_router.list_all(),
                next_cursor: None,
            })
        }
    }

    fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<rmcp::RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        async move {
            let start_time = Instant::now();
            let tool_name = request.name.clone();
            let arguments_map = request.arguments.clone().unwrap_or_default();
            // Convert Map<String, Value> to Value for logging
            let arguments = serde_json::Value::Object(arguments_map.clone());

            // Read MCP config from store
            let config = match read_store_data() {
                Ok(data) => {
                    eprintln!("[MCP Debug] call_tool - Store read success");
                    eprintln!("[MCP Debug] call_tool - permission_mode: {:?}", data.mcp_config.permission_mode);
                    eprintln!("[MCP Debug] call_tool - is_enabled: {}", data.mcp_config.is_enabled);
                    eprintln!("[MCP Debug] call_tool - allowed_tools: {:?}", data.mcp_config.allowed_tools);
                    data.mcp_config
                }
                Err(e) => {
                    eprintln!("[MCP Debug] call_tool - Store read FAILED: {}", e);
                    eprintln!("[MCP Debug] call_tool - Using default config (ReadOnly)");
                    MCPServerConfig::default()
                }
            };

            // Check if MCP server is enabled
            if !config.is_enabled {
                let error_msg = "MCP Server is disabled. Enable it in SpecForge settings.";
                if config.log_requests {
                    log_request(&tool_name, &arguments, "permission_denied", 0, Some(error_msg));
                }
                return Ok(CallToolResult::error(vec![Content::text(error_msg)]));
            }

            // Check global rate limit (100 requests per minute)
            if let Err(rate_error) = RATE_LIMITER.check_and_increment() {
                if config.log_requests {
                    log_request(&tool_name, &arguments, "rate_limited", 0, Some(&rate_error));
                }
                return Ok(CallToolResult::error(vec![Content::text(rate_error)]));
            }

            // Check tool-level rate limit (category-specific limits)
            let tool_category = get_tool_category(&tool_name);
            if let Err(_) = TOOL_RATE_LIMITERS.check(tool_category) {
                let limit_desc = TOOL_RATE_LIMITERS.get_limit_description(tool_category);
                let error_msg = format!(
                    "Tool rate limit exceeded for '{}'. Limit: {}. Please wait before making more requests.",
                    tool_name, limit_desc
                );
                if config.log_requests {
                    log_request(&tool_name, &arguments, "tool_rate_limited", 0, Some(&error_msg));
                }
                return Ok(CallToolResult::error(vec![Content::text(error_msg)]));
            }

            // Check permission
            if let Err(permission_error) = is_tool_allowed(&tool_name, &config) {
                let duration_ms = start_time.elapsed().as_millis() as u64;
                if config.log_requests {
                    log_request(&tool_name, &arguments, "permission_denied", duration_ms, Some(&permission_error));
                }
                return Ok(CallToolResult::error(vec![Content::text(permission_error)]));
            }

            // Validate path parameters in arguments
            // Support both snake_case and camelCase (for tools with serde rename_all = "camelCase")
            if let Some(path) = arguments.get("path").and_then(|v| v.as_str()) {
                if let Err(e) = validate_path(path) {
                    let error_msg = format!("Invalid path: {}", e);
                    if config.log_requests {
                        log_request(&tool_name, &arguments, "validation_error", 0, Some(&error_msg));
                    }
                    return Ok(CallToolResult::error(vec![Content::text(error_msg)]));
                }
            }
            // Check both snake_case (project_path) and camelCase (projectPath)
            let project_path_value = arguments.get("project_path")
                .or_else(|| arguments.get("projectPath"))
                .and_then(|v| v.as_str());
            if let Some(path) = project_path_value {
                if let Err(e) = validate_path(path) {
                    let error_msg = format!("Invalid project_path: {}", e);
                    if config.log_requests {
                        log_request(&tool_name, &arguments, "validation_error", 0, Some(&error_msg));
                    }
                    return Ok(CallToolResult::error(vec![Content::text(error_msg)]));
                }
            }
            // Check both snake_case (worktree_path) and camelCase (worktreePath)
            let worktree_path_value = arguments.get("worktree_path")
                .or_else(|| arguments.get("worktreePath"))
                .and_then(|v| v.as_str());
            if let Some(path) = worktree_path_value {
                if let Err(e) = validate_path(path) {
                    let error_msg = format!("Invalid worktree_path: {}", e);
                    if config.log_requests {
                        log_request(&tool_name, &arguments, "validation_error", 0, Some(&error_msg));
                    }
                    return Ok(CallToolResult::error(vec![Content::text(error_msg)]));
                }
            }

            // Validate command parameter (for add_workflow_step)
            if let Some(command) = arguments.get("command").and_then(|v| v.as_str()) {
                if let Err(e) = validate_command(command) {
                    let error_msg = format!("Invalid command: {}", e);
                    if config.log_requests {
                        log_request(&tool_name, &arguments, "validation_error", 0, Some(&error_msg));
                    }
                    return Ok(CallToolResult::error(vec![Content::text(error_msg)]));
                }
            }

            // Validate name length parameters
            if let Some(name) = arguments.get("name").and_then(|v| v.as_str()) {
                if let Err(e) = validate_string_length(name, "name", MAX_NAME_LENGTH) {
                    let error_msg = e;
                    if config.log_requests {
                        log_request(&tool_name, &arguments, "validation_error", 0, Some(&error_msg));
                    }
                    return Ok(CallToolResult::error(vec![Content::text(error_msg)]));
                }
            }
            if let Some(desc) = arguments.get("description").and_then(|v| v.as_str()) {
                if let Err(e) = validate_string_length(desc, "description", MAX_DESCRIPTION_LENGTH) {
                    let error_msg = e;
                    if config.log_requests {
                        log_request(&tool_name, &arguments, "validation_error", 0, Some(&error_msg));
                    }
                    return Ok(CallToolResult::error(vec![Content::text(error_msg)]));
                }
            }

            // Validate timeout parameter
            if let Some(timeout) = arguments.get("timeout").and_then(|v| v.as_u64()) {
                if let Err(e) = validate_timeout(timeout) {
                    let error_msg = e;
                    if config.log_requests {
                        log_request(&tool_name, &arguments, "validation_error", 0, Some(&error_msg));
                    }
                    return Ok(CallToolResult::error(vec![Content::text(error_msg)]));
                }
            }

            // Execute the tool
            let tool_context = ToolCallContext::new(self, request, context);
            let result = self.tool_router.call(tool_context).await;
            let duration_ms = start_time.elapsed().as_millis() as u64;

            // Log the request
            // Note: Write and Execute operations are ALWAYS logged (for MCP trigger detection),
            // regardless of log_requests setting. This enables DatabaseWatcher to distinguish
            // MCP-triggered operations from manual UI operations for desktop notifications.
            let tool_category = get_tool_category(&tool_name);
            let should_log = config.log_requests
                || tool_category == ToolCategory::Write
                || tool_category == ToolCategory::Execute;

            if should_log {
                // Check if this is a background process - they log themselves for proper lifecycle tracking
                let is_background = match &result {
                    Ok(call_result) => {
                        call_result.content.iter().any(|c| {
                            if let Some(text_content) = c.raw.as_text() {
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text_content.text) {
                                    json.get("background").and_then(|v| v.as_bool()).unwrap_or(false)
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        })
                    }
                    Err(_) => false,
                };

                // Skip logging for background processes - they manage their own logging lifecycle
                if !is_background {
                    match &result {
                        Ok(call_result) => {
                            let result_status = if call_result.is_error.unwrap_or(false) {
                                "error"
                            } else {
                                "success"
                            };
                            log_request(&tool_name, &arguments, result_status, duration_ms, None);
                        }
                        Err(e) => {
                            log_request(&tool_name, &arguments, "error", duration_ms, Some(&e.to_string()));
                        }
                    }
                }
            }

            result
        }
    }
}

/// Print help information about available MCP tools
fn print_help() {
    let version = env!("CARGO_PKG_VERSION");
    println!(r#"SpecForge MCP Server v{}

USAGE:
    specforge-mcp [OPTIONS]

OPTIONS:
    --help, -h      Print this help information
    --version, -v   Print version information
    --list-tools    List all available MCP tools

DESCRIPTION:
    SpecForge MCP Server provides AI assistants (Claude Code, Cursor, etc.)
    with tools to manage Git projects, worktrees, workflows, and automation.

MCP TOOLS:

  📁 PROJECT MANAGEMENT
    list_projects       List all registered projects with detailed info
    get_project         Get project details (scripts, workflows, git info)
    get_project_dependencies Get dependencies from package.json

  🌳 GIT WORKTREE
    list_worktrees      List all git worktrees for a project
    get_worktree_status Get git status (branch, staged, modified, untracked)
    get_git_diff        Get staged changes diff for commit messages

  ⚡ WORKFLOWS
    list_workflows      List all workflows, filter by project
    get_workflow        Get detailed workflow info with all steps
    create_workflow     Create a new workflow
    add_workflow_step   Add a script step to a workflow
    update_workflow     Update workflow name/description
    delete_workflow_step Remove a step from a workflow
    run_workflow        Execute a workflow synchronously
    get_workflow_execution_details Get execution logs

  📝 TEMPLATES
    list_step_templates List available step templates
    create_step_template Create a reusable step template

  🔧 NPM/PACKAGE SCRIPTS
    run_npm_script      Run npm/yarn/pnpm scripts (volta/corepack support)
                        Supports background mode with runInBackground parameter

  🔄 BACKGROUND PROCESSES
    get_background_process_output  Get output from a background process
    stop_background_process        Stop/terminate a background process
    list_background_processes      List all background processes

  🎯 MCP ACTIONS
    list_actions        List all available MCP actions
    get_action          Get action details by ID
    run_script          Execute a predefined script action
    trigger_webhook     Trigger a configured webhook action
    get_execution_status Get action execution status
    list_action_executions List recent action executions
    get_action_permissions Get permission configuration

  🤖 AI ASSISTANT
    list_ai_providers   List configured AI providers
    list_conversations  List past AI conversations

  🔔 NOTIFICATIONS
    get_notifications   Get recent notifications
    mark_notifications_read Mark notifications as read

  🔒 SECURITY
    get_security_scan_results Get vulnerability scan results
    run_security_scan   Run npm/yarn/pnpm audit

  🚀 DEPLOYMENTS
    list_deployments    List deployment history

  📂 FILE OPERATIONS
    check_file_exists   Check if files exist in project
    search_project_files Search files by pattern
    read_project_file   Read file content (security-limited)

  🛠️ SYSTEM
    get_environment_info Get system tool versions and paths

PERMISSION MODES:
    read_only           Only read operations allowed (default)
    read_write          Read and write operations allowed
    full_access         All operations including execute allowed

CONFIGURATION:
    Configure in SpecForge: Settings → MCP Server

EXAMPLES:
    # Start the MCP server (for AI integration)
    specforge-mcp

    # Get help
    specforge-mcp --help

    # List available tools
    specforge-mcp --list-tools
"#, version);
}

/// Print version information
fn print_version() {
    println!("specforge-mcp {}", env!("CARGO_PKG_VERSION"));
}

/// List all tools in a simple format
/// Uses centralized tool definitions from tools_registry
fn list_tools_simple() {
    use mcp::ALL_TOOLS;

    println!("SpecForge MCP Tools:\n");

    // Group tools by category for better readability
    let mut current_category = "";
    for tool in ALL_TOOLS.iter() {
        if tool.display_category != current_category {
            if !current_category.is_empty() {
                println!();
            }
            println!("  # {}", tool.display_category);
            current_category = tool.display_category;
        }
        println!("  {:<35} {}", tool.name, tool.description);
    }
    println!();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    for arg in &args[1..] {
        match arg.as_str() {
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--version" | "-v" => {
                print_version();
                return Ok(());
            }
            "--list-tools" => {
                list_tools_simple();
                return Ok(());
            }
            _ => {
                eprintln!("Unknown option: {}", arg);
                eprintln!("Use --help for usage information");
                std::process::exit(1);
            }
        }
    }

    // Initialize smart instance manager (only kills stale instances, allows multi-instance)
    let mut instance_manager = InstanceManager::new();
    match instance_manager.initialize().await {
        Ok(result) => {
            eprintln!("[MCP Server] Instance manager initialized");
            if result.stale_killed > 0 {
                eprintln!("[MCP Server] Cleaned up {} stale instances", result.stale_killed);
            }
            if result.orphaned_cleaned > 0 {
                eprintln!("[MCP Server] Cleaned up {} orphaned instances", result.orphaned_cleaned);
            }
            if result.active_count > 0 {
                eprintln!("[MCP Server] {} other active instances running", result.active_count);
            }
        }
        Err(e) => {
            eprintln!("[MCP Server] Warning: Instance manager init failed: {}", e);
            // Continue anyway - this is non-fatal
        }
    }

    // Debug: Log startup info
    let current_pid = std::process::id();
    eprintln!("[MCP Server] Starting SpecForge MCP Server (PID: {})...", current_pid);

    // Debug: Check database at startup
    match read_store_data() {
        Ok(data) => {
            eprintln!("[MCP Server] Database read successful");
            eprintln!("[MCP Server] Config - is_enabled: {}", data.mcp_config.is_enabled);
            eprintln!("[MCP Server] Config - permission_mode: {:?}", data.mcp_config.permission_mode);
        }
        Err(e) => eprintln!("[MCP Server] Database read failed: {}", e),
    }

    // Create the MCP server
    let server = SpecForgeMcp::new();

    // Run with stdio transport (for Claude Code integration)
    let transport = (stdin(), stdout());

    // Start the server using serve_server
    let service = rmcp::serve_server(server, transport).await?;

    // Spawn background process cleanup task
    tokio::spawn(async {
        let mut interval = tokio::time::interval(Duration::from_secs(CLEANUP_INTERVAL_SECS));
        loop {
            interval.tick().await;
            BACKGROUND_PROCESS_MANAGER.cleanup().await;
        }
    });

    // Set up signal handlers for graceful shutdown (Unix only)
    #[cfg(unix)]
    {
        let mut sigterm = signal(SignalKind::terminate())?;
        let mut sigint = signal(SignalKind::interrupt())?;
        let mut sighup = signal(SignalKind::hangup())?;

        // Wait for either service completion or signal
        tokio::select! {
            result = service.waiting() => {
                match result {
                    Ok(_) => eprintln!("[MCP Server] Service ended normally"),
                    Err(e) => eprintln!("[MCP Server] Service ended with error: {:?}", e),
                }
            }
            _ = sigterm.recv() => {
                eprintln!("[MCP Server] Received SIGTERM, shutting down gracefully...");
            }
            _ = sigint.recv() => {
                eprintln!("[MCP Server] Received SIGINT, shutting down gracefully...");
            }
            _ = sighup.recv() => {
                eprintln!("[MCP Server] Received SIGHUP (parent process died), shutting down...");
            }
        }
    }

    // Non-Unix platforms: just wait for service
    #[cfg(not(unix))]
    {
        service.waiting().await?;
    }

    // Stop all background processes before shutdown
    BACKGROUND_PROCESS_MANAGER.shutdown().await;

    // Cleanup instance manager (release lock, remove heartbeat files)
    instance_manager.shutdown().await;

    eprintln!("[MCP Server] Shutdown complete");
    Ok(())
}
