// Schema Service
// Loads schema definitions from YAML files and provides built-in schemas.

use crate::local_models::schema::SchemaDefinition;
use std::path::Path;

/// Load all schema definitions from `.specforge/schemas/*.schema.yaml` on disk.
pub fn load_schemas_from_directory(project_dir: &Path) -> Result<Vec<SchemaDefinition>, String> {
    let schemas_dir = project_dir.join(".specforge").join("schemas");
    if !schemas_dir.exists() {
        return Ok(Vec::new());
    }

    let mut schemas = Vec::new();
    let entries = std::fs::read_dir(&schemas_dir)
        .map_err(|e| format!("Failed to read schemas directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        // Only process *.schema.yaml files
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if !file_name.ends_with(".schema.yaml") {
            continue;
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read schema file {}: {}", path.display(), e))?;

        match SchemaDefinition::from_yaml(&content) {
            Ok(schema) => schemas.push(schema),
            Err(e) => {
                log::warn!("Skipping invalid schema file {}: {}", path.display(), e);
            }
        }
    }

    Ok(schemas)
}

/// Return the 3 built-in schema definitions.
pub fn get_built_in_schemas() -> Vec<SchemaDefinition> {
    let yamls = [SPEC_SCHEMA_YAML, CHANGE_REQUEST_SCHEMA_YAML, TASK_SCHEMA_YAML];

    yamls
        .iter()
        .filter_map(|yaml| match SchemaDefinition::from_yaml(yaml) {
            Ok(schema) => Some(schema),
            Err(e) => {
                log::error!("Failed to parse built-in schema: {}", e);
                None
            }
        })
        .collect()
}

/// Write the 3 built-in schemas as YAML files to `.specforge/schemas/`.
pub fn write_built_in_schemas(project_dir: &Path) -> Result<(), String> {
    let schemas_dir = project_dir.join(".specforge").join("schemas");
    std::fs::create_dir_all(&schemas_dir)
        .map_err(|e| format!("Failed to create schemas directory: {}", e))?;

    let files = [
        ("spec.schema.yaml", SPEC_SCHEMA_YAML),
        ("change-request.schema.yaml", CHANGE_REQUEST_SCHEMA_YAML),
        ("task.schema.yaml", TASK_SCHEMA_YAML),
    ];

    for (filename, content) in &files {
        let path = schemas_dir.join(filename);
        std::fs::write(&path, content.trim_start_matches('\n'))
            .map_err(|e| format!("Failed to write {}: {}", filename, e))?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Built-in schema YAML constants
// ---------------------------------------------------------------------------

const SPEC_SCHEMA_YAML: &str = r#"
name: spec
display_name: "Spec"
fields:
  title: string
  status: [draft, active, completed]
  tags: list
sections:
  - name: summary
    required: true
  - name: details
    required: false
"#;

const CHANGE_REQUEST_SCHEMA_YAML: &str = r#"
name: change-request
display_name: "Change Request"
fields:
  title: string
  priority: [high, medium, low]
  assignee: string
  tags: list
  deadline: date
sections:
  - name: summary
    required: true
  - name: acceptance-criteria
    required: true
  - name: technical-notes
    required: false
"#;

const TASK_SCHEMA_YAML: &str = r#"
name: task
display_name: "Task"
fields:
  title: string
  priority: [high, medium, low]
  assignee: string
  estimate: string
sections:
  - name: description
    required: true
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_get_built_in_schemas() {
        let schemas = get_built_in_schemas();
        assert_eq!(schemas.len(), 3);

        let names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"spec"));
        assert!(names.contains(&"change-request"));
        assert!(names.contains(&"task"));

        // Verify spec schema
        let spec = schemas.iter().find(|s| s.name == "spec").unwrap();
        assert_eq!(spec.display_name, Some("Spec".to_string()));
        assert!(spec.fields.contains_key("title"));
        assert!(spec.fields.contains_key("status"));
        assert!(spec.fields.contains_key("tags"));
        assert_eq!(spec.sections.len(), 2);

        // Verify change-request schema
        let cr = schemas.iter().find(|s| s.name == "change-request").unwrap();
        assert_eq!(cr.display_name, Some("Change Request".to_string()));
        assert!(cr.fields.contains_key("priority"));
        assert!(cr.fields.contains_key("deadline"));
        assert_eq!(cr.sections.len(), 3);
        assert!(cr.sections[0].required); // summary
        assert!(cr.sections[1].required); // acceptance-criteria

        // Verify task schema
        let task = schemas.iter().find(|s| s.name == "task").unwrap();
        assert_eq!(task.display_name, Some("Task".to_string()));
        assert!(task.fields.contains_key("estimate"));
        assert_eq!(task.sections.len(), 1);
    }

    #[test]
    fn test_write_and_load_schemas() {
        let dir = tempdir().expect("create temp dir");
        let project_dir = dir.path();

        // Write built-in schemas
        write_built_in_schemas(project_dir).expect("write schemas");

        // Verify files exist
        let schemas_dir = project_dir.join(".specforge").join("schemas");
        assert!(schemas_dir.join("spec.schema.yaml").exists());
        assert!(schemas_dir.join("change-request.schema.yaml").exists());
        assert!(schemas_dir.join("task.schema.yaml").exists());

        // Load them back
        let loaded = load_schemas_from_directory(project_dir).expect("load schemas");
        assert_eq!(loaded.len(), 3);

        let names: Vec<&str> = loaded.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"spec"));
        assert!(names.contains(&"change-request"));
        assert!(names.contains(&"task"));
    }

    #[test]
    fn test_load_schemas_empty_directory() {
        let dir = tempdir().expect("create temp dir");
        let loaded = load_schemas_from_directory(dir.path()).expect("load");
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_load_schemas_skips_non_schema_files() {
        let dir = tempdir().expect("create temp dir");
        let schemas_dir = dir.path().join(".specforge").join("schemas");
        std::fs::create_dir_all(&schemas_dir).expect("create dirs");

        // Write a valid schema
        std::fs::write(
            schemas_dir.join("spec.schema.yaml"),
            SPEC_SCHEMA_YAML.trim_start_matches('\n'),
        )
        .expect("write");

        // Write a non-schema file (should be ignored)
        std::fs::write(schemas_dir.join("README.md"), "# Schemas").expect("write readme");

        let loaded = load_schemas_from_directory(dir.path()).expect("load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "spec");
    }
}
