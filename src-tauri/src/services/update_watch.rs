//! Background update checks for the app, CLI, and managed resources.

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
const RESOURCE_CHECK_TIMEOUT: Duration = Duration::from_secs(60);

/// Newer versions found by the last check; `None` means up to date (or unknown).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableUpdates {
    pub cli: Option<String>,
    pub app: Option<String>,
    /// Per-kind resources in the active project with upstream changes, sorted.
    pub skills: Vec<String>,
    pub repositories: Vec<String>,
    pub agents: Vec<String>,
    pub plugins: Vec<String>,
}

impl AvailableUpdates {
    pub fn resource_count(&self) -> usize {
        self.skills.len() + self.repositories.len() + self.agents.len() + self.plugins.len()
    }

    fn resource_summary(&self) -> String {
        [
            (self.skills.len(), "skill", "skills"),
            (self.plugins.len(), "plugin", "plugins"),
            (self.agents.len(), "agent", "agents"),
            (self.repositories.len(), "repository", "repositories"),
        ]
        .into_iter()
        .filter(|(n, _, _)| *n > 0)
        .map(|(n, one, many)| format!("{n} {}", if n == 1 { one } else { many }))
        .collect::<Vec<_>>()
        .join(", ")
    }
}

// Serialize checks so a slower earlier check cannot restore a cleared badge.
static CHECK_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

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
    tracked_repos: Vec<CheckEntry>,
    skills: Vec<CheckEntry>,
}

fn update_names(entries: Vec<CheckEntry>, status: &str) -> Vec<String> {
    sorted_names(
        entries
            .into_iter()
            .filter(|e| e.status == status)
            .map(|e| e.name),
    )
}

fn sorted_names(names: impl Iterator<Item = String>) -> Vec<String> {
    let mut names: Vec<_> = names.collect();
    names.sort();
    names.dedup();
    names
}

fn parse_skill_updates(stdout: &str) -> Option<(Vec<String>, Vec<String>)> {
    let out: CheckOutput = serde_json::from_str(stdout).ok()?;
    Some((
        update_names(out.skills, "update_available"),
        update_names(out.tracked_repos, "behind"),
    ))
}

fn parse_agent_updates(stdout: &str) -> Option<Vec<String>> {
    // The CLI emits null, rather than [], when no agents were discovered.
    let out: Option<Vec<CheckEntry>> = serde_json::from_str(stdout).ok()?;
    Some(update_names(out.unwrap_or_default(), "update_available"))
}

#[derive(serde::Deserialize)]
struct PluginChange {
    name: String,
    action: String,
}

#[derive(serde::Deserialize)]
struct PluginCheck {
    #[serde(deserialize_with = "null_changes")]
    changes: Vec<PluginChange>,
}

fn null_changes<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<PluginChange>, D::Error> {
    use serde::Deserialize;
    Ok(Option::<Vec<PluginChange>>::deserialize(deserializer)?.unwrap_or_default())
}

fn parse_plugin_updates(stdout: &str) -> Option<Vec<String>> {
    let out: PluginCheck = serde_json::from_str(stdout).ok()?;
    // Several target bindings may report the same logical plugin package.
    Some(sorted_names(
        out.changes
            .into_iter()
            .filter(|c| c.action == "update-available")
            .map(|c| c.name),
    ))
}

async fn resource_check(cli: &str, args: &[&str], dir: Option<&str>, mode: &str) -> Option<String> {
    let args: Vec<_> = args
        .iter()
        .copied()
        .chain(["--json", mode])
        .map(String::from)
        .collect();
    match tokio::time::timeout(RESOURCE_CHECK_TIMEOUT, cli_manager::exec(cli, &args, dir)).await {
        Ok(Ok(stdout)) => Some(stdout),
        result => {
            log::warn!("Resource update check {args:?} failed: {result:?}");
            None
        }
    }
}

/// Failed checks preserve only that kind's last list; successful kinds still refresh.
async fn resource_updates(found: &mut AvailableUpdates) -> bool {
    let Some(cli) = cli_manager::detect_cli().await else {
        return false;
    };
    let (dir, is_project) = project_store::active_project_mode(&project_store::load());
    let mode = if is_project { "--project" } else { "--global" };
    let skills = resource_check(&cli, &["check"], dir.as_deref(), mode)
        .await
        .and_then(|s| parse_skill_updates(&s));
    let agents = resource_check(&cli, &["check", "agents"], dir.as_deref(), mode)
        .await
        .and_then(|s| parse_agent_updates(&s));
    let plugins = resource_check(&cli, &["plugin", "check"], dir.as_deref(), mode)
        .await
        .and_then(|s| parse_plugin_updates(&s));
    let complete = apply_resource_results(found, skills, agents, plugins);
    if !complete {
        log::warn!("Some resource updates could not be checked; keeping previous lists");
    }
    complete
}

