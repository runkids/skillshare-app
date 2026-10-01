//! Source health: skills left behind in targets, targets out of sync, and the git
//! remote state of the global source. Re-checked when the source changes and every
//! 15 minutes; surfaced as an event, a title bar badge and tray actions.

use crate::services::{cli_manager, project_store};
use std::time::Duration;
use tauri::menu::{Menu, MenuItem, MenuItemBuilder};
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tauri_plugin_notification::NotificationExt;

/// Emitted with [`SourceHealth`] whenever it changes.
pub const SOURCE_HEALTH_EVENT: &str = "source-health";

const CHECK_INTERVAL: Duration = Duration::from_secs(15 * 60);
/// `status` and `diff` only read local files, but never let a stuck CLI block the checks.
const CLI_TIMEOUT: Duration = Duration::from_secs(30);
/// A fetch over a slow or offline network gives up and keeps the last known remote state.
const FETCH_TIMEOUT: Duration = Duration::from_secs(15);
const GIT_TIMEOUT: Duration = Duration::from_secs(10);
/// Same budget as Quick Sync: long enough for a slow push or pull, short enough not to hang.
const ACTION_TIMEOUT: Duration = Duration::from_secs(120);

/// The tray item the source actions are listed after.
const ANCHOR_MENU_ID: &str = "quick_sync";
const COLLECT_MENU_ID: &str = "source_collect";
const PUSH_MENU_ID: &str = "source_push";
const PULL_MENU_ID: &str = "source_pull";

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceHealth {
    /// Skills that exist only in a target, which `collect` would copy into the source; sorted.
    pub local_skills: Vec<String>,
    /// Targets that `sync` would change; sorted.
    pub out_of_sync_targets: Vec<String>,
    /// Git state of the source; `None` outside a git repo and in project mode,
    /// where `skillshare push`/`pull` do not apply.
    pub git: Option<GitState>,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitState {
    /// Changed or untracked paths not committed yet.
    pub uncommitted: usize,
    /// Commits not pushed to the upstream branch.
    pub ahead: usize,
    /// Upstream commits not pulled, as of the last successful fetch.
    pub behind: usize,
}

#[derive(Default)]
pub struct SourceHealthState(pub tokio::sync::Mutex<SourceHealth>);

/// Serializes checks, so a source change during the periodic check waits instead of racing it.
static CHECK_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(serde::Deserialize)]
struct DiffItem {
    name: String,
    kind: String,
    reason: String,
    is_sync: bool,
}

#[derive(serde::Deserialize)]
struct DiffTarget {
    name: String,
    #[serde(default)]
    items: Vec<DiffItem>,
}

#[derive(serde::Deserialize)]
struct DiffOutput {
    #[serde(default)]
    targets: Vec<DiffTarget>,
}

/// Local-only skills and out-of-sync targets from `skillshare diff --json`.
fn parse_diff(stdout: &str) -> Option<(Vec<String>, Vec<String>)> {
    let out: DiffOutput = serde_json::from_str(stdout)
        .map_err(|e| log::warn!("Unexpected `skillshare diff --json` output: {e}"))
        .ok()?;
    let mut local = Vec::new();
    let mut drifted = Vec::new();
    for target in out.targets {
        if target.items.iter().any(|i| i.is_sync) {
            drifted.push(target.name);
        }
        local.extend(
            target
                .items
                .into_iter()
                .filter(|i| i.kind == "skill" && i.reason == "local only")
                .map(|i| i.name),
        );
    }
    for list in [&mut local, &mut drifted] {
        list.sort();
        list.dedup();
    }
    Some((local, drifted))
}

/// Counts from `git status --porcelain=v2 --branch`; no upstream means nothing ahead or behind.
fn parse_git_status(stdout: &str) -> GitState {
    let mut state = GitState::default();
    for line in stdout.lines() {
        if let Some(ab) = line.strip_prefix("# branch.ab ") {
            let mut counts = ab
                .split_whitespace()
                .map(|n| n.trim_start_matches(['+', '-']).parse().unwrap_or(0));
            state.ahead = counts.next().unwrap_or(0);
            state.behind = counts.next().unwrap_or(0);
        } else if !line.starts_with('#') && !line.is_empty() {
            state.uncommitted += 1;
        }
    }
    state
}

