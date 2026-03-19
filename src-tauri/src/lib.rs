mod commands;
mod models;
mod services;

use services::server_manager::ServerManager;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;
use tauri_plugin_notification::NotificationExt;

/// Global flag: when true, window close actually quits (instead of hiding to tray).
static APP_QUITTING: AtomicBool = AtomicBool::new(false);

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
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
        ])
        .setup(|app| {
            setup_system_tray(app)?;
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

    // Build menu items
    let quick_sync = MenuItemBuilder::with_id("quick_sync", "Quick Sync").build(app)?;
    let open_app = MenuItemBuilder::with_id("open_app", "Open Skillshare App").build(app)?;

    // Active project label (disabled, display-only)
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

/// Run `skillshare sync` for the active project directory.
async fn handle_quick_sync(app: &tauri::AppHandle) {
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

    // Send notification with result
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
