// Template Engine Service
// Uses Tera to render markdown templates with spec context variables.

use std::collections::HashMap;

/// Context variables available to templates.
pub struct TemplateContext {
    pub title: String,
    pub schema_name: String,
    pub fields: HashMap<String, String>,
    pub date: String,
}

/// Render a template string with spec context variables.
///
/// Supports `{{ title }}`, `{{ schema_name }}`, `{{ date }}`, `{{ fields.priority }}`, etc.
pub fn render_template(template_content: &str, context: &TemplateContext) -> Result<String, String> {
    let mut tera = tera::Tera::default();
    tera.add_raw_template("__inline__", template_content)
        .map_err(|e| format!("Failed to parse template: {}", e))?;

    let mut ctx = tera::Context::new();
    ctx.insert("title", &context.title);
    ctx.insert("schema_name", &context.schema_name);
    ctx.insert("date", &context.date);
    ctx.insert("fields", &context.fields);

    tera.render("__inline__", &ctx)
        .map_err(|e| format!("Failed to render template: {}", e))
}

// ---------------------------------------------------------------------------
// Built-in template constants (one per built-in schema)
// ---------------------------------------------------------------------------

pub const SPEC_TEMPLATE: &str = r#"## Summary

{{ title }}

## Details

<!-- Add detailed specification here -->
"#;

pub const CHANGE_REQUEST_TEMPLATE: &str = r#"## Summary

{{ title }}

## Acceptance Criteria

- [ ]

## Technical Notes

<!-- Implementation details, dependencies, risks -->
"#;

pub const TASK_TEMPLATE: &str = r#"## Description

{{ title }}
"#;

/// Return the built-in template for a given schema name, or `None` if unknown.
pub fn get_built_in_template(schema_name: &str) -> Option<&'static str> {
    match schema_name {
        "spec" => Some(SPEC_TEMPLATE),
        "change-request" => Some(CHANGE_REQUEST_TEMPLATE),
        "task" => Some(TASK_TEMPLATE),
        _ => None,
    }
}

/// Write built-in template files to `.specforge/templates/`.
pub fn write_built_in_templates(project_dir: &std::path::Path) -> Result<(), String> {
    let templates_dir = project_dir.join(".specforge").join("templates");
    std::fs::create_dir_all(&templates_dir)
        .map_err(|e| format!("Failed to create templates directory: {}", e))?;

    let files = [
        ("spec.md", SPEC_TEMPLATE),
        ("change-request.md", CHANGE_REQUEST_TEMPLATE),
        ("task.md", TASK_TEMPLATE),
    ];

    for (filename, content) in &files {
        let path = templates_dir.join(filename);
        std::fs::write(&path, content)
            .map_err(|e| format!("Failed to write template {}: {}", filename, e))?;
    }

    Ok(())
}

/// Try to load a custom template from `.specforge/templates/{schema_name}.md`.
/// Falls back to built-in template if not found on disk.
pub fn load_template(project_dir: &std::path::Path, schema_name: &str) -> Option<String> {
    let custom_path = project_dir
        .join(".specforge")
        .join("templates")
        .join(format!("{}.md", schema_name));

    if custom_path.exists() {
        std::fs::read_to_string(&custom_path).ok()
    } else {
        get_built_in_template(schema_name).map(|s| s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_template_basic() {
        let ctx = TemplateContext {
            title: "Add OAuth2 Support".to_string(),
            schema_name: "spec".to_string(),
            fields: HashMap::new(),
            date: "2026-03-17".to_string(),
        };

        let result = render_template(SPEC_TEMPLATE, &ctx).expect("render");
        assert!(result.contains("Add OAuth2 Support"));
        assert!(result.contains("## Summary"));
        assert!(result.contains("## Details"));
    }

    #[test]
    fn test_render_template_with_fields() {
        let mut fields = HashMap::new();
        fields.insert("priority".to_string(), "high".to_string());

        let ctx = TemplateContext {
            title: "Fix Bug".to_string(),
            schema_name: "change-request".to_string(),
            fields,
            date: "2026-03-17".to_string(),
        };

        let template = "Priority: {{ fields.priority }}\n{{ title }}";
        let result = render_template(template, &ctx).expect("render");
        assert!(result.contains("Priority: high"));
        assert!(result.contains("Fix Bug"));
    }

    #[test]
    fn test_render_template_date() {
        let ctx = TemplateContext {
            title: "Test".to_string(),
            schema_name: "task".to_string(),
            fields: HashMap::new(),
            date: "2026-03-17".to_string(),
        };

        let template = "Created: {{ date }}";
        let result = render_template(template, &ctx).expect("render");
        assert_eq!(result, "Created: 2026-03-17");
    }

    #[test]
    fn test_render_change_request_template() {
        let ctx = TemplateContext {
            title: "Upgrade Database".to_string(),
            schema_name: "change-request".to_string(),
            fields: HashMap::new(),
            date: "2026-03-17".to_string(),
        };

        let result = render_template(CHANGE_REQUEST_TEMPLATE, &ctx).expect("render");
        assert!(result.contains("Upgrade Database"));
        assert!(result.contains("## Acceptance Criteria"));
        assert!(result.contains("## Technical Notes"));
    }

    #[test]
    fn test_render_task_template() {
        let ctx = TemplateContext {
            title: "Write unit tests".to_string(),
            schema_name: "task".to_string(),
            fields: HashMap::new(),
            date: "2026-03-17".to_string(),
        };

        let result = render_template(TASK_TEMPLATE, &ctx).expect("render");
        assert!(result.contains("Write unit tests"));
        assert!(result.contains("## Description"));
    }

    #[test]
    fn test_get_built_in_template() {
        assert!(get_built_in_template("spec").is_some());
        assert!(get_built_in_template("change-request").is_some());
        assert!(get_built_in_template("task").is_some());
        assert!(get_built_in_template("unknown").is_none());
    }

    #[test]
    fn test_write_built_in_templates() {
        let dir = tempfile::tempdir().expect("create temp dir");
        write_built_in_templates(dir.path()).expect("write templates");

        let templates_dir = dir.path().join(".specforge").join("templates");
        assert!(templates_dir.join("spec.md").exists());
        assert!(templates_dir.join("change-request.md").exists());
        assert!(templates_dir.join("task.md").exists());

        // Verify content
        let content = std::fs::read_to_string(templates_dir.join("spec.md")).expect("read");
        assert!(content.contains("{{ title }}"));
        assert!(content.contains("## Summary"));
    }

    #[test]
    fn test_load_template_fallback_to_builtin() {
        let dir = tempfile::tempdir().expect("create temp dir");
        // No custom templates on disk — should fall back to built-in
        let template = load_template(dir.path(), "spec");
        assert!(template.is_some());
        assert!(template.unwrap().contains("{{ title }}"));
    }

    #[test]
    fn test_load_template_custom_override() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let templates_dir = dir.path().join(".specforge").join("templates");
        std::fs::create_dir_all(&templates_dir).expect("create dirs");

        let custom = "## Custom\n\n{{ title }} - custom template";
        std::fs::write(templates_dir.join("spec.md"), custom).expect("write custom");

        let template = load_template(dir.path(), "spec");
        assert!(template.is_some());
        assert!(template.unwrap().contains("custom template"));
    }

    #[test]
    fn test_render_invalid_template_syntax() {
        let ctx = TemplateContext {
            title: "Test".to_string(),
            schema_name: "spec".to_string(),
            fields: HashMap::new(),
            date: "2026-03-17".to_string(),
        };

        let result = render_template("{{ unclosed", &ctx);
        assert!(result.is_err());
    }
}