async fn run_with_timeout(
    limit: Duration,
    run: impl std::future::Future<Output = Result<String, String>>,
) -> Result<String, String> {
    tokio::time::timeout(limit, run)
        .await
        .unwrap_or_else(|_| Err(format!("timed out after {}s", limit.as_secs())))
}

async fn git(dir: &str, args: &[&str], limit: Duration) -> Result<String, String> {
    let mut cmd = tokio::process::Command::new("git");
    cmd.args(args)
        .current_dir(dir)
        .envs(crate::utils::env::build_env_for_child())
        // A credential prompt would wait forever without a terminal; fail instead.
        .env("GIT_TERMINAL_PROMPT", "0")
        .kill_on_drop(true);
    run_with_timeout(limit, async {
        let output = cmd.output().await.map_err(|e| e.to_string())?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    })
    .await
}

/// Git state of `dir`, or `None` when it is not in a git repo.
async fn git_state(dir: &str, fetch: bool) -> Option<GitState> {
    if fetch {
        // Offline or no remote: behind stays as of the last fetch.
        if let Err(e) = git(dir, &["fetch", "--quiet"], FETCH_TIMEOUT).await {
            log::info!("Source health: git fetch skipped: {e}");
        }
    }
    git(dir, &["status", "--porcelain=v2", "--branch"], GIT_TIMEOUT)
        .await
        .ok()
        .map(|out| parse_git_status(&out))
}

/// The current health, or `None` when the CLI could not report it.
/// The active project's dir and whether it is in project mode; identifies whose health a check found.
type ProjectMode = (Option<String>, bool);

fn active_mode() -> ProjectMode {
    project_store::active_project_mode(&project_store::load())
}

/// Whether a check's result should replace `current`: only if it changed, and only if
/// the project it checked is still active, so a slow check for the previous project
/// never overwrites the new one's state.
fn is_news(
    current: &SourceHealth,
    found: &SourceHealth,
    checked: &ProjectMode,
    active: &ProjectMode,
) -> bool {
    checked == active && current != found
}

async fn compute(fetch: bool, (dir, is_project): &ProjectMode) -> Option<SourceHealth> {
    let is_project = *is_project;
    let cli_path = cli_manager::detect_cli().await?;
    let mode = if is_project { "--project" } else { "--global" };
    let args = ["diff", "--json", mode].map(String::from);
    let diff = run_with_timeout(
        CLI_TIMEOUT,
        cli_manager::exec(&cli_path, &args, dir.as_deref()),
    )
    .await
    .map_err(|e| log::warn!("Source health: diff failed: {e}"))
    .ok()?;
    let (local_skills, out_of_sync_targets) = parse_diff(&diff)?;

    let git = if is_project {
        None
    } else {
        match run_with_timeout(
            CLI_TIMEOUT,
            cli_manager::get_source_dir(&cli_path, dir.as_deref()),
        )
        .await
        {
            Ok(source) => git_state(&source, fetch).await,
            Err(e) => {
                log::warn!("Source health: could not find the source directory: {e}");
                None
            }
        }
    };
    Some(SourceHealth {
        local_skills,
        out_of_sync_targets,
        git,
    })
}

/// Keys for findings worth a notification: each skill left in a target, and each new
/// upstream commit count. Drift and unpushed work follow the user's own edits, so the
/// badge is enough for them.
fn finding_keys(health: &SourceHealth) -> Vec<String> {
    let mut keys: Vec<String> = health
        .local_skills
        .iter()
        .map(|s| format!("local:{s}"))
        .collect();
    if let Some(git) = health.git.as_ref().filter(|g| g.behind > 0) {
        keys.push(format!("behind:{}", git.behind));
    }
    keys
}

