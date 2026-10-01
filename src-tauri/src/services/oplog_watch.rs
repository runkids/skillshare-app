//! Watch the skillshare CLI's operation log, so the update and source-health badges follow
//! changes made outside the app (the CLI web UI or a terminal). Every CLI command appends a
//! JSONL entry; once a burst of state-changing entries settles, both badges are re-checked.

use crate::services::{project_store, source_health, update_watch};
use notify::{EventKind, RecursiveMode, Watcher};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tokio::time::Instant;

/// Quiet period after the last state-changing entry, so a burst of commands refreshes once.
const SETTLE: Duration = Duration::from_millis(1500);

/// How long to wait before looking again for a log directory the CLI has not created yet.
const MISSING_RETRY: Duration = Duration::from_secs(60);

const LOG_FILE: &str = "operations.log";

/// Commands that may change skills, targets or the source. An allowlist on purpose: the
/// badge checks run `check`, `diff` and `status`, which log too, and reacting to them would loop.
const MUTATING: &[&str] = &[
    "init",
    "install",
    "uninstall",
    "update",
    "sync",
    "sync-extras",
    "extras-sync",
    "collect",
    "extras-collect",
    "commit",
    "push",
    "pull",
    "checkout",
    "discard",
    "git-root",
    "target",
    "target-file-add",
    "target-file-edit",
    "target-file-remove",
    "enable",
    "disable",
    "restore",
    "trash",
    "create-skill",
    "skill.edit",
    "skill.source",
    "set-skill-targets",
    "set-agent-targets",
    "batch-set-targets",
];

/// Groups whose entries are logged as `"<group> <verb>"`, such as `plugin update`.
const GROUPS: &[&str] = &["plugin", "mcp", "hooks"];

/// Verbs of those groups that change state; `plugin check` and the like are read-only.
const MUTATING_VERBS: &[&str] = &[
    "add",
    "edit",
    "import",
    "remove",
    "restore",
    "enable",
    "disable",
    "sync",
    "update",
    "configure",
];

/// The running watcher task.
#[derive(Default)]
pub struct OplogWatchState(std::sync::Mutex<Option<tauri::async_runtime::JoinHandle<()>>>);

/// Replace the watcher with one for the active project's log.
/// Call at startup and whenever the active project changes.
pub fn refresh(app: &AppHandle) {
    let state = app.state::<OplogWatchState>();
    let mut task = state.0.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(old) = task.take() {
        old.abort();
    }
    *task = Some(tauri::async_runtime::spawn(watch(app.clone())));
}

/// Re-check both badges from local state.
pub async fn refresh_badges(app: &AppHandle) {
    update_watch::refresh_skills(app).await;
    source_health::refresh(app, false).await;
}

async fn watch(app: AppHandle) {
    let (dir, is_project) = project_store::active_project_mode(&project_store::load());
    let path = log_path(dir.as_deref().filter(|_| is_project).map(Path::new));
    let Some(logs) = path.parent().map(Path::to_path_buf) else {
        return;
    };
    // The CLI creates the directory on its first logged command; creating it here would
    // skip the .gitignore entry the CLI adds for project logs.
    while !logs.is_dir() {
        tokio::time::sleep(MISSING_RETRY).await;
    }

    let (tx, mut events) = tokio::sync::mpsc::unbounded_channel();
    let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            let _ = tx.send(event);
        }
    });
    // Kept alive for the life of this task; aborting the task drops it and stops watching.
    let _watcher =
        match watcher.and_then(|mut w| w.watch(&logs, RecursiveMode::NonRecursive).map(|_| w)) {
            Ok(w) => w,
            Err(e) => {
                log::warn!(
                    "Operation log watch: could not watch {}: {e}",
                    logs.display()
                );
                return;
            }
        };
    log::info!("Operation log watch: watching {}", path.display());

    let mut tail = Tail::at_end(path);
    let mut debounce = Debounce::default();
    loop {
        let wake = debounce.deadline();
        tokio::select! {
            Some(event) = events.recv() => {
                if touches_log(&event) && tail.read_new().iter().any(|l| is_mutating(l)) {
                    debounce.changed(Instant::now());
                }
            }
            _ = tokio::time::sleep_until(wake.unwrap_or_else(Instant::now)), if wake.is_some() => {
                // Awaited inline: entries logged meanwhile queue up and coalesce into one follow-up.
                if debounce.take_due(Instant::now()) {
                    refresh_badges(&app).await;
                }
            }
            else => break,
        }
    }
}

