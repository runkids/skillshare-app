mod commands;
mod main_window;
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
        .plugin(services::quick_actions::plugin())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(ServerManager::new())
        .manage(services::update_watch::UpdateState::default())
        .manage(services::auto_sync::AutoSyncState::default())
        .manage(services::oplog_watch::OplogWatchState::default())
        .manage(services::quick_actions::QuickActionsState::default())
        .manage(services::source_health::SourceHealthState::default())
        .manage(services::audit::AuditState::default())
        .invoke_handler(tauri::generate_handler![
            // CLI commands
            commands::cli::detect_cli,
            commands::cli::get_cli_version,
            commands::cli::download_cli,
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
            commands::server::server_health_check,
            commands::server::get_server_port,
            // App commands
            commands::app::get_app_state,
            commands::app::get_preferred_port,
            commands::app::set_preferred_port,
            commands::app::get_notify_sync,
            commands::app::set_notify_sync,
            commands::app::get_auto_sync,
            commands::app::set_auto_sync,
            commands::app::get_available_updates,
            commands::app::open_logs_folder,
            commands::app::export_diagnostics,
            commands::app::check_updates_now,
            commands::app::get_notify_update,
            commands::app::set_notify_update,
            commands::app::reset_all_data,
            commands::quick_actions::get_quick_actions_settings,
            commands::quick_actions::set_quick_actions_settings,
            commands::quick_actions::get_quick_actions_context,
            commands::quick_actions::close_quick_actions,
            commands::quick_actions::quick_search,
            commands::quick_actions::quick_install,
            commands::quick_actions::quick_new_skill,
            commands::source_health::get_source_health,
            commands::audit::get_audit_report,
            // Activity commands
            commands::activity::get_activity,
            // Terminal commands
            commands::terminal::get_pty_env,
        ])
        .setup(|app| {
            main_window::build(app)?;
            setup_system_tray(app)?;
            services::quick_actions::setup(app.handle());
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
            services::auto_sync::refresh(app.handle());
            services::oplog_watch::refresh(app.handle());
            services::source_health::spawn_background(app.handle().clone());

            tauri::async_runtime::spawn(utils::env::load_login_shell_path());

            // Auto-start Go server if onboarding is complete (non-blocking)
            let server = app.state::<ServerManager>().inner().clone();
            server.set_app_handle(app.handle().clone());
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
    let update_all = MenuItemBuilder::with_id("update_all", "Update All (0)")
        .enabled(false)
        .build(app)?;
    app.manage(TrayUpdateItem(update_all.clone()));
    let quick_actions = MenuItemBuilder::with_id("quick_actions", "Quick Actions…").build(app)?;
    let open_app = MenuItemBuilder::with_id("open_app", "Open Skillshare App").build(app)?;

    let active_project = MenuItemBuilder::with_id("active_project", active_project_label())
        .enabled(false)
        .build(app)?;
    app.manage(TrayProjectItem(active_project.clone()));

    let check_updates = check_updates_item(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&quick_sync)
        .item(&update_all)
        .item(&quick_actions)
        .separator()
        .item(&open_app)
        .item(&active_project)
        .separator()
        .item(&check_updates)
        .item(&quit)
        .build()?;
    services::source_health::attach_tray(app, &menu)?;

    let mut tray = TrayIconBuilder::new();
    // macOS menu bar icons are monochrome templates that the system tints
    // for light and dark menu bars; other platforms keep the colour icon.
    #[cfg(target_os = "macos")]
    let template_icon =
        tauri::image::Image::from_bytes(include_bytes!("../icons/tray-template.png"))
            .inspect_err(|e| log::warn!("Failed to load tray template icon: {e}"))
            .ok();
    #[cfg(not(target_os = "macos"))]
    let template_icon: Option<tauri::image::Image<'static>> = None;
    if let Some(icon) = template_icon {
        tray = tray.icon(icon).icon_as_template(true);
    } else if let Some(icon) = app.default_window_icon() {
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
                    services::oplog_watch::refresh_badges(&app).await;
                });
            }
            "update_all" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    services::update_all::run(&app).await;
                    services::source_health::refresh(&app, false).await;
                });
            }
            "open_app" => {
                show_main_window(app);
            }
            "quick_actions" => {
                services::quick_actions::request_open(app);
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

/// The tray's resource update action, refreshed after each check.
struct TrayUpdateItem(tauri::menu::MenuItem<tauri::Wry>);

pub(crate) fn refresh_tray_update_count(app: &tauri::AppHandle, count: usize) {
    if let Some(item) = app.try_state::<TrayUpdateItem>() {
        let _ = item.0.set_text(format!("Update All ({count})"));
        let _ = item
            .0
            .set_enabled(count > 0 && !services::update_all::is_running());
    }
}

/// The tray's disabled item showing the active project, kept so its text can follow switches.
struct TrayProjectItem(tauri::menu::MenuItem<tauri::Wry>);

fn active_project_label() -> String {
    services::project_store::load()
        .active_project()
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "No active project".to_string())
}

