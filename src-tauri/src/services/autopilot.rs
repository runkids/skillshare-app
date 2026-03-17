// Autopilot Service
// Checks whether a spec should auto-advance to the next workflow phase
// after a file change. Includes rate-limiting and failure safeguards.

use crate::local_models::spec::Spec;
use crate::local_models::workflow_phase::WorkflowDefinition;
use crate::services::gate_evaluator::*;
use crate::services::workflow_engine::WorkflowEngine;
use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;
use tauri::{Emitter, Runtime};

/// Tracks per-spec rate limiting and failure counts for autopilot.
pub struct AutopilotState {
    /// Transition timestamps per spec: spec_id -> Vec<Instant>
    transition_history: Mutex<HashMap<String, Vec<Instant>>>,
    /// Consecutive agent failures per (spec_id, phase): key -> count
    agent_failures: Mutex<HashMap<String, u32>>,
    /// Specs with paused autopilot
    paused_specs: Mutex<HashMap<String, String>>,
}

/// Maximum transitions per spec in a rolling 60-minute window.
const MAX_TRANSITIONS_PER_HOUR: usize = 5;

/// Consecutive agent failures before pausing autopilot for a spec+phase.
const MAX_AGENT_FAILURES: u32 = 2;

impl Default for AutopilotState {
    fn default() -> Self {
        Self::new()
    }
}

impl AutopilotState {
    pub fn new() -> Self {
        Self {
            transition_history: Mutex::new(HashMap::new()),
            agent_failures: Mutex::new(HashMap::new()),
            paused_specs: Mutex::new(HashMap::new()),
        }
    }

    /// Record a transition for rate-limiting.
    fn record_transition(&self, spec_id: &str) {
        if let Ok(mut history) = self.transition_history.lock() {
            let timestamps = history.entry(spec_id.to_string()).or_default();
            timestamps.push(Instant::now());

            // Prune entries older than 60 minutes
            let cutoff = Instant::now() - std::time::Duration::from_secs(3600);
            timestamps.retain(|t| *t > cutoff);
        }
    }

    /// Check if a spec has exceeded the transition rate limit.
    fn is_rate_limited(&self, spec_id: &str) -> bool {
        if let Ok(history) = self.transition_history.lock() {
            if let Some(timestamps) = history.get(spec_id) {
                let cutoff = Instant::now() - std::time::Duration::from_secs(3600);
                let recent_count = timestamps.iter().filter(|t| **t > cutoff).count();
                return recent_count >= MAX_TRANSITIONS_PER_HOUR;
            }
        }
        false
    }

    /// Record an agent failure for a spec+phase.
    pub fn record_agent_failure(&self, spec_id: &str, phase: &str) {
        if let Ok(mut failures) = self.agent_failures.lock() {
            let key = format!("{spec_id}:{phase}");
            let count = failures.entry(key).or_insert(0);
            *count += 1;
        }
    }

    /// Reset agent failure count (e.g., after a successful run).
    pub fn reset_agent_failures(&self, spec_id: &str, phase: &str) {
        if let Ok(mut failures) = self.agent_failures.lock() {
            let key = format!("{spec_id}:{phase}");
            failures.remove(&key);
        }
    }

    /// Check if autopilot should be paused due to repeated agent failures.
    fn has_exceeded_failure_limit(&self, spec_id: &str, phase: &str) -> bool {
        if let Ok(failures) = self.agent_failures.lock() {
            let key = format!("{spec_id}:{phase}");
            if let Some(count) = failures.get(&key) {
                return *count >= MAX_AGENT_FAILURES;
            }
        }
        false
    }

    /// Check if autopilot is paused for a spec.
    pub fn is_paused(&self, spec_id: &str) -> bool {
        if let Ok(paused) = self.paused_specs.lock() {
            return paused.contains_key(spec_id);
        }
        false
    }

    /// Pause autopilot for a spec with a reason.
    fn pause(&self, spec_id: &str, reason: &str) {
        if let Ok(mut paused) = self.paused_specs.lock() {
            paused.insert(spec_id.to_string(), reason.to_string());
        }
    }

    /// Resume autopilot for a spec.
    pub fn resume(&self, spec_id: &str) {
        if let Ok(mut paused) = self.paused_specs.lock() {
            paused.remove(spec_id);
        }
    }
}

