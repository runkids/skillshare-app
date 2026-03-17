//! Script execution commands module
//!
//! Implements US3: Script Execution with Real-time Output
//!
//! Module structure:
//! - `types`: All struct/enum definitions and payloads
//! - `state`: Execution state management (ScriptExecutionState)
//! - `output`: Output batching and streaming
//! - `process`: Process tree management and cleanup

pub mod output;
pub mod process;
pub mod state;
pub mod types;

// Re-export commonly used types
pub use state::{RunningExecution, ScriptExecutionState};
pub use types::{
    CancelScriptResponse, ExecuteScriptResponse, ExecutionStatus, GetScriptOutputResponse,
    OutputBuffer, RunningScriptInfo, ScriptCompletedPayload, VoltaWrappedCommand,
    WriteToScriptResponse,
};

// Internal imports
use chrono::Utc;
use output::{handle_process_completion, stream_output, StreamType};
use process::{cleanup_expired_executions, kill_process_tree};
use std::path::Path;
use std::process::Stdio;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::commands::project::parse_package_json;
use crate::commands::version::detect_volta;
use crate::utils::path_resolver;

// ============================================================================
// Commands
// ============================================================================

/// Execute an npm script
/// Uses path_resolver to handle macOS GUI app PATH issues
#[tauri::command]
pub async fn execute_script(
    app: AppHandle,
    project_path: String,
    script_name: String,
    package_manager: String,
    cwd: Option<String>,
    project_name: Option<String>, // Feature 007: Optional project name for reconnection
) -> Result<ExecuteScriptResponse, String> {
    let execution_id = Uuid::new_v4().to_string();
    let working_dir = cwd.clone().unwrap_or_else(|| project_path.clone());
    let stored_project_path = cwd.unwrap_or(project_path); // Feature 007: Store actual working dir

    // Determine command based on package manager
    // Special handling for built-in commands (install, build, etc.)
    let is_builtin_command = matches!(script_name.as_str(), "install" | "i" | "ci");

    let pm_cmd = match package_manager.as_str() {
        "pnpm" => "pnpm",
        "yarn" => "yarn",
        "npm" | _ => "npm",
    };

    let pm_args: Vec<String> = if is_builtin_command {
        // For built-in commands, run directly without "run"
        vec![script_name.clone()]
    } else {
        // For package.json scripts, use "run"
        vec!["run".to_string(), script_name.clone()]
    };

    // Check if project has Volta config and Volta is available
    let path = Path::new(&working_dir);
    let volta_config = match parse_package_json(path) {
        Ok(pj) => {
            println!("[execute_script] package.json volta config: {:?}", pj.volta);
            pj.volta.clone()
        }
        Err(e) => {
            println!("[execute_script] Failed to parse package.json: {}", e);
            None
        }
    };
    let volta_status = detect_volta();
    println!(
        "[execute_script] Volta status: available={}, path={:?}",
        volta_status.available, volta_status.path
    );
    println!(
        "[execute_script] volta_config.is_some()={}, volta_status.available={}",
        volta_config.is_some(),
        volta_status.available
    );

    // Determine final command and args (with or without Volta wrapper)
    let (cmd, args): (String, Vec<String>) = if volta_config.is_some() && volta_status.available {
        // Use volta run to ensure correct versions
        let volta_cmd = volta_status.path.unwrap_or_else(|| "volta".to_string());
        let mut volta_args = vec!["run".to_string()];

        // Add all configured volta versions
        if let Some(ref config) = volta_config {
            // Node.js version
            if let Some(ref node_version) = config.node {
                volta_args.push("--node".to_string());
                volta_args.push(node_version.clone());
            }
            // npm version (only when running npm)
            if pm_cmd == "npm" {
                if let Some(ref npm_version) = config.npm {
                    volta_args.push("--npm".to_string());
                    volta_args.push(npm_version.clone());
                }
            }
            // yarn version (only when running yarn)
            if pm_cmd == "yarn" {
                if let Some(ref yarn_version) = config.yarn {
                    volta_args.push("--yarn".to_string());
                    volta_args.push(yarn_version.clone());
                }
            }
            // Note: Volta doesn't directly support --pnpm flag
            // pnpm version management should use corepack instead
        }

        volta_args.push(pm_cmd.to_string());
        volta_args.extend(pm_args);
        println!(
            "[execute_script] Using Volta: {} {:?} in {}",
            volta_cmd, volta_args, working_dir
        );
        (volta_cmd, volta_args)
    } else {
        println!(
            "[execute_script] Spawning {} {:?} in {}",
            pm_cmd, pm_args, working_dir
        );
        (pm_cmd.to_string(), pm_args)
    };

    // Use path_resolver to create command with proper PATH for macOS GUI apps
    let mut command = path_resolver::create_command(&cmd);
    command.args(&args);
    command.current_dir(&working_dir);
    // Set CI=true to prevent interactive prompts from pnpm/npm
    command.env("CI", "true");
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    command.stdin(Stdio::piped());

    // Convert to tokio command and spawn
    let mut child = tokio::process::Command::from(command)
        .spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;

    let pid = child.id();
    println!("[execute_script] Spawned process with PID: {:?}", pid);

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
    let stdin = child.stdin.take();

    let exec_id = execution_id.clone();
    let start_time = Instant::now();

    // Feature 007: Store execution state with extended fields
    let started_at_iso = Utc::now().to_rfc3339();
    let stored_path = stored_project_path.clone();
    let stored_name = project_name.clone();
    {
        let state = app.state::<ScriptExecutionState>();
        let mut executions = state.executions.write().await;
        executions.insert(
            execution_id.clone(),
            RunningExecution {
                execution_id: execution_id.clone(),
                script_name: script_name.clone(),
                started_at: start_time,
                child: Some(child),
                stdin,
                pid,
                // Feature 007: New fields
                project_path: stored_path.clone(),
                project_name: stored_name.clone(),
                output_buffer: OutputBuffer::new(),
                started_at_iso: started_at_iso.clone(),
                status: ExecutionStatus::Running,
                exit_code: None,
                completed_at: None,
            },
        );
    }

    // Performance optimization: Use shared stream handlers with batching
    let app_stdout = app.clone();
    let exec_id_stdout = exec_id.clone();
    let stdout_task = tauri::async_runtime::spawn(async move {
        stream_output(stdout, StreamType::Stdout, exec_id_stdout, app_stdout).await;
    });

    let app_stderr = app.clone();
    let exec_id_stderr = exec_id.clone();
    let stderr_task = tauri::async_runtime::spawn(async move {
        stream_output(stderr, StreamType::Stderr, exec_id_stderr, app_stderr).await;
    });

    // Spawn task to wait for process completion
    let app_wait = app.clone();
    let exec_id_wait = exec_id.clone();
    tauri::async_runtime::spawn(async move {
        // Wait for output tasks to complete
        let _ = stdout_task.await;
        let _ = stderr_task.await;

        // Use shared completion handler
        handle_process_completion(exec_id_wait, start_time, app_wait).await;
    });

    Ok(ExecuteScriptResponse {
        success: true,
        execution_id: Some(execution_id),
        error: None,
    })
}

