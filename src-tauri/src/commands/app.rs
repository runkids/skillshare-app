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

#[tauri::command]
pub fn get_preferred_port() -> u16 {
    cli_manager::load_meta().preferred_port.unwrap_or(19420)
}

#[tauri::command]
pub fn set_preferred_port(port: u16) -> Result<(), String> {
    if !(1024..=65535).contains(&port) {
        return Err("Port must be between 1024 and 65535".to_string());
    }
    let mut meta = cli_manager::load_meta();
    meta.preferred_port = Some(port);
    cli_manager::save_meta(&meta)
}

#[tauri::command]
pub fn get_preferred_theme() -> String {
    cli_manager::load_meta()
        .preferred_theme
        .unwrap_or_else(|| "system".to_string())
}

#[tauri::command]
pub fn set_preferred_theme(theme: String) -> Result<(), String> {
    if !["light", "dark", "system"].contains(&theme.as_str()) {
        return Err("Theme must be light, dark, or system".to_string());
    }
    let mut meta = cli_manager::load_meta();
    meta.preferred_theme = Some(theme);
    cli_manager::save_meta(&meta)
}

#[tauri::command]
pub fn get_notify_sync() -> bool {
    cli_manager::load_meta().notify_sync.unwrap_or(true)
}

#[tauri::command]
pub fn set_notify_sync(enabled: bool) -> Result<(), String> {
    let mut meta = cli_manager::load_meta();
    meta.notify_sync = Some(enabled);
    cli_manager::save_meta(&meta)
}

#[tauri::command]
pub fn get_notify_update() -> bool {
    cli_manager::load_meta().notify_update.unwrap_or(true)
}

#[tauri::command]
pub fn set_notify_update(enabled: bool) -> Result<(), String> {
    let mut meta = cli_manager::load_meta();
    meta.notify_update = Some(enabled);
    cli_manager::save_meta(&meta)
}

#[tauri::command]
pub async fn reset_all_data(server: State<'_, ServerManager>) -> Result<(), String> {
    // Stop server if running
    server.stop().await?;

    // Reset CLI meta to default
    let meta = crate::models::app_state::CliMeta::default();
    cli_manager::save_meta(&meta)?;

    // Reset project store to default
    let store = crate::models::project::ProjectStore::default();
    project_store::save(&store)?;

    Ok(())
}
