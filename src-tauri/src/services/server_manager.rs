use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tauri::Emitter;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

const DEFAULT_PORT: u16 = 19420;
const HEALTH_POLL_INTERVAL_MS: u64 = 500;
const HEALTH_POLL_MAX_RETRIES: u32 = 20;

/// Emitted with the new port after the server exited on its own and was restarted.
pub const SERVER_RESTARTED_EVENT: &str = "server-restarted";
/// Emitted when the server keeps exiting and supervision gives up.
pub const SERVER_STOPPED_EVENT: &str = "server-stopped";

/// How often the supervisor checks whether the server process is still alive.
const EXIT_POLL_INTERVAL: Duration = Duration::from_secs(1);
/// Wait before each restart; once exits outrun this list, supervision gives up.
const RESTART_DELAYS: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(5),
];
/// Exits older than this no longer count toward giving up.
const RESTART_WINDOW: Duration = Duration::from_secs(60);

/// How long to wait before restarting, given when the server exited unexpectedly
/// (including the exit just seen), or `None` to stop restarting.
fn next_restart_delay(exits: &[Instant], now: Instant) -> Option<Duration> {
    let recent = exits
        .iter()
        .filter(|t| now.saturating_duration_since(**t) < RESTART_WINDOW)
        .count();
    RESTART_DELAYS.get(recent.saturating_sub(1)).copied()
}

/// The server this app spawned, as recorded in the pidfile.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ServerPid {
    pid: u32,
    port: u16,
}

fn pidfile_path() -> std::path::PathBuf {
    crate::utils::paths::app_data_dir().join("server.pid")
}

fn format_pidfile(server: ServerPid) -> String {
    format!("{} {}", server.pid, server.port)
}

fn parse_pidfile(contents: &str) -> Option<ServerPid> {
    let (pid, port) = contents.trim().split_once(' ')?;
    Some(ServerPid {
        pid: pid.parse().ok()?,
        port: port.parse().ok()?,
    })
}

/// Whether the recorded server is still running and still ours to kill: `lsof_pids`
/// (the `skillshare` processes listening on the recorded port) must include its pid.
/// A pid that died and was reused by anything else fails this check.
fn is_own_orphan(recorded: ServerPid, own_pid: u32, lsof_pids: &str) -> bool {
    recorded.pid != own_pid
        && lsof_pids
            .lines()
            .any(|line| line.trim().parse::<u32>() == Ok(recorded.pid))
}

/// Kill the server a previous app instance left running (crash, SIGKILL, dev restart).
/// Only the pid from our pidfile is a candidate, so a `skillshare ui` the user
/// started in a terminal is never touched.
async fn kill_orphaned_server() {
    let path = pidfile_path();
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return;
    };
    let _ = std::fs::remove_file(&path);
    let Some(recorded) = parse_pidfile(&contents) else {
        return;
    };

    // lsof ORs its filters unless -a is given. `-c /^skillshare$/` is an exact match
    // so `skillshare-app` (the Tauri binary) never qualifies.
    let lsof_pids = tokio::process::Command::new("lsof")
        .args([
            "-t",
            "-a",
            &format!("-iTCP:{}", recorded.port),
            "-sTCP:LISTEN",
            "-c",
            "/^skillshare$/",
        ])
        .output()
        .await
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();

    if !is_own_orphan(recorded, std::process::id(), &lsof_pids) {
        return;
    }
    log::info!(
        "Killing orphaned skillshare server (pid={}) on port {}",
        recorded.pid,
        recorded.port
    );
    let _ = tokio::process::Command::new("kill")
        .args(["-TERM", &recorded.pid.to_string()])
        .output()
        .await;
    // Give it a moment to exit
    tokio::time::sleep(Duration::from_millis(300)).await;
}

/// What `start()` was asked to run, kept so the supervisor can relaunch it.
struct Launch {
    cli_path: String,
    project_dir: Option<String>,
    is_project_mode: bool,
}

