use crate::services::{cli_manager, server_manager::ServerManager};
use tauri::State;

#[tauri::command]
pub async fn detect_cli() -> Result<Option<String>, String> {
    Ok(cli_manager::detect_cli().await)
}

#[tauri::command]
pub async fn get_cli_version(cli_path: String) -> Result<String, String> {
    cli_manager::get_version(&cli_path).await
}

#[tauri::command]
pub async fn download_cli() -> Result<String, String> {
    let (version, url) = cli_manager::check_latest_release().await?;
    let path = cli_manager::download_cli(&url).await?;

    let mut meta = cli_manager::load_meta();
    meta.version = Some(version);
    meta.path = Some(path.clone());
    meta.source = Some("github-release".to_string());
    meta.installed_at = Some(chrono::Utc::now().to_rfc3339());
    cli_manager::save_meta(&meta)?;

    Ok(path)
}

#[tauri::command]
pub async fn check_cli_update() -> Result<Option<String>, String> {
    let meta = cli_manager::load_meta();
    let local_version = meta.version.unwrap_or_default();

    let (latest_version, _url) = cli_manager::check_latest_release().await?;

    // Strip leading 'v' for comparison
    let local = local_version.trim_start_matches('v');
    let latest = latest_version.trim_start_matches('v');

    if local.is_empty() || local != latest {
        Ok(Some(latest_version))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn upgrade_cli(server: State<'_, ServerManager>) -> Result<String, String> {
    let (version, url) = cli_manager::check_latest_release().await?;
    let path = cli_manager::download_cli(&url).await?;

    let mut meta = cli_manager::load_meta();
    meta.version = Some(version);
    meta.path = Some(path.clone());
    meta.source = Some("github-release".to_string());
    meta.installed_at = Some(chrono::Utc::now().to_rfc3339());
    cli_manager::save_meta(&meta)?;

    // Restart server if it was running
    if server.is_running().await {
        server.restart(&path, None).await?;
    }

    Ok(path)
}

#[tauri::command]
pub async fn run_cli(
    cli_path: String,
    args: Vec<String>,
    working_dir: Option<String>,
) -> Result<String, String> {
    cli_manager::exec(&cli_path, &args, working_dir.as_deref()).await
}
