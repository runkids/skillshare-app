// Action Runner
// Executes on_enter actions when a spec transitions to a new workflow phase.
// Each action is dispatched by matching on its `action` string.

use crate::local_models::config::SpecForgeConfig;
use crate::local_models::spec::Spec;
use crate::local_models::workflow_phase::Action;
use crate::services::agent_dispatcher::AgentDispatcher;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::{Emitter, Runtime};

/// Result of executing a single action.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionResult {
    pub action: String,
    pub success: bool,
    pub message: Option<String>,
}

/// Execute a list of on_enter actions for a phase transition.
///
/// Each action is run sequentially. Failures are recorded but do not
/// prevent subsequent actions from executing.
pub async fn run_actions<R: Runtime>(
    actions: &[Action],
    spec: &Spec,
    project_dir: &Path,
    app_handle: &tauri::AppHandle<R>,
    conn: &Connection,
    config: &SpecForgeConfig,
    agent_dispatcher: &AgentDispatcher,
) -> Vec<ActionResult> {
    let mut results = Vec::new();

    for action in actions {
        let result = match action.action.as_str() {
            "git_create_branch" => action_git_create_branch(spec, &action.config, project_dir).await,
            "move_to_archive" => action_move_to_archive(spec, project_dir),
            "run_command" => action_run_command(&action.config, project_dir).await,
            "notify" => action_notify(spec, &action.config, app_handle),
            "run_agent" => {
                action_run_agent(
                    spec,
                    &action.config,
                    project_dir,
                    conn,
                    config,
                    agent_dispatcher,
                )
                .await
            }
            other => ActionResult {
                action: other.to_string(),
                success: false,
                message: Some(format!("Unknown action: {other}")),
            },
        };

        log::info!(
            "[ActionRunner] action={} success={} message={:?}",
            result.action,
            result.success,
            result.message
        );

        results.push(result);
    }

    results
}

/// Create a git branch for the spec (e.g. `spec/{spec.id}`).
async fn action_git_create_branch(
    spec: &Spec,
    config: &Option<serde_yaml::Value>,
    project_dir: &Path,
) -> ActionResult {
    let pattern = config
        .as_ref()
        .and_then(|c| c.get("pattern"))
        .and_then(|v| v.as_str())
        .unwrap_or("spec/{{spec.id}}");

    let branch_name = pattern.replace("{{spec.id}}", &spec.id);

    let output = tokio::process::Command::new("git")
        .args(["checkout", "-b", &branch_name])
        .current_dir(project_dir)
        .output()
        .await;

    match output {
        Ok(out) if out.status.success() => ActionResult {
            action: "git_create_branch".to_string(),
            success: true,
            message: Some(format!("Created branch: {branch_name}")),
        },
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            ActionResult {
                action: "git_create_branch".to_string(),
                success: false,
                message: Some(format!("git checkout -b failed: {stderr}")),
            }
        }
        Err(e) => ActionResult {
            action: "git_create_branch".to_string(),
            success: false,
            message: Some(format!("Failed to run git: {e}")),
        },
    }
}

/// Move spec file from `.specforge/specs/{id}.md` to `.specforge/archive/{id}.md`.
fn action_move_to_archive(spec: &Spec, project_dir: &Path) -> ActionResult {
    let specs_dir = project_dir.join(".specforge").join("specs");
    let archive_dir = project_dir.join(".specforge").join("archive");

    let src = specs_dir.join(format!("{}.md", spec.id));
    let dst = archive_dir.join(format!("{}.md", spec.id));

    // Create archive/ directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&archive_dir) {
        return ActionResult {
            action: "move_to_archive".to_string(),
            success: false,
            message: Some(format!("Failed to create archive dir: {e}")),
        };
    }

    if !src.exists() {
        return ActionResult {
            action: "move_to_archive".to_string(),
            success: false,
            message: Some(format!("Source file not found: {}", src.display())),
        };
    }

    match std::fs::rename(&src, &dst) {
        Ok(()) => ActionResult {
            action: "move_to_archive".to_string(),
            success: true,
            message: Some(format!("Moved to archive: {}", dst.display())),
        },
        Err(e) => ActionResult {
            action: "move_to_archive".to_string(),
            success: false,
            message: Some(format!("Failed to move file: {e}")),
        },
    }
}

/// Execute a shell command from action config in the project directory.
async fn action_run_command(
    config: &Option<serde_yaml::Value>,
    project_dir: &Path,
) -> ActionResult {
    let command = config
        .as_ref()
        .and_then(|c| c.get("command"))
        .and_then(|v| v.as_str());

    let command = match command {
        Some(cmd) => cmd,
        None => {
            return ActionResult {
                action: "run_command".to_string(),
                success: false,
                message: Some("Missing 'command' in action config".to_string()),
            };
        }
    };

    let output = tokio::process::Command::new("sh")
        .args(["-c", command])
        .current_dir(project_dir)
        .output()
        .await;

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            if out.status.success() {
                ActionResult {
                    action: "run_command".to_string(),
                    success: true,
                    message: Some(stdout.trim().to_string()),
                }
            } else {
                ActionResult {
                    action: "run_command".to_string(),
                    success: false,
                    message: Some(format!("Command failed: {stderr}")),
                }
            }
        }
        Err(e) => ActionResult {
            action: "run_command".to_string(),
            success: false,
            message: Some(format!("Failed to execute command: {e}")),
        },
    }
}

