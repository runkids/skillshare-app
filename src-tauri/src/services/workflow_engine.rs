// Workflow Engine
// State machine logic: load workflow definitions, evaluate transitions and gates,
// determine auto-advance targets.

use crate::local_models::workflow_phase::{
    GateFailure, Transition, WorkflowDefinition,
};

/// Pure-logic engine — no I/O, no database, no side effects.
pub struct WorkflowEngine;

impl WorkflowEngine {
    /// Parse a workflow YAML string into a `WorkflowDefinition`.
    pub fn load_workflow(yaml_content: &str) -> Result<WorkflowDefinition, String> {
        serde_yaml::from_str(yaml_content)
            .map_err(|e| format!("Failed to parse workflow YAML: {e}"))
    }

    /// All transitions whose `from` matches `current_phase`.
    pub fn get_available_transitions<'a>(
        workflow: &'a WorkflowDefinition,
        current_phase: &str,
    ) -> Vec<&'a Transition> {
        workflow
            .transitions
            .iter()
            .filter(|t| t.from == current_phase)
            .collect()
    }

    /// Check whether moving from `current_phase` to `to_phase` is allowed.
    ///
    /// Returns `Ok(true)` when the transition exists and any gate passes,
    /// `Ok(false)` when there is no such transition,
    /// `Err(GateFailure)` when the transition exists but the gate blocks it.
    pub fn can_transition(
        workflow: &WorkflowDefinition,
        current_phase: &str,
        to_phase: &str,
        gate_context: &evalexpr::HashMapContext,
    ) -> Result<bool, GateFailure> {
        let transition = workflow
            .transitions
            .iter()
            .find(|t| t.from == current_phase && t.to == to_phase);

        let transition = match transition {
            Some(t) => t,
            None => return Ok(false), // no such edge
        };

        match &transition.gate {
            None => Ok(true), // un-gated transition always passes
            Some(gate) => {
                let result = evalexpr::eval_boolean_with_context(&gate.condition, gate_context);
                match result {
                    Ok(true) => Ok(true),
                    Ok(false) => Err(GateFailure {
                        condition: gate.condition.clone(),
                        message: gate.message.clone(),
                    }),
                    Err(e) => Err(GateFailure {
                        condition: gate.condition.clone(),
                        message: Some(format!("Gate evaluation error: {e}")),
                    }),
                }
            }
        }
    }

    /// The first phase ID in the workflow (entry point).
    pub fn initial_phase(workflow: &WorkflowDefinition) -> Option<&str> {
        workflow.phases.first().map(|p| p.id.as_str())
    }

    /// Scan all outgoing transitions from `current_phase` that have
    /// `auto_advance = true`. Return the first `to_phase` whose gate passes.
    pub fn check_auto_advance(
        workflow: &WorkflowDefinition,
        current_phase: &str,
        gate_context: &evalexpr::HashMapContext,
    ) -> Option<String> {
        for transition in &workflow.transitions {
            if transition.from != current_phase {
                continue;
            }
            if let Some(gate) = &transition.gate {
                if !gate.auto_advance {
                    continue;
                }
                if let Ok(true) =
                    evalexpr::eval_boolean_with_context(&gate.condition, gate_context)
                {
                    return Some(transition.to.clone());
                }
            }
        }
        None
    }

    /// Return the hard-coded "Basic SDD" workflow definition.
    pub fn get_default_workflow() -> WorkflowDefinition {
        // We build it programmatically so it is always valid.
        let yaml = default_workflow_yaml();
        // Safe: this is a compile-time constant, tested below.
        serde_yaml::from_str(&yaml)
            .unwrap_or_else(|e| panic!("Built-in workflow YAML is invalid: {e}"))
    }
}

