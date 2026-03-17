// Spec IPC commands
// Expose spec CRUD operations to the frontend via Tauri commands.

use std::collections::HashMap;
use std::path::Path;

use crate::services::{schema_service, spec_service, SpecforgeWatcher};
use crate::DatabaseState;

/// Convert a Spec to a JSON value that includes body and file_path
/// (which are `#[serde(skip)]` on the Spec struct).
fn spec_to_json(spec: &crate::local_models::spec::Spec) -> Result<serde_json::Value, String> {
    let mut val =
        serde_json::to_value(spec).map_err(|e| format!("Failed to serialize spec: {}", e))?;

    if let Some(obj) = val.as_object_mut() {
        obj.insert("body".to_string(), serde_json::Value::String(spec.body.clone()));
        if let Some(ref fp) = spec.file_path {
            obj.insert(
                "filePath".to_string(),
                serde_json::Value::String(fp.clone()),
            );
        }
    }

    Ok(val)
}

/// Create a new spec from a schema.
#[tauri::command]
pub async fn create_spec(
    schema_name: String,
    title: String,
    project_dir: String,
    db: tauri::State<'_, DatabaseState>,
    watcher: tauri::State<'_, SpecforgeWatcher>,
) -> Result<serde_json::Value, String> {
    let project = project_dir.clone();
    let schemas = schema_service::load_schemas_from_directory(Path::new(&project))?;
    let mut all_schemas = schema_service::get_built_in_schemas();
    // Project-local schemas override built-ins with the same name
    for s in schemas {
        if !all_schemas.iter().any(|b| b.name == s.name) {
            all_schemas.push(s);
        }
    }

    let spec = db.0.with_connection(|conn| {
        spec_service::create_spec(conn, Path::new(&project), &schema_name, &title, &all_schemas)
    })?;

    // Record app write so the file watcher skips this change
    let spec_file = Path::new(&project)
        .join(".specforge")
        .join("specs")
        .join(format!("{}.md", spec.id));
    watcher.record_app_write(&spec_file);

    spec_to_json(&spec)
}

/// List specs from the SQLite index with optional filters.
#[tauri::command]
pub async fn list_specs(
    _project_dir: String,
    status: Option<String>,
    workflow_phase: Option<String>,
    db: tauri::State<'_, DatabaseState>,
) -> Result<Vec<serde_json::Value>, String> {
    let rows = db.0.with_connection(|conn| {
        crate::repositories::spec_repo::list_specs(
            conn,
            status.as_deref(),
            workflow_phase.as_deref(),
        )
    })?;

    rows.iter()
        .map(|row| {
            serde_json::to_value(row).map_err(|e| format!("Failed to serialize spec row: {}", e))
        })
        .collect()
}

/// Get a single spec by ID (reads from file — file is source of truth).
#[tauri::command]
pub async fn get_spec(
    id: String,
    project_dir: String,
) -> Result<serde_json::Value, String> {
    let spec = spec_service::get_spec(Path::new(&project_dir), &id)?;
    spec_to_json(&spec)
}

/// Update a spec's fields and/or body.
#[tauri::command]
pub async fn update_spec(
    id: String,
    fields: Option<serde_json::Value>,
    body: Option<String>,
    project_dir: String,
    db: tauri::State<'_, DatabaseState>,
    watcher: tauri::State<'_, SpecforgeWatcher>,
) -> Result<serde_json::Value, String> {
    // Convert serde_json::Value fields map to HashMap<String, serde_yaml::Value>
    let yaml_fields: Option<HashMap<String, serde_yaml::Value>> = match fields {
        Some(serde_json::Value::Object(map)) => {
            let mut result = HashMap::new();
            for (k, v) in map {
                let yaml_val = json_to_yaml(&v)?;
                result.insert(k, yaml_val);
            }
            Some(result)
        }
        Some(serde_json::Value::Null) | None => None,
        Some(other) => {
            return Err(format!(
                "Expected 'fields' to be an object or null, got: {}",
                other
            ))
        }
    };

    // Record app write before the file is written
    let spec_file = Path::new(&project_dir)
        .join(".specforge")
        .join("specs")
        .join(format!("{}.md", id));
    watcher.record_app_write(&spec_file);

    let project = project_dir.clone();
    let spec = db.0.with_connection(|conn| {
        spec_service::update_spec(Path::new(&project), conn, &id, yaml_fields, body)
    })?;

    spec_to_json(&spec)
}

