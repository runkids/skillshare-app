//! Background update checks for the CLI, the app itself, and installed skills.

use crate::services::{cli_manager, project_store};
use chrono::{DateTime, Utc};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_updater::UpdaterExt;

/// Emitted with [`AvailableUpdates`] after every check.
pub const UPDATES_EVENT: &str = "updates-available";
/// Emitted when the user picks "Check for Updates…" from a menu.
pub const CHECK_REQUESTED_EVENT: &str = "check-for-updates";

const CHECK_EVERY_HOURS: i64 = 24;
/// Wake up hourly so a sleeping laptop still checks soon after the 24h mark.
const WAKE_INTERVAL: Duration = Duration::from_secs(60 * 60);
/// `skillshare check` fetches every source repo; give up rather than hang on a slow remote.
const SKILL_CHECK_TIMEOUT: Duration = Duration::from_secs(60);

/// Newer versions found by the last check; `None` means up to date (or unknown).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableUpdates {
    pub cli: Option<String>,
    pub app: Option<String>,
    /// Skills and tracked repos in the active project with upstream changes, sorted.
    pub skills: Vec<String>,
}

#[derive(Default)]
pub struct UpdateState(pub tokio::sync::Mutex<AvailableUpdates>);

fn parse_version(v: &str) -> Option<Vec<u64>> {
    let core = v.trim().trim_start_matches('v').split(['-', '+']).next()?;
    core.split('.').map(|part| part.parse().ok()).collect()
}

/// Whether `latest` is a strictly higher version than `current`.
/// Unparseable versions never count as newer, so odd local builds don't trigger alerts.
pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(latest), Some(current)) => latest > current,
        _ => false,
    }
}

fn is_due(last_check: Option<&str>, now: DateTime<Utc>) -> bool {
    last_check
        .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
        .is_none_or(|last| now.signed_duration_since(last).num_hours() >= CHECK_EVERY_HOURS)
}

async fn latest_cli(current: Option<&str>) -> Option<String> {
    let current = current?;
    match cli_manager::check_latest_release().await {
        Ok((tag, _)) => is_newer(&tag, current).then_some(tag),
        Err(e) => {
            log::warn!("CLI update check failed: {e}");
            None
        }
    }
}

async fn latest_app(app: &AppHandle) -> Option<String> {
    let updater = app
        .updater()
        .map_err(|e| log::warn!("Updater unavailable: {e}"))
        .ok()?;
    match updater.check().await {
        Ok(update) => update.map(|u| u.version),
        Err(e) => {
            log::warn!("App update check failed: {e}");
            None
        }
    }
}

/// One entry of `skillshare check --json`'s `skills` or `tracked_repos` list.
#[derive(serde::Deserialize)]
struct CheckEntry {
    name: String,
    status: String,
}

#[derive(serde::Deserialize)]
struct CheckOutput {
    #[serde(default)]
    tracked_repos: Vec<CheckEntry>,
    #[serde(default)]
    skills: Vec<CheckEntry>,
}

/// Names with updates in `skillshare check --json` output: skills marked
/// `update_available` and tracked repos that are `behind`.
fn parse_skill_updates(stdout: &str) -> Option<Vec<String>> {
    let out: CheckOutput = serde_json::from_str(stdout)
        .map_err(|e| log::warn!("Unexpected `skillshare check --json` output: {e}"))
        .ok()?;
    let repos = out
        .tracked_repos
        .into_iter()
        .filter(|r| r.status == "behind");
    let skills = out
        .skills
        .into_iter()
        .filter(|s| s.status == "update_available");
    let mut names: Vec<String> = repos.chain(skills).map(|e| e.name).collect();
    names.sort();
    names.dedup();
    Some(names)
}

/// Skills with updates in the active project; `None` when the check could not run.
/// `check` only reads upstream state; updating stays a user action in the Web UI.
async fn skill_updates() -> Option<Vec<String>> {
    let cli_path = cli_manager::detect_cli().await?;
    let (dir, is_project) = project_store::active_project_mode(&project_store::load());
    let mode = if is_project { "--project" } else { "--global" };
    let args = ["check", "--json", mode].map(String::from);
    let result = tokio::time::timeout(
        SKILL_CHECK_TIMEOUT,
        cli_manager::exec(&cli_path, &args, dir.as_deref()),
    )
    .await
    .unwrap_or_else(|_| {
        Err(format!(
            "timed out after {}s",
            SKILL_CHECK_TIMEOUT.as_secs()
        ))
    });
    match result {
        Ok(stdout) => parse_skill_updates(&stdout),
        Err(e) => {
            log::warn!("Skill update check failed: {e}");
            None
        }
    }
}

fn main_window_focused(app: &AppHandle) -> bool {
    app.get_webview_window("main")
        .and_then(|w| w.is_focused().ok())
        .unwrap_or(false)
}

/// Lines for versions not announced yet; records them as announced.
fn unannounced(
    found: &AvailableUpdates,
    meta: &mut crate::models::app_state::CliMeta,
) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(v) = &found.app {
        if meta.notified_app_version.as_ref() != Some(v) {
            lines.push(format!("Skillshare App {v} is available."));
            meta.notified_app_version = Some(v.clone());
        }
    }
    if let Some(v) = &found.cli {
        if meta.notified_cli_version.as_ref() != Some(v) {
            lines.push(format!("Skillshare CLI {v} is available."));
            meta.notified_cli_version = Some(v.clone());
        }
    }
    // Announce when a skill appears that the last notification didn't list. Remember the
    // current set either way, so a skill updated and later outdated again is announced anew.
    let new_skill = found
        .skills
        .iter()
        .any(|s| !meta.notified_skill_updates.contains(s));
    if new_skill {
        let n = found.skills.len();
        lines.push(format!(
            "{n} skill update{} available.",
            if n == 1 { " is" } else { "s are" }
        ));
    }
    meta.notified_skill_updates = found.skills.clone();
    lines
}

