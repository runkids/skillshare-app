// Path resolution utilities for macOS GUI apps
// GUI apps don't inherit shell PATH, so we need to find tools manually

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::RwLock;

/// Cache for resolved tool paths
static TOOL_PATH_CACHE: Lazy<RwLock<HashMap<String, Option<String>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Cached PATH string for child processes
static CACHED_PATH: Lazy<String> = Lazy::new(|| build_path_string());

/// Get reliable home directory for macOS GUI apps
/// Falls back to multiple sources since GUI apps may not have HOME set
pub fn get_home_dir() -> Option<String> {
    // Try HOME environment variable first
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return Some(home);
        }
    }

    // Fallback: use dirs crate (uses macOS native APIs)
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            return Some(home.to_string_lossy().to_string());
        }
    }

    // Last resort: try common macOS user path patterns
    if let Ok(user) = std::env::var("USER") {
        let home = format!("/Users/{}", user);
        if std::path::Path::new(&home).exists() {
            return Some(home);
        }
    }

    None
}

/// Build comprehensive PATH string for child processes
fn build_path_string() -> String {
    let home = get_home_dir().unwrap_or_else(|| "/tmp".to_string());

    let paths = vec![
        // Volta (highest priority for Node.js version management)
        format!("{}/.volta/bin", home),
        // Cargo/Rust
        format!("{}/.cargo/bin", home),
        // Homebrew (Apple Silicon)
        "/opt/homebrew/bin".to_string(),
        "/opt/homebrew/sbin".to_string(),
        // Homebrew (Intel)
        "/usr/local/bin".to_string(),
        "/usr/local/sbin".to_string(),
        // System paths
        "/usr/bin".to_string(),
        "/bin".to_string(),
        "/usr/sbin".to_string(),
        "/sbin".to_string(),
        // Xcode Command Line Tools
        "/Library/Developer/CommandLineTools/usr/bin".to_string(),
    ];

    paths.join(":")
}

/// Get the PATH string for child processes
pub fn get_path() -> &'static str {
    &CACHED_PATH
}

/// Get the SSH_AUTH_SOCK path for macOS GUI apps
/// macOS uses launchd to manage ssh-agent, the socket is usually at a fixed location
pub fn get_ssh_auth_sock() -> Option<String> {
    // First try environment variable
    if let Ok(sock) = std::env::var("SSH_AUTH_SOCK") {
        if !sock.is_empty() && std::path::Path::new(&sock).exists() {
            return Some(sock);
        }
    }

    // macOS launchd-managed ssh-agent sockets
    // Try to find it using glob patterns
    let patterns = [
        "/private/tmp/com.apple.launchd.*/Listeners",
        "/var/folders/*/*/T/com.apple.launchd.*/Listeners",
    ];

    for pattern in &patterns {
        if let Ok(entries) = glob::glob(pattern) {
            for entry in entries.flatten() {
                if entry.exists() {
                    return Some(entry.to_string_lossy().to_string());
                }
            }
        }
    }

    None
}

/// Common system paths to search for tools
fn get_system_paths() -> Vec<PathBuf> {
    let home = get_home_dir().unwrap_or_default();

    vec![
        // Volta (highest priority)
        PathBuf::from(format!("{}/.volta/bin", home)),
        // Homebrew (Apple Silicon)
        PathBuf::from("/opt/homebrew/bin"),
        // Homebrew (Intel)
        PathBuf::from("/usr/local/bin"),
        // System
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
        // Xcode Command Line Tools
        PathBuf::from("/Library/Developer/CommandLineTools/usr/bin"),
        // Cargo/Rust
        PathBuf::from(format!("{}/.cargo/bin", home)),
    ]
}

/// Find a tool by searching common paths
/// Returns the full path to the tool if found
pub fn find_tool(tool_name: &str) -> Option<String> {
    // Check cache first
    if let Ok(cache) = TOOL_PATH_CACHE.read() {
        if let Some(cached) = cache.get(tool_name) {
            return cached.clone();
        }
    }

    let result = find_tool_uncached(tool_name);

    // Cache the result
    if let Ok(mut cache) = TOOL_PATH_CACHE.write() {
        cache.insert(tool_name.to_string(), result.clone());
    }

    result
}