/// Execute a command from the allowed list
/// Uses path_resolver to handle macOS GUI app PATH issues
#[tauri::command]
pub async fn execute_command(
    app: AppHandle,
    command: String,
    args: Vec<String>,
    cwd: String,
    project_path: Option<String>, // Feature 007: Optional project root
    project_name: Option<String>, // Feature 007: Optional project name
) -> Result<ExecuteScriptResponse, String> {
    // Validate allowed commands
    let allowed_commands = [
        // Package managers
        "npm",
        "yarn",
        "pnpm",
        "bun",
        // Node.js
        "node",
        "npx",
        "tsx",
        // Version control
        "git",
        // Build tools
        "make",
        "cmake",
        // Rust
        "cargo",
        "rustc",
        "rustup",
        // Python
        "python",
        "python3",
        "pip",
        "pip3",
        "pipenv",
        "poetry",
        // Go
        "go",
        // Mobile development
        "expo",
        "eas",
        "flutter",
        "dart",
        "pod",
        "xcodebuild",
        "fastlane",
        // Container
        "docker",
        "docker-compose",
        // File operations
        "ls",
        "cat",
        "head",
        "tail",
        "find",
        "grep",
        "mkdir",
        "rm",
        "cp",
        "mv",
        "touch",
        "chmod",
        // Utilities
        "echo",
        "pwd",
        "which",
        "env",
        "curl",
        "wget",
        "tar",
        "unzip",
        "open",
        // macOS specific
        "brew",
        "xcrun",
    ];

    if !allowed_commands.contains(&command.as_str()) {
        return Ok(ExecuteScriptResponse {
            success: false,
            execution_id: None,
            error: Some(format!("Command '{}' is not in the allowed list", command)),
        });
    }

    let execution_id = Uuid::new_v4().to_string();

    // Check if project has Volta config and Volta is available for node-related commands
    let node_commands = ["npm", "yarn", "pnpm", "node", "npx", "tsx", "bun"];
    let path = Path::new(&cwd);
    let volta_config = if node_commands.contains(&command.as_str()) {
        match parse_package_json(path) {
            Ok(pj) => pj.volta.clone(),
            Err(_) => None,
        }
    } else {
        None
    };
    let volta_status = detect_volta();

    // Determine final command and args (with or without Volta wrapper)
    let (cmd, final_args): (String, Vec<String>) =
        if volta_config.is_some() && volta_status.available {
            let volta_cmd = volta_status.path.unwrap_or_else(|| "volta".to_string());
            let mut volta_args = vec!["run".to_string()];

            if let Some(ref config) = volta_config {
                if let Some(ref node_version) = config.node {
                    volta_args.push("--node".to_string());
                    volta_args.push(node_version.clone());
                }
                if command == "npm" {
                    if let Some(ref npm_version) = config.npm {
                        volta_args.push("--npm".to_string());
                        volta_args.push(npm_version.clone());
                    }
                }
                if command == "yarn" {
                    if let Some(ref yarn_version) = config.yarn {
                        volta_args.push("--yarn".to_string());
                        volta_args.push(yarn_version.clone());
                    }
                }
            }

            volta_args.push(command.clone());
            volta_args.extend(args.clone());
            println!(
                "[execute_command] Using Volta: {} {:?} in {}",
                volta_cmd, volta_args, cwd
            );
            (volta_cmd, volta_args)
        } else {
            println!("[execute_command] Spawning {} {:?} in {}", command, args, cwd);
            (command.clone(), args.clone())
        };

    // Use path_resolver to create command with proper PATH for macOS GUI apps
    let mut std_command = path_resolver::create_command(&cmd);
    std_command.args(&final_args);
    std_command.current_dir(&cwd);
    // Set CI=true to prevent interactive prompts from pnpm/npm
    std_command.env("CI", "true");
    std_command.stdout(Stdio::piped());
    std_command.stderr(Stdio::piped());
    std_command.stdin(Stdio::piped());

    // Convert to tokio command and spawn
    let mut child = tokio::process::Command::from(std_command)
        .spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;

    let pid = child.id();
    println!("[execute_command] Spawned process with PID: {:?}", pid);

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
    let stdin = child.stdin.take();

    let exec_id = execution_id.clone();
    let start_time = Instant::now();

    // Feature 007: Store execution state with extended fields
    let started_at_iso = Utc::now().to_rfc3339();
    let stored_path = project_path.unwrap_or_else(|| cwd.clone());
    {
        let state = app.state::<ScriptExecutionState>();
        let mut executions = state.executions.write().await;
        executions.insert(
            execution_id.clone(),
            RunningExecution {
                execution_id: execution_id.clone(),
                script_name: format!("{} {}", command, args.join(" ")),
                started_at: start_time,
                child: Some(child),
                stdin,
                pid,
                // Feature 007: New fields
                project_path: stored_path,
                project_name: project_name.clone(),
                output_buffer: OutputBuffer::new(),
                started_at_iso: started_at_iso.clone(),
                status: ExecutionStatus::Running,
                exit_code: None,
                completed_at: None,
            },
        );
    }

    // Performance optimization: Use shared stream handlers with batching
    let app_stdout = app.clone();
    let exec_id_stdout = exec_id.clone();
    let stdout_task = tauri::async_runtime::spawn(async move {
        stream_output(stdout, StreamType::Stdout, exec_id_stdout, app_stdout).await;
    });

    let app_stderr = app.clone();
    let exec_id_stderr = exec_id.clone();
    let stderr_task = tauri::async_runtime::spawn(async move {
        stream_output(stderr, StreamType::Stderr, exec_id_stderr, app_stderr).await;
    });

    // Spawn task to wait for process completion
    let app_wait = app.clone();
    let exec_id_wait = exec_id.clone();
    tauri::async_runtime::spawn(async move {
        // Wait for output tasks to complete
        let _ = stdout_task.await;
        let _ = stderr_task.await;

        // Use shared completion handler
        handle_process_completion(exec_id_wait, start_time, app_wait).await;
    });

    Ok(ExecuteScriptResponse {
        success: true,
        execution_id: Some(execution_id),
        error: None,
    })
}

