// Agent Dispatcher
// Manages AI agent subprocesses: spawning, tracking, timeout, cancellation.

use crate::local_models::config::SpecForgeConfig;
use crate::local_models::spec::Spec;
use crate::repositories::agent_run_repo::{self, AgentRun};
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Tracks a running agent subprocess.
struct AgentProcess {
    pid: u32,
    #[allow(dead_code)]
    spec_id: String,
    #[allow(dead_code)]
    started_at: String,
}

/// Manages concurrent AI agent processes with limits and lifecycle tracking.
pub struct AgentDispatcher {
    running: Arc<Mutex<HashMap<String, AgentProcess>>>,
    max_concurrent: u32,
}

impl AgentDispatcher {
    pub fn new(max_concurrent: u32) -> Self {
        Self {
            running: Arc::new(Mutex::new(HashMap::new())),
            max_concurrent,
        }
    }

    /// Dispatch an agent for a spec. Returns the agent_run ID.
    ///
    /// Errors if an agent is already running for this spec or the
    /// concurrent limit is reached.
    pub async fn dispatch(
        &self,
        spec: &Spec,
        prompt_template: &str,
        timeout_ms: u64,
        project_dir: &Path,
        conn: &Connection,
        config: &SpecForgeConfig,
    ) -> Result<String, String> {
        // 1. Check if an agent is already running for this spec
        if self.is_running(&spec.id) {
            return Err(format!(
                "Agent already running for spec: {}",
                spec.id
            ));
        }

        // 2. Check concurrent limit
        {
            let running = self
                .running
                .lock()
                .map_err(|e| format!("Lock error: {e}"))?;
            if running.len() as u32 >= self.max_concurrent {
                return Err(format!(
                    "Concurrent agent limit reached ({}/{})",
                    running.len(),
                    self.max_concurrent
                ));
            }
        }

        // 3. Render prompt from template
        let prompt = prompt_template
            .replace("{{spec.id}}", &spec.id)
            .replace("{{spec.title}}", &spec.title)
            .replace("{{spec.status}}", &spec.status)
            .replace(
                "{{spec.phase}}",
                spec.workflow_phase.as_deref().unwrap_or(""),
            );

        // 4. Build command args from config
        let args: Vec<String> = config
            .agent
            .args
            .iter()
            .map(|a| a.replace("{prompt}", &prompt))
            .collect();

        // 5. Generate run ID
        let run_id = format!("agent-{}", uuid::Uuid::new_v4());
        let now = chrono::Utc::now().to_rfc3339();
        let current_phase = spec.workflow_phase.clone().unwrap_or_default();

        // 6. Spawn subprocess
        let mut child = tokio::process::Command::new(&config.agent.command)
            .args(&args)
            .current_dir(project_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                format!(
                    "Failed to spawn agent '{}': {e}",
                    config.agent.command
                )
            })?;

        let pid = child
            .id()
            .ok_or_else(|| "Failed to get PID of spawned agent".to_string())?;

        // 7. Insert agent_run record
        let agent_run = AgentRun {
            id: run_id.clone(),
            spec_id: spec.id.clone(),
            phase: current_phase,
            prompt: prompt.clone(),
            status: "running".to_string(),
            pid: Some(pid),
            started_at: now.clone(),
            finished_at: None,
            error: None,
        };
        agent_run_repo::insert_agent_run(conn, &agent_run)?;

        // 8. Track the process
        {
            let mut running = self
                .running
                .lock()
                .map_err(|e| format!("Lock error: {e}"))?;
            running.insert(
                spec.id.clone(),
                AgentProcess {
                    pid,
                    spec_id: spec.id.clone(),
                    started_at: now,
                },
            );
        }

        // 9. Spawn background monitor task
        let running_map = self.running.clone();
        let spec_id = spec.id.clone();
        let run_id_clone = run_id.clone();

        tokio::spawn(async move {
            let result = tokio::time::timeout(
                std::time::Duration::from_millis(timeout_ms),
                child.wait(),
            )
            .await;

            // Remove from running map
            if let Ok(mut map) = running_map.lock() {
                map.remove(&spec_id);
            }

            match result {
                Ok(Ok(exit_status)) => {
                    let status = if exit_status.success() {
                        "completed"
                    } else {
                        "failed"
                    };
                    let error = if exit_status.success() {
                        None
                    } else {
                        Some(format!("Process exited with: {exit_status}"))
                    };
                    log::info!(
                        "[AgentDispatcher] agent={} spec={} status={}",
                        run_id_clone,
                        spec_id,
                        status
                    );
                    // Note: we can't update the DB here because Connection is
                    // not Send. The status will be updated on next poll/cleanup.
                    let _ = (status, error);
                }
                Ok(Err(e)) => {
                    log::error!(
                        "[AgentDispatcher] agent={} spec={} wait error: {}",
                        run_id_clone,
                        spec_id,
                        e
                    );
                }
                Err(_) => {
                    // Timeout — kill the process
                    log::warn!(
                        "[AgentDispatcher] agent={} spec={} timed out after {}ms, killing",
                        run_id_clone,
                        spec_id,
                        timeout_ms
                    );
                    #[cfg(unix)]
                    {
                        // SIGTERM first, then SIGKILL after 5s
                        unsafe {
                            libc::kill(pid as i32, libc::SIGTERM);
                        }
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        unsafe {
                            libc::kill(pid as i32, libc::SIGKILL);
                        }
                    }
                }
            }
        });