/// Notification lines when `health` has a finding the last notification didn't cover.
/// Remembers the current findings either way, so one that clears and returns is announced anew.
fn unannounced(health: &SourceHealth, notified: &mut Vec<String>) -> Vec<String> {
    let keys = finding_keys(health);
    let new = |prefix: &str| {
        keys.iter()
            .any(|k| k.starts_with(prefix) && !notified.contains(k))
    };
    let mut lines = Vec::new();
    if new("local:") {
        let n = health.local_skills.len();
        lines.push(format!(
            "{n} skill{} only in targets; collect {} into the source.",
            plural(n),
            if n == 1 { "it" } else { "them" }
        ));
    }
    if new("behind:") {
        let n = health.git.as_ref().map_or(0, |g| g.behind);
        lines.push(format!("{n} update{} to pull from the remote.", plural(n)));
    }
    *notified = keys;
    lines
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

/// Tray labels for the actions that have something to do, in menu order.
fn tray_labels(health: &SourceHealth) -> Vec<(&'static str, String)> {
    let mut items = Vec::new();
    let n = health.local_skills.len();
    if n > 0 {
        items.push((
            COLLECT_MENU_ID,
            format!("Collect {n} Local Skill{}…", plural(n)),
        ));
    }
    if let Some(git) = &health.git {
        let changes = git.uncommitted + git.ahead;
        if changes > 0 {
            items.push((
                PUSH_MENU_ID,
                format!("Push {changes} Change{}", plural(changes)),
            ));
        }
        if git.behind > 0 {
            items.push((
                PULL_MENU_ID,
                format!("Pull {} Update{}", git.behind, plural(git.behind)),
            ));
        }
    }
    items
}

/// The tray menu and the action items shown in it when they have something to do.
struct TrayActions {
    menu: Menu<Wry>,
    items: Vec<MenuItem<Wry>>,
}

/// Let the source actions appear in `menu` and handle their clicks.
pub fn attach_tray(app: &tauri::App, menu: &Menu<Wry>) -> tauri::Result<()> {
    let items = [COLLECT_MENU_ID, PUSH_MENU_ID, PULL_MENU_ID]
        .iter()
        .map(|id| MenuItemBuilder::with_id(*id, *id).build(app))
        .collect::<tauri::Result<Vec<_>>>()?;
    app.manage(TrayActions {
        menu: menu.clone(),
        items,
    });
    app.on_menu_event(|app, event| match event.id().as_ref() {
        COLLECT_MENU_ID => confirm_collect(app),
        PUSH_MENU_ID => spawn_action(app, &["push"], "Push"),
        PULL_MENU_ID => spawn_action(app, &["pull"], "Pull"),
        _ => {}
    });
    Ok(())
}

fn update_tray(app: &AppHandle, health: &SourceHealth) {
    let Some(tray) = app.try_state::<TrayActions>() else {
        return;
    };
    let result = (|| -> tauri::Result<()> {
        for item in &tray.items {
            // Not in the menu yet on the first update; nothing to remove then.
            let _ = tray.menu.remove(item);
        }
        let mut pos = tray
            .menu
            .items()?
            .iter()
            .position(|i| i.id() == ANCHOR_MENU_ID)
            .map_or(0, |i| i + 1);
        for (id, label) in tray_labels(health) {
            if let Some(item) = tray.items.iter().find(|i| i.id() == id) {
                item.set_text(label)?;
                tray.menu.insert(item, pos)?;
                pos += 1;
            }
        }
        Ok(())
    })();
    if let Err(e) = result {
        log::warn!("Failed to update tray source actions: {e}");
    }
}

/// Collect copies skills into the source, so ask before changing it.
pub(crate) fn confirm_collect(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let skills = app
            .state::<SourceHealthState>()
            .0
            .lock()
            .await
            .local_skills
            .clone();
        if skills.is_empty() {
            return;
        }
        let (_, is_project) = project_store::active_project_mode(&project_store::load());
        let mode = if is_project { "--project" } else { "--global" };
        let dialog_app = app.clone();
        dialog_app
            .dialog()
            .message(format!(
                "Copy these skills from your targets into the skills source?\n\n{}",
                skills.join(", ")
            ))
            .title("Collect Local Skills")
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Collect".into(),
                "Cancel".into(),
            ))
            .show(move |confirmed| {
                if confirmed {
                    // --json skips the CLI's own prompt, which cannot be answered without a terminal.
                    spawn_action(&app, &["collect", "--all", "--json", mode], "Collect");
                }
            });
    });
}