fn find_tool_uncached(tool_name: &str) -> Option<String> {
    // Search common paths directly (more reliable than 'which' in GUI apps)
    for base_path in get_system_paths() {
        let tool_path = base_path.join(tool_name);
        if tool_path.exists() {
            return Some(tool_path.to_string_lossy().to_string());
        }
    }

    // Special handling for Node.js tools - check NVM versions
    if matches!(
        tool_name,
        "node" | "npm" | "npx" | "corepack" | "yarn" | "pnpm"
    ) {
        if let Some(nvm_path) = find_nvm_tool(tool_name) {
            return Some(nvm_path);
        }
    }

    None
}

/// Find a tool in NVM installations
fn find_nvm_tool(tool_name: &str) -> Option<String> {
    let home = get_home_dir()?;
    let nvm_dir = format!("{}/.nvm/versions/node", home);

    if let Ok(entries) = std::fs::read_dir(&nvm_dir) {
        // Sort entries to get the latest version first
        let mut versions: Vec<_> = entries.flatten().collect();
        versions.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

        for entry in versions {
            let tool_path = entry.path().join("bin").join(tool_name);
            if tool_path.exists() {
                return Some(tool_path.to_string_lossy().to_string());
            }
        }
    }

    None
}

/// Get the full path for a tool, with fallback to the tool name itself
pub fn get_tool_path(tool_name: &str) -> String {
    find_tool(tool_name).unwrap_or_else(|| tool_name.to_string())
}

/// Create a Command with proper environment for macOS GUI apps
/// This is the recommended way to execute external tools
pub fn create_command(tool_name: &str) -> Command {
    let tool_path = get_tool_path(tool_name);
    let mut cmd = Command::new(&tool_path);

    // Clear Volta internal variables that should not persist across processes
    // _VOLTA_TOOL_RECURSION causes Volta shim to skip its logic and use fallback node
    cmd.env_remove("_VOLTA_TOOL_RECURSION");

    // Set essential environment variables
    let home = get_home_dir();
    if let Some(ref home) = home {
        cmd.env("HOME", home);

        // Volta support: set VOLTA_HOME for volta-managed projects
        let volta_home = format!("{}/.volta", home);
        if std::path::Path::new(&volta_home).exists() {
            cmd.env("VOLTA_HOME", &volta_home);
        }

        // fnm support
        let fnm_dir = format!("{}/.fnm", home);
        if std::path::Path::new(&fnm_dir).exists() {
            cmd.env("FNM_DIR", &fnm_dir);
        }

        // pnpm support - only set if user has explicitly configured
        let pnpm_home = get_pnpm_home(home);
        cmd.env("PNPM_HOME", &pnpm_home);

        // Only set PNPM_STORE_DIR if explicitly configured by user
        // Otherwise let pnpm use its default (~/.pnpm-store/v10)
        if let Some(store_dir) = std::env::var("PNPM_STORE_DIR")
            .ok()
            .or_else(|| read_npmrc_store_dir(home))
        {
            cmd.env("PNPM_STORE_DIR", &store_dir);
        }
    }

    // Set PATH so child processes can find tools
    cmd.env("PATH", get_path());

    // Set SSH_AUTH_SOCK for git operations
    if let Some(sock) = get_ssh_auth_sock() {
        cmd.env("SSH_AUTH_SOCK", &sock);
    }

    // Set LANG for proper encoding
    cmd.env("LANG", "en_US.UTF-8");
    cmd.env("LC_ALL", "en_US.UTF-8");

    // Terminal/TTY settings for interactive tools and dev servers
    cmd.env("TERM", "xterm-256color");
    cmd.env("FORCE_COLOR", "1"); // Enable colored output
    cmd.env("CI", "false"); // Not in CI environment

    // Node.js specific settings
    cmd.env(
        "NODE_ENV",
        std::env::var("NODE_ENV").unwrap_or_else(|_| "development".to_string()),
    );

    cmd
}

/// Clear the tool path cache (useful after environment changes)
#[allow(dead_code)]
pub fn clear_cache() {
    if let Ok(mut cache) = TOOL_PATH_CACHE.write() {
        cache.clear();
    }
}

