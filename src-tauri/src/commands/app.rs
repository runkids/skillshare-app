use crate::models::app_state::{AppInfo, OnboardingStatus};
use crate::services::{cli_manager, project_store, server_manager::ServerManager};
use tauri::State;

#[tauri::command]
pub async fn get_app_state(server: State<'_, ServerManager>) -> Result<AppInfo, String> {
    let meta = cli_manager::load_meta();
    let store = project_store::load();
    let running = server.is_running().await;
    let port = if running {
        Some(server.get_port().await)
    } else {
        None
    };

    Ok(AppInfo {
        cli_version: meta.version.clone(),
        cli_source: meta.source.clone(),
        server_running: running,
        server_port: port,
        onboarding: OnboardingStatus {
            completed: meta.version.is_some() && !store.projects.is_empty(),
            cli_ready: meta.version.is_some(),
            first_project_created: !store.projects.is_empty(),
            first_sync_done: running,
        },
    })
}

#[tauri::command]
pub fn get_onboarding_status() -> Result<OnboardingStatus, String> {
    let meta = cli_manager::load_meta();
    let store = project_store::load();

    Ok(OnboardingStatus {
        completed: meta.version.is_some() && !store.projects.is_empty(),
        cli_ready: meta.version.is_some(),
        first_project_created: !store.projects.is_empty(),
        first_sync_done: false,
    })
}
