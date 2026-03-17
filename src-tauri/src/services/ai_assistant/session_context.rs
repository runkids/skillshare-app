// Session Context Builder
// Feature: AI Precision Improvement (025-ai-workflow-generator)
//
// Builds SessionContext from project path or ID, including:
// - Project details (id, name, path, type, package_manager)
// - Bound workflows (project-specific + global)
// - Available scripts from package.json

use crate::models::ai_assistant::{SessionContext, WorkflowSummary};
use crate::repositories::{ProjectRepository, WorkflowRepository};
use crate::utils::database::Database;

/// Builder for creating SessionContext from project information
pub struct SessionContextBuilder {
    db: Database,
}

impl SessionContextBuilder {
    /// Create a new SessionContextBuilder
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Build SessionContext from a project path
    /// Returns None if the project is not found
    pub fn build_from_path(&self, project_path: &str) -> Option<SessionContext> {
        let project_repo = ProjectRepository::new(self.db.clone());

        // Try to find project by path
        let project = project_repo.get_by_path(project_path).ok()??;

        self.build_context_for_project(&project.id, Some(project_path))
    }

    /// Build SessionContext from a project ID
    /// Returns None if the project is not found
    pub fn build_from_id(&self, project_id: &str) -> Option<SessionContext> {
        let project_repo = ProjectRepository::new(self.db.clone());

        // Verify project exists
        let project = project_repo.get(project_id).ok()??;

        self.build_context_for_project(project_id, Some(&project.path))
    }

    /// Build SessionContext from either project_id or project_path
    /// Prefers project_id if both are provided
    pub fn build(
        &self,
        project_id: Option<&str>,
        project_path: Option<&str>,
    ) -> Option<SessionContext> {
        // Try project_id first
        if let Some(id) = project_id {
            if let Some(ctx) = self.build_from_id(id) {
                return Some(ctx);
            }
        }

        // Fall back to project_path
        if let Some(path) = project_path {
            if let Some(ctx) = self.build_from_path(path) {
                return Some(ctx);
            }
        }

        // No project context available - return empty context
        None
    }

    /// Build the full context for a project
    fn build_context_for_project(
        &self,
        project_id: &str,
        project_path: Option<&str>,
    ) -> Option<SessionContext> {
        let project_repo = ProjectRepository::new(self.db.clone());
        let workflow_repo = WorkflowRepository::new(self.db.clone());

        // Get project details
        let project = project_repo.get(project_id).ok()??;

        // Get project-specific workflows
        let project_workflows = workflow_repo
            .list_by_project(project_id)
            .unwrap_or_default();

        // Get global workflows (project_id IS NULL)
        let global_workflows = self.get_global_workflows();

        // Combine and convert to WorkflowSummary
        let mut bound_workflows: Vec<WorkflowSummary> = project_workflows
            .iter()
            .map(|w| WorkflowSummary {
                id: w.id.clone(),
                name: w.name.clone(),
                step_count: w.nodes.len(),
            })
            .collect();

        // Add global workflows
        for wf in global_workflows {
            // Avoid duplicates (shouldn't happen, but be safe)
            if !bound_workflows.iter().any(|w| w.id == wf.id) {
                bound_workflows.push(wf);
            }
        }

        // Extract available scripts from project
        let available_scripts: Vec<String> = project.scripts.keys().cloned().collect();

        // Determine project type from framework or package manager
        let project_type = self.determine_project_type(&project);

        Some(SessionContext {
            project_id: Some(project.id.clone()),
            project_name: Some(project.name.clone()),
            project_path: project_path.map(|s| s.to_string()).or(Some(project.path.clone())),
            project_type: Some(project_type),
            package_manager: Some(format!("{:?}", project.package_manager).to_lowercase()),
            available_scripts,
            bound_workflows,
            active_worktree: None, // TODO: Could be enhanced to detect active worktree
        })
    }

    /// Get global workflows (those without a project_id)
    fn get_global_workflows(&self) -> Vec<WorkflowSummary> {
        let workflow_repo = WorkflowRepository::new(self.db.clone());

        workflow_repo
            .list()
            .unwrap_or_default()
            .into_iter()
            .filter(|w| w.project_id.is_none())
            .map(|w| WorkflowSummary {
                id: w.id.clone(),
                name: w.name.clone(),
                step_count: w.nodes.len(),
            })
            .collect()
    }

