use crate::models::app_state::CliMeta;
use std::path::PathBuf;

/// Persist CLI metadata after a GitHub release download/upgrade.
pub fn save_release_meta(version: String, path: &str) -> Result<(), String> {
    let mut meta = load_meta();
    meta.version = Some(version);
    meta.path = Some(path.to_string());
    meta.source = Some("github-release".to_string());
    meta.installed_at = Some(chrono::Utc::now().to_rfc3339());
    save_meta(&meta)
}

/// Directory where the app stores its own copy of the CLI binary.
pub fn cli_dir() -> PathBuf {
    let dir = crate::utils::paths::app_data_dir().join("bin");
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// Path to the CLI metadata JSON file.
fn meta_path() -> PathBuf {
    crate::utils::paths::app_data_dir().join("cli-meta.json")
}

// ── Meta persistence ───────────────────────────────────────────────

pub fn load_meta() -> CliMeta {
    let path = meta_path();
    if path.exists() {
        let data = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        CliMeta::default()
    }
}

pub fn save_meta(meta: &CliMeta) -> Result<(), String> {
    let path = meta_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let data = serde_json::to_string_pretty(meta).map_err(|e| format!("Serialize error: {e}"))?;
    std::fs::write(&path, data).map_err(|e| format!("Write error: {e}"))
}

// ── CLI detection ──────────────────────────────────────────────────

/// Try to find the `skillshare` binary.
///
/// Search order:
///   1. System PATH (enriched with Homebrew / Cargo / common dirs on macOS)
///   2. Well-known install locations for each platform
///   3. App-managed bin directory (in-app download)
///
/// GUI apps on macOS launched from Finder only inherit a minimal system PATH,
/// so we inject the enriched PATH from `build_env_for_child()` to also cover
/// Homebrew, Cargo, and other common install locations.
pub async fn detect_cli() -> Option<String> {
    let env = crate::utils::env::build_env_for_child();

    // 1. Check PATH via `which` (Unix) or `where` (Windows)
    let find_cmd = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };
    if let Ok(output) = tokio::process::Command::new(find_cmd)
        .arg("skillshare")
        .envs(&env)
        .output()
        .await
    {
        if output.status.success() {
            // `where` on Windows may return multiple lines; take the first
            let path = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or_default()
                .trim()
                .to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }

    // 2. Check well-known install locations per platform
    //    - curl|sh  → /usr/local/bin/skillshare  (covered by PATH above)
    //    - brew     → /opt/homebrew/bin/skillshare (covered by PATH above)
    //    - Windows  → %LOCALAPPDATA%\Programs\skillshare\skillshare.exe
    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let bin = PathBuf::from(local_app_data)
                .join("Programs")
                .join("skillshare")
                .join("skillshare.exe");
            if bin.exists() {
                return Some(bin.to_string_lossy().to_string());
            }
        }
    }

    // 3. Check app-managed bin directory (in-app download)
    let bin_name = if cfg!(target_os = "windows") {
        "skillshare.exe"
    } else {
        "skillshare"
    };
    let bin = cli_dir().join(bin_name);
    if bin.exists() {
        return Some(bin.to_string_lossy().to_string());
    }

    None
}

/// Get the global config directory by running `skillshare status --json`
/// and extracting the parent of `source.path`.
/// e.g. source.path = `~/.config/skillshare/skills` → returns `~/.config/skillshare`
pub async fn get_global_config_dir(cli_path: &str) -> Result<String, String> {
    let output = exec(
        cli_path,
        &["status".to_string(), "--json".to_string()],
        None,
    )
    .await?;

    let status: serde_json::Value = serde_json::from_str(&output)
        .map_err(|e| format!("Failed to parse CLI status JSON: {e}"))?;

    let source_path = status["source"]["path"]
        .as_str()
        .ok_or("CLI status JSON missing 'source.path' field")?;

    Ok(std::path::Path::new(source_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default())
}

/// Run `skillshare version` and extract the semver version string.
/// The CLI outputs ASCII art with ANSI codes; we strip those and find the version.
pub async fn get_version(cli_path: &str) -> Result<String, String> {
    let env = crate::utils::env::build_env_for_child();
    let output = tokio::process::Command::new(cli_path)
        .arg("version")
        .envs(&env)
        .output()
        .await
        .map_err(|e| format!("Failed to run CLI: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("CLI version failed: {stderr}"));
    }

    let raw = String::from_utf8_lossy(&output.stdout).to_string();
    extract_version(&raw).ok_or_else(|| "Could not parse version from CLI output".to_string())
}

/// Best guess at how the CLI at `path` got there, for when no install was recorded.
fn infer_source(path: &str, app_bin_dir: &std::path::Path) -> &'static str {
    let windows_path = path.to_ascii_lowercase().replace('/', "\\");
    if std::path::Path::new(path).starts_with(app_bin_dir) {
        "github-release"
    } else if path.contains("/Cellar/")
        || path.starts_with("/opt/homebrew/")
        || path.starts_with("/home/linuxbrew/")
    {
        "homebrew"
    } else if windows_path.contains("\\programs\\skillshare\\") {
        "powershell-installer"
    } else {
        "system-path"
    }
}

/// [`infer_source`] on the resolved path, so Homebrew's `/usr/local/bin` symlinks count too.
pub fn guess_source(cli_path: &str) -> String {
    let resolved = std::fs::canonicalize(cli_path)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| cli_path.to_string());
    let app_bin = std::fs::canonicalize(cli_dir()).unwrap_or_else(|_| cli_dir());
    infer_source(&resolved, &app_bin).to_string()
}

