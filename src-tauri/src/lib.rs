mod commands;
mod models;
mod services;
mod utils;

use services::server_manager::ServerManager;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

/// Global flag: when true, window close actually quits (instead of hiding to tray).
static APP_QUITTING: AtomicBool = AtomicBool::new(false);

#[allow(clippy::expect_used)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Folder {
                        path: utils::paths::logs_dir(),
                        file_name: Some("app".into()),
                    }),
                ])
                .max_file_size(1_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_pty::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(ServerManager::new())
        .manage(services::update_watch::UpdateState::default())
        .invoke_handler(tauri::generate_handler![
            // CLI commands
            commands::cli::detect_cli,
            commands::cli::get_cli_version,
            commands::cli::download_cli,
            commands::cli::check_cli_update,
            commands::cli::upgrade_cli,
            commands::cli::run_cli,
            commands::cli::get_global_config_dir,
            commands::cli::detect_install_platform,
            commands::cli::cli_terminal_access,
            commands::cli::link_cli_for_terminal,
            commands::cli::install_cli,
            commands::cli::cancel_cli_install,
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
            commands::app::get_available_updates,
            commands::app::open_logs_folder,
            commands::app::check_updates_now,
            commands::app::get_notify_update,
            commands::app::set_notify_update,
            commands::app::reset_all_data,
            // Terminal commands
            commands::terminal::get_pty_env,
        ])
        .setup(|app| {
            setup_system_tray(app)?;
            #[cfg(target_os = "macos")]
            setup_app_menu(app)?;
            // Tray and app menu events both reach this global listener.
            app.on_menu_event(|app, event| {
                if event.id().as_ref() == CHECK_UPDATES_MENU_ID {
                    show_main_window(app);
                    let _ = app.emit(services::update_watch::CHECK_REQUESTED_EVENT, ());
                }
            });

            services::update_watch::spawn_background(app.handle().clone());

            tauri::async_runtime::spawn(utils::env::load_login_shell_path());

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
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = &event {
                show_main_window(app);
            }
            if let tauri::RunEvent::ExitRequested { .. } = event {
                // Safety net: stop the CLI server on any exit path
                let server = app.state::<ServerManager>().inner().clone();
                tauri::async_runtime::block_on(async move {
                    let _ = server.stop().await;
                });
            }
        });
}

// ── System Tray ─────────────────────────────────────────────────────

const CHECK_UPDATES_MENU_ID: &str = "check_updates";

fn check_updates_item(app: &tauri::App) -> tauri::Result<tauri::menu::MenuItem<tauri::Wry>> {
    tauri::menu::MenuItemBuilder::with_id(CHECK_UPDATES_MENU_ID, "Check for Updates…").build(app)
}

/// The default macOS menu plus "Check for Updates…" under About, where Mac users look for it.
#[cfg(target_os = "macos")]
fn setup_app_menu(app: &tauri::App) -> tauri::Result<()> {
    let menu = tauri::menu::Menu::default(app.handle())?;
    let items = menu.items()?;
    if let Some(app_submenu) = items.first().and_then(|item| item.as_submenu()) {
        app_submenu.insert(&check_updates_item(app)?, 1)?;
    }
    app.set_menu(menu)?;
    Ok(())
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

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

    let check_updates = check_updates_item(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&quick_sync)
        .separator()
        .item(&open_app)
        .item(&active_project)
        .separator()
        .item(&check_updates)
        .item(&quit)
        .build()?;

    let mut tray = TrayIconBuilder::new();
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    let _tray = tray
        .tooltip("Skillshare App")
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
                show_main_window(app);
            }
            "quit" => {
                APP_QUITTING.store(true, Ordering::SeqCst);
                // Stop the CLI server before exiting to avoid orphaned processes
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let server = app_handle.state::<ServerManager>();
                    let _ = server.stop().await;
                    app_handle.exit(0);
                });
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
                show_main_window(tray.app_handle());
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
    let result = services::cli_manager::exec(&cli_path, &["sync".to_string()], working_dir).await;

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

// ── Global Project Path Sync ─────────────────────────────────────────

/// Keep the Global project path in sync with the CLI's actual config directory.
/// Returns the (potentially updated) store so the caller doesn't need to re-read.
async fn sync_global_project_path(
    cli_path: &str,
    mut store: models::project::ProjectStore,
) -> models::project::ProjectStore {
    use models::project::ProjectType;

    let global_idx = store
        .projects
        .iter()
        .position(|p| p.project_type == ProjectType::Global);
    let global_idx = match global_idx {
        Some(i) => i,
        None => return store,
    };

    let config_dir = match services::cli_manager::get_global_config_dir(cli_path).await {
        Ok(dir) if !dir.is_empty() => dir,
        Ok(_) => return store,
        Err(e) => {
            log::warn!("Failed to get global config dir for path sync: {e}");
            return store;
        }
    };

    if config_dir == store.projects[global_idx].path {
        return store;
    }

    log::info!(
        "Syncing Global project path: {} -> {}",
        store.projects[global_idx].path,
        config_dir
    );

    store.projects[global_idx].path = config_dir;

    if let Err(e) = services::project_store::save(&store) {
        log::warn!("Failed to save synced Global project path: {e}");
    }

    store
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

    // Sync Global project path from CLI status (config dir may change across versions)
    let store = sync_global_project_path(&cli_path, store).await;

    // Get active project path and determine mode
    let (project_dir, is_project_mode) = services::project_store::active_project_mode(&store);

    // Start the server
    match server
        .start(&cli_path, project_dir.as_deref(), is_project_mode)
        .await
    {
        Ok(port) => log::info!("Auto-started server on port {port}"),
        Err(e) => log::warn!("Auto-start server failed: {e}"),
    }
}