/// Delete a spec (removes file from disk and row from SQLite).
#[tauri::command]
pub async fn delete_spec(
    id: String,
    project_dir: String,
    db: tauri::State<'_, DatabaseState>,
    watcher: tauri::State<'_, SpecforgeWatcher>,
) -> Result<(), String> {
    // Record app write so the watcher skips the delete event
    let spec_file = Path::new(&project_dir)
        .join(".specforge")
        .join("specs")
        .join(format!("{}.md", id));
    watcher.record_app_write(&spec_file);

    let project = project_dir.clone();
    db.0.with_connection(|conn| {
        spec_service::delete_spec(Path::new(&project), conn, &id)
    })
}

/// Initialize a `.specforge/` project directory structure.
/// After creating the directory structure, automatically starts the file watcher.
#[tauri::command]
pub async fn init_specforge_project(
    app: tauri::AppHandle,
    project_dir: String,
    preset: String,
    db: tauri::State<'_, DatabaseState>,
    watcher: tauri::State<'_, SpecforgeWatcher>,
) -> Result<(), String> {
    let project = project_dir.clone();
    db.0.with_connection(|conn| {
        spec_service::init_project(Path::new(&project), conn, &preset)
    })?;

    // Start watching the newly created .specforge/ directories
    if let Err(e) = watcher.start_watching(&app, Path::new(&project_dir), db.0.clone()) {
        log::warn!(
            "[init_specforge_project] Failed to start specforge watcher: {}",
            e
        );
    }

    Ok(())
}

/// Check whether `.specforge/` directory exists for the given project.
#[tauri::command]
pub async fn check_specforge_exists(project_dir: String) -> Result<bool, String> {
    let path = std::path::Path::new(&project_dir).join(".specforge");
    Ok(path.exists())
}

/// Sync specs from `.specforge/specs/` directory into SQLite index.
/// Returns the number of specs synced.
#[tauri::command]
pub async fn sync_specs(
    project_dir: String,
    db: tauri::State<'_, DatabaseState>,
) -> Result<usize, String> {
    let project = project_dir.clone();
    db.0.with_connection(|conn| {
        spec_service::sync_specs_from_directory(conn, Path::new(&project))
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Convert a `serde_json::Value` to `serde_yaml::Value`.
fn json_to_yaml(json: &serde_json::Value) -> Result<serde_yaml::Value, String> {
    match json {
        serde_json::Value::Null => Ok(serde_yaml::Value::Null),
        serde_json::Value::Bool(b) => Ok(serde_yaml::Value::Bool(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(serde_yaml::Value::Number(serde_yaml::Number::from(i)))
            } else if let Some(f) = n.as_f64() {
                Ok(serde_yaml::Value::Number(
                    serde_yaml::Number::from(f),
                ))
            } else {
                Err(format!("Unsupported number: {}", n))
            }
        }
        serde_json::Value::String(s) => Ok(serde_yaml::Value::String(s.clone())),
        serde_json::Value::Array(arr) => {
            let mut seq = Vec::new();
            for item in arr {
                seq.push(json_to_yaml(item)?);
            }
            Ok(serde_yaml::Value::Sequence(seq))
        }
        serde_json::Value::Object(map) => {
            let mut mapping = serde_yaml::Mapping::new();
            for (k, v) in map {
                mapping.insert(
                    serde_yaml::Value::String(k.clone()),
                    json_to_yaml(v)?,
                );
            }
            Ok(serde_yaml::Value::Mapping(mapping))
        }
    }
}