fn apply_resource_results(
    found: &mut AvailableUpdates,
    skills: Option<(Vec<String>, Vec<String>)>,
    agents: Option<Vec<String>>,
    plugins: Option<Vec<String>>,
) -> bool {
    let complete = skills.is_some() && agents.is_some() && plugins.is_some();
    if let Some((skills, repositories)) = skills {
        found.skills = skills;
        found.repositories = repositories;
    }
    if let Some(agents) = agents {
        found.agents = agents;
    }
    if let Some(plugins) = plugins {
        found.plugins = plugins;
    }
    complete
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
    let new_resource = [
        (&found.skills, &meta.notified_skill_updates),
        (&found.repositories, &meta.notified_repository_updates),
        (&found.agents, &meta.notified_agent_updates),
        (&found.plugins, &meta.notified_plugin_updates),
    ]
    .iter()
    .any(|(current, notified)| current.iter().any(|name| !notified.contains(name)));
    if new_resource {
        lines.push(format!(
            "{} {} updates.",
            found.resource_summary(),
            if found.resource_count() == 1 {
                "has"
            } else {
                "have"
            }
        ));
    }
    meta.notified_skill_updates = found.skills.clone();
    meta.notified_repository_updates = found.repositories.clone();
    meta.notified_agent_updates = found.agents.clone();
    meta.notified_plugin_updates = found.plugins.clone();
    lines
}

fn forget_cleared_resources(
    meta: &mut crate::models::app_state::CliMeta,
    found: &AvailableUpdates,
) -> bool {
    // Do not short-circuit: every kind must forget cleared entries.
    forget_cleared(&mut meta.notified_skill_updates, &found.skills)
        | forget_cleared(&mut meta.notified_repository_updates, &found.repositories)
        | forget_cleared(&mut meta.notified_agent_updates, &found.agents)
        | forget_cleared(&mut meta.notified_plugin_updates, &found.plugins)
}

fn publish(app: &AppHandle, found: &AvailableUpdates) {
    crate::refresh_tray_update_count(app, found.resource_count());
    if let Err(e) = app.emit(UPDATES_EVENT, found) {
        log::warn!("Failed to emit update result: {e}");
    }
}

/// Refresh managed resources after source changes or Update All; retains the public
/// name used by the existing watcher. Returns false if any check was inconclusive.
pub async fn refresh_skills(app: &AppHandle) -> bool {
    let _checking = CHECK_LOCK.lock().await;
    let updates = app.state::<UpdateState>();
    let mut found = updates.0.lock().await.clone();
    let complete = resource_updates(&mut found).await;
    let mut meta = cli_manager::load_meta();
    if forget_cleared_resources(&mut meta, &found) {
        if let Err(e) = cli_manager::save_meta(&meta) {
            log::warn!("Could not save notified resource updates: {e}");
        }
    }
    let mut state = updates.0.lock().await;
    if *state != found {
        *state = found.clone();
        publish(app, &found);
    }
    complete
}

/// Drop announced resources that no longer have an update, so a later update to one of
/// them is announced again. Never adds: new updates are left for the notifying check.
fn forget_cleared(notified: &mut Vec<String>, current: &[String]) -> bool {
    let before = notified.len();
    notified.retain(|s| current.contains(s));
    notified.len() != before
}

