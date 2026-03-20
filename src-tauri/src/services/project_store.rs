use crate::models::project::{Project, ProjectStore, ProjectType};
use std::path::PathBuf;

fn store_path() -> PathBuf {
    crate::utils::paths::app_data_dir().join("projects.json")
}

pub fn load() -> ProjectStore {
    let path = store_path();
    if path.exists() {
        let data = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        ProjectStore::default()
    }
}

pub fn save(store: &ProjectStore) -> Result<(), String> {
    let path = store_path();
    let data =
        serde_json::to_string_pretty(store).map_err(|e| format!("Serialize error: {e}"))?;
    std::fs::write(&path, data).map_err(|e| format!("Write error: {e}"))
}

pub fn add_project(
    store: &mut ProjectStore,
    name: String,
    path: String,
    project_type: ProjectType,
) -> Project {
    let project = Project {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        path,
        project_type,
        added_at: chrono::Utc::now().to_rfc3339(),
    };
    store.projects.push(project.clone());
    if store.active_project_id.is_none() {
        store.active_project_id = Some(project.id.clone());
    }
    project
}

pub fn remove_project(store: &mut ProjectStore, id: &str) {
    store.projects.retain(|p| p.id != id);
    if store.active_project_id.as_deref() == Some(id) {
        store.active_project_id = store.projects.first().map(|p| p.id.clone());
    }
}

/// Returns (project_dir, is_project_mode) for the active project.
/// Used by server start/restart to determine CLI flags.
pub fn active_project_mode(store: &ProjectStore) -> (Option<String>, bool) {
    let active = store.active_project();
    let dir = active.map(|p| p.path.clone());
    let is_project = active
        .map(|p| p.project_type == ProjectType::Project)
        .unwrap_or(false);
    (dir, is_project)
}

pub fn set_active(store: &mut ProjectStore, id: &str) -> Result<(), String> {
    if store.projects.iter().any(|p| p.id == id) {
        store.active_project_id = Some(id.to_string());
        Ok(())
    } else {
        Err(format!("Project {id} not found"))
    }
}