/// Cancel a running script execution
#[tauri::command]
pub async fn cancel_script(
    app: AppHandle,
    execution_id: String,
) -> Result<CancelScriptResponse, String> {
    println!("[cancel_script] Called for execution_id: {}", execution_id);

    // Extract needed data from state first, then release lock
    let (pid, child, duration_ms, should_emit) = {
        let state = app.state::<ScriptExecutionState>();
        let mut executions = state.executions.write().await;

        println!(
            "[cancel_script] Current tracked executions: {:?}",
            executions.keys().collect::<Vec<_>>()
        );

        if let Some(execution) = executions.get_mut(&execution_id) {
            // Only cancel if still running
            if execution.status != ExecutionStatus::Running {
                println!(
                    "[cancel_script] Execution {} is not running (status: {:?})",
                    execution_id, execution.status
                );
                return Ok(CancelScriptResponse {
                    success: false,
                    error: Some("Execution is not running".to_string()),
                });
            }

            println!(
                "[cancel_script] Found execution: {}, PID: {:?}",
                execution.script_name, execution.pid
            );
            let duration = execution.started_at.elapsed().as_millis() as u64;
            let pid = execution.pid;
            let child = execution.child.take();

            // Update status while we have the lock
            execution.status = ExecutionStatus::Cancelled;
            execution.exit_code = Some(-1);
            execution.completed_at = Some(Utc::now().to_rfc3339());
            execution.child = None;
            execution.stdin = None;

            (pid, child, duration, true)
        } else {
            println!("[cancel_script] Execution not found in tracked executions");
            return Ok(CancelScriptResponse {
                success: false,
                error: Some("Execution not found".to_string()),
            });
        }
    };

    // Now kill the process (outside the lock)
    if let Some(pid) = pid {
        let _ = kill_process_tree(pid);
    }

    // Kill via child handle if available
    if let Some(mut child) = child {
        let _ = child.kill().await;
    }

    println!("[cancel_script] Process tree killed");

    // Emit completion event
    if should_emit {
        let emit_result = app.emit(
            "script_completed",
            ScriptCompletedPayload {
                execution_id: execution_id.clone(),
                exit_code: -1,
                success: false,
                duration_ms,
            },
        );
        println!("[cancel_script] Emit result: {:?}", emit_result);
    }

    Ok(CancelScriptResponse {
        success: true,
        error: None,
    })
}

