// Gate Evaluator
// Builds a flat variable map from spec state and evaluates gate expressions
// using the `evalexpr` crate.

use crate::local_models::spec::Spec;
use evalexpr::*;

/// Pure evaluator — no I/O, no database.
pub struct GateEvaluator;

impl GateEvaluator {
    /// Build an `evalexpr` context from the various pieces of spec state.
    pub fn build_context(
        spec: &Spec,
        reviews: &ReviewSummary,
        git_info: &GitInfo,
        workflow_info: &WorkflowInfo,
    ) -> HashMapContext {
        let mut ctx = HashMapContext::new();

        // -- Spec top-level fields --
        let _ = ctx.set_value("spec_title".into(), Value::String(spec.title.clone()));
        let _ = ctx.set_value("spec_status".into(), Value::String(spec.status.clone()));

        // -- Spec custom fields (spec_field_{name}) --
        for (key, value) in &spec.fields {
            let var_name = format!("spec_field_{}", key);
            if let Some(s) = value.as_str() {
                let _ = ctx.set_value(var_name.into(), Value::String(s.to_string()));
            } else if let Some(n) = value.as_f64() {
                let _ = ctx.set_value(var_name.into(), Value::Float(n));
            } else if let Some(b) = value.as_bool() {
                let _ = ctx.set_value(var_name.into(), Value::Boolean(b));
            }
        }

        // -- Markdown sections (spec_section_{name}, {name}_length) --
        let sections = Spec::extract_sections(&spec.body);
        for (name, content) in &sections {
            let var_name = format!("spec_section_{}", name.replace('-', "_"));
            let _ = ctx.set_value(
                var_name.clone().into(),
                Value::Boolean(!content.trim().is_empty()),
            );
            let length_var = format!("{}_length", var_name);
            let _ = ctx.set_value(length_var.into(), Value::Int(content.len() as i64));
        }

        // -- Review state --
        let _ = ctx.set_value("reviews_count".into(), Value::Int(reviews.count as i64));
        let _ = ctx.set_value(
            "reviews_approved".into(),
            Value::Boolean(reviews.has_approval),
        );
        let _ = ctx.set_value(
            "verify_passed".into(),
            Value::Boolean(reviews.verify_passed),
        );

        // -- Git state --
        let _ = ctx.set_value(
            "git_has_branch".into(),
            Value::Boolean(git_info.has_branch),
        );
        let _ = ctx.set_value(
            "git_commit_count".into(),
            Value::Int(git_info.commit_count as i64),
        );
        let _ = ctx.set_value(
            "git_has_commits".into(),
            Value::Boolean(git_info.commit_count > 0),
        );

        // -- Workflow state --
        let _ = ctx.set_value(
            "workflow_current_phase".into(),
            Value::String(workflow_info.current_phase.clone()),
        );
        let _ = ctx.set_value(
            "workflow_time_in_phase_hours".into(),
            Value::Float(workflow_info.time_in_phase_hours),
        );
        let _ = ctx.set_value(
            "workflow_phase_count".into(),
            Value::Int(workflow_info.phase_count as i64),
        );

        ctx
    }

    /// Evaluate a gate condition expression against the given context.
    pub fn evaluate(expression: &str, context: &HashMapContext) -> Result<bool, String> {
        eval_boolean_with_context(expression, context)
            .map_err(|e| format!("Gate evaluation failed: {e}"))
    }

    /// Validate that an expression is syntactically correct (does not execute it).
    pub fn validate_expression(expression: &str) -> Result<(), String> {
        build_operator_tree::<DefaultNumericTypes>(expression)
            .map(|_| ())
            .map_err(|e| format!("Invalid expression: {e}"))
    }
}

/// Summary of review state for a spec.
#[derive(Debug, Default)]
pub struct ReviewSummary {
    pub count: usize,
    pub has_approval: bool,
    pub verify_passed: bool,
}

/// Git state for a spec.
#[derive(Debug, Default)]
pub struct GitInfo {
    pub has_branch: bool,
    pub commit_count: usize,
}

/// Workflow state for a spec.
#[derive(Debug)]
pub struct WorkflowInfo {
    pub current_phase: String,
    pub time_in_phase_hours: f64,
    pub phase_count: usize,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Helper: build a minimal Spec for testing.
    fn test_spec(body: &str, fields: HashMap<String, serde_yaml::Value>) -> Spec {
        Spec {
            id: "spec-test-0000".to_string(),
            schema: "change-request".to_string(),
            title: "Test Spec".to_string(),
            status: "draft".to_string(),
            workflow: None,
            workflow_phase: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            fields,
            body: body.to_string(),
            file_path: None,
        }
    }

