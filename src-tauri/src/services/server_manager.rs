use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

const DEFAULT_PORT: u16 = 19420;
const MAX_PORT: u16 = 19430;
const HEALTH_POLL_INTERVAL_MS: u64 = 500;
const HEALTH_POLL_MAX_RETRIES: u32 = 20;

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
    pub async fn start(
        &self,
        cli_path: &str,
        project_dir: Option<&str>,
    ) -> Result<u16, String> {
        self.stop().await?;

        let mut chosen_port = DEFAULT_PORT;

        for port in DEFAULT_PORT..=MAX_PORT {
            if !is_port_in_use(port).await {
                chosen_port = port;
                break;
            }
            if port == MAX_PORT {
                return Err(format!(
                    "All ports {DEFAULT_PORT}-{MAX_PORT} are in use"
                ));
            }
        }

        let mut cmd = Command::new(cli_path);
        cmd.args(["ui", "--port", &chosen_port.to_string(), "--no-open"]);

        if let Some(dir) = project_dir {
            cmd.args(["--dir", dir]);
        }

        // Prevent the child from inheriting stdin and suppress stdout/stderr
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());

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

    /// Restart the server with updated parameters.
    pub async fn restart(
        &self,
        cli_path: &str,
        project_dir: Option<&str>,
    ) -> Result<u16, String> {
        self.stop().await?;
        self.start(cli_path, project_dir).await
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
            tokio::time::sleep(tokio::time::Duration::from_millis(HEALTH_POLL_INTERVAL_MS)).await;
        }
        Err(format!(
            "Server on port {port} did not become ready within {}s",
            (HEALTH_POLL_INTERVAL_MS * HEALTH_POLL_MAX_RETRIES as u64) / 1000
        ))
    }
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
