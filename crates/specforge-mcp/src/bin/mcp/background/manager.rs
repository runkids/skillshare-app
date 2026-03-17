//! Background process manager implementation
//!
//! Manages background process lifecycle including starting, monitoring, and cleanup.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use chrono::Utc;
use once_cell::sync::Lazy;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{RwLock, Semaphore};
use tokio::time::timeout as tokio_timeout;
use uuid::Uuid;

use specforge_lib::utils::path_resolver;

use super::super::store::update_log_status;
use super::types::{
    BackgroundProcessInfo, BackgroundProcessState, BackgroundProcessStatus,
    CircularBuffer, ProcessOutput,
    MAX_BACKGROUND_PROCESSES, MAX_OUTPUT_BUFFER_BYTES, MAX_OUTPUT_BUFFER_LINES,
    DEFAULT_SUCCESS_TIMEOUT_MS, COMPLETED_PROCESS_TTL_SECS,
};

/// Background process manager
pub struct BackgroundProcessManager {
    processes: RwLock<HashMap<String, BackgroundProcessState>>,
    /// Semaphore to limit concurrent background processes
    semaphore: Semaphore,
}

impl BackgroundProcessManager {
    pub fn new() -> Self {
        Self {
            processes: RwLock::new(HashMap::new()),
            semaphore: Semaphore::new(MAX_BACKGROUND_PROCESSES),
        }
    }

    /// Start a background process
    pub async fn start_process(
        &self,
        script_name: String,
        project_path: String,
        command: String,
        success_pattern: Option<String>,
        success_timeout_ms: Option<u64>,
        log_entry_id: Option<i64>,
    ) -> Result<BackgroundProcessInfo, String> {
        // Try to acquire semaphore
        let _permit = self.semaphore.try_acquire()
            .map_err(|_| format!("Maximum background processes ({}) reached", MAX_BACKGROUND_PROCESSES))?;

        // Generate unique ID
        let id = format!("bp_{}", Uuid::new_v4().to_string().split('-').next().unwrap_or("unknown"));

        // Use path_resolver for proper environment setup
        let mut cmd = path_resolver::create_async_command("sh");
        cmd.arg("-c")
            .arg(&command)
            .current_dir(&project_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Spawn the child process
        let mut child = cmd.spawn()
            .map_err(|e| format!("Failed to spawn command: {}", e))?;

        let pid = child.id().unwrap_or(0);

        // Create initial info
        let info = BackgroundProcessInfo {
            id: id.clone(),
            pid,
            script_name: script_name.clone(),
            project_path: project_path.clone(),
            status: if success_pattern.is_some() {
                BackgroundProcessStatus::Starting
            } else {
                BackgroundProcessStatus::Running
            },
            started_at: Utc::now().to_rfc3339(),
            completed_at: None,
            exit_code: None,
            pattern_matched: false,
            command: command.clone(),
        };

        // Create shared state
        let pattern_matched = Arc::new(AtomicBool::new(false));
        let completed = Arc::new(AtomicBool::new(false));
        let output_buffer = Arc::new(tokio::sync::Mutex::new(
            CircularBuffer::new(MAX_OUTPUT_BUFFER_LINES, MAX_OUTPUT_BUFFER_BYTES)
        ));

        // Take stdout and stderr
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        // Clone for tasks
        let pattern_matched_clone = pattern_matched.clone();
        let completed_clone = completed.clone();
        let output_buffer_clone = output_buffer.clone();
        let _id_clone = id.clone();

        // Compile regex pattern if provided
        let compiled_pattern = if let Some(ref pattern) = success_pattern {
            Some(regex::Regex::new(pattern)
                .map_err(|e| format!("Invalid success pattern regex: {}", e))?)
        } else {
            None
        };
        let compiled_pattern = Arc::new(compiled_pattern);

        // Spawn output reader task
        let output_task = tokio::spawn({
            let compiled_pattern = compiled_pattern.clone();
            async move {
                let mut handles = Vec::new();

                if let Some(stdout) = stdout {
                    let buffer = output_buffer_clone.clone();
                    let pattern = compiled_pattern.clone();
                    let matched = pattern_matched_clone.clone();
                    handles.push(tokio::spawn(async move {
                        let reader = BufReader::new(stdout);
                        let mut lines = reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            // Check pattern match
                            if let Some(ref regex) = *pattern {
                                if regex.is_match(&line) {
                                    matched.store(true, Ordering::SeqCst);
                                }
                            }
                            buffer.lock().await.push(format!("[stdout] {}", line));
                        }
                    }));
                }

                if let Some(stderr) = stderr {
                    let buffer = output_buffer_clone.clone();
                    let pattern = compiled_pattern.clone();
                    let matched = pattern_matched_clone.clone();
                    handles.push(tokio::spawn(async move {
                        let reader = BufReader::new(stderr);
                        let mut lines = reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            // Check pattern match on stderr too
                            if let Some(ref regex) = *pattern {
                                if regex.is_match(&line) {
                                    matched.store(true, Ordering::SeqCst);
                                }
                            }
                            buffer.lock().await.push(format!("[stderr] {}", line));
                        }
                    }));
                }

                // Wait for all readers to complete
                for handle in handles {
                    let _ = handle.await;
                }

                completed_clone.store(true, Ordering::SeqCst);
            }
        });

