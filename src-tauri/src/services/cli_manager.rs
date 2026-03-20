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
    let dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.skillshare.app")
        .join("bin");
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// Path to the CLI metadata JSON file.
fn meta_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.skillshare.app")
        .join("cli-meta.json")
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

/// Try to find the `skillshare` binary: first on PATH, then in the app bin dir.
pub async fn detect_cli() -> Option<String> {
    // Check PATH via `which` (Unix) or `where` (Windows)
    let find_cmd = if cfg!(target_os = "windows") { "where" } else { "which" };
    if let Ok(output) = tokio::process::Command::new(find_cmd)
        .arg("skillshare")
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

    // Check app bin directory
    let bin_name = if cfg!(target_os = "windows") { "skillshare.exe" } else { "skillshare" };
    let bin = cli_dir().join(bin_name);
    if bin.exists() {
        return Some(bin.to_string_lossy().to_string());
    }

    None
}

/// Run `skillshare version` and extract the semver version string.
/// The CLI outputs ASCII art with ANSI codes; we strip those and find the version.
pub async fn get_version(cli_path: &str) -> Result<String, String> {
    let output = tokio::process::Command::new(cli_path)
        .arg("version")
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

/// Strip ANSI escape codes (CSI and OSC sequences) from a string.
fn strip_ansi(raw: &str) -> String {
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
    let mut cmd = tokio::process::Command::new(cli_path);
    cmd.args(args);
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

    let asset_prefix = format!("skillshare_{os}_{arch}");
    let ext = if cfg!(target_os = "windows") { ".zip" } else { ".tar.gz" };

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

    let archive_name = if cfg!(target_os = "windows") { "skillshare.zip" } else { "skillshare.tar.gz" };
    let archive_path = tmp_dir.join(archive_name);
    std::fs::write(&archive_path, &bytes).map_err(|e| {
        std::fs::remove_dir_all(&tmp_dir).ok();
        format!("Failed to write archive: {e}")
    })?;

    // Extract
    #[cfg(target_os = "windows")]
    {
        let extract_status = tokio::process::Command::new("powershell")
            .args(["-Command", &format!(
                "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                archive_path.display(), tmp_dir.display()
            )])
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
    let bin_name = if cfg!(target_os = "windows") { "skillshare.exe" } else { "skillshare" };
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
