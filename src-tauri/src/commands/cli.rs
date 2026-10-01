use crate::services::{cli_manager, server_manager::ServerManager, update_watch};
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
    // No recorded install (e.g. after Reset app data) or the old catch-all label: infer from the path.
    let guessed = matches!(meta.source.as_deref(), None | Some("system-path"))
        .then(|| cli_manager::guess_source(&cli_path));
    let source_changed = guessed
        .as_deref()
        .is_some_and(|g| meta.source.as_deref() != Some(g));
    if meta.version.is_none() || version_changed || path_changed || source_changed {
        meta.version = Some(version.clone());
        meta.binary_modified_ms = cli_manager::binary_modified_ms(&cli_path);
        meta.path = Some(cli_path);
        if let Some(source) = guessed {
            meta.source = Some(source);
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
    let mut meta = cli_manager::load_meta();
    cli_manager::refresh_cached_version(&mut meta).await?;
    let local_version = meta.version.unwrap_or_default();

    let (latest_version, _url) = cli_manager::check_latest_release().await?;

    if local_version.is_empty() || update_watch::is_newer(&latest_version, &local_version) {
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
        let (project_dir, is_project_mode) =
            crate::services::project_store::active_project_mode(&store);
        server
            .restart(&path, project_dir.as_deref(), is_project_mode)
            .await?;
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

#[tauri::command]
pub async fn get_global_config_dir(cli_path: String) -> Result<String, String> {
    cli_manager::get_global_config_dir(&cli_path).await
}

#[tauri::command]
pub async fn cli_terminal_access(cli_path: String) -> Result<cli_manager::TerminalAccess, String> {
    Ok(cli_manager::terminal_access(&cli_path).await)
}

#[tauri::command]
pub async fn link_cli_for_terminal(
    cli_path: String,
) -> Result<cli_manager::TerminalAccess, String> {
    cli_manager::link_for_terminal(&cli_path)?;
    Ok(cli_manager::terminal_access(&cli_path).await)
}

#[tauri::command]
pub async fn detect_install_platform() -> Result<cli_manager::InstallPlatform, String> {
    Ok(cli_manager::detect_install_platform().await)
}

#[tauri::command]
pub async fn install_cli(
    app: tauri::AppHandle,
    method: String,
) -> Result<cli_manager::InstallResult, String> {
    cli_manager::install_cli(app, &method).await
}

#[tauri::command]
pub fn cancel_cli_install() -> Result<bool, String> {
    cli_manager::cancel_install()
}
