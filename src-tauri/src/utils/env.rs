use std::collections::HashMap;

/// PATH from the user's login shell. A Finder-launched app only gets the system
/// PATH, which misses tools installed via mise, nvm, ~/.local/bin and the like.
static LOGIN_SHELL_PATH: tokio::sync::OnceCell<Option<String>> = tokio::sync::OnceCell::const_new();

/// Read the login shell's PATH once. Later calls return immediately.
pub async fn load_login_shell_path() {
    LOGIN_SHELL_PATH
        .get_or_init(|| async {
            // Windows has no login shell; GUI apps already inherit the user's PATH.
            if cfg!(target_os = "windows") {
                return None;
            }
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
/// Uses the platform separator (`:` on Unix, `;` on Windows).
fn merge_path(parts: &[String]) -> String {
    let mut seen = std::collections::HashSet::new();
    let dirs: Vec<_> = parts
        .iter()
        .flat_map(std::env::split_paths)
        .filter(|dir| !dir.as_os_str().is_empty() && seen.insert(dir.clone()))
        .collect();
    // Cannot fail: entries from split_paths never contain the separator.
    std::env::join_paths(dirs)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(any(target_os = "linux", test))]
fn library_path_without_appdir(library_path: &str, app_dir: &std::path::Path) -> String {
    library_path
        .split(':')
        .filter(|entry| !std::path::Path::new(entry).starts_with(app_dir))
        .collect::<Vec<_>>()
        .join(":")
}

/// Build a HashMap of environment variables suitable for child PTY processes.
/// PATH is the login shell's PATH, then common tool paths (Volta, fnm, Homebrew,
/// Cargo, Go, ~/bin, ~/.local/bin), then the process PATH, so that CLIs installed
/// via those managers are discoverable.
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

    #[cfg(target_os = "linux")]
    if let (Ok(app_dir), Ok(library_path)) =
        (std::env::var("APPDIR"), std::env::var("LD_LIBRARY_PATH"))
    {
        let app_dir = std::path::Path::new(&app_dir);
        if app_dir.is_absolute() {
            // Override even when empty: omitting the key would inherit the bundled libraries.
            env.insert(
                "LD_LIBRARY_PATH".to_string(),
                library_path_without_appdir(&library_path, app_dir),
            );
        }
    }

    env
}

#[cfg(test)]
mod tests {
    use super::merge_path;

    fn join(dirs: &[&str]) -> String {
        std::env::join_paths(dirs)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    #[test]
    fn library_path_removes_only_appdir_entries() {
        let app_dir = std::path::Path::new("/tmp/.mount_skillshare");
        for (input, expected) in [
            (
                "/custom/lib:/tmp/.mount_skillshare/usr/lib:/tmp/.mount_skillshare/lib:/usr/lib",
                "/custom/lib:/usr/lib",
            ),
            ("/tmp/.mount_skillshare/usr/lib", ""),
            ("/custom/lib:/usr/lib", "/custom/lib:/usr/lib"),
            (
                "/tmp/.mount_skillshare-other/lib",
                "/tmp/.mount_skillshare-other/lib",
            ),
            ("", ""),
        ] {
            assert_eq!(super::library_path_without_appdir(input, app_dir), expected);
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn appimage_child_does_not_inherit_bundled_libraries() {
        const MARKER: &str = "SKILLSHARE_TEST_APPIMAGE_ENV";
        if let Ok(expected) = std::env::var(MARKER) {
            let inherited = std::env::var("LD_LIBRARY_PATH").ok();
            let output = std::process::Command::new("/usr/bin/printenv")
                .arg("LD_LIBRARY_PATH")
                .envs(super::build_env_for_child())
                .output()
                .unwrap_or_else(|err| panic!("spawn child: {err}"));
            assert!(output.status.success());
            assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), expected);
            assert_eq!(std::env::var("LD_LIBRARY_PATH").ok(), inherited);
            return;
        }

        // Isolate the AppImage environment from other tests in this process.
        for (app_dir, library_path, expected) in [
            (
                "/tmp/.mount_skillshare",
                "/tmp/.mount_skillshare/usr/lib:/custom/lib",
                "/custom/lib",
            ),
            (
                "/tmp/.mount_skillshare",
                "/tmp/.mount_skillshare/usr/lib",
                "",
            ),
            ("", "/custom/lib", "/custom/lib"),
        ] {
            let output = std::process::Command::new(
                std::env::current_exe().unwrap_or_else(|err| panic!("test executable: {err}")),
            )
            .args([
                "--exact",
                "utils::env::tests::appimage_child_does_not_inherit_bundled_libraries",
                "--nocapture",
            ])
            .env(MARKER, expected)
            .env("APPDIR", app_dir)
            .env("LD_LIBRARY_PATH", library_path)
            .output()
            .unwrap_or_else(|err| panic!("spawn isolated test: {err}"));
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[test]
    fn merge_path_keeps_first_copy_in_order() {
        let parts = [join(&["/a", "/b"]), join(&["/b", "/c"]), join(&["/a"])];
        assert_eq!(merge_path(&parts), join(&["/a", "/b", "/c"]));
    }

    #[test]
    fn merge_path_drops_empty_entries() {
        let parts = [join(&["/a", "", "/b"]), String::new()];
        assert_eq!(merge_path(&parts), join(&["/a", "/b"]));
    }

    #[cfg(unix)]
    #[test]
    fn merge_path_uses_colon_on_unix() {
        let parts = ["/a:/b".to_string(), "/c".to_string()];
        assert_eq!(merge_path(&parts), "/a:/b:/c");
    }

    #[cfg(windows)]
    #[test]
    fn merge_path_uses_semicolon_on_windows() {
        let parts = [r"C:\a;C:\b".to_string(), r"C:\c".to_string()];
        assert_eq!(merge_path(&parts), r"C:\a;C:\b;C:\c");
    }
}
