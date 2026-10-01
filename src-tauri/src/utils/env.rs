use std::collections::HashMap;

/// PATH from the user's login shell. A Finder-launched app only gets the system
/// PATH, which misses tools installed via mise, nvm, ~/.local/bin and the like.
static LOGIN_SHELL_PATH: tokio::sync::OnceCell<Option<String>> = tokio::sync::OnceCell::const_new();

/// Read the login shell's PATH once. Later calls return immediately.
pub async fn load_login_shell_path() {
    LOGIN_SHELL_PATH
        .get_or_init(|| async {
            let shell = std::env::var("SHELL").ok()?;
            crate::services::cli_manager::login_shell_path(&shell).await
        })
        .await;
}

/// The login shell's PATH, if it has been read.
pub fn login_shell_path() -> Option<String> {
    LOGIN_SHELL_PATH.get().cloned().flatten()
}

/// Join PATH entries in order, keeping the first copy of each directory.
fn merge_path(parts: &[String]) -> String {
    let mut seen = std::collections::HashSet::new();
    parts
        .iter()
        .flat_map(|p| p.split(':'))
        .filter(|dir| !dir.is_empty() && seen.insert(*dir))
        .collect::<Vec<_>>()
        .join(":")
}

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

    // The terminal's PATH wins, so the app finds the same tools the user's terminal does.
    let mut path_parts: Vec<String> = LOGIN_SHELL_PATH
        .get()
        .cloned()
        .flatten()
        .into_iter()
        .collect();
    path_parts.extend(prepend_dirs);
    path_parts.push(system_path);
    env.insert("PATH".to_string(), merge_path(&path_parts));

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

#[cfg(test)]
mod tests {
    use super::merge_path;

    #[test]
    fn merge_path_keeps_first_copy_in_order() {
        let parts = ["/a:/b".to_string(), "/b:/c".to_string(), "/a".to_string()];
        assert_eq!(merge_path(&parts), "/a:/b:/c");
    }
}