/// Check all updates, publish the result, and notify while the app is in the background.
pub async fn check(app: &AppHandle, notify: bool) -> AvailableUpdates {
    let _checking = CHECK_LOCK.lock().await;
    let mut meta = cli_manager::load_meta();
    if let Err(e) = cli_manager::refresh_cached_version(&mut meta).await {
        log::warn!("Could not refresh cached CLI version: {e}");
    }
    let state = app.state::<UpdateState>();
    let mut found = state.0.lock().await.clone();
    resource_updates(&mut found).await;
    found.cli = latest_cli(meta.version.as_deref()).await;
    found.app = latest_app(app).await;
    forget_cleared_resources(&mut meta, &found);
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
    publish(app, &found);
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
    fn refresh_forgets_skills_that_were_updated() {
        let mut notified = vec!["a".to_string(), "b".to_string()];
        forget_cleared(&mut notified, &["b".to_string(), "c".to_string()]);
        assert_eq!(notified, vec!["b".to_string()]);
    }

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
            ..Default::default()
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
            Some((vec!["pdf".to_string()], vec!["_team".to_string()]))
        );
    }

    #[test]
    fn unparseable_check_output_is_unknown() {
        assert_eq!(parse_skill_updates("No tracked repositories"), None);
    }

    #[test]
    fn agent_check_accepts_array_and_null_without_counting_local_or_dirty_entries() {
        assert_eq!(parse_agent_updates("null"), Some(vec![]));
        assert_eq!(
            parse_agent_updates(
                r#"[{"name":"tutor","status":"update_available"},{"name":"mine","status":"local"},{"name":"edited","status":"dirty"}]"#
            ),
            Some(vec!["tutor".into()])
        );
        assert_eq!(parse_agent_updates(r#"{"error":"bad config"}"#), None);
    }

    #[test]
    fn plugin_check_deduplicates_targets_and_ignores_native_unknowns() {
        assert_eq!(
            parse_plugin_updates(
                r#"{"changes":[{"name":"hud","action":"update-available"},{"name":"hud","action":"update-available"},{"name":"native","action":"native-check"},{"name":"ok","action":"noop"},{"name":"blocked","action":"blocked"}]}"#
            ),
            Some(vec!["hud".into()])
        );
        assert_eq!(parse_plugin_updates(r#"{"changes":null}"#), Some(vec![]));
        assert_eq!(parse_plugin_updates(r#"{"error":"bad config"}"#), None);
    }

    #[test]
    fn resource_notifications_count_kinds_once_and_forget_all_cleared_kinds() {
        let found = AvailableUpdates {
            skills: vec!["pdf".into(), "xlsx".into(), "docx".into()],
            plugins: vec!["hud".into()],
            agents: vec!["tutor".into()],
            repositories: vec!["_team".into()],
            ..Default::default()
        };
        let mut meta = crate::models::app_state::CliMeta::default();
        assert_eq!(found.resource_count(), 6);
        assert_eq!(
            unannounced(&found, &mut meta),
            vec!["3 skills, 1 plugin, 1 agent, 1 repository have updates."]
        );
        assert!(unannounced(&found, &mut meta).is_empty());
        assert!(forget_cleared_resources(
            &mut meta,
            &AvailableUpdates::default()
        ));
        assert_eq!(unannounced(&found, &mut meta).len(), 1);
    }

    #[test]
    fn new_plugin_is_announced_without_reannouncing_unchanged_skills() {
        let mut meta = crate::models::app_state::CliMeta::default();
        let mut found = skills(&["pdf"]);
        unannounced(&found, &mut meta);
        found.plugins.push("hud".into());
        assert_eq!(
            unannounced(&found, &mut meta),
            vec!["1 skill, 1 plugin have updates."]
        );
        found.skills.clear();
        assert!(unannounced(&found, &mut meta).is_empty());
    }

    #[test]
    fn partial_checks_preserve_failed_kinds_while_clearing_successful_kinds() {
        let mut found = AvailableUpdates {
            skills: vec!["pdf".into()],
            repositories: vec!["_team".into()],
            agents: vec!["tutor".into()],
            plugins: vec!["hud".into()],
            ..Default::default()
        };
        assert!(!apply_resource_results(
            &mut found,
            None,
            Some(vec![]),
            None
        ));
        assert_eq!(found.skills, vec!["pdf"]);
        assert_eq!(found.repositories, vec!["_team"]);
        assert_eq!(found.plugins, vec!["hud"]);
        assert!(found.agents.is_empty());
        assert!(apply_resource_results(
            &mut found,
            Some((vec![], vec![])),
            Some(vec![]),
            Some(vec![])
        ));
        assert_eq!(found.resource_count(), 0);
    }

    #[test]
    fn legacy_meta_loads_new_notification_lists_as_empty() {
        let meta: crate::models::app_state::CliMeta =
            serde_json::from_str(r#"{"notifiedSkillUpdates":["pdf"]}"#).unwrap_or_default();
        assert_eq!(meta.notified_skill_updates, vec!["pdf"]);
        assert!(meta.notified_agent_updates.is_empty());
        assert!(meta.notified_repository_updates.is_empty());
        assert!(meta.notified_plugin_updates.is_empty());
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