/// Kill all processes tracked by this app (safe mode - only kills SpecForge-started processes)
#[tauri::command]
pub async fn kill_all_node_processes(app: AppHandle) -> Result<CancelScriptResponse, String> {
    // Collect data to kill outside the lock
    let to_kill: Vec<(String, Option<u32>, Option<tokio::process::Child>, u64)> = {
        let state = app.state::<ScriptExecutionState>();
        let mut executions = state.executions.write().await;

        println!(
            "[kill_all_node_processes] Starting, tracked executions: {}",
            executions.len()
        );

        // Feature 007: Only kill running executions
        let running_ids: Vec<String> = executions
            .iter()
            .filter(|(_, exec)| exec.status == ExecutionStatus::Running)
            .map(|(id, _)| id.clone())
            .collect();

        println!(
            "[kill_all_node_processes] Running execution IDs to kill: {:?}",
            running_ids
        );

        running_ids
            .into_iter()
            .filter_map(|exec_id| {
                if let Some(execution) = executions.get_mut(&exec_id) {
                    println!(
                        "[kill_all_node_processes] Processing {}: {}, PID: {:?}",
                        exec_id, execution.script_name, execution.pid
                    );

                    let pid = execution.pid;
                    let child = execution.child.take();
                    let duration_ms = execution.started_at.elapsed().as_millis() as u64;

                    // Update status while we have the lock
                    execution.status = ExecutionStatus::Cancelled;
                    execution.exit_code = Some(-1);
                    execution.completed_at = Some(Utc::now().to_rfc3339());
                    execution.child = None;
                    execution.stdin = None;

                    Some((exec_id, pid, child, duration_ms))
                } else {
                    None
                }
            })
            .collect()
    };

    let mut killed_count = 0;

    // Kill processes outside the lock
    for (exec_id, pid, child, duration_ms) in to_kill {
        // Kill the process tree using PID
        if let Some(pid) = pid {
            let _ = kill_process_tree(pid);
        }

        // Also try via child handle
        if let Some(mut child) = child {
            let _ = child.kill().await;
        }

        killed_count += 1;
        println!(
            "[kill_all_node_processes] Killed process tree for {}",
            exec_id
        );

        // Emit completion event
        let emit_result = app.emit(
            "script_completed",
            ScriptCompletedPayload {
                execution_id: exec_id.clone(),
                exit_code: -1,
                success: false,
                duration_ms,
            },
        );
        println!(
            "[kill_all_node_processes] Emit result for {}: {:?}",
            exec_id, emit_result
        );
    }

    println!(
        "[kill_all_node_processes] Completed, killed_count: {}",
        killed_count
    );

    Ok(CancelScriptResponse {
        success: true,
        error: if killed_count > 0 {
            Some(format!("Stopped {} process(es)", killed_count))
        } else {
            Some("No running processes to stop".to_string())
        },
    })
}

