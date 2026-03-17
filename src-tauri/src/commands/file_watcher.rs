// File watcher commands
// Commands for managing package.json file watching and .specforge/ directory monitoring

use crate::services::{FileWatcherManager, SpecforgeWatcher};
use crate::DatabaseState;
use std::path::Path;
use tauri::{AppHandle, State};

/// Response type for file watcher commands
#[derive(serde::Serialize)]
pub struct FileWatcherResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Start watching a project's package.json file for changes
#[tauri::command]
pub async fn watch_project(
    app: AppHandle,
    state: State<'_, FileWatcherManager>,
    project_path: String,
) -> Result<FileWatcherResponse, String> {
    match state.watch_project(&app, &project_path) {
        Ok(()) => Ok(FileWatcherResponse {
            success: true,
            error: None,
        }),
        Err(e) => Ok(FileWatcherResponse {
            success: false,
            error: Some(e),
        }),
    }
}

/// Stop watching a project's package.json file
#[tauri::command]
pub async fn unwatch_project(
    state: State<'_, FileWatcherManager>,
    project_path: String,
) -> Result<FileWatcherResponse, String> {
    match state.unwatch_project(&project_path) {
        Ok(()) => Ok(FileWatcherResponse {
            success: true,
            error: None,
        }),
        Err(e) => Ok(FileWatcherResponse {
            success: false,
            error: Some(e),
        }),
    }
}

/// Stop watching all projects
#[tauri::command]
pub async fn unwatch_all_projects(
    state: State<'_, FileWatcherManager>,
) -> Result<FileWatcherResponse, String> {
    match state.unwatch_all() {
        Ok(()) => Ok(FileWatcherResponse {
            success: true,
            error: None,
        }),
        Err(e) => Ok(FileWatcherResponse {
            success: false,
            error: Some(e),
        }),
    }
}

/// Get list of watched project paths
#[tauri::command]
pub async fn get_watched_projects(
    state: State<'_, FileWatcherManager>,
) -> Result<Vec<String>, String> {
    state.get_watched_paths()
}

/// Start watching a project's `.specforge/` directory for external file changes.
/// Automatically syncs specs and schemas to SQLite when files change on disk.
#[tauri::command]
pub async fn watch_specforge(
    app: AppHandle,
    watcher: State<'_, SpecforgeWatcher>,
    db: State<'_, DatabaseState>,
    project_dir: String,
) -> Result<FileWatcherResponse, String> {
    match watcher.start_watching(&app, Path::new(&project_dir), db.0.clone()) {
        Ok(()) => Ok(FileWatcherResponse {
            success: true,
            error: None,
        }),
        Err(e) => Ok(FileWatcherResponse {
            success: false,
            error: Some(e),
        }),
    }
}

/// Stop watching the `.specforge/` directory.
#[tauri::command]
pub async fn unwatch_specforge(
    watcher: State<'_, SpecforgeWatcher>,
) -> Result<FileWatcherResponse, String> {
    match watcher.stop_watching() {
        Ok(()) => Ok(FileWatcherResponse {
            success: true,
            error: None,
        }),
        Err(e) => Ok(FileWatcherResponse {
            success: false,
            error: Some(e),
        }),
    }
}
