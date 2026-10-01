//! Background update checks for the CLI and the app itself.

use crate::services::cli_manager;
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

/// Newer versions found by the last check; `None` means up to date (or unknown).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableUpdates {
    pub cli: Option<String>,
    pub app: Option<String>,
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
    lines
}

/// Check both components, publish the result, and notify while the app is in the background.
pub async fn check(app: &AppHandle, notify: bool) -> AvailableUpdates {
    let mut meta = cli_manager::load_meta();
    if let Err(e) = cli_manager::refresh_cached_version(&mut meta).await {
        log::warn!("Could not refresh cached CLI version: {e}");
    }
    let found = AvailableUpdates {
        cli: latest_cli(meta.version.as_deref()).await,
        app: latest_app(app).await,
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

    *app.state::<UpdateState>().0.lock().await = found.clone();
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
        };
        let first = unannounced(&found, &mut meta).len();
        let second = unannounced(&found, &mut meta).len();
        assert_eq!((first, second), (1, 0));
    }
}
