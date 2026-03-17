// Spec Service
// Core spec operations: coordinates file I/O (source of truth) and SQLite (index/cache).

use crate::local_models::schema::SchemaDefinition;
use crate::local_models::spec::Spec;
use crate::repositories::spec_repo::{self, SpecRow};
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::Path;

/// Create a new spec: generate ID, write markdown file, insert into SQLite.
pub fn create_spec(
    conn: &Connection,
    project_dir: &Path,
    schema_name: &str,
    title: &str,
    schemas: &[SchemaDefinition],
) -> Result<Spec, String> {
    let schema_def = schemas
        .iter()
        .find(|s| s.name == schema_name)
        .ok_or_else(|| format!("Schema not found: {}", schema_name))?;

    let id = Spec::generate_id(title);
    let now = chrono::Utc::now().to_rfc3339();

    // Build default fields from schema definition
    let fields = build_default_fields(schema_def);

    // Build body: try template engine first, fall back to section headings
    let body = build_body_from_template(project_dir, schema_name, title, &fields)
        .unwrap_or_else(|| build_body_from_schema(schema_def));

    let spec = Spec {
        id: id.clone(),
        schema: schema_name.to_string(),
        title: title.to_string(),
        status: "draft".to_string(),
        workflow: None,
        workflow_phase: None,
        created_at: now.clone(),
        updated_at: now,
        fields,
        body,
        file_path: None,
    };

    // Write to disk
    let file_path = write_spec_file(project_dir, &spec)?;

    // Insert into SQLite index
    let row = spec_to_row(&spec, &file_path);
    spec_repo::insert_spec(conn, &row)?;

    Ok(spec)
}

/// Read a spec from its markdown file on disk (file is source of truth).
pub fn get_spec(project_dir: &Path, id: &str) -> Result<Spec, String> {
    let file_path = spec_file_path(project_dir, id);
    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read spec file {}: {}", file_path.display(), e))?;

    let mut spec = Spec::from_markdown(&content)?;
    spec.file_path = Some(file_path.to_string_lossy().to_string());
    Ok(spec)
}

/// List all specs from the SQLite index.
pub fn list_specs(conn: &Connection) -> Result<Vec<SpecRow>, String> {
    spec_repo::list_specs(conn, None, None)
}

/// Update a spec: apply field/body changes, write back to disk, update SQLite.
pub fn update_spec(
    project_dir: &Path,
    conn: &Connection,
    id: &str,
    fields: Option<HashMap<String, serde_yaml::Value>>,
    body: Option<String>,
) -> Result<Spec, String> {
    // Read current file
    let mut spec = get_spec(project_dir, id)?;

    // Apply field changes
    if let Some(new_fields) = fields {
        for (key, value) in new_fields {
            // Allow updating top-level spec fields via the fields map
            match key.as_str() {
                "title" => {
                    if let Some(s) = value.as_str() {
                        spec.title = s.to_string();
                    }
                }
                "status" => {
                    if let Some(s) = value.as_str() {
                        spec.status = s.to_string();
                    }
                }
                "workflow" => {
                    spec.workflow = value.as_str().map(|s| s.to_string());
                }
                "workflow_phase" => {
                    spec.workflow_phase = value.as_str().map(|s| s.to_string());
                }
                _ => {
                    spec.fields.insert(key, value);
                }
            }
        }
    }

    // Apply body change
    if let Some(new_body) = body {
        spec.body = new_body;
    }

    // Update timestamp
    spec.updated_at = chrono::Utc::now().to_rfc3339();

    // Write back to disk
    let file_path = write_spec_file(project_dir, &spec)?;

    // Update SQLite index
    let row = spec_to_row(&spec, &file_path);
    spec_repo::update_spec(conn, &row)?;

    Ok(spec)
}

/// Delete a spec: remove file from disk and row from SQLite.
pub fn delete_spec(project_dir: &Path, conn: &Connection, id: &str) -> Result<(), String> {
    let file_path = spec_file_path(project_dir, id);

    // Remove file if it exists
    if file_path.exists() {
        std::fs::remove_file(&file_path)
            .map_err(|e| format!("Failed to delete spec file: {}", e))?;
    }

    // Remove from SQLite index
    spec_repo::delete_spec(conn, id)?;

    Ok(())
}

