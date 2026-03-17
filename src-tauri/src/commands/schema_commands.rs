// Schema IPC commands
// Expose schema read operations to the frontend via Tauri commands.

use std::path::Path;

use crate::services::schema_service;

/// List all available schemas (project-local + built-in).
/// Project-local schemas in `.specforge/schemas/` take precedence over built-ins
/// with the same name.
#[tauri::command]
pub async fn list_schemas(
    project_dir: String,
) -> Result<Vec<serde_json::Value>, String> {
    let mut schemas = schema_service::get_built_in_schemas();
    let local = schema_service::load_schemas_from_directory(Path::new(&project_dir))?;

    // Local schemas override built-ins with the same name
    for local_schema in local {
        if let Some(pos) = schemas.iter().position(|s| s.name == local_schema.name) {
            schemas[pos] = local_schema;
        } else {
            schemas.push(local_schema);
        }
    }

    schemas
        .iter()
        .map(|s| {
            serde_json::to_value(s)
                .map_err(|e| format!("Failed to serialize schema: {}", e))
        })
        .collect()
}

/// Get a single schema by name (checks project-local first, then built-in).
#[tauri::command]
pub async fn get_schema(
    name: String,
    project_dir: String,
) -> Result<serde_json::Value, String> {
    // Try project-local schemas first
    let local = schema_service::load_schemas_from_directory(Path::new(&project_dir))?;
    if let Some(schema) = local.into_iter().find(|s| s.name == name) {
        return serde_json::to_value(&schema)
            .map_err(|e| format!("Failed to serialize schema: {}", e));
    }

    // Fall back to built-in schemas
    let builtins = schema_service::get_built_in_schemas();
    if let Some(schema) = builtins.into_iter().find(|s| s.name == name) {
        return serde_json::to_value(&schema)
            .map_err(|e| format!("Failed to serialize schema: {}", e));
    }

    Err(format!("Schema not found: {}", name))
}
