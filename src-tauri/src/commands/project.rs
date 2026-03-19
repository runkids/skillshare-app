use crate::models::project::{Project, ProjectType};
use crate::services::project_store;

#[tauri::command]
pub fn list_projects() -> Vec<Project> {
    project_store::load().projects
}

#[tauri::command]
pub fn get_active_project() -> Option<Project> {
    let store = project_store::load();
    store.active_project().cloned()
}

#[tauri::command]
pub fn add_project(
    name: String,
    path: String,
    project_type: ProjectType,
) -> Result<Project, String> {
    let mut store = project_store::load();
    let project = project_store::add_project(&mut store, name, path, project_type);
    project_store::save(&store)?;
    Ok(project)
}

#[tauri::command]
pub fn remove_project(id: String) -> Result<(), String> {
    let mut store = project_store::load();
    project_store::remove_project(&mut store, &id);
    project_store::save(&store)
}

#[tauri::command]
pub fn switch_project(id: String) -> Result<(), String> {
    let mut store = project_store::load();
    project_store::set_active(&mut store, &id)?;
    project_store::save(&store)
}