    fn default_workflow_info() -> WorkflowInfo {
        WorkflowInfo {
            current_phase: "discuss".to_string(),
            time_in_phase_hours: 1.0,
            phase_count: 1,
        }
    }

    #[test]
    fn test_simple_boolean() {
        let spec = test_spec("", HashMap::new());
        let reviews = ReviewSummary {
            count: 1,
            has_approval: true,
            verify_passed: false,
        };
        let git = GitInfo::default();
        let wf = default_workflow_info();

        let ctx = GateEvaluator::build_context(&spec, &reviews, &git, &wf);
        let result = GateEvaluator::evaluate("reviews_approved == true", &ctx);
        assert_eq!(result, Ok(true));

        let result = GateEvaluator::evaluate("reviews_approved == false", &ctx);
        assert_eq!(result, Ok(false));
    }

    #[test]
    fn test_numeric_comparison() {
        let spec = test_spec("", HashMap::new());
        let reviews = ReviewSummary {
            count: 3,
            has_approval: true,
            verify_passed: false,
        };
        let git = GitInfo::default();
        let wf = default_workflow_info();

        let ctx = GateEvaluator::build_context(&spec, &reviews, &git, &wf);
        assert_eq!(
            GateEvaluator::evaluate("reviews_count >= 2", &ctx),
            Ok(true)
        );
        assert_eq!(
            GateEvaluator::evaluate("reviews_count >= 5", &ctx),
            Ok(false)
        );
    }

    #[test]
    fn test_string_comparison() {
        let mut fields = HashMap::new();
        fields.insert(
            "priority".to_string(),
            serde_yaml::Value::String("high".to_string()),
        );
        let spec = test_spec("", fields);
        let reviews = ReviewSummary::default();
        let git = GitInfo::default();
        let wf = default_workflow_info();

        let ctx = GateEvaluator::build_context(&spec, &reviews, &git, &wf);
        assert_eq!(
            GateEvaluator::evaluate("spec_field_priority == \"high\"", &ctx),
            Ok(true)
        );
        assert_eq!(
            GateEvaluator::evaluate("spec_field_priority == \"low\"", &ctx),
            Ok(false)
        );
    }

    #[test]
    fn test_compound_expression() {
        let spec = test_spec("## Summary\n\nSome content\n", HashMap::new());
        let reviews = ReviewSummary {
            count: 1,
            has_approval: true,
            verify_passed: false,
        };
        let git = GitInfo::default();
        let wf = default_workflow_info();

        let ctx = GateEvaluator::build_context(&spec, &reviews, &git, &wf);
        assert_eq!(
            GateEvaluator::evaluate(
                "spec_section_summary == true && reviews_approved == true",
                &ctx
            ),
            Ok(true)
        );
        assert_eq!(
            GateEvaluator::evaluate(
                "spec_section_summary == true && reviews_approved == false",
                &ctx
            ),
            Ok(false)
        );
    }

    #[test]
    fn test_section_detection() {
        let body_with_content = "## Summary\n\nThis has content.\n";
        let body_empty_section = "## Summary\n\n\n";

        let spec_filled = test_spec(body_with_content, HashMap::new());
        let spec_empty = test_spec(body_empty_section, HashMap::new());
        let reviews = ReviewSummary::default();
        let git = GitInfo::default();
        let wf = default_workflow_info();

        let ctx = GateEvaluator::build_context(&spec_filled, &reviews, &git, &wf);
        assert_eq!(
            GateEvaluator::evaluate("spec_section_summary == true", &ctx),
            Ok(true)
        );

        let ctx = GateEvaluator::build_context(&spec_empty, &reviews, &git, &wf);
        assert_eq!(
            GateEvaluator::evaluate("spec_section_summary == true", &ctx),
            Ok(false)
        );
    }

    #[test]
    fn test_validate_expression_valid() {
        assert!(GateEvaluator::validate_expression("reviews_count >= 2").is_ok());
        assert!(GateEvaluator::validate_expression("a == true && b == false").is_ok());
    }

    #[test]
    fn test_validate_expression_invalid() {
        let result = GateEvaluator::validate_expression("((( unclosed");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid expression"));
    }
}