/// Modification time of the CLI binary in ms since the Unix epoch.
pub fn binary_modified_ms(cli_path: &str) -> Option<u64> {
    let modified = std::fs::metadata(cli_path).ok()?.modified().ok()?;
    let since_epoch = modified.duration_since(std::time::UNIX_EPOCH).ok()?;
    u64::try_from(since_epoch.as_millis()).ok()
}

/// Whether the cached version must be re-read because the binary changed on disk.
fn needs_version_refresh(meta: &CliMeta, modified_ms: u64) -> bool {
    meta.version.is_none() || meta.binary_modified_ms != Some(modified_ms)
}

/// Re-read and persist the CLI version when the binary changed since it was cached.
/// Upgrades can happen outside the app (web UI, terminal), which leaves the cache stale.
pub async fn refresh_cached_version(meta: &mut CliMeta) -> Result<(), String> {
    let Some(path) = meta.path.clone() else {
        return Ok(());
    };
    let Some(modified_ms) = binary_modified_ms(&path) else {
        return Ok(());
    };
    if !needs_version_refresh(meta, modified_ms) {
        return Ok(());
    }
    meta.version = Some(get_version(&path).await?);
    meta.binary_modified_ms = Some(modified_ms);
    save_meta(meta)
}

/// Strip ANSI escape codes (CSI and OSC sequences) from a string.
pub fn strip_ansi(raw: &str) -> String {
    let mut clean = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if let Some(&next) = chars.peek() {
                if next == '[' {
                    // CSI sequence: consume until ASCII letter
                    chars.next();
                    while let Some(&c) = chars.peek() {
                        chars.next();
                        if c.is_ascii_alphabetic() {
                            break;
                        }
                    }
                } else if next == ']' {
                    // OSC sequence: consume until BEL or ST
                    chars.next();
                    while let Some(c) = chars.next() {
                        if c == '\x07' {
                            break;
                        }
                        if c == '\x1b' {
                            chars.next(); // skip backslash in ST
                            break;
                        }
                    }
                }
            }
        } else {
            clean.push(ch);
        }
    }
    clean
}

/// Strip ANSI escape codes and extract a semver version (e.g. "v0.17.6" or "0.17.6").
fn extract_version(raw: &str) -> Option<String> {
    let clean = strip_ansi(raw);

    // Find version pattern: v?MAJOR.MINOR.PATCH
    for word in clean.split_whitespace() {
        let trimmed = word.trim_start_matches('v');
        let parts: Vec<&str> = trimmed.split('.').collect();
        if parts.len() >= 2 && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit())) {
            return Some(format!("v{trimmed}"));
        }
    }
    None
}

// ── CLI execution ──────────────────────────────────────────────────

/// Execute an arbitrary CLI command and return its stdout.
pub async fn exec(
    cli_path: &str,
    args: &[String],
    working_dir: Option<&str>,
) -> Result<String, String> {
    let env = crate::utils::env::build_env_for_child();
    let mut cmd = tokio::process::Command::new(cli_path);
    cmd.args(args).envs(&env);
    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Failed to exec CLI: {e}"))?;

    if output.status.success() {
        let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(strip_ansi(&raw))
    } else {
        let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr).trim().to_string());
        let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout).trim().to_string());
        Err(format!(
            "CLI exited with {}: {}",
            output.status,
            if stderr.is_empty() { stdout } else { stderr }
        ))
    }
}

