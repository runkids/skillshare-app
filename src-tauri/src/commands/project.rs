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
    app: tauri::AppHandle,
    name: String,
    path: String,
    project_type: ProjectType,
) -> Result<Project, String> {
    let mut store = project_store::load();

    if project_type == ProjectType::Global {
        let has_global = store
            .projects
            .iter()
            .any(|p| p.project_type == ProjectType::Global);
        if has_global {
            return Err(
                "A global project already exists. Remove it first to add a new one.".to_string(),
            );
        }
    }

    let project = project_store::add_project(&mut store, name, path, project_type)?;
    project_store::save(&store)?;
    crate::refresh_tray_project_label(&app);
    crate::services::auto_sync::refresh(&app);
    crate::services::source_health::project_changed(&app);
    Ok(project)
}

#[tauri::command]
pub fn remove_project(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let mut store = project_store::load();
    project_store::remove_project(&mut store, &id);
    project_store::save(&store)?;
    crate::refresh_tray_project_label(&app);
    crate::services::auto_sync::refresh(&app);
    crate::services::source_health::project_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn switch_project(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let mut store = project_store::load();
    project_store::set_active(&mut store, &id)?;
    project_store::save(&store)?;
    crate::refresh_tray_project_label(&app);
    crate::services::auto_sync::refresh(&app);
    crate::services::source_health::project_changed(&app);
    Ok(())
}