/// Scan `.specforge/specs/*.md`, parse each, upsert into SQLite. Return count synced.
pub fn sync_specs_from_directory(conn: &Connection, project_dir: &Path) -> Result<usize, String> {
    let specs_dir = project_dir.join(".specforge").join("specs");
    if !specs_dir.exists() {
        return Ok(0);
    }

    let entries = std::fs::read_dir(&specs_dir)
        .map_err(|e| format!("Failed to read specs directory: {}", e))?;

    let mut count = 0;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if !file_name.ends_with(".md") {
            continue;
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Skipping unreadable spec file {}: {}", path.display(), e);
                continue;
            }
        };

        let spec = match Spec::from_markdown(&content) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("Skipping invalid spec file {}: {}", path.display(), e);
                continue;
            }
        };

        // Compute relative file path for storage
        let rel_path = path
            .strip_prefix(project_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();

        let row = spec_to_row(&spec, &rel_path);
        spec_repo::upsert_spec(conn, &row)?;
        count += 1;
    }

    Ok(count)
}

/// Initialize a `.specforge/` project directory structure.
///
/// - `preset = "basic-sdd"`: create dirs + write built-in schemas + default workflow
/// - `preset = "blank"`: create empty directory structure only
pub fn init_project(project_dir: &Path, conn: &Connection, preset: &str) -> Result<(), String> {
    let base = project_dir.join(".specforge");

    // Create directory structure
    let subdirs = ["schemas", "templates", "workflows", "specs", "archive"];
    for sub in &subdirs {
        std::fs::create_dir_all(base.join(sub))
            .map_err(|e| format!("Failed to create .specforge/{}: {}", sub, e))?;
    }

    if preset == "basic-sdd" {
        // Write built-in schemas
        crate::services::schema_service::write_built_in_schemas(project_dir)?;

        // Write built-in templates
        crate::services::template_engine::write_built_in_templates(project_dir)?;

        // Write default workflow YAML
        write_default_workflow(project_dir)?;

        // Sync schemas into SQLite
        let schemas =
            crate::services::schema_service::load_schemas_from_directory(project_dir)?;
        for schema in &schemas {
            let fields_json = serde_json::to_string(&schema.fields)
                .unwrap_or_else(|_| "{}".to_string());
            let rel_path = format!(".specforge/schemas/{}.schema.yaml", schema.name);
            crate::repositories::schema_repo::upsert_schema(
                conn,
                &schema.name,
                schema.display_name.as_deref(),
                &rel_path,
                &fields_json,
            )?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Compute the file path for a spec on disk.
fn spec_file_path(project_dir: &Path, id: &str) -> std::path::PathBuf {
    project_dir
        .join(".specforge")
        .join("specs")
        .join(format!("{}.md", id))
}

/// Write a spec as markdown to `.specforge/specs/{id}.md`, returning the relative path.
fn write_spec_file(project_dir: &Path, spec: &Spec) -> Result<String, String> {
    let specs_dir = project_dir.join(".specforge").join("specs");
    std::fs::create_dir_all(&specs_dir)
        .map_err(|e| format!("Failed to create specs directory: {}", e))?;

    let abs_path = specs_dir.join(format!("{}.md", spec.id));
    let content = spec.to_markdown()?;

    // Use advisory file locking for concurrent safety
    use fs2::FileExt;
    let file = std::fs::File::create(&abs_path)
        .map_err(|e| format!("Failed to create spec file: {}", e))?;
    file.lock_exclusive()
        .map_err(|e| format!("Failed to lock spec file: {}", e))?;
    std::fs::write(&abs_path, content)
        .map_err(|e| format!("Failed to write spec file: {}", e))?;
    file.unlock()
        .map_err(|e| format!("Failed to unlock spec file: {}", e))?;

    let rel_path = format!(".specforge/specs/{}.md", spec.id);
    Ok(rel_path)
}

/// Convert a Spec + relative file path into a SpecRow for SQLite.
fn spec_to_row(spec: &Spec, file_path: &str) -> SpecRow {
    let fields_json = if spec.fields.is_empty() {
        None
    } else {
        serde_json::to_string(&spec.fields).ok()
    };

    SpecRow {
        id: spec.id.clone(),
        schema_id: spec.schema.clone(),
        title: spec.title.clone(),
        status: spec.status.clone(),
        workflow_id: spec.workflow.clone(),
        workflow_phase: spec.workflow_phase.clone(),
        file_path: file_path.to_string(),
        fields_json,
        created_at: spec.created_at.clone(),
        updated_at: spec.updated_at.clone(),
    }
}

/// Build default field values from a schema definition.
///
/// For enum fields, picks the first allowed value; for other types, leaves empty.
fn build_default_fields(schema: &SchemaDefinition) -> HashMap<String, serde_yaml::Value> {
    use crate::local_models::schema::FieldType;

    let mut fields = HashMap::new();
    for (name, field_type) in &schema.fields {
        // Skip "title" and "status" — those live in spec top-level fields
        if name == "title" || name == "status" {
            continue;
        }
        match field_type {
            FieldType::Simple(type_name) => match type_name.as_str() {
                "string" | "date" => {
                    fields.insert(name.clone(), serde_yaml::Value::String(String::new()));
                }
                "number" => {
                    fields.insert(
                        name.clone(),
                        serde_yaml::Value::Number(serde_yaml::Number::from(0)),
                    );
                }
                "list" => {
                    fields.insert(name.clone(), serde_yaml::Value::Sequence(Vec::new()));
                }
                _ => {}
            },
            FieldType::Enum(values) => {
                if let Some(first) = values.first() {
                    fields.insert(
                        name.clone(),
                        serde_yaml::Value::String(first.clone()),
                    );
                }
            }
        }
    }
    fields
}

/// Build a body by rendering a Tera template (custom or built-in) with spec context.
/// Returns `None` if no template is available for the schema.
fn build_body_from_template(
    project_dir: &Path,
    schema_name: &str,
    title: &str,
    fields: &HashMap<String, serde_yaml::Value>,
) -> Option<String> {
    use crate::services::template_engine;

    let template_content = template_engine::load_template(project_dir, schema_name)?;

    // Convert serde_yaml field values to strings for the template context
    let string_fields: HashMap<String, String> = fields
        .iter()
        .map(|(k, v)| {
            let s = match v {
                serde_yaml::Value::String(s) => s.clone(),
                serde_yaml::Value::Number(n) => n.to_string(),
                serde_yaml::Value::Bool(b) => b.to_string(),
                _ => String::new(),
            };
            (k.clone(), s)
        })
        .collect();

    let ctx = template_engine::TemplateContext {
        title: title.to_string(),
        schema_name: schema_name.to_string(),
        fields: string_fields,
        date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
    };

    match template_engine::render_template(&template_content, &ctx) {
        Ok(rendered) => Some(rendered),
        Err(e) => {
            log::warn!(
                "Failed to render template for schema '{}': {}. Falling back to section headings.",
                schema_name,
                e
            );
            None
        }
    }
}

/// Build a markdown body with `## Heading` sections from a schema definition.
fn build_body_from_schema(schema: &SchemaDefinition) -> String {
    let mut body = String::new();
    for section in &schema.sections {
        let display_name = section
            .name
            .split('-')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(c) => {
                        let upper: String = c.to_uppercase().collect();
                        format!("{}{}", upper, chars.collect::<String>())
                    }
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        if !body.is_empty() {
            body.push('\n');
        }
        body.push_str(&format!("## {}\n\n", display_name));
    }
    body
}

/// Write a default spec-driven workflow YAML to `.specforge/workflows/`.
fn write_default_workflow(project_dir: &Path) -> Result<(), String> {
    let workflows_dir = project_dir.join(".specforge").join("workflows");
    std::fs::create_dir_all(&workflows_dir)
        .map_err(|e| format!("Failed to create workflows directory: {}", e))?;

    let content = r#"name: default
phases:
  - name: discuss
    display_name: "Discuss"
    description: "Initial discussion and scoping"
  - name: refine
    display_name: "Refine"
    description: "Refine requirements and acceptance criteria"
  - name: implement
    display_name: "Implement"
    description: "Implementation phase"
  - name: review
    display_name: "Review"
    description: "Code review and QA"
  - name: done
    display_name: "Done"
    description: "Completed"
"#;

    let path = workflows_dir.join("default.yaml");
    std::fs::write(&path, content)
        .map_err(|e| format!("Failed to write default workflow: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::schema::run_migrations;
    use rusqlite::Connection;
    use tempfile::tempdir;

    fn setup() -> (tempfile::TempDir, Connection) {
        let dir = tempdir().expect("create temp dir");
        let conn = Connection::open_in_memory().expect("open in-memory db");
        run_migrations(&conn).expect("run migrations");
        (dir, conn)
    }

    #[test]
    fn test_create_and_get_spec() {
        let (dir, conn) = setup();
        let schemas = crate::services::schema_service::get_built_in_schemas();

        let spec = create_spec(&conn, dir.path(), "change-request", "Add OAuth2", &schemas)
            .expect("create");

        assert!(spec.id.starts_with("spec-"));
        assert_eq!(spec.schema, "change-request");
        assert_eq!(spec.title, "Add OAuth2");
        assert_eq!(spec.status, "draft");

        // Should have default fields from schema (priority, assignee, tags, deadline)
        assert!(spec.fields.contains_key("priority"));
        assert!(spec.fields.contains_key("assignee"));
        assert!(spec.fields.contains_key("tags"));
        assert!(spec.fields.contains_key("deadline"));

        // Body should have section headings
        assert!(spec.body.contains("## Summary"));
        assert!(spec.body.contains("## Acceptance Criteria"));
        assert!(spec.body.contains("## Technical Notes"));

        // Should be readable from disk
        let loaded = get_spec(dir.path(), &spec.id).expect("get");
        assert_eq!(loaded.id, spec.id);
        assert_eq!(loaded.title, spec.title);

        // Should be in SQLite index
        let rows = list_specs(&conn).expect("list");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, spec.id);
    }

    #[test]
    fn test_create_spec_unknown_schema() {
        let (dir, conn) = setup();
        let schemas = crate::services::schema_service::get_built_in_schemas();

        let result = create_spec(&conn, dir.path(), "nonexistent", "Title", &schemas);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Schema not found"));
    }

    #[test]
    fn test_update_spec() {
        let (dir, conn) = setup();
        let schemas = crate::services::schema_service::get_built_in_schemas();

        let spec = create_spec(&conn, dir.path(), "task", "My Task", &schemas).expect("create");

        let mut new_fields = HashMap::new();
        new_fields.insert(
            "title".to_string(),
            serde_yaml::Value::String("Updated Task".to_string()),
        );
        new_fields.insert(
            "status".to_string(),
            serde_yaml::Value::String("active".to_string()),
        );
        new_fields.insert(
            "priority".to_string(),
            serde_yaml::Value::String("high".to_string()),
        );

        let updated = update_spec(
            dir.path(),
            &conn,
            &spec.id,
            Some(new_fields),
            Some("## Description\n\nUpdated body content\n".to_string()),
        )
        .expect("update");

        assert_eq!(updated.title, "Updated Task");
        assert_eq!(updated.status, "active");
        assert_eq!(
            updated.fields.get("priority"),
            Some(&serde_yaml::Value::String("high".to_string()))
        );
        assert!(updated.body.contains("Updated body content"));

        // Verify persisted on disk
        let reloaded = get_spec(dir.path(), &spec.id).expect("get");
        assert_eq!(reloaded.title, "Updated Task");
    }

    #[test]
    fn test_delete_spec() {
        let (dir, conn) = setup();
        let schemas = crate::services::schema_service::get_built_in_schemas();

        let spec = create_spec(&conn, dir.path(), "spec", "To Delete", &schemas).expect("create");
        let file_path = dir
            .path()
            .join(".specforge")
            .join("specs")
            .join(format!("{}.md", spec.id));
        assert!(file_path.exists());

        delete_spec(dir.path(), &conn, &spec.id).expect("delete");

        assert!(!file_path.exists());

        let rows = list_specs(&conn).expect("list");
        assert!(rows.is_empty());
    }

    #[test]
    fn test_sync_specs_from_directory() {
        let (dir, conn) = setup();
        let schemas = crate::services::schema_service::get_built_in_schemas();

        // Create some specs via the service (writes files + inserts into DB)
        let spec1 =
            create_spec(&conn, dir.path(), "spec", "Spec One", &schemas).expect("create 1");
        let spec2 =
            create_spec(&conn, dir.path(), "task", "Task Two", &schemas).expect("create 2");

        // Clear the SQLite index to simulate a fresh sync
        spec_repo::delete_spec(&conn, &spec1.id).expect("delete from db");
        spec_repo::delete_spec(&conn, &spec2.id).expect("delete from db");
        let rows = list_specs(&conn).expect("list");
        assert!(rows.is_empty());

        // Sync from directory should re-populate
        let count = sync_specs_from_directory(&conn, dir.path()).expect("sync");
        assert_eq!(count, 2);

        let rows = list_specs(&conn).expect("list");
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_sync_specs_empty_directory() {
        let (dir, conn) = setup();
        let count = sync_specs_from_directory(&conn, dir.path()).expect("sync");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_init_project_blank() {
        let (dir, conn) = setup();
        init_project(dir.path(), &conn, "blank").expect("init blank");

        let base = dir.path().join(".specforge");
        assert!(base.join("schemas").is_dir());
        assert!(base.join("templates").is_dir());
        assert!(base.join("workflows").is_dir());
        assert!(base.join("specs").is_dir());
        assert!(base.join("archive").is_dir());

        // No schema files should exist
        let entries: Vec<_> = std::fs::read_dir(base.join("schemas"))
            .expect("read schemas dir")
            .collect();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_init_project_basic_sdd() {
        let (dir, conn) = setup();
        init_project(dir.path(), &conn, "basic-sdd").expect("init basic-sdd");

        let base = dir.path().join(".specforge");

        // Directories created
        assert!(base.join("schemas").is_dir());
        assert!(base.join("specs").is_dir());
        assert!(base.join("workflows").is_dir());

        // Built-in schemas written
        assert!(base.join("schemas").join("spec.schema.yaml").exists());
        assert!(base
            .join("schemas")
            .join("change-request.schema.yaml")
            .exists());
        assert!(base.join("schemas").join("task.schema.yaml").exists());

        // Default workflow written
        assert!(base.join("workflows").join("default.yaml").exists());

        // Built-in templates written
        assert!(base.join("templates").join("spec.md").exists());
        assert!(base.join("templates").join("change-request.md").exists());
        assert!(base.join("templates").join("task.md").exists());

        // Schemas synced to SQLite
        let schema_rows =
            crate::repositories::schema_repo::list_schemas(&conn).expect("list schemas");
        assert_eq!(schema_rows.len(), 3);
    }

    #[test]
    fn test_build_default_fields() {
        let schemas = crate::services::schema_service::get_built_in_schemas();
        let cr = schemas
            .iter()
            .find(|s| s.name == "change-request")
            .unwrap();

        let fields = build_default_fields(cr);

        // "title" should be excluded (it's a top-level spec field)
        assert!(!fields.contains_key("title"));

        // priority is an enum, should default to first value "high"
        assert_eq!(
            fields.get("priority"),
            Some(&serde_yaml::Value::String("high".to_string()))
        );

        // assignee is a string
        assert_eq!(
            fields.get("assignee"),
            Some(&serde_yaml::Value::String(String::new()))
        );

        // tags is a list
        assert!(fields
            .get("tags")
            .map(|v| v.is_sequence())
            .unwrap_or(false));

        // deadline is a date (string type)
        assert_eq!(
            fields.get("deadline"),
            Some(&serde_yaml::Value::String(String::new()))
        );
    }

    #[test]
    fn test_build_body_from_schema() {
        let schemas = crate::services::schema_service::get_built_in_schemas();
        let cr = schemas
            .iter()
            .find(|s| s.name == "change-request")
            .unwrap();

        let body = build_body_from_schema(cr);

        assert!(body.contains("## Summary"));
        assert!(body.contains("## Acceptance Criteria"));
        assert!(body.contains("## Technical Notes"));
    }
}