/// Create an async Command (tokio) with proper environment for macOS GUI apps
/// Use this for async operations like in deploy.rs
pub fn create_async_command(tool_name: &str) -> tokio::process::Command {
    let tool_path = get_tool_path(tool_name);
    let mut cmd = tokio::process::Command::new(&tool_path);

    // Clear Volta internal variables that should not persist across processes
    // _VOLTA_TOOL_RECURSION causes Volta shim to skip its logic and use fallback node
    cmd.env_remove("_VOLTA_TOOL_RECURSION");

    // Set essential environment variables
    let home = get_home_dir();
    if let Some(ref home) = home {
        cmd.env("HOME", home);

        // Volta support: set VOLTA_HOME for volta-managed projects
        let volta_home = format!("{}/.volta", home);
        if std::path::Path::new(&volta_home).exists() {
            cmd.env("VOLTA_HOME", &volta_home);
        }

        // fnm support
        let fnm_dir = format!("{}/.fnm", home);
        if std::path::Path::new(&fnm_dir).exists() {
            cmd.env("FNM_DIR", &fnm_dir);
        }

        // pnpm support - only set if user has explicitly configured
        let pnpm_home = get_pnpm_home(home);
        cmd.env("PNPM_HOME", &pnpm_home);

        // Only set PNPM_STORE_DIR if explicitly configured by user
        // Otherwise let pnpm use its default (~/.pnpm-store/v10)
        if let Some(store_dir) = std::env::var("PNPM_STORE_DIR")
            .ok()
            .or_else(|| read_npmrc_store_dir(home))
        {
            cmd.env("PNPM_STORE_DIR", &store_dir);
        }
    }

    // Set PATH so child processes can find tools
    cmd.env("PATH", get_path());

    // Set SSH_AUTH_SOCK for git operations
    if let Some(sock) = get_ssh_auth_sock() {
        cmd.env("SSH_AUTH_SOCK", &sock);
    }

    // Set LANG for proper encoding
    cmd.env("LANG", "en_US.UTF-8");
    cmd.env("LC_ALL", "en_US.UTF-8");

    cmd
}

/// Get PNPM_HOME path with fallback to defaults
fn get_pnpm_home(home: &str) -> String {
    std::env::var("PNPM_HOME").ok().unwrap_or_else(|| {
        let macos_default = format!("{}/Library/pnpm", home);
        let linux_default = format!("{}/.local/share/pnpm", home);
        if std::path::Path::new(&macos_default).exists() {
            macos_default
        } else if std::path::Path::new(&linux_default).exists() {
            linux_default
        } else {
            #[cfg(target_os = "macos")]
            {
                macos_default
            }
            #[cfg(not(target_os = "macos"))]
            {
                linux_default
            }
        }
    })
}

/// Read store-dir from pnpm config files
/// Checks multiple locations: pnpm global rc, ~/.npmrc
/// Returns None if not found
fn read_npmrc_store_dir(home: &str) -> Option<String> {
    // pnpm config file locations (in priority order)
    let config_paths = [
        // pnpm global config (macOS)
        format!("{}/Library/Preferences/pnpm/rc", home),
        // pnpm global config (Linux)
        format!("{}/.config/pnpm/rc", home),
        // Legacy pnpm location
        format!("{}/Library/pnpm/rc", home),
        format!("{}/.local/share/pnpm/rc", home),
        // npm config (also used by pnpm)
        format!("{}/.npmrc", home),
    ];

    for config_path in &config_paths {
        if let Some(store_dir) = read_store_dir_from_file(config_path, home) {
            return Some(store_dir);
        }
    }
    None
}

/// Parse a single config file for store-dir setting
fn read_store_dir_from_file(path: &str, home: &str) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;

    for line in content.lines() {
        let line = line.trim();
        // Skip comments and empty lines
        if line.starts_with('#') || line.starts_with(';') || line.is_empty() {
            continue;
        }
        // Look for store-dir setting (pnpm uses this key)
        if let Some(value) = line.strip_prefix("store-dir=") {
            let value = value.trim();
            // Handle quoted values
            let value = value.trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                // Expand ~ to home directory
                return Some(value.replace('~', home));
            }
        }
    }
    None
}