        Ok(run_id)
    }

    /// Check if an agent is currently running for a spec.
    pub fn is_running(&self, spec_id: &str) -> bool {
        self.running
            .lock()
            .map(|map| map.contains_key(spec_id))
            .unwrap_or(false)
    }

    /// Cancel a running agent for a spec.
    pub fn cancel(&self, spec_id: &str) -> Result<(), String> {
        let mut running = self
            .running
            .lock()
            .map_err(|e| format!("Lock error: {e}"))?;

        let process = running
            .remove(spec_id)
            .ok_or_else(|| format!("No running agent for spec: {spec_id}"))?;

        #[cfg(unix)]
        unsafe {
            libc::kill(process.pid as i32, libc::SIGTERM);
        }

        #[cfg(not(unix))]
        {
            let _ = process.pid;
            log::warn!("[AgentDispatcher] Process kill not implemented on this platform");
        }

        Ok(())
    }

    /// Clean up orphaned agent runs on startup.
    ///
    /// Finds agent_runs with status "running" whose PIDs no longer exist,
    /// and marks them as "orphaned".
    pub fn cleanup_orphans(&self, conn: &Connection) -> Result<usize, String> {
        let running_agents = agent_run_repo::get_running_agents(conn)?;
        let mut cleaned = 0;

        for agent in &running_agents {
            let is_alive = if let Some(pid) = agent.pid {
                process_exists(pid)
            } else {
                false
            };

            if !is_alive {
                agent_run_repo::update_agent_status(
                    conn,
                    &agent.id,
                    "orphaned",
                    Some("Process no longer exists (cleaned up on startup)"),
                )?;
                cleaned += 1;
                log::info!(
                    "[AgentDispatcher] Cleaned orphaned agent: {} (pid={:?})",
                    agent.id,
                    agent.pid
                );
            }
        }

        Ok(cleaned)
    }

    /// Get the count of currently running agents.
    pub fn running_count(&self) -> usize {
        self.running
            .lock()
            .map(|map| map.len())
            .unwrap_or(0)
    }
}

/// Check if a process with the given PID exists.
fn process_exists(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // Avoid overflow: i32::MAX is the largest valid PID value.
        // Values above that would wrap negative and have special meaning in kill().
        if pid > i32::MAX as u32 {
            return false;
        }
        // kill(pid, 0) checks existence without sending a signal
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_dispatcher() {
        let dispatcher = AgentDispatcher::new(5);
        assert_eq!(dispatcher.running_count(), 0);
        assert!(!dispatcher.is_running("some-spec"));
    }

    #[test]
    fn test_cancel_nonexistent() {
        let dispatcher = AgentDispatcher::new(5);
        let result = dispatcher.cancel("no-such-spec");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No running agent"));
    }

    #[test]
    fn test_process_exists_self() {
        // Our own PID should exist
        let pid = std::process::id();
        assert!(process_exists(pid));
    }

    #[test]
    fn test_process_exists_invalid() {
        // PID 0 should not be a valid target
        // (on Unix, kill(0, 0) sends to process group — use a very high PID)
        assert!(!process_exists(u32::MAX));
    }

    #[test]
    fn test_cleanup_orphans() {
        use crate::utils::schema::run_migrations;

        let conn = Connection::open_in_memory().expect("open in-memory db");
        run_migrations(&conn).expect("run migrations");

        // Insert a dummy spec for FK
        conn.execute(
            r#"
            INSERT INTO specs (id, schema_id, title, status, file_path, created_at, updated_at)
            VALUES ('spec-test', 'test', 'Test', 'draft', 'test.md', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')
            "#,
            [],
        )
        .expect("insert dummy spec");

        // Insert a "running" agent with an invalid PID
        let orphan = AgentRun {
            id: "run-orphan".to_string(),
            spec_id: "spec-test".to_string(),
            phase: "implement".to_string(),
            prompt: "test".to_string(),
            status: "running".to_string(),
            pid: Some(u32::MAX), // definitely not running
            started_at: "2026-01-01T00:00:00Z".to_string(),
            finished_at: None,
            error: None,
        };
        agent_run_repo::insert_agent_run(&conn, &orphan).expect("insert");

        let dispatcher = AgentDispatcher::new(3);
        let cleaned = dispatcher.cleanup_orphans(&conn).expect("cleanup");
        assert_eq!(cleaned, 1);

        // Verify status updated
        let updated = agent_run_repo::get_agent_run(&conn, "run-orphan")
            .expect("get")
            .expect("should exist");
        assert_eq!(updated.status, "orphaned");
    }
}