/// Kill processes using specific ports
#[tauri::command]
pub async fn kill_ports(ports: Vec<u16>) -> Result<CancelScriptResponse, String> {
    let mut killed_count = 0;

    for port in &ports {
        // Use lsof to find processes using this port
        let output = path_resolver::create_command("lsof")
            .args(["-ti", &format!(":{}", port)])
            .output();

        if let Ok(output) = output {
            let pids_str = String::from_utf8_lossy(&output.stdout);
            for pid_str in pids_str.lines() {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    println!("[kill_ports] Killing PID {} using port {}", pid, port);
                    let _ = kill_process_tree(pid);
                    killed_count += 1;
                }
            }
        }
    }

    Ok(CancelScriptResponse {
        success: true,
        error: if killed_count > 0 {
            Some(format!(
                "Killed {} process(es) on ports {:?}",
                killed_count, ports
            ))
        } else {
            Some(format!("No processes found on ports {:?}", ports))
        },
    })
}

/// Check if specific ports are in use
#[tauri::command]
pub async fn check_ports(ports: Vec<u16>) -> Result<Vec<u16>, String> {
    let mut in_use = Vec::new();

    for port in ports {
        let output = path_resolver::create_command("lsof")
            .args(["-ti", &format!(":{}", port)])
            .output();

        if let Ok(output) = output {
            if !output.stdout.is_empty() {
                in_use.push(port);
            }
        }
    }

    Ok(in_use)
}

