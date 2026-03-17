// SpecForge - Tauri Application
// Migrated from Electron version

// Allow shadowing of model modules by command modules - commands are internal use only
// while models need to be publicly exported for the MCP binary
#![allow(hidden_glob_reexports)]

mod commands;
pub mod services; // Local services with Tauri dependencies
pub mod repositories; // Local repositories with Tauri dependencies

// Local models (Tauri-specific, not shared with MCP binary)
#[path = "models/mod.rs"]
pub mod local_models;

// Re-export from specforge-lib
pub use specforge_lib::models;
pub use specforge_lib::utils;

// Re-export models for use in commands
pub use specforge_lib::models::*;

use std::sync::Arc;

use commands::workflow::WorkflowExecutionState;
use commands::{
    config, file_watcher, git, mcp, notification,
    schema_commands, settings, shortcuts, spec_commands, workflow,
    workflow_commands,
};
use services::{FileWatcherManager, SpecforgeWatcher};
use tauri::{Emitter, Manager};
use utils::database::{Database, get_database_path};

/// Database state wrapper for Tauri
pub struct DatabaseState(pub Arc<Database>);

/// Database watcher for MCP-triggered changes
pub struct DatabaseWatcher {
    inner: std::sync::Mutex<Option<notify::RecommendedWatcher>>,
}

impl DatabaseWatcher {
    pub fn new() -> Self {
        Self {
            inner: std::sync::Mutex::new(None),
        }
    }