    /// Determine project type from framework or other indicators
    fn determine_project_type(&self, project: &crate::models::Project) -> String {
        // Check framework first
        if let Some(ref framework) = project.framework {
            if !framework.is_empty() {
                return framework.clone();
            }
        }

        // Check UI framework
        if let Some(ref ui_framework) = project.ui_framework {
            if !ui_framework.is_empty() {
                return ui_framework.clone();
            }
        }

        // Fall back to package manager
        // Note: SpecForge currently only supports Node.js package managers
        match project.package_manager {
            crate::models::PackageManager::Npm
            | crate::models::PackageManager::Yarn
            | crate::models::PackageManager::Pnpm
            | crate::models::PackageManager::Bun => "Node.js".to_string(),
            crate::models::PackageManager::Unknown => "Unknown".to_string(),
        }
    }
}

/// Build a simple SessionContext from just project path (used for backward compatibility)
/// This does filesystem detection without database lookup
pub fn build_simple_session_context(project_path: &str) -> SessionContext {
    use std::path::Path;

    let path = Path::new(project_path);
    let project_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    // Detect project type from filesystem
    let (project_type, package_manager, scripts) = detect_project_info(path);

    SessionContext {
        project_id: None, // No database ID available
        project_name: Some(project_name),
        project_path: Some(project_path.to_string()),
        project_type: Some(project_type),
        package_manager: Some(package_manager),
        available_scripts: scripts,
        bound_workflows: Vec::new(), // No database access, can't get workflows
        active_worktree: None,
    }
}

/// Detect project information from filesystem
fn detect_project_info(path: &std::path::Path) -> (String, String, Vec<String>) {
    // Check for package.json (Node.js project)
    let package_json_path = path.join("package.json");
    if package_json_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&package_json_path) {
            return parse_node_project(&content, path);
        }
    }

    // Check for Cargo.toml (Rust project)
    if path.join("Cargo.toml").exists() {
        return ("Rust".to_string(), "cargo".to_string(), vec![
            "build".to_string(),
            "test".to_string(),
            "run".to_string(),
            "check".to_string(),
        ]);
    }

    // Check for pyproject.toml (Python project)
    if path.join("pyproject.toml").exists() {
        return ("Python".to_string(), "poetry".to_string(), Vec::new());
    }

    // Check for requirements.txt (Python project)
    if path.join("requirements.txt").exists() {
        return ("Python".to_string(), "pip".to_string(), Vec::new());
    }

    // Check for go.mod (Go project)
    if path.join("go.mod").exists() {
        return ("Go".to_string(), "go".to_string(), vec![
            "build".to_string(),
            "test".to_string(),
            "run".to_string(),
        ]);
    }

    // Default - unknown project type
    ("Unknown".to_string(), "unknown".to_string(), Vec::new())
}

/// Parse Node.js project info from package.json
fn parse_node_project(content: &str, path: &std::path::Path) -> (String, String, Vec<String>) {
    let project_type = "Node.js".to_string();

    // Detect package manager from lockfiles
    let package_manager = if path.join("pnpm-lock.yaml").exists() {
        "pnpm".to_string()
    } else if path.join("yarn.lock").exists() {
        "yarn".to_string()
    } else if path.join("bun.lockb").exists() {
        "bun".to_string()
    } else {
        "npm".to_string()
    };

    // Extract scripts from package.json
    let scripts = extract_scripts(content);

    (project_type, package_manager, scripts)
}

/// Extract script names from package.json content
fn extract_scripts(content: &str) -> Vec<String> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
        if let Some(scripts) = json.get("scripts").and_then(|s| s.as_object()) {
            return scripts.keys().cloned().collect();
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_simple_session_context() {
        // Use a temp directory as project path
        let temp_dir = std::env::temp_dir();
        let ctx = build_simple_session_context(temp_dir.to_str().unwrap());

        assert!(ctx.project_path.is_some());
        assert!(ctx.project_name.is_some());
        // project_id should be None for simple context
        assert!(ctx.project_id.is_none());
    }

    #[test]
    fn test_detect_unknown_project() {
        let temp_dir = std::env::temp_dir();
        let (project_type, _, _) = detect_project_info(&temp_dir);
        // Temp dir likely won't have project files
        assert_eq!(project_type, "Unknown");
    }

    #[test]
    fn test_extract_scripts() {
        let content = r#"{
            "name": "test",
            "scripts": {
                "build": "tsc",
                "test": "jest",
                "dev": "vite"
            }
        }"#;

        let scripts = extract_scripts(content);
        assert_eq!(scripts.len(), 3);
        assert!(scripts.contains(&"build".to_string()));
        assert!(scripts.contains(&"test".to_string()));
        assert!(scripts.contains(&"dev".to_string()));
    }
}