/// The canonical YAML representation of the default workflow.
/// Also written to `.specforge/workflows/default.workflow.yaml` on init.
pub fn default_workflow_yaml() -> String {
    r##"name: "Basic SDD"
description: "Spec-Driven Development workflow"
autopilot: false
phases:
  - id: discuss
    name: "Discuss"
    color: "#6366f1"
  - id: specify
    name: "Specify"
    color: "#8b5cf6"
  - id: review
    name: "Review"
    color: "#f59e0b"
  - id: implement
    name: "Implement"
    color: "#10b981"
  - id: verify
    name: "Verify"
    color: "#06b6d4"
  - id: archive
    name: "Archive"
    color: "#64748b"
transitions:
  - from: discuss
    to: specify
  - from: specify
    to: review
    gate:
      condition: "spec_section_summary == true"
      message: "Summary section required"
      auto_advance: true
  - from: review
    to: implement
    gate:
      condition: "reviews_approved == true"
      message: "Spec must be approved"
      auto_advance: true
  - from: implement
    to: verify
    gate:
      condition: "git_has_commits == true"
      message: "At least one commit required"
      auto_advance: true
  - from: verify
    to: archive
    gate:
      condition: "verify_passed == true"
      message: "Verification must pass"
      auto_advance: true
"##
    .to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use evalexpr::{ContextWithMutableVariables, HashMapContext, Value};

    fn load_default() -> WorkflowDefinition {
        WorkflowEngine::load_workflow(&default_workflow_yaml())
            .expect("default workflow YAML should parse")
    }

    fn ctx_with(vars: &[(&str, Value)]) -> HashMapContext {
        let mut ctx = HashMapContext::new();
        for (k, v) in vars {
            ctx.set_value(k.to_string(), v.clone())
                .expect("set_value should succeed");
        }
        ctx
    }

    #[test]
    fn test_load_workflow_yaml() {
        let wf = load_default();
        assert_eq!(wf.name, "Basic SDD");
        assert_eq!(wf.phases.len(), 6);
        assert_eq!(wf.transitions.len(), 5);
        assert!(!wf.autopilot);
    }

    #[test]
    fn test_get_available_transitions() {
        let wf = load_default();
        let from_discuss = WorkflowEngine::get_available_transitions(&wf, "discuss");
        assert_eq!(from_discuss.len(), 1);
        assert_eq!(from_discuss[0].to, "specify");

        let from_archive = WorkflowEngine::get_available_transitions(&wf, "archive");
        assert!(from_archive.is_empty());
    }

    #[test]
    fn test_can_transition_no_gate() {
        let wf = load_default();
        let ctx = HashMapContext::new();
        // discuss -> specify has no gate, should always pass
        let result = WorkflowEngine::can_transition(&wf, "discuss", "specify", &ctx);
        assert_eq!(result, Ok(true));
    }

    #[test]
    fn test_can_transition_gate_passes() {
        let wf = load_default();
        let ctx = ctx_with(&[("spec_section_summary", Value::Boolean(true))]);
        let result = WorkflowEngine::can_transition(&wf, "specify", "review", &ctx);
        assert_eq!(result, Ok(true));
    }

    #[test]
    fn test_can_transition_gate_fails() {
        let wf = load_default();
        let ctx = ctx_with(&[("spec_section_summary", Value::Boolean(false))]);
        let result = WorkflowEngine::can_transition(&wf, "specify", "review", &ctx);
        assert!(result.is_err());
        let failure = result.unwrap_err();
        assert_eq!(failure.condition, "spec_section_summary == true");
        assert_eq!(
            failure.message,
            Some("Summary section required".to_string())
        );
    }

    #[test]
    fn test_initial_phase() {
        let wf = load_default();
        assert_eq!(WorkflowEngine::initial_phase(&wf), Some("discuss"));
    }

    #[test]
    fn test_check_auto_advance() {
        let wf = load_default();

        // specify -> review gate passes and auto_advance=true
        let ctx = ctx_with(&[("spec_section_summary", Value::Boolean(true))]);
        let target = WorkflowEngine::check_auto_advance(&wf, "specify", &ctx);
        assert_eq!(target, Some("review".to_string()));

        // gate fails -> no auto-advance
        let ctx = ctx_with(&[("spec_section_summary", Value::Boolean(false))]);
        let target = WorkflowEngine::check_auto_advance(&wf, "specify", &ctx);
        assert_eq!(target, None);
    }

    #[test]
    fn test_invalid_transition() {
        let wf = load_default();
        let ctx = HashMapContext::new();
        // discuss -> implement has no edge
        let result = WorkflowEngine::can_transition(&wf, "discuss", "implement", &ctx);
        assert_eq!(result, Ok(false));
    }

    #[test]
    fn test_get_default_workflow() {
        let wf = WorkflowEngine::get_default_workflow();
        assert_eq!(wf.name, "Basic SDD");
        assert_eq!(wf.phases.len(), 6);
    }
}