/// Configure environment variables for a std::process::Command
/// Use this when you need to create Command manually (e.g., for process_group on Unix)
/// This applies the same environment setup as create_command()
pub fn configure_std_command_env(cmd: &mut Command) {
    // Clear Volta internal variables that should not persist across processes
    // _VOLTA_TOOL_RECURSION causes Volta shim to skip its logic and use fallback node
    cmd.env_remove("_VOLTA_TOOL_RECURSION");

    // Set essential environment variables
    let home = get_home_dir();
    if let Some(ref home) = home {
        cmd.env("HOME", home);

        // Volta support: set VOLTA_HOME for volta-managed projects
        let volta_home = format!("{}/.volta", home);
        if std::path::Path::new(&volta_home).exists() {
            cmd.env("VOLTA_HOME", &volta_home);
        }

        // fnm support
        let fnm_dir = format!("{}/.fnm", home);
        if std::path::Path::new(&fnm_dir).exists() {
            cmd.env("FNM_DIR", &fnm_dir);
        }

        // pnpm support
        let pnpm_home = get_pnpm_home(home);
        cmd.env("PNPM_HOME", &pnpm_home);

        // Only set PNPM_STORE_DIR if explicitly configured by user
        if let Some(store_dir) = std::env::var("PNPM_STORE_DIR")
            .ok()
            .or_else(|| read_npmrc_store_dir(home))
        {
            cmd.env("PNPM_STORE_DIR", &store_dir);
        }
    }

    // Set PATH so child processes can find tools
    cmd.env("PATH", get_path());

    // Set SSH_AUTH_SOCK for git operations
    if let Some(sock) = get_ssh_auth_sock() {
        cmd.env("SSH_AUTH_SOCK", &sock);
    }

    // Set LANG for proper encoding
    cmd.env("LANG", "en_US.UTF-8");
    cmd.env("LC_ALL", "en_US.UTF-8");

    // Terminal/TTY settings for interactive tools and dev servers
    cmd.env("TERM", "xterm-256color");
    cmd.env("FORCE_COLOR", "1");
    cmd.env("CI", "false");

    // Node.js specific settings
    cmd.env(
        "NODE_ENV",
        std::env::var("NODE_ENV").unwrap_or_else(|_| "development".to_string()),
    );
}

/// Build environment variables map for child processes
/// This ensures child processes have access to necessary paths
pub fn build_env_for_child() -> HashMap<String, String> {
    let mut env = HashMap::new();

    // Clear Volta internal variables that should not persist across processes
    // _VOLTA_TOOL_RECURSION causes Volta shim to skip its logic and use fallback node
    env.insert("_VOLTA_TOOL_RECURSION".to_string(), String::new());

    if let Some(home) = get_home_dir() {
        // Volta support
        let volta_home = format!("{}/.volta", &home);
        if std::path::Path::new(&volta_home).exists() {
            env.insert("VOLTA_HOME".to_string(), volta_home);
        }

        // fnm support
        let fnm_dir = format!("{}/.fnm", &home);
        if std::path::Path::new(&fnm_dir).exists() {
            env.insert("FNM_DIR".to_string(), fnm_dir);
        }

        // pnpm support - only set if user has explicitly configured
        let pnpm_home = get_pnpm_home(&home);
        env.insert("PNPM_HOME".to_string(), pnpm_home);

        // Only set PNPM_STORE_DIR if explicitly configured by user
        // Otherwise let pnpm use its default (~/.pnpm-store/v10)
        if let Some(store_dir) = std::env::var("PNPM_STORE_DIR")
            .ok()
            .or_else(|| read_npmrc_store_dir(&home))
        {
            env.insert("PNPM_STORE_DIR".to_string(), store_dir);
        }

        env.insert("HOME".to_string(), home);
    }

    env.insert("PATH".to_string(), get_path().to_string());

    if let Some(sock) = get_ssh_auth_sock() {
        env.insert("SSH_AUTH_SOCK".to_string(), sock);
    }

    // Encoding
    env.insert("LANG".to_string(), "en_US.UTF-8".to_string());
    env.insert("LC_ALL".to_string(), "en_US.UTF-8".to_string());

    // Terminal settings
    env.insert("TERM".to_string(), "xterm-256color".to_string());

    // User's default shell (for spawning login shells)
    if let Ok(shell) = std::env::var("SHELL") {
        env.insert("SHELL".to_string(), shell);
    } else {
        env.insert("SHELL".to_string(), "/bin/zsh".to_string());
    }
    env.insert("FORCE_COLOR".to_string(), "1".to_string());
    env.insert("CI".to_string(), "false".to_string());

    // Node.js settings
    env.insert(
        "NODE_ENV".to_string(),
        std::env::var("NODE_ENV").unwrap_or_else(|_| "development".to_string()),
    );

    env
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_home_dir() {
        let home = get_home_dir();
        assert!(home.is_some());
        assert!(std::path::Path::new(&home.unwrap()).exists());
    }

    #[test]
    fn test_find_git() {
        let git = find_tool("git");
        assert!(git.is_some());
    }

    #[test]
    fn test_create_command() {
        let cmd = create_command("git");
        // Command should be created successfully
        assert!(cmd.get_program().to_string_lossy().contains("git"));
    }
}
