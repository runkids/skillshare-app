use std::ffi::OsStr;

/// A child process that opens no console window on Windows.
///
/// The release app is a GUI-subsystem binary with no console of its own, so Windows
/// gives every console program it starts (the CLI, git, `where`) a new visible window.
/// Every spawn goes through here; children of a hidden-console process share it.
/// Nobody can answer a git credential prompt on a hidden console, so git fails at
/// once instead of waiting forever, as it does on macOS and Linux without a tty.
pub fn command(program: impl AsRef<OsStr>) -> tokio::process::Command {
    #[allow(unused_mut)]
    let mut cmd = tokio::process::Command::new(program);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .env("GIT_TERMINAL_PROMPT", "0");
    cmd
}

#[cfg(test)]
mod tests {
    /// A spawn that bypasses `command` brings back the console windows of issue #106.
    #[test]
    fn app_code_spawns_only_through_command() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut stack = vec![src];
        let mut offenders = Vec::new();
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {dir:?}: {e}")) {
                let path = entry
                    .unwrap_or_else(|e| panic!("entry in {dir:?}: {e}"))
                    .path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.ends_with("utils/process.rs") || path.extension() != Some("rs".as_ref()) {
                    continue;
                }
                let text =
                    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
                // Test modules spawn themselves with std::process; only app code is checked.
                let app_code = text
                    .split("#[cfg(test)]\nmod tests")
                    .next()
                    .unwrap_or_default();
                if app_code.contains("Command::new(") {
                    offenders.push(path.display().to_string());
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "use utils::process::command in {offenders:?}"
        );
    }
}
