use std::collections::HashMap;

/// Build a HashMap of environment variables suitable for child PTY processes.
/// Prepends common tool paths (Volta, fnm, Homebrew, Cargo, Go, ~/bin, ~/.local/bin)
/// to the system PATH so that CLIs installed via those managers are discoverable.
pub fn build_env_for_child() -> HashMap<String, String> {
    let mut env: HashMap<String, String> = HashMap::new();

    let home = dirs::home_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    // ── Tool-specific dirs ──────────────────────────────────────────
    let volta_home = format!("{home}/.volta");
    let fnm_dir = format!("{home}/.fnm");

    if std::path::Path::new(&volta_home).exists() {
        env.insert("VOLTA_HOME".to_string(), volta_home.clone());
    }
    if std::path::Path::new(&fnm_dir).exists() {
        env.insert("FNM_DIR".to_string(), fnm_dir.clone());
    }

    // ── PATH construction ───────────────────────────────────────────
    let system_path = std::env::var("PATH").unwrap_or_default();

    let prepend_dirs = [
        format!("{volta_home}/bin"),
        format!("{fnm_dir}/aliases/default/bin"),
        "/opt/homebrew/bin".to_string(),
        "/opt/homebrew/sbin".to_string(),
        "/usr/local/bin".to_string(),
        format!("{home}/.cargo/bin"),
        "/usr/local/go/bin".to_string(),
        format!("{home}/go/bin"),
        format!("{home}/bin"),
        format!("{home}/.local/bin"),
    ];

    let mut path_parts: Vec<String> = prepend_dirs.into_iter().collect();
    if !system_path.is_empty() {
        path_parts.push(system_path);
    }
    env.insert("PATH".to_string(), path_parts.join(":"));

    // ── Locale / terminal ───────────────────────────────────────────
    env.insert("HOME".to_string(), home);
    env.insert("LANG".to_string(), "en_US.UTF-8".to_string());
    env.insert("LC_ALL".to_string(), "en_US.UTF-8".to_string());
    env.insert("TERM".to_string(), "xterm-256color".to_string());

    // ── Pass-through from current process ──────────────────────────
    if let Ok(val) = std::env::var("SSH_AUTH_SOCK") {
        env.insert("SSH_AUTH_SOCK".to_string(), val);
    }
    if let Ok(val) = std::env::var("SHELL") {
        env.insert("SHELL".to_string(), val);
    }

    env
}