/// Check if a spec should auto-advance and emit an event if so.
///
/// Called by the file watcher after a spec file change is synced.
/// This only evaluates gates — it does NOT perform the transition itself.
/// The frontend or a follow-up handler should call `advance_spec` to execute.
pub fn check_autopilot<R: Runtime>(
    spec: &Spec,
    workflow: &WorkflowDefinition,
    conn: &Connection,
    project_dir: &std::path::Path,
    app_handle: &tauri::AppHandle<R>,
    state: &AutopilotState,
) {
    // Only proceed if the workflow has autopilot enabled
    if !workflow.autopilot {
        return;
    }

    let current_phase = match spec.workflow_phase.as_deref() {
        Some(p) => p,
        None => return,
    };

    // Check if autopilot is paused for this spec
    if state.is_paused(&spec.id) {
        log::debug!(
            "[Autopilot] Skipping paused spec: {}",
            spec.id
        );
        return;
    }

    // Check rate limit
    if state.is_rate_limited(&spec.id) {
        log::warn!(
            "[Autopilot] Rate limit exceeded for spec: {}. Pausing autopilot.",
            spec.id
        );
        state.pause(&spec.id, "rate_limit_exceeded");

        let _ = app_handle.emit(
            "specforge://autopilot-paused",
            serde_json::json!({
                "specId": spec.id,
                "reason": "rate_limit_exceeded",
                "message": format!(
                    "Autopilot paused: exceeded {} transitions per hour",
                    MAX_TRANSITIONS_PER_HOUR
                ),
            }),
        );
        return;
    }

    // Check agent failure limit
    if state.has_exceeded_failure_limit(&spec.id, current_phase) {
        log::warn!(
            "[Autopilot] Agent failure limit reached for spec={} phase={}. Pausing.",
            spec.id,
            current_phase
        );
        state.pause(&spec.id, "agent_failure_limit");

        let _ = app_handle.emit(
            "specforge://autopilot-paused",
            serde_json::json!({
                "specId": spec.id,
                "reason": "agent_failure_limit",
                "message": format!(
                    "Autopilot paused: {} consecutive agent failures in phase '{}'",
                    MAX_AGENT_FAILURES,
                    current_phase
                ),
            }),
        );
        return;
    }

    // Build gate context
    let reviews = build_review_summary(conn, &spec.id);
    let git_info = build_git_info(spec, project_dir);
    let workflow_info = build_workflow_info(conn, spec);

    let gate_context = GateEvaluator::build_context(spec, &reviews, &git_info, &workflow_info);

    // Check if any auto_advance gate passes
    if let Some(target_phase) =
        WorkflowEngine::check_auto_advance(workflow, current_phase, &gate_context)
    {
        log::info!(
            "[Autopilot] Auto-advance triggered: spec={} {} -> {}",
            spec.id,
            current_phase,
            target_phase
        );

        state.record_transition(&spec.id);

        // Emit event so the frontend (or a handler) can execute the transition
        let _ = app_handle.emit(
            "specforge://autopilot-advance",
            serde_json::json!({
                "specId": spec.id,
                "fromPhase": current_phase,
                "toPhase": target_phase,
            }),
        );
    }
}

// ---------------------------------------------------------------------------
// Gate context helpers (duplicated from workflow_commands for decoupling)
// These run in a sync context (file watcher callback).
// ---------------------------------------------------------------------------

fn build_review_summary(conn: &Connection, spec_id: &str) -> ReviewSummary {
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

    let verify_passed: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM spec_reviews WHERE spec_id = ?1 AND phase = 'verify' AND approved = 1",
            rusqlite::params![spec_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    ReviewSummary {
        count: count as usize,
        has_approval,
        verify_passed,
    }
}

fn build_git_info(spec: &Spec, project_dir: &std::path::Path) -> GitInfo {
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

fn build_workflow_info(conn: &Connection, spec: &Spec) -> WorkflowInfo {
    use crate::repositories::workflow_instance_repo;

    let current_phase = spec
        .workflow_phase
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    let (time_in_phase_hours, phase_count) =
        match workflow_instance_repo::get_instance_by_spec(conn, &spec.id) {
            Ok(Some(instance)) => {
                let updated = chrono::DateTime::parse_from_rfc3339(&instance.updated_at)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc));
                let hours = updated
                    .map(|dt| {
                        let elapsed = chrono::Utc::now() - dt;
                        elapsed.num_minutes() as f64 / 60.0
                    })
                    .unwrap_or(0.0);

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autopilot_state_defaults() {
        let state = AutopilotState::new();
        assert!(!state.is_paused("some-spec"));
        assert!(!state.is_rate_limited("some-spec"));
        assert!(!state.has_exceeded_failure_limit("some-spec", "implement"));
    }

    #[test]
    fn test_rate_limiting() {
        let state = AutopilotState::new();

        // Record MAX_TRANSITIONS_PER_HOUR transitions
        for _ in 0..MAX_TRANSITIONS_PER_HOUR {
            state.record_transition("spec-001");
        }

        assert!(state.is_rate_limited("spec-001"));
        assert!(!state.is_rate_limited("spec-002"));
    }

    #[test]
    fn test_agent_failure_tracking() {
        let state = AutopilotState::new();

        assert!(!state.has_exceeded_failure_limit("spec-001", "implement"));

        state.record_agent_failure("spec-001", "implement");
        assert!(!state.has_exceeded_failure_limit("spec-001", "implement"));

        state.record_agent_failure("spec-001", "implement");
        assert!(state.has_exceeded_failure_limit("spec-001", "implement"));

        // Different phase should not be affected
        assert!(!state.has_exceeded_failure_limit("spec-001", "review"));

        // Reset should clear
        state.reset_agent_failures("spec-001", "implement");
        assert!(!state.has_exceeded_failure_limit("spec-001", "implement"));
    }

    #[test]
    fn test_pause_and_resume() {
        let state = AutopilotState::new();

        assert!(!state.is_paused("spec-001"));

        state.pause("spec-001", "test_reason");
        assert!(state.is_paused("spec-001"));
        assert!(!state.is_paused("spec-002"));

        state.resume("spec-001");
        assert!(!state.is_paused("spec-001"));
    }
}