// ── Platform-aware install ─────────────────────────────────────────

/// Event emitted for every line the installer prints.
pub const INSTALL_OUTPUT_EVENT: &str = "cli-install-output";

const INSTALL_SH_URL: &str = "https://raw.githubusercontent.com/runkids/skillshare/main/install.sh";
const INSTALL_PS1_URL: &str =
    "https://raw.githubusercontent.com/runkids/skillshare/main/install.ps1";

#[derive(serde::Serialize)]
pub struct InstallPlatform {
    pub os: &'static str,
    pub arch: &'static str,
    pub brew: bool,
}

#[derive(serde::Serialize, Clone)]
struct InstallOutput {
    stream: &'static str,
    line: String,
}

#[derive(serde::Serialize)]
pub struct InstallResult {
    pub path: String,
    pub version: String,
}

/// Sender used by `cancel_install` to stop the running installer.
static INSTALL_CANCEL: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>> =
    std::sync::Mutex::new(None);

/// How to put the CLI's folder on the user's terminal PATH.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathHint {
    /// The CLI's folder, with the home prefix shown as `~`.
    pub dir: String,
    /// A one-line command that persists the folder on PATH for `shell`.
    pub command: String,
}

/// Build the PATH hint for `cli_dir`, or `None` when the login shell already has it.
fn path_hint_for(cli_dir: &str, home: &str, shell: &str, shell_path: &str) -> Option<PathHint> {
    if shell_path
        .split(':')
        .any(|p| p.trim_end_matches('/') == cli_dir)
    {
        return None;
    }
    let rel = cli_dir.strip_prefix(home).filter(|r| r.starts_with('/'));
    let shown = rel.map_or_else(|| cli_dir.to_string(), |r| format!("~{r}"));
    let expr = rel.map_or_else(|| cli_dir.to_string(), |r| format!("$HOME{r}"));
    let export = |rc: &str| format!(r#"echo 'export PATH="{expr}:$PATH"' >> {rc}"#);
    let command = match shell.rsplit('/').next().unwrap_or(shell) {
        "fish" => format!("fish_add_path {shown}"),
        "zsh" => export("~/.zshrc"),
        "bash" if cfg!(target_os = "macos") => export("~/.bash_profile"),
        "bash" => export("~/.bashrc"),
        _ => export("~/.profile"),
    };
    Some(PathHint {
        dir: shown,
        command,
    })
}

/// PATH as the user's login shell sets it up (GUI apps do not inherit it).
async fn login_shell_path(shell: &str) -> Option<String> {
    let run = tokio::process::Command::new(shell)
        .args(["-ilc", "/usr/bin/env"])
        .stdin(std::process::Stdio::null())
        .output();
    let output = tokio::time::timeout(std::time::Duration::from_secs(5), run)
        .await
        .ok()?
        .ok()?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|l| l.strip_prefix("PATH=").map(str::to_string))
}

/// Suggest a PATH fix when the CLI works in the app but not in the user's terminal.
/// Windows installs update PATH themselves, and the app-only copy is meant to stay private.
pub async fn path_hint(cli_path: &str) -> Option<PathHint> {
    if cfg!(target_os = "windows") {
        return None;
    }
    let dir = std::path::Path::new(cli_path).parent()?;
    if dir.starts_with(crate::utils::paths::app_data_dir()) {
        return None;
    }
    let shell = std::env::var("SHELL").ok()?;
    let shell_path = login_shell_path(&shell).await?;
    let home = dirs::home_dir()?;
    path_hint_for(
        &dir.to_string_lossy(),
        &home.to_string_lossy(),
        &shell,
        &shell_path,
    )
}

/// Whether the user can run the CLI from a terminal, and what to do if not.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalAccess {
    /// The CLI is the app's own copy and nothing on the terminal side points at it yet.
    pub needs_link: bool,
    /// `~/.local/bin/skillshare` links to the app's copy.
    pub linked: bool,
    /// The folder the terminal would use is not on the login shell's PATH.
    pub path_hint: Option<PathHint>,
}

/// Where the app links its own CLI copy so terminals can run it.
fn terminal_link_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".local/bin/skillshare"))
}

fn links_to(link: &std::path::Path, target: &std::path::Path) -> bool {
    std::fs::read_link(link).is_ok_and(|dest| dest == target)
}

