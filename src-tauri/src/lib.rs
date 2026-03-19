mod commands;
mod models;
mod services;

use services::server_manager::ServerManager;

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
        .setup(|_app| Ok(()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
