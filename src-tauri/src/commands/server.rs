use crate::services::{project_store, server_manager::ServerManager};
use tauri::State;

#[tauri::command]
pub async fn start_server(
    server: State<'_, ServerManager>,
    cli_path: String,
    project_dir: Option<String>,
) -> Result<u16, String> {
    let store = project_store::load();
    let (_, is_project_mode) = project_store::active_project_mode(&store);
    server
        .start(&cli_path, project_dir.as_deref(), is_project_mode)
        .await
}

#[tauri::command]
pub async fn stop_server(server: State<'_, ServerManager>) -> Result<(), String> {
    server.stop().await
}

#[tauri::command]
pub async fn server_health_check(server: State<'_, ServerManager>) -> Result<bool, String> {
    Ok(server.is_running().await)
}

#[tauri::command]
pub async fn get_server_port(server: State<'_, ServerManager>) -> Result<u16, String> {
    Ok(server.get_port().await)
}