/// Get list of script executions (Feature 007: includes completed scripts within 5 min retention)
#[tauri::command]
pub async fn get_running_scripts(app: AppHandle) -> Result<Vec<RunningScriptInfo>, String> {
    // Feature 007 (T025): Clean up expired scripts first
    cleanup_expired_executions(&app).await;

    let state = app.state::<ScriptExecutionState>();
    let executions = state.executions.read().await;

    // Feature 007: Include all scripts (running + completed within retention period)
    let scripts: Vec<RunningScriptInfo> = executions
        .values()
        .map(|exec| RunningScriptInfo {
            // Original fields
            execution_id: exec.execution_id.clone(),
            script_name: exec.script_name.clone(),
            started_at_ms: exec.started_at.elapsed().as_millis() as u64,
            // Feature 007: New fields
            project_path: exec.project_path.clone(),
            project_name: exec.project_name.clone(),
            started_at: exec.started_at_iso.clone(),
            status: exec.status.clone(),
            exit_code: exec.exit_code,
            completed_at: exec.completed_at.clone(),
        })
        .collect();

    Ok(scripts)
}

/// Get buffered output for a script execution (Feature 007: for reconnection support)
#[tauri::command]
pub async fn get_script_output(
    app: AppHandle,
    execution_id: String,
) -> Result<GetScriptOutputResponse, String> {
    // Feature 007 (T025): Clean up expired scripts first
    cleanup_expired_executions(&app).await;

    let state = app.state::<ScriptExecutionState>();
    let executions = state.executions.read().await;

    if let Some(exec) = executions.get(&execution_id) {
        Ok(GetScriptOutputResponse {
            success: true,
            execution_id,
            output: Some(exec.output_buffer.get_combined_output()),
            lines: Some(exec.output_buffer.get_lines()),
            truncated: exec.output_buffer.is_truncated(),
            buffer_size: exec.output_buffer.size(),
            error: None,
        })
    } else {
        Ok(GetScriptOutputResponse {
            success: false,
            execution_id,
            output: None,
            lines: None,
            truncated: false,
            buffer_size: 0,
            error: Some("Execution not found".to_string()),
        })
    }
}

/// Security: Maximum input size for stdin (1MB)
const MAX_STDIN_INPUT_SIZE: usize = 1_048_576;

