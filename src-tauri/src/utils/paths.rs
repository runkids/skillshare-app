use std::path::PathBuf;

/// The CLI's global config file, independent of its configurable skills source.
pub fn global_config_path() -> Result<PathBuf, String> {
    let env_path = |name| std::env::var_os(name).map(PathBuf::from);
    let (windows_app_data, home) = if cfg!(windows) {
        (env_path("APPDATA"), env_path("USERPROFILE"))
    } else {
        // build_env_for_child passes this same home to the CLI as HOME.
        (None, dirs::home_dir())
    };
    resolve_cli_config_path(
        env_path("SKILLSHARE_CONFIG"),
        env_path("XDG_CONFIG_HOME"),
        windows_app_data,
        home,
    )
}

/// Match the CLI's config.ConfigPath and config.BaseDir precedence.
fn resolve_cli_config_path(
    config_override: Option<PathBuf>,
    xdg_config_home: Option<PathBuf>,
    windows_app_data: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Result<PathBuf, String> {
    let nonempty = |path: &PathBuf| !path.as_os_str().is_empty();
    if let Some(path) = config_override.filter(nonempty) {
        return Ok(path);
    }
    let base = xdg_config_home
        .filter(nonempty)
        .or_else(|| windows_app_data.filter(nonempty))
        .or_else(|| home.filter(nonempty).map(|path| path.join(".config")))
        .ok_or_else(|| "Could not determine the CLI's global config directory".to_string())?;
    Ok(base.join("skillshare").join("config.yaml"))
}

/// Return the app-specific data directory.
///
/// - Production: `~/Library/Application Support/com.skillshare.app/`  (macOS)
/// - Debug:      `~/Library/Application Support/com.skillshare.app-dev/`
///
/// This prevents dev and production builds from stomping on each other's
/// CLI binaries, project lists, and preferences.
pub fn app_data_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));

    let dir_name = if cfg!(debug_assertions) {
        "com.skillshare.app-dev"
    } else {
        "com.skillshare.app"
    };

    let dir = base.join(dir_name);
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// Folder for the app's own log and the CLI server's output.
pub fn logs_dir() -> PathBuf {
    let dir = app_data_dir().join("logs");
    std::fs::create_dir_all(&dir).ok();
    dir
}

#[cfg(test)]
mod tests {
    use super::resolve_cli_config_path;
    use std::path::PathBuf;

    fn path(value: &str) -> Option<PathBuf> {
        Some(PathBuf::from(value))
    }

    #[test]
    fn config_override_preserves_the_exact_filename_and_wins_over_other_paths() {
        let config = PathBuf::from("/custom/settings.yaml");
        assert_eq!(
            resolve_cli_config_path(
                Some(config.clone()),
                path("/xdg"),
                path("/roaming"),
                path("/home/user"),
            ),
            Ok(config)
        );
    }

    #[test]
    fn config_override_is_not_trimmed_or_expanded() {
        for value in [" settings.yaml ", "~/custom.yaml", "relative/settings.yaml"] {
            assert_eq!(
                resolve_cli_config_path(path(value), None, None, None),
                Ok(PathBuf::from(value))
            );
        }
    }

    #[test]
    fn xdg_config_home_takes_priority_over_windows_app_data() {
        assert_eq!(
            resolve_cli_config_path(None, path("/xdg"), path("/roaming"), path("/home/user")),
            Ok(PathBuf::from("/xdg").join("skillshare/config.yaml"))
        );
    }

    #[test]
    fn empty_overrides_fall_through_to_the_home_default() {
        assert_eq!(
            resolve_cli_config_path(path(""), path(""), path(""), path("/home/user")),
            Ok(PathBuf::from("/home/user").join(".config/skillshare/config.yaml"))
        );
    }

    #[test]
    fn unix_defaults_use_dot_config_including_on_macos() {
        for home in ["/home/user", "/Users/user"] {
            assert_eq!(
                resolve_cli_config_path(None, None, None, path(home)),
                Ok(PathBuf::from(home).join(".config/skillshare/config.yaml"))
            );
        }
    }

    #[test]
    fn windows_uses_app_data_with_a_home_fallback() {
        assert_eq!(
            resolve_cli_config_path(None, None, path("/roaming"), path("/profile")),
            Ok(PathBuf::from("/roaming").join("skillshare/config.yaml"))
        );
        assert_eq!(
            resolve_cli_config_path(None, None, None, path("/profile")),
            Ok(PathBuf::from("/profile").join(".config/skillshare/config.yaml"))
        );
    }

    #[test]
    fn explicit_locations_do_not_require_a_home_directory() {
        for (config, xdg, app_data, expected) in [
            (path("/custom.yaml"), None, None, "/custom.yaml"),
            (None, path("/xdg"), None, "/xdg/skillshare/config.yaml"),
            (
                None,
                None,
                path("/roaming"),
                "/roaming/skillshare/config.yaml",
            ),
        ] {
            assert_eq!(
                resolve_cli_config_path(config, xdg, app_data, None),
                Ok(PathBuf::from(expected))
            );
        }
        assert!(resolve_cli_config_path(None, None, None, None).is_err());
        assert!(resolve_cli_config_path(path(""), path(""), path(""), path("")).is_err());
    }
}
