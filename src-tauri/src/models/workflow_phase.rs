// Workflow Phase data models
// Represents the spec-driven development workflow state machine:
// phases, transitions, gates, and workflow instances.

use serde::{Deserialize, Serialize};

/// A complete workflow definition loaded from YAML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub autopilot: bool,
    pub phases: Vec<Phase>,
    pub transitions: Vec<Transition>,
}

/// A single phase in the workflow (e.g. "discuss", "specify", "review").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    #[serde(default)]
    pub on_enter: Vec<Action>,
}

/// A directed edge between two phases, optionally gated by a condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub from: String,
    pub to: String,
    pub gate: Option<Gate>,
}

/// A boolean gate condition evaluated via `evalexpr`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate {
    pub condition: String,
    pub message: Option<String>,
    #[serde(default)]
    pub auto_advance: bool,
}

/// An action executed when entering a phase (e.g. "run-agent", "notify").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub action: String,
    pub config: Option<serde_yaml::Value>,
}

/// Runtime state: a workflow instance bound to a specific spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInstance {
    pub id: String,
    pub spec_id: String,
    pub workflow_id: String,
    pub current_phase: String,
    pub started_at: String,
    pub updated_at: String,
}

/// Result returned after a successful phase transition.
#[derive(Debug, Clone, Serialize)]
pub struct TransitionResult {
    pub from_phase: String,
    pub to_phase: String,
    pub gate_passed: bool,
    pub actions_executed: Vec<String>,
}

/// Returned when a gate condition blocks a transition.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GateFailure {
    pub condition: String,
    pub message: Option<String>,
}
