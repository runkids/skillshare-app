use crate::services::{cli_manager, server_manager::ServerManager};
use tauri::State;

#[tauri::command]
pub async fn detect_cli() -> Result<Option<String>, String> {
    Ok(cli_manager::detect_cli().await)
}

#[tauri::command]
pub async fn get_cli_version(cli_path: String) -> Result<String, String> {
    let version = cli_manager::get_version(&cli_path).await?;

    // Always persist current version so app state stays in sync after upgrades
    let mut meta = cli_manager::load_meta();
    let version_changed = meta.version.as_deref() != Some(&version);
    let path_changed = meta.path.as_deref() != Some(&cli_path);
    if meta.version.is_none() || version_changed || path_changed {
        meta.version = Some(version.clone());
        meta.path = Some(cli_path);
        if meta.source.is_none() {
            meta.source = Some("system-path".to_string());
        }
        cli_manager::save_meta(&meta)?;
    }

    Ok(version)
}

#[tauri::command]
pub async fn download_cli() -> Result<String, String> {
    let (version, url) = cli_manager::check_latest_release().await?;
    let path = cli_manager::download_cli(&url).await?;
    cli_manager::save_release_meta(version, &path)?;
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
    cli_manager::save_release_meta(version, &path)?;

    // Restart server if it was running
    if server.is_running().await {
        let store = crate::services::project_store::load();
        let (project_dir, is_project_mode) = crate::services::project_store::active_project_mode(&store);
        server.restart(&path, project_dir.as_deref(), is_project_mode).await?;
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