pub async fn terminal_access(cli_path: &str) -> TerminalAccess {
    if cfg!(target_os = "windows") {
        return TerminalAccess::default();
    }
    let target = std::path::Path::new(cli_path);
    if !target.starts_with(crate::utils::paths::app_data_dir()) {
        return TerminalAccess {
            path_hint: path_hint(cli_path).await,
            ..TerminalAccess::default()
        };
    }
    match terminal_link_path().filter(|link| links_to(link, target)) {
        Some(link) => TerminalAccess {
            linked: true,
            path_hint: path_hint(&link.to_string_lossy()).await,
            ..TerminalAccess::default()
        },
        None => TerminalAccess {
            needs_link: true,
            ..TerminalAccess::default()
        },
    }
}

/// Symlink `link` to `cli_path`. Only an old link to the app's copy or a dangling link is
/// replaced; a real file or a link to another CLI belongs to the user and is left alone.
#[cfg(unix)]
fn link_cli_at(
    cli_path: &str,
    link: &std::path::Path,
    app_dir: &std::path::Path,
) -> Result<(), String> {
    if let Ok(meta) = std::fs::symlink_metadata(link) {
        let replaceable = meta.file_type().is_symlink()
            && std::fs::read_link(link)
                .is_ok_and(|dest| dest.starts_with(app_dir) || !dest.exists());
        if !replaceable {
            return Err(format!(
                "{} already exists. Remove it first if you want the app's copy there.",
                link.display()
            ));
        }
        std::fs::remove_file(link)
            .map_err(|e| format!("Could not replace {}: {e}", link.display()))?;
    }
    if let Some(dir) = link.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    }
    std::os::unix::fs::symlink(cli_path, link)
        .map_err(|e| format!("Could not create {}: {e}", link.display()))
}

/// Make the app's CLI copy runnable from a terminal via `~/.local/bin/skillshare`.
pub fn link_for_terminal(cli_path: &str) -> Result<(), String> {
    #[cfg(unix)]
    {
        let link = terminal_link_path().ok_or("Home folder not found")?;
        link_cli_at(cli_path, &link, &crate::utils::paths::app_data_dir())
    }
    #[cfg(not(unix))]
    {
        let _ = cli_path;
        Err("Linking the CLI for the terminal is not supported on Windows".to_string())
    }
}

/// OS, CPU architecture and whether Homebrew is on the (enriched) PATH.
pub async fn detect_install_platform() -> InstallPlatform {
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "macos"
    };
    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    };

    let brew = !cfg!(target_os = "windows")
        && tokio::process::Command::new("which")
            .arg("brew")
            .envs(crate::utils::env::build_env_for_child())
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false);

    InstallPlatform { os, arch, brew }
}

/// Program and arguments for an install method.
///
/// The unix script installs into `~/.local/bin` (on the enriched PATH) because the
/// default `/usr/local/bin` needs `sudo`, which cannot prompt inside the app.
fn install_invocation(method: &str) -> Result<(&'static str, Vec<String>), String> {
    match method {
        "brew" if !cfg!(target_os = "windows") => {
            Ok(("sh", vec!["-c".into(), "brew install skillshare".into()]))
        }
        "script" if !cfg!(target_os = "windows") => Ok((
            "sh",
            vec![
                "-c".into(),
                format!(
                    "mkdir -p \"$HOME/.local/bin\" && curl -fsSL {INSTALL_SH_URL} | INSTALL_DIR=\"$HOME/.local/bin\" sh"
                ),
            ],
        )),
        "powershell" if cfg!(target_os = "windows") => Ok((
            "powershell",
            vec![
                "-NoProfile".into(),
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-Command".into(),
                format!("irm {INSTALL_PS1_URL} | iex"),
            ],
        )),
        other => Err(format!("Install method '{other}' is not supported on this platform")),
    }
}