    pub fn start_watching(
        &self,
        handle: &tauri::AppHandle,
        db_path: std::path::PathBuf,
    ) -> Result<(), String> {
        use notify::{Watcher, RecursiveMode, Event, EventKind};

        let handle = handle.clone();
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Modify(_)) {
                    let _ = handle.emit("database-changed", ());
                }
            }
        })
        .map_err(|e| format!("Failed to create watcher: {}", e))?;

        // Watch the WAL file for changes (SQLite WAL mode)
        let wal_path = db_path.with_extension("db-wal");
        if wal_path.exists() {
            watcher
                .watch(&wal_path, RecursiveMode::NonRecursive)
                .map_err(|e| format!("Failed to watch WAL: {}", e))?;
        }

        // Also watch the main db file
        watcher
            .watch(&db_path, RecursiveMode::NonRecursive)
            .map_err(|e| format!("Failed to watch db: {}", e))?;

        let mut inner = self.inner.lock().map_err(|e| format!("Lock error: {}", e))?;
        *inner = Some(watcher);
        Ok(())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load environment variables from .env file (for OAuth credentials)
    // Try project root first, then current dir
    let _ = dotenvy::from_filename("../.env").or_else(|_| dotenvy::dotenv());

    // Initialize SQLite database
    let db = match initialize_database() {
        Ok(db) => Arc::new(db),
        Err(e) => {
            eprintln!("[SpecForge] Failed to initialize database: {}", e);
            // Database is required - all commands use Repository layer
            // If initialization fails, we need to handle this gracefully
            // Try one more time with a fresh database path
            let db_path = get_database_path().unwrap_or_else(|_| {
                dirs::data_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join("specforge.db")
            });
            eprintln!("[SpecForge] Attempting recovery at: {:?}", db_path);
            Arc::new(
                Database::new(db_path)
                    .expect("Failed to create database - application cannot start")
            )
        }
    };

    tauri::Builder::default()
        .manage(DatabaseState(db.clone()))
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        // Plugins
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build()) // Keyboard shortcuts enhancement
        // App state
        .manage(WorkflowExecutionState::default())
        .manage(FileWatcherManager::new())
        .manage(DatabaseWatcher::new())
        .manage(SpecforgeWatcher::new())
        // Register commands
        .invoke_handler(tauri::generate_handler![
            // Global config commands
            config::get_config,
            config::update_config,
            // Settings commands (US7)
            settings::load_settings,
            settings::save_settings,
            // Notification settings commands
            settings::load_notification_settings,
            settings::save_notification_settings,
            settings::load_projects,
            settings::save_projects,
            settings::load_workflows,
            settings::save_workflows,
            settings::load_store_data,
            // Store path management
            settings::get_store_path,
            settings::set_store_path,
            settings::reset_store_path,
            settings::open_store_location,
            // Template preferences commands
            settings::get_template_preferences,
            settings::toggle_template_favorite,
            settings::add_template_favorite,
            settings::remove_template_favorite,
            settings::record_template_usage,
            settings::clear_recently_used_templates,
            settings::toggle_template_category_collapse,
            settings::expand_all_template_categories,
            settings::collapse_template_categories,
            settings::set_template_preferred_view,
            // Workflow commands (US4)
            // Note: load_workflows is provided by settings module
            workflow::save_workflow,
            workflow::delete_workflow,
            workflow::execute_workflow,
            workflow::cancel_execution,
            workflow::continue_execution,
            workflow::get_running_executions,
            workflow::get_workflow_output,
            workflow::restore_running_executions,
            workflow::kill_process,
            workflow::get_available_workflows,
            // Feature 013: Cycle detection for workflow triggers
            workflow::detect_workflow_cycle,
            // Feature 013: Child execution query
            workflow::get_child_executions,
            // Execution history commands
            workflow::load_execution_history,
            workflow::load_all_execution_history,
            workflow::save_execution_history,
            workflow::delete_execution_history,
            workflow::clear_workflow_execution_history,
            workflow::update_execution_history_settings,
            // Git commands (009-git-integration)
            git::get_git_status,
            git::stage_files,
            git::unstage_files,
            git::create_commit,
            git::get_branches,
            git::create_branch,
            git::switch_branch,
            git::delete_branch,
            git::get_commit_history,
            git::git_push,
            git::git_pull,
            git::list_stashes,
            git::create_stash,
            git::apply_stash,
            git::drop_stash,
            // Git remote management
            git::get_remotes,
            git::add_remote,
            git::remove_remote,
            // Git discard changes
            git::discard_changes,
            git::clean_untracked,
            // Git fetch and rebase
            git::git_fetch,
            git::git_rebase,
            git::git_rebase_abort,
            git::git_rebase_continue,
            // Git authentication
            git::get_git_auth_status,
            git::test_remote_connection,
            // Git diff viewer (010-git-diff-viewer)
            git::get_file_diff,
            // Keyboard shortcuts commands
            shortcuts::load_keyboard_shortcuts,
            shortcuts::save_keyboard_shortcuts,
            shortcuts::register_global_toggle_shortcut,
            shortcuts::unregister_global_shortcuts,
            shortcuts::toggle_window_visibility,
            shortcuts::get_registered_shortcuts,
            shortcuts::is_shortcut_registered,
            // File watcher commands (package.json monitoring)
            file_watcher::watch_project,
            file_watcher::unwatch_project,
            file_watcher::unwatch_all_projects,
            file_watcher::get_watched_projects,
            // Specforge directory watcher (.specforge/specs/ and .specforge/schemas/)
            file_watcher::watch_specforge,
            file_watcher::unwatch_specforge,
            // MCP Server Integration
            mcp::get_mcp_server_info,
            mcp::test_mcp_connection,
            mcp::get_mcp_tools,
            mcp::get_mcp_config,
            mcp::save_mcp_config,
            mcp::update_mcp_config,
            mcp::get_mcp_tools_with_permissions,
            mcp::get_mcp_logs,
            mcp::clear_mcp_logs,
            // Notification Center
            notification::get_notifications,
            notification::get_unread_notification_count,
            notification::mark_notification_read,
            notification::mark_all_notifications_read,
            notification::delete_notification,
            notification::cleanup_old_notifications,
            notification::clear_all_notifications,
            // Spec commands (spec-driven development)
            spec_commands::create_spec,
            spec_commands::list_specs,
            spec_commands::get_spec,
            spec_commands::update_spec,
            spec_commands::delete_spec,
            spec_commands::init_specforge_project,
            spec_commands::sync_specs,
            spec_commands::check_specforge_exists,
            // Schema commands (spec-driven development)
            schema_commands::list_schemas,
            schema_commands::get_schema,
            // Workflow phase commands (spec-driven development)
            workflow_commands::advance_spec,
            workflow_commands::review_spec,
            workflow_commands::get_workflow_status,
            workflow_commands::get_gate_status,
            workflow_commands::get_agent_runs,
        ])
        // Setup hook
        .setup(|app| {
            let handle = app.handle().clone();

            // Start database watcher for MCP-triggered changes
            if let Ok(db_path) = get_database_path() {
                let db_watcher = app.handle().state::<DatabaseWatcher>();
                if let Err(e) = db_watcher.start_watching(&handle, db_path) {
                    log::warn!("[setup] Failed to start database watcher: {}", e);
                }
            }

            // Cleanup old notifications on startup
            services::notification::cleanup_old_notifications(app.handle());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Initialize SQLite database with migrations
fn initialize_database() -> Result<Database, String> {
    // Get database path
    let db_path = get_database_path()?;

    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create database directory: {}", e))?;
    }

    println!("[SpecForge] Initializing database at: {:?}", db_path);

    // Create or open database (this also runs migrations internally)
    let db = Database::new(db_path.clone())?;
    println!("[SpecForge] Database schema migrations complete");

    Ok(db)
}