#[derive(Clone)]
pub struct ServerManager {
    process: Arc<Mutex<Option<Child>>>,
    port: Arc<Mutex<u16>>,
    /// Bumped by every `stop()` (and so every `start()`); a supervisor only acts
    /// while the generation it was started with is still current.
    generation: Arc<AtomicU64>,
    /// When the server exited unexpectedly, for the restart backoff.
    exits: Arc<Mutex<Vec<Instant>>>,
    app: Arc<OnceLock<tauri::AppHandle>>,
}

impl ServerManager {
    pub fn new() -> Self {
        Self {
            process: Arc::new(Mutex::new(None)),
            port: Arc::new(Mutex::new(DEFAULT_PORT)),
            generation: Arc::new(AtomicU64::new(0)),
            exits: Arc::new(Mutex::new(Vec::new())),
            app: Arc::new(OnceLock::new()),
        }
    }

    /// Give the manager a handle for emitting server events to the web view.
    pub fn set_app_handle(&self, app: tauri::AppHandle) {
        let _ = self.app.set(app);
    }

    pub async fn get_port(&self) -> u16 {
        *self.port.lock().await
    }

    /// Start the skillshare UI server and restart it if it exits on its own.
    /// Stops any existing process first. Tries ports from 19420 to 19430 until one works.
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
        let launch = Launch {
            cli_path: cli_path.to_string(),
            project_dir: project_dir.map(str::to_string),
            is_project_mode,
        };
        self.stop().await?;
        let generation = self.generation.load(Ordering::SeqCst);
        self.exits.lock().await.clear();
        let port = self.launch(&launch).await?;
        let this = self.clone();
        tauri::async_runtime::spawn(async move { this.supervise(launch, generation).await });
        Ok(port)
    }

    /// Replace the server process and wait until it answers. Returns its port.
    async fn launch(&self, launch: &Launch) -> Result<u16, String> {
        let Launch {
            cli_path,
            project_dir,
            is_project_mode,
        } = launch;
        let (project_dir, is_project_mode) = (project_dir.as_deref(), *is_project_mode);
        self.kill_process().await;

        // Use preferred port from settings, try up to 10 ports from there
        let meta = crate::services::cli_manager::load_meta();
        let base_port = meta.preferred_port.unwrap_or(DEFAULT_PORT);
        let end_port = base_port + 10;

        // Kill the server a previous app instance left behind, if any
        kill_orphaned_server().await;

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

        if let Some(pid) = child.id() {
            let record = format_pidfile(ServerPid {
                pid,
                port: chosen_port,
            });
            if let Err(e) = std::fs::write(pidfile_path(), record) {
                log::warn!("Cannot write server pidfile: {e}");
            }
        }

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

    /// Kill the running server process if any and end its supervision.
    pub async fn stop(&self) -> Result<(), String> {
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.kill_process().await;
        Ok(())
    }

    async fn kill_process(&self) {
        let mut proc = self.process.lock().await;
        if let Some(ref mut child) = *proc {
            child.kill().await.ok();
            child.wait().await.ok();
            let _ = std::fs::remove_file(pidfile_path());
        }
        *proc = None;
    }

    /// Restart the server with backoff whenever it exits without `stop()`, until it
    /// keeps exiting (see `next_restart_delay`) or the generation moves on.
    async fn supervise(self, launch: Launch, generation: u64) {
        while let Some(status) = self.wait_for_exit(generation).await {
            log::warn!("skillshare server exited unexpectedly ({status})");
            loop {
                let delay = {
                    let mut exits = self.exits.lock().await;
                    let now = Instant::now();
                    exits.push(now);
                    exits.retain(|t| now.saturating_duration_since(*t) < RESTART_WINDOW);
                    next_restart_delay(&exits, now)
                };
                let Some(delay) = delay else {
                    log::error!("skillshare server keeps exiting; not restarting it again");
                    self.emit(SERVER_STOPPED_EVENT, ());
                    return;
                };
                tokio::time::sleep(delay).await;
                if self.generation.load(Ordering::SeqCst) != generation {
                    return;
                }
                match self.launch(&launch).await {
                    Ok(port) => {
                        log::info!("Restarted skillshare server on port {port}");
                        self.emit(SERVER_RESTARTED_EVENT, port);
                        break;
                    }
                    Err(e) => log::warn!("Failed to restart skillshare server: {e}"),
                }
            }
        }
    }

    /// Wait until the server process exits. `None` once `stop()` has ended this generation.
    async fn wait_for_exit(&self, generation: u64) -> Option<std::process::ExitStatus> {
        loop {
            tokio::time::sleep(EXIT_POLL_INTERVAL).await;
            let mut proc = self.process.lock().await;
            if self.generation.load(Ordering::SeqCst) != generation {
                return None;
            }
            if let Some(status) = proc.as_mut()?.try_wait().ok().flatten() {
                return Some(status);
            }
        }
    }

    fn emit<S: serde::Serialize + Clone>(&self, event: &str, payload: S) {
        if let Some(app) = self.app.get() {
            if let Err(e) = app.emit(event, payload) {
                log::warn!("Failed to emit {event}: {e}");
            }
        }
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

pub(crate) fn tail(text: &str, lines: usize) -> &str {
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
    use super::*;

    fn exits_ago(now: Instant, secs: &[u64]) -> Vec<Instant> {
        secs.iter().map(|s| now - Duration::from_secs(*s)).collect()
    }

    #[test]
    fn restart_delays_back_off_with_each_recent_exit() {
        let now = Instant::now();
        let delays: Vec<_> = [vec![0], vec![5, 0], vec![10, 5, 0]]
            .iter()
            .map(|ago| next_restart_delay(&exits_ago(now, ago), now))
            .collect();
        assert_eq!(
            delays,
            [
                Some(Duration::from_secs(1)),
                Some(Duration::from_secs(2)),
                Some(Duration::from_secs(5))
            ]
        );
    }

    #[test]
    fn restarting_gives_up_after_too_many_exits_within_a_minute() {
        let now = Instant::now();
        assert_eq!(
            next_restart_delay(&exits_ago(now, &[30, 20, 10, 0]), now),
            None
        );
    }

    #[test]
    fn exits_older_than_a_minute_do_not_count() {
        let now = Instant::now();
        assert_eq!(
            next_restart_delay(&exits_ago(now, &[300, 200, 100, 0]), now),
            Some(Duration::from_secs(1))
        );
    }

    #[test]
    fn pidfile_round_trips() {
        let server = ServerPid {
            pid: 4242,
            port: 19421,
        };
        assert_eq!(parse_pidfile(&format_pidfile(server)), Some(server));
    }

    #[test]
    fn malformed_pidfile_is_ignored() {
        assert_eq!(parse_pidfile("not a pid"), None);
    }

    const RECORDED: ServerPid = ServerPid {
        pid: 4242,
        port: 19420,
    };

    #[test]
    fn recorded_server_still_listening_is_killed() {
        assert!(is_own_orphan(RECORDED, 1, "4242\n"));
    }

    #[test]
    fn other_skillshare_on_the_port_is_left_alone() {
        // A `skillshare ui` the user started themselves: not the pid we recorded.
        assert!(!is_own_orphan(RECORDED, 1, "5151\n"));
    }

    #[test]
    fn recorded_server_no_longer_listening_is_left_alone() {
        assert!(!is_own_orphan(RECORDED, 1, ""));
    }

    #[test]
    fn own_pid_is_never_killed() {
        assert!(!is_own_orphan(RECORDED, 4242, "4242\n"));
    }

    #[test]
    fn tail_keeps_the_last_lines() {
        assert_eq!(tail("a\nb\nc\n", 2), "b\nc");
    }

    #[test]
    fn tail_returns_everything_when_short() {
        assert_eq!(tail("only line\n", 20), "only line");
    }
}
