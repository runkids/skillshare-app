//! Watch the active skills source. Once edits settle, re-check skill updates (so the
//! badge clears after updating skills) and, with opt-in auto-sync, run Quick Sync.

use crate::services::{cli_manager, project_store, update_watch};
use notify::{EventKind, RecursiveMode, Watcher};
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tokio::time::Instant;

/// Quiet period after the last change before syncing, so a multi-file save syncs once.
const SETTLE: Duration = Duration::from_secs(2);

/// The running watcher task.
#[derive(Default)]
pub struct AutoSyncState(std::sync::Mutex<Option<tauri::async_runtime::JoinHandle<()>>>);

/// Replace the watcher with one for the current active project.
/// Call at startup and whenever the auto-sync setting or the active project changes.
pub fn refresh(app: &AppHandle) {
    let state = app.state::<AutoSyncState>();
    let mut task = state.0.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(old) = task.take() {
        old.abort();
    }
    *task = Some(tauri::async_runtime::spawn(watch(app.clone())));
}

async fn watch(app: AppHandle) {
    let Some(cli_path) = cli_manager::detect_cli().await else {
        log::warn!("Source watch: CLI not found");
        return;
    };
    let working_dir = project_store::load()
        .active_project()
        .map(|p| p.path.clone());
    let source = match cli_manager::get_source_dir(&cli_path, working_dir.as_deref()).await {
        Ok(dir) => std::path::PathBuf::from(dir),
        Err(e) => {
            log::warn!("Source watch: could not find the source directory: {e}");
            return;
        }
    };

    let (tx, mut events) = tokio::sync::mpsc::unbounded_channel();
    let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            let _ = tx.send(event);
        }
    });
    // Kept alive for the life of this task; aborting the task drops it and stops watching.
    let _watcher =
        match watcher.and_then(|mut w| w.watch(&source, RecursiveMode::Recursive).map(|_| w)) {
            Ok(w) => w,
            Err(e) => {
                log::warn!("Source watch: could not watch {}: {e}", source.display());
                return;
            }
        };
    log::info!("Source watch: watching {}", source.display());

    let (done_tx, mut done) = tokio::sync::mpsc::unbounded_channel::<()>();
    let mut scheduler = Scheduler::default();
    loop {
        let wake = scheduler.next_wakeup();
        tokio::select! {
            Some(event) = events.recv() => {
                if is_source_change(&event, &source) {
                    scheduler.changed(Instant::now());
                }
            }
            _ = tokio::time::sleep_until(wake.unwrap_or_else(Instant::now)), if wake.is_some() => {
                if scheduler.start_if_due(Instant::now()) {
                    let app = app.clone();
                    let done_tx = done_tx.clone();
                    tauri::async_runtime::spawn(async move {
                        if cli_manager::load_meta().auto_sync.unwrap_or(false) {
                            log::info!("Auto-sync: source changed, syncing");
                            crate::handle_quick_sync(&app).await;
                            crate::services::audit::run(&app).await;
                        }
                        update_watch::refresh_skills(&app).await;
                        crate::services::source_health::refresh(&app, false).await;
                        let _ = done_tx.send(());
                    });
                }
            }
            Some(()) = done.recv() => scheduler.finished(),
            else => break,
        }
    }
}

/// Decides when to sync: once changes have settled, never while a sync runs,
/// and changes made during a sync coalesce into a single follow-up run.
#[derive(Debug, Default)]
struct Scheduler {
    /// When the latest burst of changes will have settled.
    due_at: Option<Instant>,
    syncing: bool,
}

impl Scheduler {
    fn changed(&mut self, now: Instant) {
        self.due_at = Some(now + SETTLE);
    }

    /// Whether a sync should start now; if so, it is marked as running.
    fn start_if_due(&mut self, now: Instant) -> bool {
        if self.syncing || self.due_at.is_none_or(|due| now < due) {
            return false;
        }
        self.due_at = None;
        self.syncing = true;
        true
    }

    fn finished(&mut self) {
        self.syncing = false;
    }

    /// When to call [`Self::start_if_due`] next; `None` while syncing or idle.
    fn next_wakeup(&self) -> Option<Instant> {
        if self.syncing {
            None
        } else {
            self.due_at
        }
    }
}

fn is_source_change(event: &notify::Event, source: &Path) -> bool {
    !matches!(event.kind, EventKind::Access(_)) && event.paths.iter().any(|p| is_tracked(p, source))
}

/// Whether a changed path is skill content rather than git internals or editor/OS temp files.
fn is_tracked(path: &Path, source: &Path) -> bool {
    let rel = path.strip_prefix(source).unwrap_or(path);
    if rel.components().any(|c| c.as_os_str() == ".git") {
        return false;
    }
    let name = rel.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    let temp = name == ".DS_Store"
        || name == "4913" // Vim's write-permission probe
        || name.ends_with('~')
        || name.starts_with(".#")
        || [".swp", ".swx", ".tmp"].iter().any(|ext| name.ends_with(ext));
    !temp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syncs_once_after_a_burst_of_changes_settles() {
        let t0 = Instant::now();
        let mut s = Scheduler::default();
        s.changed(t0);
        s.changed(t0 + Duration::from_millis(1500));
        let early = s.start_if_due(t0 + SETTLE);
        let settled = s.start_if_due(t0 + Duration::from_millis(1500) + SETTLE);
        assert_eq!((early, settled), (false, true));
    }

    #[test]
    fn does_nothing_without_changes() {
        let mut s = Scheduler::default();
        assert!(!s.start_if_due(Instant::now() + SETTLE));
    }

    #[test]
    fn never_starts_a_second_sync_while_one_runs() {
        let t0 = Instant::now();
        let mut s = Scheduler::default();
        s.changed(t0);
        s.start_if_due(t0 + SETTLE);
        s.changed(t0 + SETTLE);
        assert!(!s.start_if_due(t0 + SETTLE * 3));
    }

    #[test]
    fn changes_during_a_sync_coalesce_into_one_follow_up() {
        let t0 = Instant::now();
        let mut s = Scheduler::default();
        s.changed(t0);
        s.start_if_due(t0 + SETTLE);
        for ms in [100, 200, 300] {
            s.changed(t0 + SETTLE + Duration::from_millis(ms));
        }
        s.finished();
        let later = t0 + SETTLE * 3;
        assert_eq!(
            (s.start_if_due(later), s.start_if_due(later)),
            (true, false)
        );
    }

    #[test]
    fn no_follow_up_when_nothing_changed_during_the_sync() {
        let t0 = Instant::now();
        let mut s = Scheduler::default();
        s.changed(t0);
        s.start_if_due(t0 + SETTLE);
        s.finished();
        assert_eq!(s.next_wakeup(), None);
    }

    #[test]
    fn skill_files_are_tracked() {
        let source = Path::new("/src/skills");
        assert!(is_tracked(&source.join("pdf/SKILL.md"), source));
    }

    #[test]
    fn git_internals_and_temp_files_are_ignored() {
        let source = Path::new("/src/skills");
        let ignored = [
            "_team/.git/index",
            ".git/HEAD",
            "pdf/.SKILL.md.swp",
            "pdf/SKILL.md~",
            "pdf/.#SKILL.md",
            "pdf/4913",
            ".DS_Store",
        ];
        let tracked: Vec<_> = ignored
            .iter()
            .filter(|p| is_tracked(&source.join(p), source))
            .collect();
        assert!(tracked.is_empty(), "should be ignored: {tracked:?}");
    }
}
