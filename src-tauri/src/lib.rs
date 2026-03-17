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

use commands::script::ScriptExecutionState;
use commands::workflow::WorkflowExecutionState;
use commands::ai_cli::CLIExecutorState;
use commands::{
    ai, ai_assistant, ai_cli, apk, audit, deploy, file_watcher, git, incoming_webhook, ipa, mcp, monorepo, notification, project, script, security,
    settings, shortcuts, snapshot, step_template, toolchain, version, webhook, workflow, worktree,
};
use services::{DatabaseWatcher, FileWatcherManager, IncomingWebhookManager, LockfileWatcherManager};
use commands::snapshot::LockfileWatcherState;
use services::ai_assistant::StreamManager;
use tauri::Manager;
use utils::database::{Database, get_database_path};

/// Database state wrapper for Tauri
pub struct DatabaseState(pub Arc<Database>);

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
        .plugin(tauri_plugin_pty::init()) // Feature 008: PTY for interactive terminals
        .plugin(tauri_plugin_notification::init()) // Feature 015: Webhook desktop notifications
        .plugin(tauri_plugin_global_shortcut::Builder::new().build()) // Keyboard shortcuts enhancement
        .plugin(tauri_plugin_oauth::init()) // OAuth for deploy feature
        // App state
        .manage(ScriptExecutionState::default())
        .manage(WorkflowExecutionState::default())
        .manage(IncomingWebhookManager::new())
        .manage(FileWatcherManager::new())
        .manage(DatabaseWatcher::new())
        .manage(LockfileWatcherState(Arc::new(LockfileWatcherManager::new())))
        .manage(CLIExecutorState::new())
        .manage(StreamManager::new())
        // Register commands
        .invoke_handler(tauri::generate_handler![
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
            // Project commands (US2)
            project::scan_project,
            project::save_project,
            project::remove_project,
            project::refresh_project,
            project::get_workspace_packages,
            project::trash_node_modules,
            // Script commands (US3)
            script::execute_script,
            script::execute_command,
            script::cancel_script,
            script::kill_all_node_processes,
            script::kill_ports,
            script::check_ports,
            script::get_running_scripts,
            // Feature 007: Terminal session reconnect
            script::get_script_output,
            // Feature 008: stdin interaction
            script::write_to_script,
            // Feature 008: PTY environment variables
            script::get_pty_env,
            // Volta-wrapped command for PTY execution
            script::get_volta_wrapped_command,
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
            // Worktree commands (US5)
            worktree::is_git_repo,
            worktree::list_branches,
            worktree::list_worktrees,
            worktree::add_worktree,
            worktree::remove_worktree,
            worktree::get_merged_worktrees,
            worktree::get_behind_commits,
            worktree::sync_worktree,
            // Enhanced worktree commands (001-worktree-enhancements)
            worktree::get_worktree_status,
            worktree::get_all_worktree_statuses,
            worktree::execute_script_in_worktree,
            // Editor integration commands (001-worktree-enhancements US3)
            worktree::open_in_editor,
            worktree::get_available_editors,
            // Worktree template commands (001-worktree-enhancements US5)
            worktree::save_worktree_template,
            worktree::delete_worktree_template,
            worktree::list_worktree_templates,
            worktree::get_default_worktree_templates,
            worktree::get_next_feature_number,
            worktree::create_worktree_from_template,
            // Terminal commands
            worktree::get_available_terminals,
            worktree::set_preferred_terminal,
            worktree::open_in_terminal,
            // Gitignore management commands
            worktree::check_gitignore_has_worktrees,
            worktree::add_worktrees_to_gitignore,
            // IPA commands (US6)
            ipa::check_has_ipa_files,
            ipa::scan_project_ipa,
            // APK commands
            apk::check_has_apk_files,
            apk::scan_project_apk,
            // Security commands (005-package-security-audit)
            security::detect_package_manager,
            security::check_cli_installed,
            security::run_security_audit,
            security::get_security_scan,
            security::get_all_security_scans,
            security::save_security_scan,
            security::snooze_scan_reminder,
            security::dismiss_scan_reminder,
            // Audit commands (security audit log)
            audit::get_audit_events,
            audit::get_audit_stats,
            audit::export_audit_events,
            // Version management commands (006-node-package-manager)
            version::get_version_requirement,
            version::get_system_environment,
            version::check_version_compatibility,
            version::get_wrapped_command,
            // Monorepo commands (008-monorepo-support)
            monorepo::detect_monorepo_tools,
            monorepo::get_tool_version,
            monorepo::get_nx_targets,
            monorepo::run_nx_command,
            monorepo::get_turbo_pipelines,
            monorepo::run_turbo_command,
            monorepo::get_turbo_cache_status,
            monorepo::clear_turbo_cache,
            monorepo::get_nx_cache_status,
            monorepo::clear_nx_cache,
            monorepo::get_dependency_graph,
            monorepo::run_batch_scripts,
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
            // Step template commands (011-workflow-step-templates)
            step_template::load_custom_step_templates,
            step_template::save_custom_step_template,
            step_template::delete_custom_step_template,
            // Webhook commands (012-workflow-webhook-support)
            webhook::test_webhook,
            webhook::validate_template_variables,
            // Incoming webhook commands (012-workflow-webhook-support)
            // Per-workflow server architecture: each workflow has its own HTTP server
            incoming_webhook::generate_incoming_webhook_token,
            incoming_webhook::get_incoming_webhook_status,
            incoming_webhook::create_incoming_webhook_config,
            incoming_webhook::regenerate_incoming_webhook_token,
            incoming_webhook::check_port_available,
            incoming_webhook::generate_webhook_secret,
            // Keyboard shortcuts commands
            shortcuts::load_keyboard_shortcuts,
            shortcuts::save_keyboard_shortcuts,
            shortcuts::register_global_toggle_shortcut,
            shortcuts::unregister_global_shortcuts,
            shortcuts::toggle_window_visibility,
            shortcuts::get_registered_shortcuts,
            shortcuts::is_shortcut_registered,
            // Deploy commands (015-one-click-deploy)
            deploy::start_oauth_flow,
            deploy::get_connected_platforms,
            deploy::disconnect_platform,
            deploy::start_deployment,
            deploy::get_deployment_history,
            deploy::delete_deployment_history_item,
            deploy::clear_deployment_history,
            deploy::get_deployment_config,
            deploy::save_deployment_config,
            deploy::delete_deployment_config,
            deploy::detect_framework,
            deploy::redeploy,
            // Multi Deploy Accounts (016-multi-deploy-accounts)
            deploy::get_deploy_accounts,
            deploy::get_accounts_by_platform,
            deploy::add_deploy_account,
            deploy::remove_deploy_account,
            deploy::update_deploy_account,
            deploy::bind_project_account,
            deploy::unbind_project_account,
            deploy::get_project_binding,
            deploy::get_deploy_preferences,
            deploy::set_default_account,
            // GitHub Pages workflow generation
            deploy::generate_github_actions_workflow,
            // Cloudflare Pages integration
            deploy::validate_cloudflare_token,
            deploy::add_cloudflare_account,
            deploy::check_account_usage,
            // Secure backup commands
            deploy::export_deploy_backup,
            deploy::import_deploy_backup,
            // Deploy UI Enhancement (018-deploy-ui-enhancement)
            deploy::get_deployment_stats,
            deploy::get_platform_site_info,
            // File watcher commands (package.json monitoring)
            file_watcher::watch_project,
            file_watcher::unwatch_project,
            file_watcher::unwatch_all_projects,
            file_watcher::get_watched_projects,
            // Toolchain conflict detection (017-toolchain-conflict-detection)
            toolchain::detect_toolchain_conflict,
            toolchain::build_toolchain_command,
            toolchain::get_toolchain_preference,
            toolchain::set_toolchain_preference,
            toolchain::clear_toolchain_preference,
            toolchain::get_environment_diagnostics,
            toolchain::humanize_toolchain_error,
            // Corepack management
            toolchain::get_corepack_status_cmd,
            toolchain::detect_pnpm_home_conflict_cmd,
            toolchain::enable_corepack,
            toolchain::fix_pnpm_home_conflict,
            toolchain::get_all_toolchain_preferences,
            // AI Integration (020-ai-cli-integration)
            ai::ai_list_providers,
            ai::ai_add_service,
            ai::ai_update_service,
            ai::ai_delete_provider,
            ai::ai_set_default_provider,
            ai::ai_test_connection,
            ai::ai_list_models,
            ai::ai_probe_models,
            ai::ai_list_templates,
            ai::ai_add_template,
            ai::ai_update_template,
            ai::ai_delete_template,
            ai::ai_set_default_template,
            ai::ai_get_project_settings,
            ai::ai_update_project_settings,
            ai::ai_generate_commit_message,
            ai::ai_generate_code_review,
            ai::ai_generate_staged_review,
            ai::ai_generate_security_analysis,
            ai::ai_generate_security_summary,
            ai::ai_store_api_key,
            ai::ai_check_api_key_status,
            // AI CLI Integration (020-ai-cli-integration)
            ai_cli::ai_cli_detect_tools,
            ai_cli::ai_cli_detect_tool,
            ai_cli::ai_cli_list_tools,
            ai_cli::ai_cli_save_tool,
            ai_cli::ai_cli_delete_tool,
            ai_cli::ai_cli_get_tool,
            ai_cli::ai_cli_get_tool_by_type,
            ai_cli::ai_cli_execute,
            ai_cli::ai_cli_cancel,
            ai_cli::ai_cli_get_history,
            ai_cli::ai_cli_clear_history,
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
            // MCP Action Commands (021-mcp-actions)
            mcp::get_pending_action_requests,
            mcp::respond_to_action_request,
            mcp::list_mcp_actions,
            mcp::get_mcp_action,
            mcp::create_mcp_action,
            mcp::update_mcp_action,
            mcp::delete_mcp_action,
            mcp::get_mcp_action_executions,
            mcp::get_mcp_action_execution,
            mcp::list_mcp_action_permissions,
            mcp::update_mcp_action_permission,
            mcp::delete_mcp_action_permission,
            mcp::cleanup_mcp_action_executions,
            // Notification Center (021-mcp-actions)
            notification::get_notifications,
            notification::get_unread_notification_count,
            notification::mark_notification_read,
            notification::mark_all_notifications_read,
            notification::delete_notification,
            notification::cleanup_old_notifications,
            notification::clear_all_notifications,
            // AI Assistant (022-ai-assistant-tab)
            ai_assistant::ai_assistant_create_conversation,
            ai_assistant::ai_assistant_get_conversation,
            ai_assistant::ai_assistant_list_conversations,
            ai_assistant::ai_assistant_update_conversation,
            ai_assistant::ai_assistant_update_conversation_service,
            ai_assistant::ai_assistant_update_conversation_context,
            ai_assistant::ai_assistant_delete_conversation,
            ai_assistant::ai_assistant_send_message,
            ai_assistant::ai_assistant_cancel_stream,
            ai_assistant::ai_assistant_get_active_stream,
            ai_assistant::ai_assistant_get_messages,
            // AI Assistant - Tool Calls (022-ai-assistant-tab US2)
            ai_assistant::ai_assistant_get_tools,
            ai_assistant::ai_assistant_approve_tool_call,
            ai_assistant::ai_assistant_deny_tool_call,
            ai_assistant::ai_assistant_stop_tool_execution,
            ai_assistant::ai_assistant_continue_after_tool,
            ai_assistant::ai_assistant_get_suggestions,
            ai_assistant::ai_assistant_execute_tool_direct,
            // AI Assistant - Interactive Elements (023-enhanced-ai-chat US3)
            ai_assistant::ai_assistant_parse_interactive,
            ai_assistant::ai_assistant_execute_lazy_action,
            // AI Assistant - Autocomplete & Context (023-enhanced-ai-chat US5)
            ai_assistant::ai_assistant_get_autocomplete,
            ai_assistant::ai_assistant_summarize_context,
            // AI Assistant - Background Process Management
            ai_assistant::ai_assistant_spawn_background_process,
            ai_assistant::ai_assistant_stop_background_process,
            ai_assistant::ai_assistant_list_background_processes,
            ai_assistant::ai_assistant_get_background_process,
            // Time Machine - Snapshot commands (025-ai-workflow-generator)
            snapshot::list_snapshots,
            snapshot::get_snapshot,
            snapshot::get_snapshot_with_dependencies,
            snapshot::get_latest_snapshot,
            snapshot::delete_snapshot,
            snapshot::prune_snapshots,
            snapshot::capture_snapshot,
            snapshot::compare_snapshots,
            snapshot::get_diff_ai_prompt,
            snapshot::get_comparison_candidates,
            snapshot::analyze_diff_patterns,
            snapshot::get_security_insights,
            snapshot::get_insight_summary,
            snapshot::dismiss_insight,
            snapshot::get_snapshot_storage_stats,
            snapshot::cleanup_orphaned_storage,
            snapshot::request_ai_analysis,
            // Security Guardian - Dependency Integrity (025-ai-workflow-generator US3)
            snapshot::check_dependency_integrity,
            snapshot::check_typosquatting,
            // Execution Replay (025-ai-workflow-generator US4)
            snapshot::prepare_replay,
            snapshot::execute_replay,
            snapshot::restore_lockfile,
            // Security Insights Dashboard (025-ai-workflow-generator US5)
            snapshot::get_project_security_overview,
            // Searchable Execution History (025-ai-workflow-generator US6)
            snapshot::search_snapshots,
            snapshot::get_snapshot_timeline,
            snapshot::generate_security_audit_report,
            snapshot::export_security_report,
            // Time Machine - Lockfile Watcher & Settings (025-ai-workflow-generator)
            snapshot::capture_manual_snapshot,
            snapshot::get_time_machine_settings,
            snapshot::update_time_machine_settings,
            snapshot::start_lockfile_watching,
            snapshot::stop_lockfile_watching,
            snapshot::get_lockfile_watcher_status,
            snapshot::get_lockfile_watched_projects,
            // Lockfile Validation (Lockfile Security Enhancement)
            snapshot::get_lockfile_validation_config,
            snapshot::save_lockfile_validation_config,
            snapshot::validate_lockfile_manual,
            snapshot::add_blocked_package,
            snapshot::remove_blocked_package,
            snapshot::add_allowed_registry,
            snapshot::remove_allowed_registry,
            snapshot::reset_lockfile_validation_config,
        ])
        // Setup hook - sync incoming webhook server and start database watcher on app start
        .setup(|app| {
            let handle = app.handle().clone();

            // Initialize BackgroundProcessManager with app handle for Tauri event emission
            {
                let bg_handle = handle.clone();
                tauri::async_runtime::spawn(async move {
                    use crate::services::ai_assistant::background_process::BACKGROUND_PROCESS_MANAGER;
                    BACKGROUND_PROCESS_MANAGER.set_app_handle(bg_handle).await;
                    log::info!("[setup] BackgroundProcessManager initialized with app handle");
                });
            }

            // Start database watcher for MCP-triggered changes
            if let Ok(db_path) = get_database_path() {
                let db_watcher = app.handle().state::<DatabaseWatcher>();
                if let Err(e) = db_watcher.start_watching(&handle, db_path) {
                    log::warn!("[setup] Failed to start database watcher: {}", e);
                }
            }

            // Start lockfile watchers for Time Machine auto-capture
            {
                let db_state = app.handle().state::<DatabaseState>();
                let lockfile_watcher = app.handle().state::<LockfileWatcherState>();
                let repo = repositories::SnapshotRepository::new(db_state.0.as_ref().clone());

                // Check if auto-watch is enabled
                match repo.get_time_machine_settings() {
                    Ok(settings) if settings.auto_watch_enabled => {
                        // Update watcher config with settings
                        let watcher = lockfile_watcher.0.clone();
                        let config = services::LockfileWatcherConfig {
                            debounce_ms: settings.debounce_ms as u64,
                            auto_capture: true,
                        };
                        if let Err(e) = watcher.set_config(config) {
                            log::warn!("[setup] Failed to set lockfile watcher config: {}", e);
                        }

                        // Get all registered projects and start watching their lockfiles
                        let project_repo = repositories::ProjectRepository::new(db_state.0.as_ref().clone());
                        match project_repo.list() {
                            Ok(projects) => {
                                let app_handle = app.handle().clone();
                                for project in projects {
                                    if let Err(e) = watcher.watch_project(&app_handle, db_state.0.as_ref().clone(), &project.path) {
                                        log::debug!("[setup] Skipped watching {}: {}", project.path, e);
                                    } else {
                                        log::info!("[setup] Started watching lockfile for: {}", project.path);
                                    }
                                }
                            }
                            Err(e) => {
                                log::warn!("[setup] Failed to list projects for lockfile watching: {}", e);
                            }
                        }
                    }
                    Ok(_) => {
                        log::info!("[setup] Time Machine auto-watch is disabled");
                    }
                    Err(e) => {
                        log::warn!("[setup] Failed to get Time Machine settings: {}", e);
                    }
                }
            }

            // Cleanup old notifications on startup
            services::notification::cleanup_old_notifications(app.handle());

            // Sync incoming webhook server
            tauri::async_runtime::spawn(async move {
                // Small delay to ensure store is ready
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                if let Err(e) = incoming_webhook::sync_incoming_webhook_server(&handle).await {
                    log::warn!("[setup] Failed to sync incoming webhook server: {}", e);
                }
            });
            Ok(())
        })
        // Handle window events for cleanup
        .on_window_event(|_window, event| {
            use tauri::WindowEvent;
            if let WindowEvent::Destroyed = event {
                // Clean up background processes when app is closing
                log::info!("[shutdown] Cleaning up background processes...");
                tauri::async_runtime::block_on(async {
                    use crate::services::ai_assistant::background_process::BACKGROUND_PROCESS_MANAGER;
                    BACKGROUND_PROCESS_MANAGER.shutdown().await;
                    log::info!("[shutdown] Background processes cleaned up");
                });
            }
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

    // ============================================================
    // CRITICAL DEBUG: Verify actual database file and state
    // ============================================================
    let _ = db.with_connection(|conn| {
        // 1. PRAGMA database_list - 這是 SQLite 真正在用的檔案路徑（權威）
        println!("=== [STARTUP] PRAGMA database_list (SQLite actual file) ===");
        match conn.prepare("PRAGMA database_list;") {
            Ok(mut stmt) => {
                let rows: Vec<(i64, String, String)> = stmt
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                    .map(|iter| iter.filter_map(|r| r.ok()).collect())
                    .unwrap_or_default();
                for (seq, name, file) in &rows {
                    println!("[STARTUP] DB seq={}, name={}, file={}", seq, name, file);
                }
            }
            Err(e) => println!("[STARTUP] PRAGMA database_list failed: {}", e),
        }

        // 2. journal_mode and foreign_keys
        let journal: String = conn
            .query_row("PRAGMA journal_mode;", [], |r| r.get(0))
            .unwrap_or_else(|_| "unknown".to_string());
        let fk: i64 = conn
            .query_row("PRAGMA foreign_keys;", [], |r| r.get(0))
            .unwrap_or(-1);
        println!("[STARTUP] journal_mode={}, foreign_keys={}", journal, fk);

        // 3. Row counts - 驗證資料是否存在
        let projects_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM projects;", [], |r| r.get(0))
            .unwrap_or(-1);
        let configs_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM deployment_configs;", [], |r| r.get(0))
            .unwrap_or(-1);
        println!("[STARTUP] rowcounts: projects={}, deployment_configs={}", projects_count, configs_count);

        // 4. List deployment_configs if any
        if configs_count > 0 {
            println!("[STARTUP] deployment_configs contents:");
            if let Ok(mut stmt) = conn.prepare("SELECT project_id, platform, account_id FROM deployment_configs") {
                let rows: Vec<(String, String, Option<String>)> = stmt
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                    .map(|iter| iter.filter_map(|r| r.ok()).collect())
                    .unwrap_or_default();
                for (pid, plat, aid) in &rows {
                    println!("  - project_id={}, platform={}, account_id={:?}", pid, plat, aid);
                }
            }
        }

        println!("=== [STARTUP] Database diagnostics complete ===");
        Ok(())
    });

    Ok(db)
}