fn touches_log(event: &notify::Event) -> bool {
    !matches!(event.kind, EventKind::Access(_))
        && event
            .paths
            .iter()
            .any(|p| p.file_name() == Some(LOG_FILE.as_ref()))
}

/// Where the CLI writes its operation log: `<project>/.skillshare/logs` (or the visible
/// `skillshare/` project dir) in project mode, `$XDG_STATE_HOME/skillshare/logs` globally.
fn log_path(project: Option<&Path>) -> PathBuf {
    let base = match project {
        Some(dir) => [".skillshare", "skillshare"]
            .iter()
            .map(|name| dir.join(name))
            .find(|d| d.join("config.yaml").is_file())
            .unwrap_or_else(|| dir.join(".skillshare")),
        None => state_dir(),
    };
    base.join("logs").join(LOG_FILE)
}

/// The CLI's `config.StateDir`.
fn state_dir() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_STATE_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(xdg).join("skillshare");
    }
    if cfg!(windows) {
        if let Some(dir) = dirs::config_dir() {
            return dir.join("skillshare");
        }
    }
    dirs::home_dir()
        .unwrap_or_default()
        .join(".local/state/skillshare")
}

/// Whether a log line records a command that may have changed state. Dry runs change nothing.
fn is_mutating(line: &str) -> bool {
    #[derive(serde::Deserialize)]
    struct Entry {
        cmd: String,
        #[serde(default)]
        args: serde_json::Map<String, serde_json::Value>,
    }
    let Ok(entry) = serde_json::from_str::<Entry>(line) else {
        return false;
    };
    let dry_run = ["dry_run", "dryRun"]
        .iter()
        .any(|k| entry.args.get(*k) == Some(&serde_json::Value::Bool(true)));
    if dry_run {
        return false;
    }
    match entry.cmd.split_once(' ') {
        Some((group, verb)) => GROUPS.contains(&group) && MUTATING_VERBS.contains(&verb),
        None => MUTATING.contains(&entry.cmd.as_str()),
    }
}

/// Reads only what was appended to the log since the last read.
struct Tail {
    path: PathBuf,
    offset: u64,
}

impl Tail {
    /// Start at the current end, so past entries never trigger a refresh.
    fn at_end(path: PathBuf) -> Self {
        let offset = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        Self { path, offset }
    }

    /// Complete lines appended since the last read; a partly written line waits for the next.
    fn read_new(&mut self) -> Vec<String> {
        let Ok(mut file) = std::fs::File::open(&self.path) else {
            self.offset = 0;
            return Vec::new();
        };
        let len = file.metadata().map(|m| m.len()).unwrap_or(0);
        if len < self.offset {
            // The CLI rewrote the file to trim old entries right after appending the newest.
            self.offset = len;
            let mut text = String::new();
            let _ = file.read_to_string(&mut text);
            return text.lines().last().map(String::from).into_iter().collect();
        }
        let mut buf = Vec::new();
        if file.seek(SeekFrom::Start(self.offset)).is_err() || file.read_to_end(&mut buf).is_err() {
            return Vec::new();
        }
        let complete = buf.iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1);
        self.offset += complete as u64;
        String::from_utf8_lossy(&buf[..complete])
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect()
    }
}

/// Fires once after changes have been quiet for [`SETTLE`].
#[derive(Debug, Default)]
struct Debounce(Option<Instant>);

impl Debounce {
    fn changed(&mut self, now: Instant) {
        self.0 = Some(now + SETTLE);
    }

    /// Whether the quiet period is over; if so, the pending change is consumed.
    fn take_due(&mut self, now: Instant) -> bool {
        if self.0.is_none_or(|due| now < due) {
            return false;
        }
        self.0 = None;
        true
    }