/// Run a CLI action for the active project, report the result, then re-check.
pub(crate) fn spawn_action(app: &AppHandle, args: &[&str], label: &'static str) {
    let app = app.clone();
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    tauri::async_runtime::spawn(async move {
        let Some(cli_path) = cli_manager::detect_cli().await else {
            log::warn!("{label}: CLI not found");
            return;
        };
        let dir = project_store::load()
            .active_project()
            .map(|p| p.path.clone());
        let result = run_with_timeout(
            ACTION_TIMEOUT,
            cli_manager::exec(&cli_path, &args, dir.as_deref()),
        )
        .await;
        let body = match &result {
            Ok(_) => format!("{label} finished."),
            Err(e) => {
                log::warn!("{label} failed: {e}");
                e.chars().take(200).collect()
            }
        };
        if cli_manager::load_meta().notify_sync.unwrap_or(true) {
            let title = if result.is_ok() {
                format!("Skillshare {label} Complete")
            } else {
                format!("{label} failed")
            };
            let _ = app.notification().builder().title(title).body(body).show();
        }
        refresh(&app, false).await;
        if label == "Pull" && result.is_ok() {
            super::audit::run(&app).await;
        }
        crate::services::update_watch::refresh_skills(&app).await;
    });
}

/// Re-check source health and publish it if it changed. `fetch` also contacts the
/// git remote, so callers reacting to local edits pass `false`.
pub async fn refresh(app: &AppHandle, fetch: bool) {
    let _checking = CHECK_LOCK.lock().await;
    let checked = active_mode();
    let Some(health) = compute(fetch, &checked).await else {
        return;
    };
    {
        // Checked under the state lock, so a project switch's reset can't slip in between.
        let state = app.state::<SourceHealthState>();
        let mut current = state.0.lock().await;
        if !is_news(&current, &health, &checked, &active_mode()) {
            return;
        }
        *current = health.clone();
    }
    publish(app, &health);

    // In the foreground the title bar badge is enough.
    let focused = app
        .get_webview_window("main")
        .and_then(|w| w.is_focused().ok())
        .unwrap_or(false);
    let mut meta = cli_manager::load_meta();
    if focused || !meta.notify_update.unwrap_or(true) {
        return;
    }
    let before = meta.notified_source_health.clone();
    let lines = unannounced(&health, &mut meta.notified_source_health);
    if !lines.is_empty() {
        let _ = app
            .notification()
            .builder()
            .title("Skillshare Source Needs Attention")
            .body(lines.join("\n"))
            .show();
    }
    if meta.notified_source_health != before {
        if let Err(e) = cli_manager::save_meta(&meta) {
            log::warn!("Could not save notified source health: {e}");
        }
    }
}

fn publish(app: &AppHandle, health: &SourceHealth) {
    update_tray(app, health);
    if let Err(e) = app.emit(SOURCE_HEALTH_EVENT, health) {
        log::warn!("Failed to emit source health: {e}");
    }
}

/// The active project changed: stop showing the previous project's state at once,
/// then re-check the new one from local state (no fetch).
pub fn project_changed(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let cleared = SourceHealth::default();
        *app.state::<SourceHealthState>().0.lock().await = cleared.clone();
        publish(&app, &cleared);
        refresh(&app, false).await;
    });
}

