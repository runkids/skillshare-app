use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

const DEFAULT_PORT: u16 = 19420;
const HEALTH_POLL_INTERVAL_MS: u64 = 500;
const HEALTH_POLL_MAX_RETRIES: u32 = 20;

/// Kill orphaned `skillshare` CLI processes listening on the given port range.
/// This handles the case where a previous app instance was killed without
/// graceful shutdown (e.g., dev mode restart, crash, SIGKILL).
///
/// IMPORTANT: Uses exact command match (`-c /^skillshare$/`) to avoid killing
/// `skillshare-app` (the Tauri binary) which would match the prefix.
/// Also skips our own PID as a safety net.
async fn kill_orphaned_servers(base_port: u16, end_port: u16) {
    let own_pid = std::process::id();

    for port in base_port..=end_port {
        if !is_port_in_use(port).await {
            continue;
        }
        // lsof ORs its filters unless -a is given: without it this matched any
        // process on the port (or connected to it) and every `skillshare` process.
        // Only a `skillshare` listening on this port counts as our orphan.
        let output = tokio::process::Command::new("lsof")
            .args([
                "-t",
                "-a",
                &format!("-iTCP:{port}"),
                "-sTCP:LISTEN",
                "-c",
                "/^skillshare$/",
            ])
            .output()
            .await;

        if let Ok(output) = output {
            let pids = String::from_utf8_lossy(&output.stdout);
            for pid_str in pids.trim().lines() {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    if pid == own_pid {
                        continue;
                    }
                    log::info!("Killing orphaned skillshare process (pid={pid}) on port {port}");
                    let _ = tokio::process::Command::new("kill")
                        .args(["-TERM", &pid.to_string()])
                        .output()
                        .await;
                    // Give it a moment to exit
                    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct ServerManager {
    process: Arc<Mutex<Option<Child>>>,
    port: Arc<Mutex<u16>>,
}

impl ServerManager {
    pub fn new() -> Self {
        Self {
            process: Arc::new(Mutex::new(None)),
            port: Arc::new(Mutex::new(DEFAULT_PORT)),
        }
    }

    pub async fn get_port(&self) -> u16 {
        *self.port.lock().await
    }

    /// Start the skillshare UI server. Stops any existing process first.
    /// Tries ports from 19420 to 19430 until one works.
    ///
    /// The server is launched via `current_dir()`:
    /// - Project mode: `cd {project_dir} && skillshare ui -p --port N --no-open`
    /// - Global mode:  `cd ~ && skillshare ui --port N --no-open`
    pub async fn start(
        &self,
        cli_path: &str,
        project_dir: Option<&str>,
        is_project_mode: bool,
    ) -> Result<u16, String> {
        self.stop().await?;

        // Use preferred port from settings, try up to 10 ports from there
        let meta = crate::services::cli_manager::load_meta();
        let base_port = meta.preferred_port.unwrap_or(DEFAULT_PORT);
        let end_port = base_port + 10;

        // Kill any orphaned skillshare processes from a previous app instance
        kill_orphaned_servers(base_port, end_port).await;

        let mut chosen_port = None;

        for port in base_port..=end_port {
            if !is_port_in_use(port).await {
                chosen_port = Some(port);
                break;
            }
        }

        let chosen_port = chosen_port.ok_or_else(|| {
            format!(
                "All ports {base_port}-{end_port} are in use. \
                 Try changing the port in Settings or kill existing processes."
            )
        })?;

        let mut cmd = Command::new(cli_path);

        if is_project_mode {
            cmd.args(["ui", "-p", "--port", &chosen_port.to_string(), "--no-open"]);
        } else {
            cmd.args(["ui", "--port", &chosen_port.to_string(), "--no-open"]);
        }

        if let Some(dir) = project_dir {
            let resolved = if dir.starts_with('~') {
                let home = dirs::home_dir()
                    .ok_or_else(|| "Unable to resolve home directory".to_string())?;
                dir.replacen('~', &home.to_string_lossy(), 1)
            } else {
                dir.to_string()
            };
            cmd.current_dir(&resolved);
        }

        // Finder-launched apps get a minimal PATH; the Web UI shells out to git, brew, editors,
        // and finds agent CLIs (claude, codex, ...) on it.
        crate::utils::env::load_login_shell_path().await;
        cmd.envs(crate::utils::env::build_env_for_child());
        cmd.stdin(std::process::Stdio::null());
        // Keep the server's output so a failed start can say why. Truncated on every start.
        let log_path = server_log_path();
        match std::fs::File::create(&log_path).and_then(|f| Ok((f.try_clone()?, f))) {
            Ok((out, err)) => {
                cmd.stdout(out);
                cmd.stderr(err);
            }
            Err(e) => {
                log::warn!("Cannot write {}: {e}", log_path.display());
                cmd.stdout(std::process::Stdio::null());
                cmd.stderr(std::process::Stdio::null());
            }
        }

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn server: {e}"))?;

        {
            let mut proc = self.process.lock().await;
            *proc = Some(child);
        }
        {
            let mut p = self.port.lock().await;
            *p = chosen_port;
        }

        // Wait for the server to become ready
        self.wait_for_ready(chosen_port).await?;

        Ok(chosen_port)
    }

    /// Kill the running server process if any.
    pub async fn stop(&self) -> Result<(), String> {
        let mut proc = self.process.lock().await;
        if let Some(ref mut child) = *proc {
            child.kill().await.ok();
            child.wait().await.ok();
        }
        *proc = None;
        Ok(())
    }

    /// Check if the server is currently responding on its port.
    pub async fn is_running(&self) -> bool {
        let port = self.get_port().await;
        health_check(port).await
    }

    /// Poll the server health endpoint until ready or timeout (10s).
    async fn wait_for_ready(&self, port: u16) -> Result<(), String> {
        for _ in 0..HEALTH_POLL_MAX_RETRIES {
            if health_check(port).await {
                return Ok(());
            }
            let exited = match self.process.lock().await.as_mut() {
                Some(child) => child.try_wait().ok().flatten(),
                None => None,
            };
            if let Some(status) = exited {
                return Err(with_server_output(format!(
                    "Server exited early ({status})"
                )));
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(HEALTH_POLL_INTERVAL_MS)).await;
        }
        Err(with_server_output(format!(
            "Server on port {port} did not become ready within {}s",
            (HEALTH_POLL_INTERVAL_MS * HEALTH_POLL_MAX_RETRIES as u64) / 1000
        )))
    }
}

const SERVER_LOG_TAIL_LINES: usize = 20;

fn server_log_path() -> std::path::PathBuf {
    crate::utils::paths::logs_dir().join("server.log")
}

/// Append the end of the server's output to a start failure, when there is any.
fn with_server_output(message: String) -> String {
    let output = std::fs::read_to_string(server_log_path()).unwrap_or_default();
    let output = crate::services::cli_manager::strip_ansi(&output);
    match tail(output.trim_start(), SERVER_LOG_TAIL_LINES) {
        "" => message,
        tail => format!("{message}\n\nServer output:\n{tail}"),
    }
}

fn tail(text: &str, lines: usize) -> &str {
    let text = text.trim_end();
    let start = text
        .rmatch_indices('\n')
        .nth(lines.saturating_sub(1))
        .map_or(0, |(i, _)| i + 1);
    &text[start..]
}

/// Check if the server health endpoint responds on the given port.
pub async fn health_check(port: u16) -> bool {
    let url = format!("http://localhost:{port}/api/overview");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build();

    let Ok(client) = client else {
        return false;
    };

    matches!(client.get(&url).send().await, Ok(resp) if resp.status().is_success())
}

/// Quick check if a port is in use by attempting a health check.
async fn is_port_in_use(port: u16) -> bool {
    // Try a TCP connect to see if something is listening
    tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::tail;

    #[test]
    fn tail_keeps_the_last_lines() {
        assert_eq!(tail("a\nb\nc\n", 2), "b\nc");
    }

    #[test]
    fn tail_returns_everything_when_short() {
        assert_eq!(tail("only line\n", 20), "only line");
    }
}
