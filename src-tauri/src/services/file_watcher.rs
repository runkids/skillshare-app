// File Watcher Service
// Monitors package.json files for changes and emits events to frontend
// Also monitors SQLite database for MCP-triggered changes

use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebouncedEvent, DebouncedEventKind, Debouncer};
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_notification::NotificationExt;

/// Event payload sent to frontend when package.json changes
#[derive(Clone, serde::Serialize)]
pub struct PackageJsonChangedPayload {
    /// The project path (parent directory of package.json)
    pub project_path: String,
    /// The full path to the changed file
    pub file_path: String,
}

/// Event payload sent to frontend when database changes (e.g., from MCP server)
#[derive(Clone, serde::Serialize)]
pub struct DatabaseChangedPayload {
    /// Source of the change (e.g., "mcp", "external")
    pub source: String,
    /// Timestamp of the change
    pub timestamp: String,
}

/// Manages file watchers for multiple project paths
pub struct FileWatcherManager {
    /// Map of project path -> watcher
    watchers: Arc<Mutex<HashMap<String, Debouncer<RecommendedWatcher>>>>,
}

impl Default for FileWatcherManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FileWatcherManager {
    pub fn new() -> Self {
        Self {
            watchers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start watching a project's package.json file
    pub fn watch_project<R: Runtime>(
        &self,
        app_handle: &AppHandle<R>,
        project_path: &str,
    ) -> Result<(), String> {
        let mut watchers = self.watchers.lock().map_err(|e| e.to_string())?;

        // Already watching this path
        if watchers.contains_key(project_path) {
            return Ok(());
        }

        let package_json_path = Path::new(project_path).join("package.json");
        if !package_json_path.exists() {
            return Err(format!(
                "package.json not found at {}",
                package_json_path.display()
            ));
        }

        let app_handle = app_handle.clone();
        let project_path_owned = project_path.to_string();
        let package_json_path_owned = package_json_path.clone();

        // Create debounced watcher (500ms debounce)
        let mut debouncer = new_debouncer(
            Duration::from_millis(500),
            move |res: Result<Vec<DebouncedEvent>, notify::Error>| {
                match res {
                    Ok(events) => {
                        for event in events {
                            // Check if this is the package.json file
                            if event.path == package_json_path_owned {
                                if let DebouncedEventKind::Any = event.kind {
                                    log::info!(
                                        "[FileWatcher] package.json changed: {}",
                                        package_json_path_owned.display()
                                    );

                                    // Emit event to frontend
                                    let payload = PackageJsonChangedPayload {
                                        project_path: project_path_owned.clone(),
                                        file_path: package_json_path_owned
                                            .to_string_lossy()
                                            .to_string(),
                                    };

                                    if let Err(e) =
                                        app_handle.emit("package-json-changed", payload.clone())
                                    {
                                        log::error!("[FileWatcher] Failed to emit event: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("[FileWatcher] Watch error: {:?}", e);
                    }
                }
            },
        )
        .map_err(|e| format!("Failed to create debouncer: {}", e))?;

        // Watch the package.json file specifically
        debouncer
            .watcher()
            .watch(&package_json_path, RecursiveMode::NonRecursive)
            .map_err(|e| format!("Failed to watch path: {}", e))?;

        log::info!(
            "[FileWatcher] Started watching: {}",
            package_json_path.display()
        );
        watchers.insert(project_path.to_string(), debouncer);

        Ok(())
    }

    /// Stop watching a project
    pub fn unwatch_project(&self, project_path: &str) -> Result<(), String> {
        let mut watchers = self.watchers.lock().map_err(|e| e.to_string())?;

        if watchers.remove(project_path).is_some() {
            log::info!("[FileWatcher] Stopped watching: {}", project_path);
        }

        Ok(())
    }

    /// Stop watching all projects
    pub fn unwatch_all(&self) -> Result<(), String> {
        let mut watchers = self.watchers.lock().map_err(|e| e.to_string())?;
        let count = watchers.len();
        watchers.clear();
        log::info!("[FileWatcher] Stopped watching {} projects", count);
        Ok(())
    }

    /// Get list of watched project paths
    pub fn get_watched_paths(&self) -> Result<Vec<String>, String> {
        let watchers = self.watchers.lock().map_err(|e| e.to_string())?;
        Ok(watchers.keys().cloned().collect())
    }
}

/// Database Watcher for monitoring SQLite database changes
/// Watches the WAL file for changes triggered by MCP server or other processes
pub struct DatabaseWatcher {
    /// The database watcher instance
    watcher: Arc<Mutex<Option<Debouncer<RecommendedWatcher>>>>,
    /// Path to the database file
    db_path: Arc<Mutex<Option<PathBuf>>>,
    /// Last seen MCP log ID (for tracking new logs)
    last_log_id: Arc<AtomicI64>,
}

impl Default for DatabaseWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl DatabaseWatcher {
    pub fn new() -> Self {
        Self {
            watcher: Arc::new(Mutex::new(None)),
            db_path: Arc::new(Mutex::new(None)),
            last_log_id: Arc::new(AtomicI64::new(0)),
        }
    }

    /// Get recent MCP logs since the last check
    fn get_recent_mcp_logs(db_path: &PathBuf, last_id: i64) -> Vec<(i64, String, String)> {
        let conn = match Connection::open(db_path) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("[DatabaseWatcher] Failed to open database: {}", e);
                return vec![];
            }
        };

        let mut stmt = match conn.prepare(
            "SELECT id, tool, result FROM mcp_logs WHERE id > ?1 ORDER BY id DESC LIMIT 5",
        ) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("[DatabaseWatcher] Failed to prepare statement: {}", e);
                return vec![];
            }
        };

        let rows = match stmt.query_map([last_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        }) {
            Ok(r) => r,
            Err(e) => {
                log::warn!("[DatabaseWatcher] Failed to query logs: {}", e);
                return vec![];
            }
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// Generate notification message from MCP tool name
    fn tool_to_message(tool: &str, result: &str) -> String {
        let action = match tool {
            "create_workflow" => "Created workflow",
            "add_workflow_step" => "Added workflow step",
            "create_step_template" => "Created step template",
            "run_workflow" => {
                if result == "success" {
                    "Workflow executed successfully"
                } else {
                    "Workflow execution completed"
                }
            }
            "get_project" => "Queried project info",
            "list_worktrees" => "Listed worktrees",
            "get_worktree_status" => "Checked worktree status",
            "get_git_diff" => "Retrieved git diff",
            "list_workflows" => "Listed workflows",
            "get_workflow" => "Queried workflow details",
            "list_step_templates" => "Listed step templates",
            _ => "Updated data",
        };
        action.to_string()
    }

    /// Start watching the database WAL file for changes
    /// Emits "mcp:database-changed" event when changes are detected
    pub fn start_watching<R: Runtime>(
        &self,
        app_handle: &AppHandle<R>,
        db_path: PathBuf,
    ) -> Result<(), String> {
        let mut watcher_guard = self.watcher.lock().map_err(|e| e.to_string())?;
        let mut path_guard = self.db_path.lock().map_err(|e| e.to_string())?;

        // Already watching
        if watcher_guard.is_some() {
            log::info!("[DatabaseWatcher] Already watching database");
            return Ok(());
        }

        // Check if database exists
        if !db_path.exists() {
            return Err(format!("Database not found at {}", db_path.display()));
        }

        // Initialize last_log_id with current max ID
        if let Ok(conn) = Connection::open(&db_path) {
            if let Ok(max_id) = conn.query_row::<i64, _, _>(
                "SELECT COALESCE(MAX(id), 0) FROM mcp_logs",
                [],
                |row| row.get(0),
            ) {
                self.last_log_id.store(max_id, Ordering::SeqCst);
                log::info!("[DatabaseWatcher] Initialized last_log_id to {}", max_id);
            }
        }

        let app_handle = app_handle.clone();
        let db_path_clone = db_path.clone();
        let last_log_id = self.last_log_id.clone();

        // Create debounced watcher (300ms debounce to batch rapid changes)
        let mut debouncer = new_debouncer(
            Duration::from_millis(300),
            move |res: Result<Vec<DebouncedEvent>, notify::Error>| {
                match res {
                    Ok(events) => {
                        // Check if any event is for database files
                        let has_db_change = events.iter().any(|e| {
                            let path_str = e.path.to_string_lossy();
                            path_str.contains("specforge.db")
                        });

                        if has_db_change {
                            log::info!("[DatabaseWatcher] Database change detected");

                            // Get recent MCP logs to determine what changed
                            let current_last_id = last_log_id.load(Ordering::SeqCst);
                            let recent_logs = Self::get_recent_mcp_logs(&db_path_clone, current_last_id);

                            // Update last_log_id if we got new logs
                            if let Some((max_id, _, _)) = recent_logs.first() {
                                last_log_id.store(*max_id, Ordering::SeqCst);
                            }

                            // Generate notification message
                            let notification_body = if recent_logs.is_empty() {
                                "Data updated via MCP".to_string()
                            } else {
                                // Get unique tool actions (most recent first)
                                let actions: Vec<String> = recent_logs
                                    .iter()
                                    .map(|(_, tool, result)| Self::tool_to_message(tool, result))
                                    .collect::<Vec<_>>()
                                    .into_iter()
                                    .take(3)
                                    .collect();

                                if actions.len() == 1 {
                                    format!("MCP: {}", actions[0])
                                } else {
                                    format!("MCP: {} (+{} more)", actions[0], actions.len() - 1)
                                }
                            };

                            let payload = DatabaseChangedPayload {
                                source: "mcp".to_string(),
                                timestamp: chrono::Utc::now().to_rfc3339(),
                            };

                            // Emit event to frontend
                            if let Err(e) = app_handle.emit("mcp:database-changed", payload) {
                                log::error!("[DatabaseWatcher] Failed to emit event: {}", e);
                            }

                            // Send desktop notification only for MCP write operations
                            // Note: Only send notification when there are actual MCP logs,
                            // to avoid notifying on manual UI operations
                            let has_write_op = recent_logs.iter().any(|(_, tool, _)| {
                                matches!(
                                    tool.as_str(),
                                    "create_workflow"
                                        | "add_workflow_step"
                                        | "create_step_template"
                                        | "run_workflow"
                                )
                            });

                            if has_write_op {
                                if let Err(e) = app_handle
                                    .notification()
                                    .builder()
                                    .title("SpecForge")
                                    .body(&notification_body)
                                    .show()
                                {
                                    log::warn!("[DatabaseWatcher] Failed to send notification: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("[DatabaseWatcher] Watch error: {:?}", e);
                    }
                }
            },
        )
        .map_err(|e| format!("Failed to create database watcher: {}", e))?;

        // Watch the database directory (to catch WAL, SHM, and main DB changes)
        let db_dir = db_path.parent().ok_or("Invalid database path")?;
        debouncer
            .watcher()
            .watch(db_dir, RecursiveMode::NonRecursive)
            .map_err(|e| format!("Failed to watch database directory: {}", e))?;

        log::info!(
            "[DatabaseWatcher] Started watching database at: {}",
            db_path.display()
        );

        *watcher_guard = Some(debouncer);
        *path_guard = Some(db_path);

        Ok(())
    }

    /// Stop watching the database
    pub fn stop_watching(&self) -> Result<(), String> {
        let mut watcher_guard = self.watcher.lock().map_err(|e| e.to_string())?;
        let mut path_guard = self.db_path.lock().map_err(|e| e.to_string())?;

        if watcher_guard.take().is_some() {
            log::info!("[DatabaseWatcher] Stopped watching database");
        }
        *path_guard = None;

        Ok(())
    }

    /// Check if currently watching
    pub fn is_watching(&self) -> bool {
        self.watcher
            .lock()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }
}

// =============================================================================
// Lockfile Watcher (Time Machine Feature 025)
// =============================================================================

use crate::models::snapshot::{LockfileState, LockfileType};
use crate::repositories::SnapshotRepository;
use crate::services::snapshot::{SnapshotCaptureService, SnapshotStorage};
use crate::utils::database::Database;
use sha2::{Digest, Sha256};
use std::fs;

/// Event payload sent to frontend when a lockfile changes
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LockfileChangedPayload {
    /// The project path
    pub project_path: String,
    /// The lockfile type that changed
    pub lockfile_type: String,
    /// The new lockfile hash
    pub new_hash: String,
}

/// Event payload sent to frontend when a snapshot is auto-captured
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotAutoCapturedPayload {
    /// The project path
    pub project_path: String,
    /// The new snapshot ID
    pub snapshot_id: String,
    /// The trigger source
    pub trigger_source: String,
}

/// Lockfile watcher configuration
#[derive(Clone)]
pub struct LockfileWatcherConfig {
    /// Debounce duration in milliseconds (default: 2000)
    pub debounce_ms: u64,
    /// Whether to auto-capture snapshots on change
    pub auto_capture: bool,
}

impl Default for LockfileWatcherConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 2000,
            auto_capture: true,
        }
    }
}

/// Manages lockfile watchers for Time Machine
/// Monitors lockfile changes and triggers snapshot capture
pub struct LockfileWatcherManager {
    /// Map of project path -> watcher
    watchers: Arc<Mutex<HashMap<String, Debouncer<RecommendedWatcher>>>>,
    /// Configuration
    config: Arc<Mutex<LockfileWatcherConfig>>,
}

impl Default for LockfileWatcherManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LockfileWatcherManager {
    pub fn new() -> Self {
        Self {
            watchers: Arc::new(Mutex::new(HashMap::new())),
            config: Arc::new(Mutex::new(LockfileWatcherConfig::default())),
        }
    }

    /// Set the watcher configuration
    pub fn set_config(&self, config: LockfileWatcherConfig) -> Result<(), String> {
        let mut guard = self.config.lock().map_err(|e| e.to_string())?;
        *guard = config;
        Ok(())
    }

    /// Get the current configuration
    pub fn get_config(&self) -> Result<LockfileWatcherConfig, String> {
        let guard = self.config.lock().map_err(|e| e.to_string())?;
        Ok(guard.clone())
    }

    /// Detect which lockfile exists in a project
    fn detect_lockfile(project_path: &Path) -> Option<(LockfileType, PathBuf)> {
        let lockfile_checks = [
            (LockfileType::Pnpm, "pnpm-lock.yaml"),
            (LockfileType::Npm, "package-lock.json"),
            (LockfileType::Yarn, "yarn.lock"),
            (LockfileType::Bun, "bun.lockb"),
        ];

        for (lockfile_type, filename) in lockfile_checks {
            let path = project_path.join(filename);
            if path.exists() {
                return Some((lockfile_type, path));
            }
        }
        None
    }

    /// Compute SHA-256 hash of a file
    fn compute_file_hash(path: &Path) -> Result<String, String> {
        let content = fs::read(path)
            .map_err(|e| format!("Failed to read file {}: {}", path.display(), e))?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Start watching a project's lockfile
    pub fn watch_project<R: Runtime>(
        &self,
        app_handle: &AppHandle<R>,
        db: Database,
        project_path: &str,
    ) -> Result<(), String> {
        let mut watchers = self.watchers.lock().map_err(|e| e.to_string())?;

        // Already watching this path
        if watchers.contains_key(project_path) {
            log::info!("[LockfileWatcher] Already watching: {}", project_path);
            return Ok(());
        }

        let project_path_buf = PathBuf::from(project_path);

        // Detect lockfile
        let (lockfile_type, lockfile_path) = Self::detect_lockfile(&project_path_buf)
            .ok_or_else(|| format!("No lockfile found in project: {}", project_path))?;

        log::info!(
            "[LockfileWatcher] Detected {} lockfile at: {}",
            lockfile_type.as_str(),
            lockfile_path.display()
        );

        // Get initial hash and store state
        let initial_hash = Self::compute_file_hash(&lockfile_path)?;
        let repo = SnapshotRepository::new(db.clone());

        // Initialize or update lockfile state
        let state = LockfileState {
            project_path: project_path.to_string(),
            lockfile_type: Some(lockfile_type.clone()),
            lockfile_hash: initial_hash.clone(),
            last_snapshot_id: None,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        repo.update_lockfile_state(&state)?;

        // Get debounce duration from config
        let config = self.config.lock().map_err(|e| e.to_string())?;
        let debounce_ms = config.debounce_ms;
        let auto_capture = config.auto_capture;
        drop(config);

        let app_handle = app_handle.clone();
        let project_path_owned = project_path.to_string();
        let lockfile_path_clone = lockfile_path.clone();
        let lockfile_type_clone = lockfile_type.clone();
        let db_clone = db.clone();
        let last_hash = Arc::new(Mutex::new(initial_hash));

        // Create debounced watcher
        let mut debouncer = new_debouncer(
            Duration::from_millis(debounce_ms),
            move |res: Result<Vec<DebouncedEvent>, notify::Error>| {
                match res {
                    Ok(events) => {
                        // Check if any event is for our lockfile
                        let lockfile_changed = events.iter().any(|e| {
                            e.path == lockfile_path_clone
                        });

                        if lockfile_changed {
                            log::info!(
                                "[LockfileWatcher] Lockfile change detected: {}",
                                lockfile_path_clone.display()
                            );

                            // Compute new hash
                            match Self::compute_file_hash(&lockfile_path_clone) {
                                Ok(new_hash) => {
                                    // Check if hash actually changed
                                    let mut last_hash_guard = last_hash.lock().unwrap();
                                    if new_hash == *last_hash_guard {
                                        log::info!(
                                            "[LockfileWatcher] Hash unchanged, ignoring: {}",
                                            project_path_owned
                                        );
                                        return;
                                    }

                                    log::info!(
                                        "[LockfileWatcher] Hash changed: {} -> {}",
                                        &last_hash_guard[..8],
                                        &new_hash[..8]
                                    );
                                    *last_hash_guard = new_hash.clone();
                                    drop(last_hash_guard);

                                    // Emit lockfile-changed event
                                    let payload = LockfileChangedPayload {
                                        project_path: project_path_owned.clone(),
                                        lockfile_type: lockfile_type_clone.as_str().to_string(),
                                        new_hash: new_hash.clone(),
                                    };

                                    if let Err(e) = app_handle.emit("lockfile-changed", payload) {
                                        log::error!(
                                            "[LockfileWatcher] Failed to emit lockfile-changed event: {}",
                                            e
                                        );
                                    }

                                    // Auto-capture snapshot if enabled
                                    if auto_capture {
                                        let storage_base = app_handle
                                            .path()
                                            .app_data_dir()
                                            .unwrap_or_else(|_| PathBuf::from("."));
                                        let storage = SnapshotStorage::new(storage_base.join("snapshots"));
                                        let capture_service = SnapshotCaptureService::new(storage, db_clone.clone());

                                        match capture_service.capture_lockfile_change_snapshot(&project_path_owned) {
                                            Ok(snapshot) => {
                                                log::info!(
                                                    "[LockfileWatcher] Auto-captured snapshot: {}",
                                                    snapshot.id
                                                );

                                                // Update lockfile state with new snapshot
                                                let repo = SnapshotRepository::new(db_clone.clone());
                                                let state = LockfileState {
                                                    project_path: project_path_owned.clone(),
                                                    lockfile_type: Some(lockfile_type_clone.clone()),
                                                    lockfile_hash: new_hash,
                                                    last_snapshot_id: Some(snapshot.id.clone()),
                                                    updated_at: chrono::Utc::now().to_rfc3339(),
                                                };
                                                if let Err(e) = repo.update_lockfile_state(&state) {
                                                    log::error!(
                                                        "[LockfileWatcher] Failed to update lockfile state: {}",
                                                        e
                                                    );
                                                }

                                                // Emit snapshot-auto-captured event
                                                let snapshot_payload = SnapshotAutoCapturedPayload {
                                                    project_path: project_path_owned.clone(),
                                                    snapshot_id: snapshot.id,
                                                    trigger_source: "lockfile_change".to_string(),
                                                };

                                                if let Err(e) = app_handle.emit("snapshot:auto-captured", snapshot_payload) {
                                                    log::error!(
                                                        "[LockfileWatcher] Failed to emit snapshot event: {}",
                                                        e
                                                    );
                                                }
                                            }
                                            Err(e) => {
                                                log::error!(
                                                    "[LockfileWatcher] Failed to capture snapshot: {}",
                                                    e
                                                );
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::error!("[LockfileWatcher] Failed to compute hash: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("[LockfileWatcher] Watch error: {:?}", e);
                    }
                }
            },
        )
        .map_err(|e| format!("Failed to create lockfile watcher: {}", e))?;

        // Watch the lockfile
        debouncer
            .watcher()
            .watch(&lockfile_path, RecursiveMode::NonRecursive)
            .map_err(|e| format!("Failed to watch lockfile: {}", e))?;

        log::info!(
            "[LockfileWatcher] Started watching: {} ({})",
            project_path,
            lockfile_type.as_str()
        );
        watchers.insert(project_path.to_string(), debouncer);

        Ok(())
    }

    /// Stop watching a project's lockfile
    pub fn unwatch_project(&self, project_path: &str) -> Result<(), String> {
        let mut watchers = self.watchers.lock().map_err(|e| e.to_string())?;

        if watchers.remove(project_path).is_some() {
            log::info!("[LockfileWatcher] Stopped watching: {}", project_path);
        }

        Ok(())
    }

    /// Stop watching all projects
    pub fn unwatch_all(&self) -> Result<(), String> {
        let mut watchers = self.watchers.lock().map_err(|e| e.to_string())?;
        let count = watchers.len();
        watchers.clear();
        log::info!("[LockfileWatcher] Stopped watching {} projects", count);
        Ok(())
    }

    /// Get list of watched project paths
    pub fn get_watched_paths(&self) -> Result<Vec<String>, String> {
        let watchers = self.watchers.lock().map_err(|e| e.to_string())?;
        Ok(watchers.keys().cloned().collect())
    }

    /// Check if a project is being watched
    pub fn is_watching(&self, project_path: &str) -> Result<bool, String> {
        let watchers = self.watchers.lock().map_err(|e| e.to_string())?;
        Ok(watchers.contains_key(project_path))
    }

    /// Check if lockfile has changed since last snapshot
    pub fn has_lockfile_changed(&self, db: &Database, project_path: &str) -> Result<bool, String> {
        let project_path_buf = PathBuf::from(project_path);

        // Detect lockfile
        let (_, lockfile_path) = Self::detect_lockfile(&project_path_buf)
            .ok_or_else(|| format!("No lockfile found in project: {}", project_path))?;

        // Compute current hash
        let current_hash = Self::compute_file_hash(&lockfile_path)?;

        // Get stored state
        let repo = SnapshotRepository::new(db.clone());
        match repo.get_lockfile_state(project_path)? {
            Some(state) => Ok(current_hash != state.lockfile_hash),
            None => Ok(true), // No previous state, consider it changed
        }
    }

    /// Manually trigger a snapshot check for a project
    pub fn check_and_trigger_snapshot<R: Runtime>(
        &self,
        app_handle: &AppHandle<R>,
        db: &Database,
        project_path: &str,
    ) -> Result<Option<String>, String> {
        // Check if lockfile changed
        if !self.has_lockfile_changed(db, project_path)? {
            log::info!(
                "[LockfileWatcher] No lockfile change for: {}",
                project_path
            );
            return Ok(None);
        }

        // Capture snapshot
        let storage_base = app_handle
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| PathBuf::from("."));
        let storage = SnapshotStorage::new(storage_base.join("snapshots"));
        let capture_service = SnapshotCaptureService::new(storage, db.clone());

        let snapshot = capture_service.capture_lockfile_change_snapshot(project_path)?;

        // Update lockfile state
        let project_path_buf = PathBuf::from(project_path);
        if let Some((lockfile_type, lockfile_path)) = Self::detect_lockfile(&project_path_buf) {
            let current_hash = Self::compute_file_hash(&lockfile_path)?;
            let repo = SnapshotRepository::new(db.clone());
            let state = LockfileState {
                project_path: project_path.to_string(),
                lockfile_type: Some(lockfile_type),
                lockfile_hash: current_hash,
                last_snapshot_id: Some(snapshot.id.clone()),
                updated_at: chrono::Utc::now().to_rfc3339(),
            };
            repo.update_lockfile_state(&state)?;
        }

        // Emit event
        let payload = SnapshotAutoCapturedPayload {
            project_path: project_path.to_string(),
            snapshot_id: snapshot.id.clone(),
            trigger_source: "lockfile_change".to_string(),
        };

        if let Err(e) = app_handle.emit("snapshot:auto-captured", payload) {
            log::error!(
                "[LockfileWatcher] Failed to emit snapshot event: {}",
                e
            );
        }

        Ok(Some(snapshot.id))
    }
}
