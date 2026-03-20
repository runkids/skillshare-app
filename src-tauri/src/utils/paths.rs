use std::path::PathBuf;

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