/// Re-read the active project and update the tray label; call after the store changes.
pub(crate) fn refresh_tray_project_label(app: &tauri::AppHandle) {
    if let Some(item) = app.try_state::<TrayProjectItem>() {
        if let Err(e) = item.0.set_text(active_project_label()) {
            log::warn!("Failed to update tray project label: {e}");
        }
    }
}

/// Emitted after a tray Quick Sync succeeds so the web view can show the new state.
const SYNC_COMPLETED_EVENT: &str = "sync-completed";

/// Long enough for a slow git pull, short enough that a hung prompt doesn't block forever.
const QUICK_SYNC_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

/// Held for a whole sync so tray and auto-sync runs never overlap.
static SYNC_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn handle_quick_sync(app: &tauri::AppHandle) {
    let _running = SYNC_LOCK.lock().await;
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
    let args = ["sync".to_string(), "--json".to_string()];
    let result = tokio::time::timeout(
        QUICK_SYNC_TIMEOUT,
        services::cli_manager::exec(&cli_path, &args, working_dir),
    )
    .await
    .unwrap_or_else(|_| Err(format!("timed out after {}s", QUICK_SYNC_TIMEOUT.as_secs())));

    match &result {
        Ok(_) => {
            let _ = app.emit(SYNC_COMPLETED_EVENT, ());
        }
        Err(e) => log::warn!("Quick Sync failed: {e}"),
    }

    if meta.notify_sync.unwrap_or(true) {
        let (title, body) = quick_sync_notification(&result);
        let _ = app.notification().builder().title(title).body(body).show();
    }
}

/// The totals `skillshare sync --json` prints on success.
#[derive(serde::Deserialize)]
struct SyncJsonSummary {
    targets: usize,
    linked: usize,
    updated: usize,
    pruned: usize,
}

/// Notification title and body for a Quick Sync result (stdout of `sync --json`, or the error).
fn quick_sync_notification(result: &Result<String, String>) -> (&'static str, String) {
    const MAX_ERROR_CHARS: usize = 200;
    match result {
        Ok(stdout) => {
            let body = match serde_json::from_str::<SyncJsonSummary>(stdout) {
                Ok(s) if s.targets == 0 => "No targets to sync".to_string(),
                Ok(s) => format!(
                    "Synced to {} target{}: {} linked, {} updated, {} pruned",
                    s.targets,
                    if s.targets == 1 { "" } else { "s" },
                    s.linked,
                    s.updated,
                    s.pruned
                ),
                Err(_) => "Sync finished successfully".to_string(),
            };
            ("Skillshare Sync Complete", body)
        }
        Err(e) => {
            // With --json the CLI reports failures as {"error": "..."} on stdout.
            #[derive(serde::Deserialize)]
            struct JsonError {
                error: String,
            }
            let message = e
                .find('{')
                .and_then(|i| serde_json::from_str::<JsonError>(&e[i..]).ok())
                .map(|j| j.error)
                .unwrap_or_else(|| e.clone());
            let mut body: String = message.chars().take(MAX_ERROR_CHARS).collect();
            if message.chars().count() > MAX_ERROR_CHARS {
                body.push('…');
            }
            ("Sync failed", body)
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_json_reports_targets_and_counts() {
        let stdout = r#"{"targets":2,"linked":3,"local":1,"updated":1,"pruned":0,"details":[]}"#;
        assert_eq!(
            quick_sync_notification(&Ok(stdout.to_string())),
            (
                "Skillshare Sync Complete",
                "Synced to 2 targets: 3 linked, 1 updated, 0 pruned".to_string()
            )
        );
    }

    #[test]
    fn sync_json_with_zero_targets_says_nothing_to_sync() {
        let stdout = r#"{"targets":0,"linked":0,"local":0,"updated":0,"pruned":0}"#;
        assert_eq!(
            quick_sync_notification(&Ok(stdout.to_string())).1,
            "No targets to sync"
        );
    }

    #[test]
    fn unparseable_sync_output_falls_back_to_generic_message() {
        assert_eq!(
            quick_sync_notification(&Ok("Synced!".to_string())).1,
            "Sync finished successfully"
        );
    }

    #[test]
    fn failed_sync_shows_the_cli_json_error() {
        let err = r#"CLI exited with exit status: 1: {"error": "config not found"}"#;
        assert_eq!(
            quick_sync_notification(&Err(err.to_string())),
            ("Sync failed", "config not found".to_string())
        );
    }
}
