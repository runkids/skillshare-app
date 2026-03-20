mod commands;
mod models;
mod services;

use services::server_manager::ServerManager;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;
use tauri_plugin_notification::NotificationExt;

/// Global flag: when true, window close actually quits (instead of hiding to tray).
static APP_QUITTING: AtomicBool = AtomicBool::new(false);

#[allow(clippy::expect_used)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(ServerManager::new())
        .invoke_handler(tauri::generate_handler![
            // CLI commands
            commands::cli::detect_cli,
            commands::cli::get_cli_version,
            commands::cli::download_cli,
            commands::cli::check_cli_update,
            commands::cli::upgrade_cli,
            commands::cli::run_cli,
            // Project commands
            commands::project::list_projects,
            commands::project::get_active_project,
            commands::project::add_project,
            commands::project::remove_project,
            commands::project::switch_project,
            // Server commands
            commands::server::start_server,
            commands::server::stop_server,
            commands::server::restart_server,
            commands::server::server_health_check,
            commands::server::get_server_port,
            // App commands
            commands::app::get_app_state,
            commands::app::get_onboarding_status,
            commands::app::get_preferred_port,
            commands::app::set_preferred_port,
            commands::app::get_preferred_theme,
            commands::app::set_preferred_theme,
            commands::app::get_notify_sync,
            commands::app::set_notify_sync,
            commands::app::get_notify_update,
            commands::app::set_notify_update,
            commands::app::reset_all_data,
        ])
        .setup(|app| {
            setup_system_tray(app)?;

            // Background CLI update check (non-blocking)
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                check_cli_update_background(&app_handle).await;
            });

            // Auto-start Go server if onboarding is complete (non-blocking)
            let server = app.state::<ServerManager>().inner().clone();
            tauri::async_runtime::spawn(async move {
                auto_start_server(server).await;
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if !APP_QUITTING.load(Ordering::SeqCst) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ── System Tray ─────────────────────────────────────────────────────

fn setup_system_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{MenuBuilder, MenuItemBuilder};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let quick_sync = MenuItemBuilder::with_id("quick_sync", "Quick Sync").build(app)?;
    let open_app = MenuItemBuilder::with_id("open_app", "Open Skillshare App").build(app)?;

    let store = services::project_store::load();
    let project_label = store
        .active_project()
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "No active project".to_string());
    let active_project = MenuItemBuilder::with_id("active_project", &project_label)
        .enabled(false)
        .build(app)?;

    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&quick_sync)
        .separator()
        .item(&open_app)
        .item(&active_project)
        .separator()
        .item(&quit)
        .build()?;

    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "quick_sync" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    handle_quick_sync(&app).await;
                });
            }
            "open_app" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.unminimize();
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                APP_QUITTING.store(true, Ordering::SeqCst);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.unminimize();
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

async fn handle_quick_sync(app: &tauri::AppHandle) {
    let meta = services::cli_manager::load_meta();
    let cli_path = match services::cli_manager::detect_cli().await {
        Some(p) => p,
        None => {
            log::warn!("Quick Sync: CLI not found");
            return;
        }
    };

    let store = services::project_store::load();
    let working_dir = store.active_project().map(|p| p.path.as_str());
    let result =
        services::cli_manager::exec(&cli_path, &["sync".to_string()], working_dir).await;

    if let Err(e) = &result {
        log::warn!("Quick Sync failed: {e}");
    }

    if meta.notify_sync.unwrap_or(true) {
        let _ = app
            .notification()
            .builder()
            .title("Skillshare Sync Complete")
            .body(match &result {
                Ok(_) => "Sync finished successfully".to_string(),
                Err(e) => format!("Sync failed: {e}"),
            })
            .show();
    }
}

// ── Background Update Check ─────────────────────────────────────────

/// Check if a newer CLI version is available. Skips if checked within 24 hours.
/// Sends a notification if an update is found.
async fn check_cli_update_background(app: &tauri::AppHandle) {
    use chrono::Utc;

    let mut meta = services::cli_manager::load_meta();

    // Skip if we checked within the last 24 hours
    if let Some(ref last_check) = meta.last_update_check {
        if let Ok(last) = chrono::DateTime::parse_from_rfc3339(last_check) {
            let hours_since = Utc::now()
                .signed_duration_since(last)
                .num_hours();
            if hours_since < 24 {
                log::info!("CLI update check skipped — last checked {hours_since}h ago");
                return;
            }
        }
    }

    let current_version = meta.version.clone().unwrap_or_default();
    if current_version.is_empty() {
        // No CLI installed yet, nothing to compare
        return;
    }

    // Fetch latest release
    let (latest_tag, _url) = match services::cli_manager::check_latest_release().await {
        Ok(r) => r,
        Err(e) => {
            log::warn!("CLI update check failed: {e}");
            return;
        }
    };

    // Update last_update_check regardless of result
    meta.last_update_check = Some(Utc::now().to_rfc3339());
    if let Err(e) = services::cli_manager::save_meta(&meta) {
        log::warn!("Failed to save CLI meta after update check: {e}");
    }

    // Compare versions (strip leading 'v')
    let current = current_version.trim_start_matches('v');
    let latest = latest_tag.trim_start_matches('v');

    if current != latest {
        log::info!("CLI update available: {current} -> {latest}");
        if meta.notify_update.unwrap_or(true) {
            let _ = app
                .notification()
                .builder()
                .title("Skillshare Update Available")
                .body(format!("Version {latest_tag} is available"))
                .show();
        }
    } else {
        log::info!("CLI is up to date ({current})");
    }
}

// ── Auto-Start Server ───────────────────────────────────────────────

/// If onboarding is complete (CLI installed + project exists), start the
/// Go server automatically so the UI is ready when the user opens the app.
/// Runs silently — failures are logged but never block the app.
async fn auto_start_server(server: ServerManager) {
    // Check if onboarding is complete
    let meta = services::cli_manager::load_meta();
    let store = services::project_store::load();

    let cli_installed = meta.version.is_some();
    let has_project = !store.projects.is_empty();

    if !cli_installed || !has_project {
        log::info!("Auto-start skipped: onboarding not complete");
        return;
    }

    // Detect CLI binary
    let cli_path = match services::cli_manager::detect_cli().await {
        Some(p) => p,
        None => {
            log::warn!("Auto-start: CLI not found despite meta indicating installation");
            return;
        }
    };

    // Get active project path and determine mode
    let (project_dir, is_project_mode) = services::project_store::active_project_mode(&store);

    // Start the server
    match server.start(&cli_path, project_dir.as_deref(), is_project_mode).await {
        Ok(port) => log::info!("Auto-started server on port {port}"),
        Err(e) => log::warn!("Auto-start server failed: {e}"),
    }
}
