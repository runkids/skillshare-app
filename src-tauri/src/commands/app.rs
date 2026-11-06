use crate::models::app_state::{AppInfo, OnboardingStatus};
use crate::models::project::ProjectType;
use crate::services::{
    auto_sync, cli_manager, project_store, server_manager::ServerManager, update_watch,
};
use tauri::State;

/// The stored Global path may be stale until startup repairs it. Check the CLI's
/// actual config file, including a SKILLSHARE_CONFIG override with another name.
fn global_config_exists(
    store: &crate::models::project::ProjectStore,
    config_path: &std::path::Path,
) -> bool {
    store
        .projects
        .iter()
        .any(|p| p.project_type == ProjectType::Global)
        && config_path.is_file()
}

#[tauri::command]
pub async fn get_app_state(server: State<'_, ServerManager>) -> Result<AppInfo, String> {
    let mut meta = cli_manager::load_meta();
    if let Err(e) = cli_manager::refresh_cached_version(&mut meta).await {
        log::warn!("Could not refresh cached CLI version: {e}");
    }
    let store = project_store::load();
    let running = server.is_running().await;
    let port = if running {
        Some(server.get_port().await)
    } else {
        None
    };

    let cli_ready = meta.version.is_some();
    let config_exists = crate::utils::paths::global_config_path()
        .is_ok_and(|path| global_config_exists(&store, &path));

    Ok(AppInfo {
        cli_version: meta.version.clone(),
        cli_source: meta.source.clone(),
        server_running: running,
        server_port: port,
        onboarding: OnboardingStatus {
            completed: cli_ready && config_exists,
            cli_ready,
            first_project_created: config_exists,
            first_sync_done: running,
        },
    })
}

/// Run the tray's Quick Sync from the title bar, which shows the summary or error itself.
#[tauri::command]
pub async fn quick_sync(app: tauri::AppHandle) -> Result<String, String> {
    crate::handle_quick_sync(&app, false).await
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
pub fn get_auto_sync() -> bool {
    cli_manager::load_meta().auto_sync.unwrap_or(false)
}

#[tauri::command]
pub fn set_auto_sync(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let mut meta = cli_manager::load_meta();
    meta.auto_sync = Some(enabled);
    cli_manager::save_meta(&meta)?;
    auto_sync::refresh(&app);
    crate::tray::refresh_auto_sync(&app);
    Ok(())
}

#[tauri::command]
pub async fn get_available_updates(
    state: State<'_, update_watch::UpdateState>,
) -> Result<update_watch::AvailableUpdates, String> {
    Ok(state.0.lock().await.clone())
}

#[tauri::command]
pub async fn check_versions_now(
    app: tauri::AppHandle,
) -> Result<update_watch::AvailableUpdates, String> {
    Ok(update_watch::check_versions(&app).await)
}

/// Re-run the skill, agent and plugin checks; slow, since each fetches its sources.
#[tauri::command]
pub async fn check_resources_now(
    app: tauri::AppHandle,
    state: State<'_, update_watch::UpdateState>,
) -> Result<update_watch::AvailableUpdates, String> {
    update_watch::refresh_skills(&app).await;
    Ok(state.0.lock().await.clone())
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

/// Open the folder holding the app log and the CLI server's output.
#[tauri::command]
pub fn open_logs_folder(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let dir = crate::utils::paths::logs_dir();
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| format!("Could not open {}: {e}", dir.display()))
}

/// Write a diagnostics report to Downloads and reveal it, so users can attach it to a bug report.
#[tauri::command]
pub async fn export_diagnostics(
    app: tauri::AppHandle,
    server: State<'_, ServerManager>,
) -> Result<String, String> {
    use crate::services::diagnostics::{build_report, redact_home, DiagnosticsInput};
    use tauri_plugin_opener::OpenerExt;

    crate::utils::env::load_login_shell_path().await;
    let logs = crate::utils::paths::logs_dir();
    let input = DiagnosticsInput {
        app_version: app.package_info().version.to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        meta: cli_manager::load_meta(),
        server_running: server.is_running().await,
        server_port: server.get_port().await,
        login_shell_path: crate::utils::env::login_shell_path(),
        app_log: std::fs::read_to_string(logs.join("app.log")).unwrap_or_default(),
        server_log: std::fs::read_to_string(logs.join("server.log")).unwrap_or_default(),
    };
    let home = dirs::home_dir().map(|h| h.to_string_lossy().into_owned());
    let report = build_report(&input, home.as_deref());

    let dir = dirs::download_dir()
        .or_else(dirs::home_dir)
        .ok_or("Could not find the Downloads folder")?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let path = dir.join(format!("skillshare-app-diagnostics-{stamp}.txt"));
    std::fs::write(&path, report)
        .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    // The report is saved either way; a missing file manager must not read as a failed export.
    if let Err(e) = app.opener().reveal_item_in_dir(&path) {
        log::warn!("Could not reveal {}: {e}", path.display());
    }

    let shown = path.to_string_lossy();
    Ok(match home.as_deref() {
        Some(home) => redact_home(&shown, home),
        None => shown.into_owned(),
    })
}

#[tauri::command]
pub async fn reset_all_data(
    app: tauri::AppHandle,
    server: State<'_, ServerManager>,
) -> Result<(), String> {
    // Stop server if running
    server.stop().await?;

    // Reset CLI meta to default
    let meta = crate::models::app_state::CliMeta::default();
    cli_manager::save_meta(&meta)?;

    // Reset project store to default
    let store = crate::models::project::ProjectStore::default();
    project_store::save(&store)?;
    crate::refresh_tray_project_label(&app);
    crate::tray::refresh_auto_sync(&app);
    auto_sync::refresh(&app);
    crate::services::oplog_watch::refresh(&app);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::global_config_exists;
    use crate::models::project::{Project, ProjectStore, ProjectType};

    #[test]
    fn onboarding_checks_the_config_file_even_with_a_stale_global_path() {
        let root = std::env::temp_dir().join(format!("ss-onboarding-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap_or_else(|err| panic!("create config fixture: {err}"));
        let config = root.join("custom.yaml");
        std::fs::write(&config, "sources:\n  skills: /repos/custom-skills\n")
            .unwrap_or_else(|err| panic!("write config fixture: {err}"));
        let mut store = ProjectStore {
            projects: vec![Project {
                id: "global".into(),
                name: "Global".into(),
                path: "/repos".into(),
                project_type: ProjectType::Global,
                added_at: String::new(),
            }],
            active_project_id: Some("global".into()),
        };

        assert!(global_config_exists(&store, &config));
        assert!(!global_config_exists(&store, &root.join("missing.yaml")));
        assert!(!global_config_exists(&store, &root));

        // A config file alone must not skip the first-time Global project setup.
        store.projects[0].project_type = ProjectType::Project;
        assert!(!global_config_exists(&store, &config));
        assert!(!global_config_exists(&ProjectStore::default(), &config));
        std::fs::remove_dir_all(root).ok();
    }
}