/// Write to script stdin (Feature 008: stdin interaction support)
#[tauri::command]
pub async fn write_to_script(
    app: AppHandle,
    execution_id: String,
    input: String,
) -> Result<WriteToScriptResponse, String> {
    // Security: Limit input size
    if input.len() > MAX_STDIN_INPUT_SIZE {
        println!(
            "[write_to_script] SECURITY: Input too large ({} bytes), rejected",
            input.len()
        );
        return Ok(WriteToScriptResponse {
            success: false,
            error: Some(format!(
                "Input too large (max {} bytes)",
                MAX_STDIN_INPUT_SIZE
            )),
        });
    }

    // Extract stdin handle and script name, then release lock
    let (stdin_result, script_name) = {
        let state = app.state::<ScriptExecutionState>();
        let mut executions = state.executions.write().await;

        if let Some(execution) = executions.get_mut(&execution_id) {
            // Check if process is still running
            if execution.status != ExecutionStatus::Running {
                return Ok(WriteToScriptResponse {
                    success: false,
                    error: Some("Process is not running".to_string()),
                });
            }

            let script_name = execution.script_name.clone();

            // Take stdin temporarily for writing
            if execution.stdin.is_some() {
                (Ok(execution.stdin.take()), script_name)
            } else {
                (Err("Stdin handle not available".to_string()), script_name)
            }
        } else {
            return Ok(WriteToScriptResponse {
                success: false,
                error: Some("Execution not found".to_string()),
            });
        }
    };

    // Write to stdin outside the lock
    match stdin_result {
        Ok(Some(mut stdin)) => {
            let bytes = input.as_bytes();

            // Log the write attempt (sanitize for logging - show length and type)
            let input_type = if input.starts_with('\x1b') {
                "ANSI escape sequence"
            } else if input == "\n" {
                "newline"
            } else if input == " " {
                "space"
            } else if input == "\t" {
                "tab"
            } else if input == "\x03" {
                "Ctrl+C"
            } else {
                "text input"
            };
            println!(
                "[write_to_script] Writing {} ({} bytes) to script '{}' (execution: {})",
                input_type,
                bytes.len(),
                script_name,
                execution_id
            );

            match stdin.write_all(bytes).await {
                Ok(_) => {
                    // Put stdin back
                    let state = app.state::<ScriptExecutionState>();
                    let mut executions = state.executions.write().await;
                    if let Some(execution) = executions.get_mut(&execution_id) {
                        execution.stdin = Some(stdin);
                    }
                    Ok(WriteToScriptResponse {
                        success: true,
                        error: None,
                    })
                }
                Err(e) => {
                    println!("[write_to_script] Failed to write to stdin: {}", e);
                    Ok(WriteToScriptResponse {
                        success: false,
                        error: Some(format!("Failed to write to stdin: {}", e)),
                    })
                }
            }
        }
        Ok(None) => Ok(WriteToScriptResponse {
            success: false,
            error: Some("Stdin handle not available".to_string()),
        }),
        Err(e) => Ok(WriteToScriptResponse {
            success: false,
            error: Some(e),
        }),
    }
}

/// Get environment variables for PTY sessions
/// This ensures PTY processes have access to the same environment as other commands
#[tauri::command]
pub async fn get_pty_env() -> Result<std::collections::HashMap<String, String>, String> {
    Ok(path_resolver::build_env_for_child())
}

/// Get command wrapped with Volta if project has volta config
/// This ensures PTY terminal uses the correct Node.js version
#[tauri::command]
pub async fn get_volta_wrapped_command(
    command: String,
    args: Vec<String>,
    cwd: String,
) -> Result<VoltaWrappedCommand, String> {
    let path = Path::new(&cwd);
    let volta_config = match parse_package_json(path) {
        Ok(pj) => pj.volta.clone(),
        Err(_) => None,
    };
    let volta_status = detect_volta();

    if volta_config.is_some() && volta_status.available {
        let volta_cmd = volta_status.path.unwrap_or_else(|| "volta".to_string());
        let mut volta_args = vec!["run".to_string()];

        // Add all configured volta versions
        if let Some(ref config) = volta_config {
            // Node.js version
            if let Some(ref node_version) = config.node {
                volta_args.push("--node".to_string());
                volta_args.push(node_version.clone());
            }
            // npm version (only when running npm)
            if command == "npm" {
                if let Some(ref npm_version) = config.npm {
                    volta_args.push("--npm".to_string());
                    volta_args.push(npm_version.clone());
                }
            }
            // yarn version (only when running yarn)
            if command == "yarn" {
                if let Some(ref yarn_version) = config.yarn {
                    volta_args.push("--yarn".to_string());
                    volta_args.push(yarn_version.clone());
                }
            }
        }

        volta_args.push(command);
        volta_args.extend(args);

        println!(
            "[get_volta_wrapped_command] Wrapping with Volta: {} {:?}",
            volta_cmd, volta_args
        );

        Ok(VoltaWrappedCommand {
            command: volta_cmd,
            args: volta_args,
            use_volta: true,
        })
    } else {
        println!(
            "[get_volta_wrapped_command] No Volta wrap needed: {} {:?}",
            command, args
        );
        Ok(VoltaWrappedCommand {
            command,
            args,
            use_volta: false,
        })
    }
}
