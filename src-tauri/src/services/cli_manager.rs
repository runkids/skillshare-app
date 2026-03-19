use crate::models::app_state::CliMeta;
use std::path::PathBuf;

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
    // Check PATH via `which`
    if let Ok(output) = tokio::process::Command::new("which")
        .arg("skillshare")
        .output()
        .await
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }

    // Check app bin directory
    let bin = cli_dir().join("skillshare");
    if bin.exists() {
        return Some(bin.to_string_lossy().to_string());
    }

    None
}

/// Run `skillshare version` and return the trimmed stdout.
pub async fn get_version(cli_path: &str) -> Result<String, String> {
    let output = tokio::process::Command::new(cli_path)
        .arg("version")
        .output()
        .await
        .map_err(|e| format!("Failed to run CLI: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("CLI version failed: {stderr}"))
    }
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
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
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

    let asset_prefix = format!("skillshare_darwin_{arch}");

    let assets = body["assets"]
        .as_array()
        .ok_or("Missing assets in release")?;

    let download_url = assets
        .iter()
        .find_map(|a| {
            let name = a["name"].as_str().unwrap_or_default();
            if name.starts_with(&asset_prefix) && name.ends_with(".tar.gz") {
                a["browser_download_url"].as_str().map(|s| s.to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| format!("No matching asset for {asset_prefix}"))?;

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

    let tarball_path = tmp_dir.join("skillshare.tar.gz");
    std::fs::write(&tarball_path, &bytes).map_err(|e| {
        std::fs::remove_dir_all(&tmp_dir).ok();
        format!("Failed to write tarball: {e}")
    })?;

    // Extract with tar
    let extract_status = tokio::process::Command::new("tar")
        .args(["xzf", "skillshare.tar.gz"])
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

    // Move extracted binary to bin dir
    let extracted = tmp_dir.join("skillshare");
    let dest = bin_dir.join("skillshare");

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