        // If success pattern provided, wait for it with timeout
        let mut final_info = info.clone();
        if success_pattern.is_some() {
            let timeout_ms = success_timeout_ms.unwrap_or(DEFAULT_SUCCESS_TIMEOUT_MS);
            let deadline = Instant::now() + Duration::from_millis(timeout_ms);

            loop {
                if Instant::now() >= deadline {
                    final_info.status = BackgroundProcessStatus::TimedOut;
                    break;
                }

                if pattern_matched.load(Ordering::SeqCst) {
                    final_info.status = BackgroundProcessStatus::Running;
                    final_info.pattern_matched = true;
                    break;
                }

                if completed.load(Ordering::SeqCst) {
                    // Process exited before pattern matched
                    final_info.status = BackgroundProcessStatus::Failed;
                    break;
                }

                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        // Get initial output
        let initial_output = {
            let buffer = output_buffer.lock().await;
            buffer.tail(20)
        };

        // Create state - move buffer out of Arc
        let state = BackgroundProcessState {
            info: final_info.clone(),
            child: Some(child),
            output_buffer: CircularBuffer::new(MAX_OUTPUT_BUFFER_LINES, MAX_OUTPUT_BUFFER_BYTES),
            _output_task: Some(output_task),
            log_entry_id,
            start_timestamp_ms: Utc::now().timestamp_millis(),
        };

        // Copy initial output to state buffer
        {
            let mut processes = self.processes.write().await;
            let mut state = state;
            for line in initial_output {
                state.output_buffer.push(line);
            }
            processes.insert(id.clone(), state);
        }

        // Spawn a task to move output from Arc buffer to state buffer and monitor process
        let id_for_monitor = id.clone();
        let start_timestamp_ms = Utc::now().timestamp_millis();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(500));
            loop {
                interval.tick().await;

                // Copy new output from shared buffer
                let new_lines: Vec<String> = {
                    let mut buffer = output_buffer.lock().await;
                    let lines = buffer.tail(buffer.len());
                    buffer.lines.clear();
                    buffer.total_bytes = 0;
                    lines
                };

                let mut processes = BACKGROUND_PROCESS_MANAGER.processes.write().await;
                if let Some(state) = processes.get_mut(&id_for_monitor) {
                    for line in new_lines {
                        state.output_buffer.push(line);
                    }

                    // Update pattern_matched status
                    if pattern_matched.load(Ordering::SeqCst) && !state.info.pattern_matched {
                        state.info.pattern_matched = true;
                        if state.info.status == BackgroundProcessStatus::Starting {
                            state.info.status = BackgroundProcessStatus::Running;
                        }
                    }

                    // Check if process completed
                    if let Some(ref mut child) = state.child {
                        match child.try_wait() {
                            Ok(Some(status)) => {
                                state.info.exit_code = status.code();
                                let new_status = if status.success() {
                                    BackgroundProcessStatus::Completed
                                } else {
                                    BackgroundProcessStatus::Failed
                                };
                                state.info.status = new_status.clone();
                                state.info.completed_at = Some(Utc::now().to_rfc3339());
                                state.child = None;

                                // Update AI Activity log
                                if let Some(log_id) = state.log_entry_id {
                                    let duration_ms = (Utc::now().timestamp_millis() - start_timestamp_ms).max(0) as u64;
                                    let (result_str, error_msg) = match new_status {
                                        BackgroundProcessStatus::Completed => ("success", None),
                                        BackgroundProcessStatus::Failed => {
                                            let msg = status.code().map(|c| format!("Process failed with exit code {}", c));
                                            ("error", msg)
                                        }
                                        _ => ("error", None),
                                    };
                                    update_log_status(log_id, result_str, duration_ms, error_msg.as_deref());
                                }
                                break;
                            }
                            Ok(None) => {
                                // Still running
                            }
                            Err(_) => {
                                state.info.status = BackgroundProcessStatus::Failed;
                                state.info.completed_at = Some(Utc::now().to_rfc3339());

                                // Update AI Activity log
                                if let Some(log_id) = state.log_entry_id {
                                    let duration_ms = (Utc::now().timestamp_millis() - start_timestamp_ms).max(0) as u64;
                                    update_log_status(log_id, "error", duration_ms, Some("Process monitoring error"));
                                }
                                break;
                            }
                        }
                    } else if completed.load(Ordering::SeqCst) {
                        break;
                    }
                } else {
                    break; // Process removed
                }
            }
        });

        Ok(final_info)
    }

    /// Get process output
    pub async fn get_output(&self, id: &str, tail_lines: usize) -> Result<ProcessOutput, String> {
        let processes = self.processes.read().await;
        let state = processes.get(id)
            .ok_or_else(|| format!("Process not found: {}", id))?;

        let output_lines = state.output_buffer.tail(tail_lines);
        let total_lines = state.output_buffer.len();

        Ok(ProcessOutput {
            process_id: id.to_string(),
            status: state.info.status.clone(),
            pid: state.info.pid,
            script_name: state.info.script_name.clone(),
            started_at: state.info.started_at.clone(),
            output_lines,
            has_more: total_lines > tail_lines,
            total_lines,
        })
    }

    /// Stop a process
    pub async fn stop_process(&self, id: &str, force: bool) -> Result<(), String> {
        let mut processes = self.processes.write().await;
        let state = processes.get_mut(id)
            .ok_or_else(|| format!("Process not found: {}", id))?;

        if let Some(ref mut child) = state.child {
            if force {
                child.kill().await
                    .map_err(|e| format!("Failed to kill process: {}", e))?;
            } else {
                // Send SIGTERM on Unix
                #[cfg(unix)]
                {
                    if let Some(pid) = child.id() {
                        unsafe {
                            libc::kill(pid as i32, libc::SIGTERM);
                        }
                    }
                }
                #[cfg(not(unix))]
                {
                    child.kill().await
                        .map_err(|e| format!("Failed to terminate process: {}", e))?;
                }
            }

            state.info.status = BackgroundProcessStatus::Stopped;
            state.info.completed_at = Some(Utc::now().to_rfc3339());
            state.child = None;

            // Update AI Activity log
            if let Some(log_id) = state.log_entry_id {
                let duration_ms = (Utc::now().timestamp_millis() - state.start_timestamp_ms).max(0) as u64;
                update_log_status(log_id, "stopped", duration_ms, Some("Process stopped by user"));
            }
        }

        Ok(())
    }

    /// List all processes
    pub async fn list_processes(&self) -> Vec<BackgroundProcessInfo> {
        let processes = self.processes.read().await;
        processes.values().map(|s| s.info.clone()).collect()
    }

    /// Stop all processes (called on shutdown)
    pub async fn shutdown(&self) {
        eprintln!("[MCP Server] Stopping all background processes...");

        let mut processes = self.processes.write().await;

        for (id, state) in processes.iter_mut() {
            if let Some(ref mut child) = state.child {
                eprintln!("[MCP Server] Stopping background process: {}", id);

                // Try graceful termination first
                #[cfg(unix)]
                {
                    if let Some(pid) = child.id() {
                        unsafe {
                            libc::kill(pid as i32, libc::SIGTERM);
                        }
                    }
                }

                // Wait up to 5 seconds for graceful termination
                let timeout = tokio_timeout(
                    Duration::from_secs(5),
                    child.wait()
                ).await;

                if timeout.is_err() {
                    // Force kill if still running
                    let _ = child.kill().await;
                }
            }
        }

        processes.clear();
        eprintln!("[MCP Server] All background processes stopped");
    }

    /// Cleanup completed processes older than TTL
    pub async fn cleanup(&self) {
        let now = Utc::now();
        let mut processes = self.processes.write().await;

        let to_remove: Vec<String> = processes.iter()
            .filter(|(_, state)| {
                if let Some(ref completed_at) = state.info.completed_at {
                    if let Ok(completed_time) = chrono::DateTime::parse_from_rfc3339(completed_at) {
                        let elapsed = now.signed_duration_since(completed_time);
                        return elapsed.num_seconds() > COMPLETED_PROCESS_TTL_SECS as i64;
                    }
                }
                false
            })
            .map(|(id, _)| id.clone())
            .collect();

        for id in to_remove {
            processes.remove(&id);
            eprintln!("[MCP Server] Cleaned up completed process: {}", id);
        }
    }
}

/// Global background process manager
pub static BACKGROUND_PROCESS_MANAGER: Lazy<BackgroundProcessManager> =
    Lazy::new(|| BackgroundProcessManager::new());