    fn deadline(&self) -> Option<Instant> {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn entry(cmd: &str, args: &str) -> String {
        format!(r#"{{"ts":"2026-10-01T23:05:39+08:00","cmd":"{cmd}","args":{args},"status":"ok"}}"#)
    }

    fn temp_log(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ss-oplog-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).ok();
        dir.join(LOG_FILE)
    }

    fn append(path: &Path, text: &str) {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            f.write_all(text.as_bytes()).ok();
        }
    }

    #[test]
    fn read_only_entries_from_the_badge_checks_never_trigger() {
        let lines = [
            entry("check", r#"{"scope":"global","updates_available":0}"#),
            entry("diff", r#"{"scope":"global"}"#),
            entry("plugin check", "{}"),
            entry("audit", "{}"),
            entry("backup", "{}"),
            entry("upgrade", "{}"),
        ];
        let triggering: Vec<_> = lines.iter().filter(|l| is_mutating(l)).collect();
        assert!(triggering.is_empty(), "should not trigger: {triggering:?}");
    }

    #[test]
    fn state_changing_entries_trigger() {
        let lines = [
            entry("sync", r#"{"dry_run":false,"scope":"global"}"#),
            entry("install", r#"{"mode":"global","source":"x"}"#),
            entry("update", "{}"),
            entry("collect", "{}"),
            entry("push", "{}"),
            entry("target", r#"{"action":"add"}"#),
            entry("plugin update", "{}"),
            entry("mcp configure", "{}"),
            entry("hooks disable", "{}"),
        ];
        let ignored: Vec<_> = lines.iter().filter(|l| !is_mutating(l)).collect();
        assert!(ignored.is_empty(), "should trigger: {ignored:?}");
    }

    #[test]
    fn dry_runs_and_unparsable_lines_do_not_trigger() {
        let lines = [
            entry("install", r#"{"dry_run":true}"#),
            entry("extras-sync", r#"{"dryRun":true}"#),
            "not json".to_string(),
        ];
        assert!(!lines.iter().any(|l| is_mutating(l)));
    }

    #[test]
    fn reads_only_lines_appended_since_the_last_read() {
        let path = temp_log("append");
        append(&path, &format!("{}\n", entry("sync", "{}")));
        let mut tail = Tail::at_end(path.clone());
        append(&path, &format!("{}\n", entry("check", "{}")));
        assert_eq!(tail.read_new(), vec![entry("check", "{}")]);
    }

    #[test]
    fn a_partly_written_line_waits_for_its_newline() {
        let path = temp_log("partial");
        let mut tail = Tail::at_end(path.clone());
        let line = entry("sync", "{}");
        append(&path, &line[..10]);
        let first = tail.read_new();
        append(&path, &format!("{}\n", &line[10..]));
        assert_eq!((first, tail.read_new()), (vec![], vec![line]));
    }

    #[test]
    fn a_trimmed_log_yields_only_its_newest_line() {
        let path = temp_log("trim");
        append(&path, &format!("{}\n", entry("check", "{}")).repeat(5));
        let mut tail = Tail::at_end(path.clone());
        std::fs::write(
            &path,
            format!("{}\n{}\n", entry("check", "{}"), entry("sync", "{}")),
        )
        .ok();
        assert_eq!(tail.read_new(), vec![entry("sync", "{}")]);
    }

    #[test]
    fn a_burst_of_entries_refreshes_once() {
        let t0 = Instant::now();
        let mut d = Debounce::default();
        let mut refreshes = 0;
        for ms in (0..10).map(|i| i * 200) {
            let now = t0 + Duration::from_millis(ms);
            refreshes += usize::from(d.take_due(now));
            d.changed(now);
        }
        let quiet = t0 + Duration::from_millis(1800) + SETTLE;
        refreshes += usize::from(d.take_due(quiet));
        refreshes += usize::from(d.take_due(quiet + SETTLE));
        assert_eq!(refreshes, 1);
    }

    #[test]
    fn project_logs_live_beside_the_project_config() {
        let dir = Path::new("/nonexistent/app");
        assert_eq!(
            log_path(Some(dir)),
            dir.join(".skillshare/logs/operations.log")
        );
    }
}