/// Send a notification via Tauri event emission.
fn action_notify<R: Runtime>(
    spec: &Spec,
    config: &Option<serde_yaml::Value>,
    app_handle: &tauri::AppHandle<R>,
) -> ActionResult {
    let default_message = format!("Spec '{}' transitioned to a new phase", spec.title);
    let message = config
        .as_ref()
        .and_then(|c| c.get("message"))
        .and_then(|v| v.as_str())
        .map(|m| m.replace("{{spec.title}}", &spec.title).replace("{{spec.id}}", &spec.id))
        .unwrap_or(default_message);

    // Emit event to frontend
    let payload = serde_json::json!({
        "specId": spec.id,
        "message": message,
    });

    match app_handle.emit("specforge://action-notify", &payload) {
        Ok(()) => ActionResult {
            action: "notify".to_string(),
            success: true,
            message: Some(message),
        },
        Err(e) => ActionResult {
            action: "notify".to_string(),
            success: false,
            message: Some(format!("Failed to emit notification: {e}")),
        },
    }
}

/// Delegate to AgentDispatcher to spawn an AI agent subprocess.
async fn action_run_agent(
    spec: &Spec,
    config: &Option<serde_yaml::Value>,
    project_dir: &Path,
    conn: &Connection,
    app_config: &SpecForgeConfig,
    dispatcher: &AgentDispatcher,
) -> ActionResult {
    let prompt_template = config
        .as_ref()
        .and_then(|c| c.get("prompt"))
        .and_then(|v| v.as_str())
        .unwrap_or("Implement the changes described in spec {{spec.id}}: {{spec.title}}");

    let timeout_ms = config
        .as_ref()
        .and_then(|c| c.get("timeout_ms"))
        .and_then(|v| v.as_u64())
        .unwrap_or(300_000); // 5 min default

    match dispatcher
        .dispatch(spec, prompt_template, timeout_ms, project_dir, conn, app_config)
        .await
    {
        Ok(run_id) => ActionResult {
            action: "run_agent".to_string(),
            success: true,
            message: Some(format!("Agent dispatched: {run_id}")),
        },
        Err(e) => ActionResult {
            action: "run_agent".to_string(),
            success: false,
            message: Some(e),
        },
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_result_serialization() {
        let result = ActionResult {
            action: "git_create_branch".to_string(),
            success: true,
            message: Some("Created branch: spec/test-001".to_string()),
        };

        let json = serde_json::to_value(&result).expect("serialize");
        assert_eq!(json["action"], "git_create_branch");
        assert_eq!(json["success"], true);
        assert_eq!(json["message"], "Created branch: spec/test-001");
    }

    #[test]
    fn test_move_to_archive_missing_source() {
        let tmp = tempfile::tempdir().expect("create temp dir");
        let spec = Spec {
            id: "spec-nonexistent".to_string(),
            schema: "test".to_string(),
            title: "Test".to_string(),
            status: "draft".to_string(),
            workflow: None,
            workflow_phase: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            fields: std::collections::HashMap::new(),
            body: String::new(),
            file_path: None,
        };

        let result = action_move_to_archive(&spec, tmp.path());
        assert!(!result.success);
        assert!(result
            .message
            .as_ref()
            .unwrap_or(&String::new())
            .contains("not found"));
    }

    #[test]
    fn test_move_to_archive_success() {
        let tmp = tempfile::tempdir().expect("create temp dir");
        let specs_dir = tmp.path().join(".specforge").join("specs");
        std::fs::create_dir_all(&specs_dir).expect("create specs dir");

        let spec_id = "spec-archive-test";
        let src = specs_dir.join(format!("{spec_id}.md"));
        std::fs::write(&src, "---\nid: test\n---\n").expect("write spec file");

        let spec = Spec {
            id: spec_id.to_string(),
            schema: "test".to_string(),
            title: "Test".to_string(),
            status: "draft".to_string(),
            workflow: None,
            workflow_phase: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            fields: std::collections::HashMap::new(),
            body: String::new(),
            file_path: None,
        };

        let result = action_move_to_archive(&spec, tmp.path());
        assert!(result.success);

        // Verify file moved
        assert!(!src.exists());
        let dst = tmp
            .path()
            .join(".specforge")
            .join("archive")
            .join(format!("{spec_id}.md"));
        assert!(dst.exists());
    }
}
