//! Tray Update All for managed resources, sharing Quick Sync's execution lock.

use super::{cli_manager, project_store, update_watch};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use update_watch::{AvailableUpdates, UpdateState};

const UPDATE_TIMEOUT: Duration = Duration::from_secs(120);
static RUN_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub fn is_running() -> bool {
    RUN_LOCK.try_lock().is_err()
}

fn update_commands(found: &AvailableUpdates, mode: &str) -> Vec<Vec<String>> {
    let mut commands: Vec<Vec<String>> = Vec::new();
    if !found.skills.is_empty() || !found.repositories.is_empty() {
        commands.push(
            ["update", "--all", "--json", mode]
                .map(String::from)
                .to_vec(),
        );
    }
    if !found.agents.is_empty() {
        commands.push(
            ["update", "agents", "--all", "--json", mode]
                .map(String::from)
                .to_vec(),
        );
    }
    // plugin update requires a package name. --json disables interactive prompts;
    // the CLI still checks native support and preserves its preview/apply safeguards.
    for name in &found.plugins {
        commands.push(
            ["plugin", "update", name, "--json", mode]
                .map(String::from)
                .to_vec(),
        );
    }
    commands
}

async fn exec(cli: &str, args: &[String], dir: Option<&str>) -> Result<String, String> {
    tokio::time::timeout(UPDATE_TIMEOUT, cli_manager::exec(cli, args, dir))
        .await
        .unwrap_or_else(|_| Err(format!("timed out after {}s", UPDATE_TIMEOUT.as_secs())))
}

fn notification(errors: &[String], checked: bool, remaining: usize) -> (&'static str, String) {
    if !errors.is_empty() {
        return (
            "Skillshare Update Failed",
            errors.join("\n").chars().take(200).collect(),
        );
    }
    if !checked {
        return (
            "Skillshare Update Incomplete",
            "Updates ran, but the final check failed. Check again in the app.".into(),
        );
    }
    if remaining > 0 {
        return (
            "Skillshare Update Incomplete",
            format!(
                "{remaining} update(s) remain. Open the app to review skipped or blocked updates."
            ),
        );
    }
    (
        "Skillshare Update Complete",
        "All resource updates applied and synced.".into(),
    )
}

pub async fn run(app: &AppHandle) {
    let Ok(running) = RUN_LOCK.try_lock() else {
        return;
    };
    let found = app.state::<UpdateState>().0.lock().await.clone();
    if found.resource_count() == 0 {
        return;
    }
    crate::refresh_tray_update_count(app, found.resource_count());

    // Keep Quick Sync and auto-sync from racing writes during updates and syncs.
    let syncing = crate::SYNC_LOCK.lock().await;
    let (dir, is_project) = project_store::active_project_mode(&project_store::load());
    let mode = if is_project { "--project" } else { "--global" };
    let mut errors = Vec::new();
    if let Some(cli) = cli_manager::detect_cli().await {
        for args in update_commands(&found, mode) {
            if let Err(e) = exec(&cli, &args, dir.as_deref()).await {
                errors.push(format!("{}: {e}", args[..args.len() - 2].join(" ")));
            }
        }
        // Sync skills and agents only; sync --all would also change extras, MCP and hooks.
        let mut syncs = vec![vec!["sync", "--json", mode]];
        if !found.agents.is_empty() {
            syncs.push(vec!["sync", "agents", "--json", mode]);
        }
        let mut synced = false;
        for args in syncs {
            let args: Vec<_> = args.into_iter().map(String::from).collect();
            match exec(&cli, &args, dir.as_deref()).await {
                Ok(_) => synced = true,
                Err(e) => errors.push(format!("{}: {e}", args.join(" "))),
            }
        }
        if synced {
            let _ = app.emit(crate::SYNC_COMPLETED_EVENT, ());
        }
    } else {
        errors.push("Skillshare CLI not found.".into());
    }
    let checked = update_watch::refresh_skills(app).await;
    let remaining = app.state::<UpdateState>().0.lock().await.resource_count();
    drop(syncing);
    drop(running);
    crate::refresh_tray_update_count(app, remaining);
    if !errors.is_empty() {
        log::warn!("Update All failed: {}", errors.join("; "));
    }
    if cli_manager::load_meta().notify_sync.unwrap_or(true) {
        let (title, body) = notification(&errors, checked, remaining);
        let _ = app.notification().builder().title(title).body(body).show();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_cover_each_kind_and_require_named_plugins_without_force() {
        let found = AvailableUpdates {
            repositories: vec!["_team".into()],
            agents: vec!["tutor".into()],
            plugins: vec!["hud".into()],
            ..Default::default()
        };
        assert_eq!(
            update_commands(&found, "--project"),
            vec![
                vec!["update", "--all", "--json", "--project"],
                vec!["update", "agents", "--all", "--json", "--project"],
                vec!["plugin", "update", "hud", "--json", "--project"],
            ]
        );
    }

    #[test]
    fn app_and_cli_updates_are_not_resource_commands() {
        let found = AvailableUpdates {
            app: Some("1.0.0".into()),
            cli: Some("1.0.0".into()),
            ..Default::default()
        };
        assert!(update_commands(&found, "--global").is_empty());
    }

    #[test]
    fn failed_or_inconclusive_checks_and_skips_never_report_success() {
        assert_eq!(
            notification(&["blocked".into()], true, 0).0,
            "Skillshare Update Failed"
        );
        assert_eq!(
            notification(&[], false, 0).0,
            "Skillshare Update Incomplete"
        );
        assert_eq!(notification(&[], true, 1).0, "Skillshare Update Incomplete");
        assert_eq!(notification(&[], true, 0).0, "Skillshare Update Complete");
    }
}