/// Check at launch, then every 15 minutes, fetching the remote each time.
pub fn spawn_background(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        // git needs the user's PATH when the app was launched from Finder.
        crate::utils::env::load_login_shell_path().await;
        loop {
            refresh(&app, true).await;
            tokio::time::sleep(CHECK_INTERVAL).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn health(local: &[&str], behind: usize) -> SourceHealth {
        SourceHealth {
            local_skills: local.iter().map(|s| s.to_string()).collect(),
            out_of_sync_targets: Vec::new(),
            git: Some(GitState {
                behind,
                ..Default::default()
            }),
        }
    }

    #[test]
    fn check_for_a_previous_project_is_dropped() {
        let checked = (Some("/old".to_string()), true);
        let active = (Some("/new".to_string()), true);
        let found = health(&["a"], 0);
        assert!(!is_news(
            &SourceHealth::default(),
            &found,
            &checked,
            &active
        ));
    }

    #[test]
    fn changed_health_for_the_active_project_is_published() {
        let mode = (None, false);
        let found = health(&["a"], 0);
        assert!(is_news(&SourceHealth::default(), &found, &mode, &mode));
    }

    #[test]
    fn unchanged_health_is_not_published_again() {
        let mode = (None, false);
        let found = health(&["a"], 0);
        assert!(!is_news(&found.clone(), &found, &mode, &mode));
    }

    #[test]
    fn diff_lists_local_skills_once_and_targets_sync_would_change() {
        let stdout = r#"{
          "targets": [
            {"name": "claude", "synced": false, "items": [
              {"action": "remove", "name": "mine", "kind": "skill", "reason": "local only", "is_sync": false},
              {"action": "local", "name": "a.md", "kind": "agent", "reason": "local file", "is_sync": false}
            ]},
            {"name": "codex", "synced": false, "items": [
              {"action": "add", "name": "pdf", "kind": "skill", "reason": "source only", "is_sync": true},
              {"action": "remove", "name": "mine", "kind": "skill", "reason": "local only", "is_sync": false}
            ]},
            {"name": "cursor", "synced": true, "items": []}
          ],
          "extras": []
        }"#;
        assert_eq!(
            parse_diff(stdout),
            Some((vec!["mine".to_string()], vec!["codex".to_string()]))
        );
    }

    #[test]
    fn unparseable_diff_output_is_unknown() {
        assert_eq!(parse_diff("config not found"), None);
    }

    #[test]
    fn git_status_counts_changes_and_upstream_distance() {
        let stdout = "# branch.oid abc\n# branch.head main\n# branch.upstream origin/main\n\
                      # branch.ab +2 -3\n1 .M N... 100644 100644 100644 a b pdf/SKILL.md\n? new/SKILL.md\n";
        assert_eq!(
            parse_git_status(stdout),
            GitState {
                uncommitted: 2,
                ahead: 2,
                behind: 3
            }
        );
    }

    #[test]
    fn git_status_without_upstream_is_neither_ahead_nor_behind() {
        let stdout = "# branch.oid abc\n# branch.head main\n";
        assert_eq!(parse_git_status(stdout), GitState::default());
    }

    #[test]
    fn same_findings_are_announced_once() {
        let mut notified = Vec::new();
        let first = unannounced(&health(&["a"], 2), &mut notified).len();
        let second = unannounced(&health(&["a"], 2), &mut notified).len();
        assert_eq!((first, second), (2, 0));
    }

    #[test]
    fn fewer_findings_are_not_announced() {
        let mut notified = Vec::new();
        unannounced(&health(&["a", "b"], 0), &mut notified);
        assert!(unannounced(&health(&["a"], 0), &mut notified).is_empty());
    }

    #[test]
    fn new_upstream_commits_are_announced_again() {
        let mut notified = Vec::new();
        unannounced(&health(&[], 1), &mut notified);
        assert_eq!(unannounced(&health(&[], 3), &mut notified).len(), 1);
    }

    #[test]
    fn tray_offers_only_actions_with_work() {
        let mut h = health(&["a", "b"], 0);
        h.git = Some(GitState {
            uncommitted: 1,
            ahead: 1,
            behind: 0,
        });
        assert_eq!(
            tray_labels(&h),
            vec![
                (COLLECT_MENU_ID, "Collect 2 Local Skills…".to_string()),
                (PUSH_MENU_ID, "Push 2 Changes".to_string()),
            ]
        );
    }

    #[test]
    fn tray_offers_pull_when_behind() {
        assert_eq!(
            tray_labels(&health(&[], 1)),
            vec![(PULL_MENU_ID, "Pull 1 Update".to_string())]
        );
    }
}
