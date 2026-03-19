use crate::services::server_manager::ServerManager;
use tauri::State;

#[tauri::command]
pub async fn start_server(
    server: State<'_, ServerManager>,
    cli_path: String,
    project_dir: Option<String>,
) -> Result<u16, String> {
    server
        .start(&cli_path, project_dir.as_deref())
        .await
}

#[tauri::command]
pub async fn stop_server(server: State<'_, ServerManager>) -> Result<(), String> {
    server.stop().await
}

#[tauri::command]
pub async fn restart_server(
    server: State<'_, ServerManager>,
    cli_path: String,
    project_dir: Option<String>,
) -> Result<u16, String> {
    server
        .restart(&cli_path, project_dir.as_deref())
        .await
}

#[tauri::command]
pub async fn server_health_check(server: State<'_, ServerManager>) -> Result<bool, String> {
    Ok(server.is_running().await)
}

#[tauri::command]
pub async fn get_server_port(server: State<'_, ServerManager>) -> Result<u16, String> {
    Ok(server.get_port().await)
}
