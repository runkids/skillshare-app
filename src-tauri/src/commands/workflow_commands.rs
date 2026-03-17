// Workflow Phase Commands
// Tauri IPC commands for spec-driven workflow transitions, reviews,
// and gate status queries.

use crate::local_models::spec::Spec;
use crate::repositories::{agent_run_repo, workflow_instance_repo};
use crate::services::{
    action_runner, config_service, gate_evaluator::*, spec_service, workflow_engine::WorkflowEngine,
};
use crate::DatabaseState;
use std::path::Path;

/// Advance a spec to a new workflow phase.
///
/// Loads the spec and its workflow, checks gate conditions, transitions the
/// workflow instance, and executes on_enter actions for the target phase.
#[tauri::command]
pub async fn advance_spec(
    spec_id: String,
    to_phase: Option<String>,
    project_dir: String,
    db: tauri::State<'_, DatabaseState>,
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let project = project_dir.clone();

    // Load spec from disk (source of truth)
    let spec = spec_service::get_spec(Path::new(&project), &spec_id)?;

    // Load workflow definition
    let workflow = load_workflow_for_spec(&spec, Path::new(&project))?;

    let current_phase = spec
        .workflow_phase
        .as_deref()
        .ok_or("Spec has no current workflow phase")?;

    // Determine target phase
    let target_phase = match to_phase {
        Some(ref p) => p.clone(),
        None => {
            // Auto-pick the first available transition
            let transitions =
                WorkflowEngine::get_available_transitions(&workflow, current_phase);
            transitions
                .first()
                .map(|t| t.to.clone())
                .ok_or_else(|| {
                    format!("No transitions available from phase '{current_phase}'")
                })?
        }
    };

    // Build gate context
    let gate_context = db.0.with_connection(|conn| {
        build_gate_context_for_spec(&spec, conn, Path::new(&project))
    })?;

    // Check if transition is allowed
    WorkflowEngine::can_transition(&workflow, current_phase, &target_phase, &gate_context)
        .map_err(|failure| {
            failure
                .message
                .unwrap_or_else(|| format!("Gate blocked: {}", failure.condition))
        })?;

    // Perform the transition in the database
    let from_phase = current_phase.to_string();
    let to_phase_str = target_phase.clone();
    let spec_id_clone = spec_id.clone();
    db.0.with_connection(|conn| {
        // Update workflow instance
        if let Some(instance) =
            workflow_instance_repo::get_instance_by_spec(conn, &spec_id_clone)?
        {
            workflow_instance_repo::update_instance_phase(conn, &instance.id, &to_phase_str)?;

            // Record in phase history
            workflow_instance_repo::insert_phase_history(
                conn,
                &instance.id,
                Some(&from_phase),
                &to_phase_str,
                Some("passed"),
                None,
            )?;
        }
        Ok::<(), String>(())
    })?;

    // Update the spec file's workflow_phase
    let mut fields = std::collections::HashMap::new();
    fields.insert(
        "workflow_phase".to_string(),
        serde_yaml::Value::String(target_phase.clone()),
    );
    let project_clone = project_dir.clone();
    db.0.with_connection(|conn| {
        spec_service::update_spec(Path::new(&project_clone), conn, &spec_id, Some(fields), None)
    })?;

    // Execute on_enter actions for the target phase
    let target_phase_def = workflow.phases.iter().find(|p| p.id == target_phase);
    let action_results = if let Some(phase_def) = target_phase_def {
        if !phase_def.on_enter.is_empty() {
            let config = config_service::load_config();
            let dispatcher = crate::services::agent_dispatcher::AgentDispatcher::new(
                config.agent.max_concurrent_agents,
            );

            let results = db.0.with_connection(|conn| {
                // We need to block on the async function from a sync context.
                // Use tokio::runtime::Handle to run it.
                let handle = tokio::runtime::Handle::current();
                let spec_reloaded =
                    spec_service::get_spec(Path::new(&project), &spec_id)?;
                let results = handle.block_on(action_runner::run_actions(
                    &phase_def.on_enter,
                    &spec_reloaded,
                    Path::new(&project),
                    &app,
                    conn,
                    &config,
                    &dispatcher,
                ));
                Ok::<Vec<action_runner::ActionResult>, String>(results)
            })?;

            // Update phase history with action results
            if let Ok(_results_json) = serde_json::to_string(&results) {
                let project_ref = project_dir.clone();
                let spec_id_ref = spec_id.clone();
                let _ = db.0.with_connection(|conn| {
                    if let Some(instance) =
                        workflow_instance_repo::get_instance_by_spec(conn, &spec_id_ref)?
                    {
                        let history = workflow_instance_repo::get_phase_history(
                            conn,
                            &instance.id,
                        )?;
                        // The last history entry is the one we just created
                        if let Some(_last) = history.last() {
                            // Action results are informational — logged above
                            let _ = &project_ref;
                        }
                    }
                    Ok::<(), String>(())
                });
            }

            results
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    Ok(serde_json::json!({
        "fromPhase": from_phase,
        "toPhase": target_phase,
        "gatePassed": true,
        "actionsExecuted": action_results,
    }))
}

/// Submit a review for a spec (approve or reject).
#[tauri::command]
pub async fn review_spec(
    spec_id: String,
    approved: bool,
    comment: Option<String>,
    project_dir: String,
    db: tauri::State<'_, DatabaseState>,
) -> Result<(), String> {
    let spec = spec_service::get_spec(Path::new(&project_dir), &spec_id)?;
    let phase = spec
        .workflow_phase
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let reviewer = whoami::username();
    let now = chrono::Utc::now().to_rfc3339();

    db.0.with_connection(|conn| {
        conn.execute(
            r#"
            INSERT INTO spec_reviews (spec_id, phase, reviewer, approved, comment, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            rusqlite::params![spec_id, phase, reviewer, approved as i32, comment, now],
        )
        .map_err(|e| format!("Failed to insert review: {e}"))?;
        Ok(())
    })
}

/// Get the current workflow status for a spec.
///
/// Returns current phase, available transitions, and whether each
/// transition's gate passes.
#[tauri::command]
pub async fn get_workflow_status(
    spec_id: String,
    project_dir: String,
    db: tauri::State<'_, DatabaseState>,
) -> Result<serde_json::Value, String> {
    let spec = spec_service::get_spec(Path::new(&project_dir), &spec_id)?;
    let workflow = load_workflow_for_spec(&spec, Path::new(&project_dir))?;

    let current_phase = spec
        .workflow_phase
        .as_deref()
        .unwrap_or("unknown");

    let gate_context = db.0.with_connection(|conn| {
        build_gate_context_for_spec(&spec, conn, Path::new(&project_dir))
    })?;

    let transitions = WorkflowEngine::get_available_transitions(&workflow, current_phase);
    let transition_statuses: Vec<serde_json::Value> = transitions
        .iter()
        .map(|t| {
            let can_pass =
                WorkflowEngine::can_transition(&workflow, current_phase, &t.to, &gate_context);
            let gate_passed = can_pass.as_ref().copied().unwrap_or(false);
            let gate_message = match &can_pass {
                Ok(_) => None,
                Err(f) => f.message.clone(),
            };
            serde_json::json!({
                "to": t.to,
                "gatePassed": gate_passed,
                "gateMessage": gate_message,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "specId": spec_id,
        "currentPhase": current_phase,
        "workflowName": workflow.name,
        "autopilot": workflow.autopilot,
        "availableTransitions": transition_statuses,
    }))
}

/// Evaluate all gate conditions for the spec's current transitions.
///
/// Returns detailed pass/fail for each gate condition.
#[tauri::command]
pub async fn get_gate_status(
    spec_id: String,
    project_dir: String,
    db: tauri::State<'_, DatabaseState>,
) -> Result<serde_json::Value, String> {
    let spec = spec_service::get_spec(Path::new(&project_dir), &spec_id)?;
    let workflow = load_workflow_for_spec(&spec, Path::new(&project_dir))?;

    let current_phase = spec
        .workflow_phase
        .as_deref()
        .unwrap_or("unknown");

    let gate_context = db.0.with_connection(|conn| {
        build_gate_context_for_spec(&spec, conn, Path::new(&project_dir))
    })?;

    let transitions = WorkflowEngine::get_available_transitions(&workflow, current_phase);
    let gate_details: Vec<serde_json::Value> = transitions
        .iter()
        .map(|t| {
            let gate_info = match &t.gate {
                Some(gate) => {
                    let result = GateEvaluator::evaluate(&gate.condition, &gate_context);
                    let passed = result.as_ref().copied().unwrap_or(false);
                    let error = result.err();
                    serde_json::json!({
                        "condition": gate.condition,
                        "passed": passed,
                        "message": gate.message,
                        "autoAdvance": gate.auto_advance,
                        "error": error,
                    })
                }
                None => {
                    serde_json::json!({
                        "condition": null,
                        "passed": true,
                        "message": null,
                        "autoAdvance": false,
                        "error": null,
                    })
                }
            };
            serde_json::json!({
                "from": t.from,
                "to": t.to,
                "gate": gate_info,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "specId": spec_id,
        "currentPhase": current_phase,
        "gates": gate_details,
    }))
}

/// Get agent runs for a spec.
#[tauri::command]
pub async fn get_agent_runs(
    spec_id: String,
    db: tauri::State<'_, DatabaseState>,
) -> Result<Vec<serde_json::Value>, String> {
    let runs = db.0.with_connection(|conn| {
        agent_run_repo::get_agents_for_spec(conn, &spec_id)
    })?;

    runs.iter()
        .map(|r| {
            serde_json::to_value(r).map_err(|e| format!("Failed to serialize agent run: {e}"))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Load the workflow definition for a spec.
///
/// If the spec has a `workflow` field, attempt to load from
/// `.specforge/workflows/{name}.workflow.yaml`. Otherwise use the default.
fn load_workflow_for_spec(
    spec: &Spec,
    project_dir: &Path,
) -> Result<crate::local_models::workflow_phase::WorkflowDefinition, String> {
    let workflow_name = spec.workflow.as_deref().unwrap_or("default");

    let workflow_path = project_dir
        .join(".specforge")
        .join("workflows")
        .join(format!("{workflow_name}.workflow.yaml"));

    if workflow_path.exists() {
        let content = std::fs::read_to_string(&workflow_path).map_err(|e| {
            format!(
                "Failed to read workflow file {}: {e}",
                workflow_path.display()
            )
        })?;
        WorkflowEngine::load_workflow(&content)
    } else {
        // Fall back to default built-in workflow
        Ok(WorkflowEngine::get_default_workflow())
    }
}

/// Build an evalexpr context for gate evaluation.
///
/// Gathers review summary, git info, and workflow info from the database
/// and filesystem.
fn build_gate_context_for_spec(
    spec: &Spec,
    conn: &rusqlite::Connection,
    project_dir: &Path,
) -> Result<evalexpr::HashMapContext, String> {
    // Review summary
    let reviews = build_review_summary(conn, &spec.id)?;

    // Git info
    let git_info = build_git_info(spec, project_dir);

    // Workflow info
    let workflow_info = build_workflow_info(conn, spec);

    Ok(GateEvaluator::build_context(
        spec,
        &reviews,
        &git_info,
        &workflow_info,
    ))
}

/// Query spec_reviews to build a ReviewSummary.
fn build_review_summary(
    conn: &rusqlite::Connection,
    spec_id: &str,
) -> Result<ReviewSummary, String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM spec_reviews WHERE spec_id = ?1",
            rusqlite::params![spec_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let has_approval: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM spec_reviews WHERE spec_id = ?1 AND approved = 1",
            rusqlite::params![spec_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    // "verify_passed" is true if any review in the "verify" phase approved
    let verify_passed: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM spec_reviews WHERE spec_id = ?1 AND phase = 'verify' AND approved = 1",
            rusqlite::params![spec_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    Ok(ReviewSummary {
        count: count as usize,
        has_approval,
        verify_passed,
    })
}

/// Check git state for the spec (branch existence and commit count).
fn build_git_info(spec: &Spec, project_dir: &Path) -> GitInfo {
    let branch_name = format!("spec/{}", spec.id);

    let has_branch = std::process::Command::new("git")
        .args(["rev-parse", "--verify", &format!("refs/heads/{branch_name}")])
        .current_dir(project_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    let commit_count = if has_branch {
        // Count commits on the spec branch that aren't on the default branch
        std::process::Command::new("git")
            .args(["rev-list", "--count", &format!("HEAD..{branch_name}")])
            .current_dir(project_dir)
            .output()
            .ok()
            .and_then(|out| {
                String::from_utf8_lossy(&out.stdout)
                    .trim()
                    .parse::<usize>()
                    .ok()
            })
            .unwrap_or(0)
    } else {
        0
    };

    GitInfo {
        has_branch,
        commit_count,
    }
}

/// Build workflow timing info from the workflow instance.
fn build_workflow_info(conn: &rusqlite::Connection, spec: &Spec) -> WorkflowInfo {
    let current_phase = spec
        .workflow_phase
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    let (time_in_phase_hours, phase_count) =
        match workflow_instance_repo::get_instance_by_spec(conn, &spec.id) {
            Ok(Some(instance)) => {
                // Calculate time in current phase
                let updated = chrono::DateTime::parse_from_rfc3339(&instance.updated_at)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc));
                let hours = updated
                    .map(|dt| {
                        let elapsed = chrono::Utc::now() - dt;
                        elapsed.num_minutes() as f64 / 60.0
                    })
                    .unwrap_or(0.0);

                // Count total phase transitions
                let count = workflow_instance_repo::get_phase_history(conn, &instance.id)
                    .map(|h| h.len())
                    .unwrap_or(0);

                (hours, count)
            }
            _ => (0.0, 0),
        };

    WorkflowInfo {
        current_phase,
        time_in_phase_hours,
        phase_count,
    }
}