/// Check the CLI, the app, and skills, publish the result, and notify while the app is in the background.
pub async fn check(app: &AppHandle, notify: bool) -> AvailableUpdates {
    let mut meta = cli_manager::load_meta();
    if let Err(e) = cli_manager::refresh_cached_version(&mut meta).await {
        log::warn!("Could not refresh cached CLI version: {e}");
    }
    let state = app.state::<UpdateState>();
    // A failed skill check keeps the last known list instead of clearing the badge.
    let skills = match skill_updates().await {
        Some(skills) => skills,
        None => state.0.lock().await.skills.clone(),
    };
    let found = AvailableUpdates {
        cli: latest_cli(meta.version.as_deref()).await,
        app: latest_app(app).await,
        skills,
    };
    meta.last_update_check = Some(Utc::now().to_rfc3339());

    // In the foreground the title bar badge is enough.
    if notify && meta.notify_update.unwrap_or(true) && !main_window_focused(app) {
        let lines = unannounced(&found, &mut meta);
        if !lines.is_empty() {
            let _ = app
                .notification()
                .builder()
                .title("Skillshare Update Available")
                .body(lines.join("\n"))
                .show();
        }
    }
    if let Err(e) = cli_manager::save_meta(&meta) {
        log::warn!("Failed to save CLI meta after update check: {e}");
    }

    *state.0.lock().await = found.clone();
    if let Err(e) = app.emit(UPDATES_EVENT, &found) {
        log::warn!("Failed to emit update result: {e}");
    }
    found
}

/// Check at launch (so the badge is current), then again whenever 24h have passed.
pub fn spawn_background(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut first = true;
        loop {
            let due = is_due(
                cli_manager::load_meta().last_update_check.as_deref(),
                Utc::now(),
            );
            if first || due {
                check(&app, due).await;
            }
            first = false;
            tokio::time::sleep(WAKE_INTERVAL).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn higher_version_is_newer() {
        assert!(is_newer("v0.23.0", "v0.22.10"));
    }

    #[test]
    fn same_or_older_version_is_not_newer() {
        assert!(!is_newer("v0.22.2", "0.23.0"));
    }

    #[test]
    fn unparseable_version_is_not_newer() {
        assert!(!is_newer("v0.23.0", "dev"));
    }

    #[test]
    fn check_is_due_after_a_day_or_without_history() {
        let now = Utc::now();
        let recent = (now - chrono::Duration::hours(2)).to_rfc3339();
        let old = (now - chrono::Duration::hours(25)).to_rfc3339();
        assert_eq!(
            [
                is_due(None, now),
                is_due(Some(&recent), now),
                is_due(Some(&old), now)
            ],
            [true, false, true]
        );
    }

    #[test]
    fn each_version_is_announced_once() {
        let mut meta = crate::models::app_state::CliMeta::default();
        let found = AvailableUpdates {
            cli: Some("v0.24.0".into()),
            app: None,
            skills: Vec::new(),
        };
        let first = unannounced(&found, &mut meta).len();
        let second = unannounced(&found, &mut meta).len();
        assert_eq!((first, second), (1, 0));
    }

    fn skills(names: &[&str]) -> AvailableUpdates {
        AvailableUpdates {
            skills: names.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn parses_skills_and_repos_with_updates() {
        let stdout = r#"{
          "tracked_repos": [
            {"name": "_team", "status": "behind", "behind": 3},
            {"name": "_docs", "status": "up_to_date", "behind": 0}
          ],
          "skills": [
            {"name": "pdf", "source": "github.com/a/b", "version": "abc", "status": "update_available"},
            {"name": "mine", "source": "", "version": "", "status": "local"},
            {"name": "gone", "source": "github.com/a/c", "version": "def", "status": "stale"}
          ]
        }"#;
        assert_eq!(
            parse_skill_updates(stdout),
            Some(vec!["_team".to_string(), "pdf".to_string()])
        );
    }

    #[test]
    fn unparseable_check_output_is_unknown() {
        assert_eq!(parse_skill_updates("No tracked repositories"), None);
    }

    #[test]
    fn same_skill_set_is_announced_once() {
        let mut meta = crate::models::app_state::CliMeta::default();
        let first = unannounced(&skills(&["a", "b"]), &mut meta).len();
        let second = unannounced(&skills(&["a", "b"]), &mut meta).len();
        assert_eq!((first, second), (1, 0));
    }

    #[test]
    fn shrinking_skill_set_is_not_announced() {
        let mut meta = crate::models::app_state::CliMeta::default();
        unannounced(&skills(&["a", "b"]), &mut meta);
        assert!(unannounced(&skills(&["a"]), &mut meta).is_empty());
    }

    #[test]
    fn skill_with_a_new_update_is_announced_again() {
        let mut meta = crate::models::app_state::CliMeta::default();
        unannounced(&skills(&["a"]), &mut meta);
        unannounced(&skills(&[]), &mut meta);
        assert_eq!(unannounced(&skills(&["a"]), &mut meta).len(), 1);
    }
}