/// Forward each line of `reader` to the frontend; returns the last non-empty line.
async fn forward_lines<R>(app: tauri::AppHandle, stream: &'static str, reader: R) -> Option<String>
where
    R: tokio::io::AsyncRead + Unpin,
{
    use tauri::Emitter;
    use tokio::io::AsyncBufReadExt;

    let mut lines = tokio::io::BufReader::new(reader).lines();
    let mut last = None;
    loop {
        match lines.next_line().await {
            Ok(Some(raw)) => {
                let line = strip_ansi(&raw);
                if !line.trim().is_empty() {
                    last = Some(line.clone());
                }
                if let Err(e) = app.emit(INSTALL_OUTPUT_EVENT, InstallOutput { stream, line }) {
                    log::warn!("Failed to emit installer output: {e}");
                }
            }
            Ok(None) => break,
            Err(e) => {
                log::warn!("Failed to read installer {stream}: {e}");
                break;
            }
        }
    }
    last
}

/// Run the installer for `method`, streaming output, then verify the CLI with `version`.
pub async fn install_cli(app: tauri::AppHandle, method: &str) -> Result<InstallResult, String> {
    let (program, args) = install_invocation(method)?;
    let mut cmd = tokio::process::Command::new(program);
    cmd.args(&args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    if !cfg!(target_os = "windows") {
        // The enriched PATH is unix-style; Windows keeps its own environment.
        cmd.envs(crate::utils::env::build_env_for_child());
    }
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start the {method} installer: {e}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or("Installer stdout was not captured")?;
    let stderr = child
        .stderr
        .take()
        .ok_or("Installer stderr was not captured")?;
    let out_task = tokio::spawn(forward_lines(app.clone(), "stdout", stdout));
    let err_task = tokio::spawn(forward_lines(app, "stderr", stderr));

    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
    set_cancel_sender(Some(cancel_tx))?;

    let status = tokio::select! {
        status = child.wait() => Some(status),
        _ = cancel_rx => None,
    };
    set_cancel_sender(None)?;

    let Some(status) = status else {
        child
            .kill()
            .await
            .map_err(|e| format!("Failed to stop the installer: {e}"))?;
        return Err("Installation cancelled".to_string());
    };

    let status = status.map_err(|e| format!("Failed to wait for the installer: {e}"))?;
    let last_out = out_task
        .await
        .map_err(|e| format!("Installer output task failed: {e}"))?;
    let last_err = err_task
        .await
        .map_err(|e| format!("Installer output task failed: {e}"))?;
    if !status.success() {
        let detail = last_err.or(last_out).unwrap_or_default();
        return Err(format!("Installer exited with {status}: {detail}"));
    }

    let path = detect_cli()
        .await
        .ok_or("Installer finished but the skillshare binary was not found on PATH")?;
    let version = get_version(&path).await?;

    let mut meta = load_meta();
    meta.version = Some(version.clone());
    meta.path = Some(path.clone());
    meta.source = Some(format!("install-{method}"));
    meta.installed_at = Some(chrono::Utc::now().to_rfc3339());
    save_meta(&meta)?;

    Ok(InstallResult { path, version })
}

fn set_cancel_sender(tx: Option<tokio::sync::oneshot::Sender<()>>) -> Result<(), String> {
    let mut guard = INSTALL_CANCEL
        .lock()
        .map_err(|e| format!("Install state lock poisoned: {e}"))?;
    *guard = tx;
    Ok(())
}

/// Stop the running installer. Returns false when none is running.
pub fn cancel_install() -> Result<bool, String> {
    let mut guard = INSTALL_CANCEL
        .lock()
        .map_err(|e| format!("Install state lock poisoned: {e}"))?;
    Ok(guard.take().is_some_and(|tx| tx.send(()).is_ok()))
}

// ── Release checking & download ────────────────────────────────────

/// Returns (version_tag, download_url) for the latest GitHub release.
pub async fn check_latest_release() -> Result<(String, String), String> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.github.com/repos/runkids/skillshare/releases/latest")
        .header("User-Agent", "skillshare-app")
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    if resp.status() == reqwest::StatusCode::FORBIDDEN {
        return Err("GitHub API rate limit exceeded. Try again later.".to_string());
    }

    if !resp.status().is_success() {
        return Err(format!("GitHub API returned status {}", resp.status()));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse release JSON: {e}"))?;

    let tag = body["tag_name"]
        .as_str()
        .ok_or("Missing tag_name in release")?
        .to_string();

    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "amd64"
    };

    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "darwin"
    };

    let asset_prefix = format!("skillshare_{}_{os}_{arch}", tag.trim_start_matches('v'));
    let ext = if cfg!(target_os = "windows") {
        ".zip"
    } else {
        ".tar.gz"
    };

    let assets = body["assets"]
        .as_array()
        .ok_or("Missing assets in release")?;

    let download_url = assets
        .iter()
        .find_map(|a| {
            let name = a["name"].as_str().unwrap_or_default();
            if name.starts_with(&asset_prefix) && name.ends_with(ext) {
                a["browser_download_url"].as_str().map(|s| s.to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| format!("No matching asset for {asset_prefix}{ext}"))?;

    Ok((tag, download_url))
}

/// Download the CLI tarball, extract it, and install to the app bin dir.
/// Returns the path to the installed binary.
pub async fn download_cli(url: &str) -> Result<String, String> {
    let bin_dir = cli_dir();
    let tmp_dir = bin_dir.join("_tmp_download");

    // Clean up any previous partial download
    if tmp_dir.exists() {
        std::fs::remove_dir_all(&tmp_dir).ok();
    }
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("Failed to create temp dir: {e}"))?;

    // Download tarball
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .header("User-Agent", "skillshare-app")
        .send()
        .await
        .map_err(|e| format!("Download failed: {e}"))?;

    if !resp.status().is_success() {
        std::fs::remove_dir_all(&tmp_dir).ok();
        return Err(format!("Download returned status {}", resp.status()));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to read download body: {e}"))?;

    let archive_name = if cfg!(target_os = "windows") {
        "skillshare.zip"
    } else {
        "skillshare.tar.gz"
    };
    let archive_path = tmp_dir.join(archive_name);
    std::fs::write(&archive_path, &bytes).map_err(|e| {
        std::fs::remove_dir_all(&tmp_dir).ok();
        format!("Failed to write archive: {e}")
    })?;

    // Extract
    #[cfg(target_os = "windows")]
    {
        let extract_status = tokio::process::Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                    archive_path.display(),
                    tmp_dir.display()
                ),
            ])
            .status()
            .await
            .map_err(|e| {
                std::fs::remove_dir_all(&tmp_dir).ok();
                format!("Failed to run PowerShell extract: {e}")
            })?;
        if !extract_status.success() {
            std::fs::remove_dir_all(&tmp_dir).ok();
            return Err("ZIP extraction failed".to_string());
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let extract_status = tokio::process::Command::new("tar")
            .args(["xzf", archive_name])
            .current_dir(&tmp_dir)
            .status()
            .await
            .map_err(|e| {
                std::fs::remove_dir_all(&tmp_dir).ok();
                format!("Failed to run tar: {e}")
            })?;
        if !extract_status.success() {
            std::fs::remove_dir_all(&tmp_dir).ok();
            return Err("tar extraction failed".to_string());
        }
    }

    // Move extracted binary to bin dir
    let bin_name = if cfg!(target_os = "windows") {
        "skillshare.exe"
    } else {
        "skillshare"
    };
    let extracted = tmp_dir.join(bin_name);
    let dest = bin_dir.join(bin_name);

    if !extracted.exists() {
        std::fs::remove_dir_all(&tmp_dir).ok();
        return Err("Extracted binary not found in tarball".to_string());
    }

    // Remove old binary if present
    if dest.exists() {
        std::fs::remove_file(&dest).ok();
    }

    std::fs::rename(&extracted, &dest).map_err(|e| {
        std::fs::remove_dir_all(&tmp_dir).ok();
        format!("Failed to move binary: {e}")
    })?;

    // chmod +x
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&dest, perms)
            .map_err(|e| format!("Failed to set permissions: {e}"))?;
    }

    // Clean up temp dir
    std::fs::remove_dir_all(&tmp_dir).ok();

    Ok(dest.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_path_hint_when_login_shell_already_has_the_folder() {
        let hint = path_hint_for("/h/.local/bin", "/h", "/bin/zsh", "/usr/bin:/h/.local/bin/");
        assert_eq!(hint, None);
    }

    #[test]
    fn zsh_hint_appends_home_relative_export_to_zshrc() {
        let hint = path_hint_for("/h/.local/bin", "/h", "/bin/zsh", "/usr/bin");
        assert_eq!(
            hint.map(|h| h.command).as_deref(),
            Some(r#"echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc"#)
        );
    }

    #[test]
    fn fish_hint_uses_fish_add_path() {
        let hint = path_hint_for("/h/.local/bin", "/h", "/usr/local/bin/fish", "/usr/bin");
        assert_eq!(
            hint.map(|h| h.command).as_deref(),
            Some("fish_add_path ~/.local/bin")
        );
    }

    fn meta_with(version: Option<&str>, modified_ms: Option<u64>) -> CliMeta {
        CliMeta {
            version: version.map(str::to_string),
            binary_modified_ms: modified_ms,
            ..CliMeta::default()
        }
    }

    #[test]
    fn keeps_cached_version_while_binary_is_unchanged() {
        let meta = meta_with(Some("v0.22.2"), Some(10));
        assert!(!needs_version_refresh(&meta, 10));
    }

    #[test]
    fn rereads_version_after_binary_was_replaced() {
        let meta = meta_with(Some("v0.22.2"), Some(10));
        assert!(needs_version_refresh(&meta, 20));
    }

    #[test]
    fn rereads_version_when_nothing_is_cached() {
        assert!(needs_version_refresh(&meta_with(None, Some(10)), 10));
    }

    /// A fresh scratch folder holding `app/skillshare` (the app's copy) and `bin/`.
    #[cfg(unix)]
    fn link_sandbox(name: &str) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("ss-link-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let app = root.join("app");
        std::fs::create_dir_all(&app).ok();
        std::fs::write(app.join("skillshare"), "").ok();
        (root.clone(), app, root.join("bin/skillshare"))
    }

    #[cfg(unix)]
    #[test]
    fn links_the_app_copy_into_a_new_bin_folder() {
        let (_root, app, link) = link_sandbox("new");
        let cli = app.join("skillshare");
        let linked = link_cli_at(&cli.to_string_lossy(), &link, &app).is_ok();
        assert_eq!((linked, links_to(&link, &cli)), (true, true));
    }

    #[cfg(unix)]
    #[test]
    fn never_replaces_a_real_cli_file() {
        let (_root, app, link) = link_sandbox("file");
        std::fs::create_dir_all(link.parent().unwrap_or(&app)).ok();
        std::fs::write(&link, "user binary").ok();
        let result = link_cli_at(&app.join("skillshare").to_string_lossy(), &link, &app);
        assert!(
            result.is_err() && std::fs::read_to_string(&link).is_ok_and(|s| s == "user binary")
        );
    }

    #[cfg(unix)]
    #[test]
    fn replaces_a_dangling_link() {
        let (root, app, link) = link_sandbox("dangling");
        std::fs::create_dir_all(link.parent().unwrap_or(&app)).ok();
        std::os::unix::fs::symlink(root.join("gone"), &link).ok();
        let cli = app.join("skillshare");
        let linked = link_cli_at(&cli.to_string_lossy(), &link, &app).is_ok();
        assert_eq!((linked, links_to(&link, &cli)), (true, true));
    }

    #[test]
    fn app_downloaded_cli_is_not_labelled_system_path() {
        let bin = std::path::Path::new("/data/com.skillshare.app/bin");
        assert_eq!(
            infer_source("/data/com.skillshare.app/bin/skillshare", bin),
            "github-release"
        );
    }

    #[test]
    fn homebrew_cellar_path_is_homebrew() {
        let bin = std::path::Path::new("/data/app/bin");
        assert_eq!(
            infer_source("/usr/local/Cellar/skillshare/0.23.0/bin/skillshare", bin),
            "homebrew"
        );
    }

    #[test]
    fn powershell_install_dir_is_recognised() {
        let bin = std::path::Path::new("/data/app/bin");
        let path = r"C:\Users\me\AppData\Local\Programs\skillshare\skillshare.exe";
        assert_eq!(infer_source(path, bin), "powershell-installer");
    }

    #[test]
    fn other_locations_stay_system_path() {
        let bin = std::path::Path::new("/data/app/bin");
        assert_eq!(
            infer_source("/home/me/.local/bin/skillshare", bin),
            "system-path"
        );
    }

    #[test]
    fn rejects_unknown_install_method() {
        assert!(install_invocation("npm").is_err());
    }

    #[test]
    fn brew_and_script_are_unix_only_and_powershell_is_windows_only() {
        assert_eq!(
            install_invocation("brew").is_ok(),
            !cfg!(target_os = "windows")
        );
        assert_eq!(
            install_invocation("script").is_ok(),
            !cfg!(target_os = "windows")
        );
        assert_eq!(
            install_invocation("powershell").is_ok(),
            cfg!(target_os = "windows")
        );
    }

    #[test]
    fn script_installs_to_user_bin_without_sudo() {
        if let Ok((_, args)) = install_invocation("script") {
            assert!(args[1].contains("INSTALL_DIR=\"$HOME/.local/bin\""));
        }
    }
}
