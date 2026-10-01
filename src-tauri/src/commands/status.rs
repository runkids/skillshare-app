//! Actions and the re-check behind the title bar status panel. Each action reuses the
//! tray's implementation, so locks, confirm dialogs, timeouts and notifications match.

use crate::services::{audit, oplog_watch, source_health, update_all, update_watch};
use tauri::AppHandle;

#[derive(Debug, PartialEq)]
enum StatusAction {
    UpdateAll,
    Sync,
    Collect,
    Push,
    Pull,
}

fn parse_action(action: &str) -> Option<StatusAction> {
    match action {
        "update_all" => Some(StatusAction::UpdateAll),
        "sync" => Some(StatusAction::Sync),
        "collect" => Some(StatusAction::Collect),
        "push" => Some(StatusAction::Push),
        "pull" => Some(StatusAction::Pull),
        _ => None,
    }
}

/// Start a panel action and return at once; its result arrives as a notification and events.
#[tauri::command]
pub fn run_status_action(app: AppHandle, action: String) -> Result<(), String> {
    let action = parse_action(&action).ok_or_else(|| format!("Unknown status action: {action}"))?;
    match action {
        StatusAction::UpdateAll => {
            tauri::async_runtime::spawn(async move {
                update_all::run(&app).await;
                source_health::refresh(&app, false).await;
            });
        }
        StatusAction::Sync => {
            tauri::async_runtime::spawn(async move {
                crate::handle_quick_sync(&app).await;
                oplog_watch::refresh_badges(&app).await;
            });
        }
        StatusAction::Collect => source_health::confirm_collect(&app),
        StatusAction::Push => source_health::spawn_action(&app, &["push"], "Push"),
        StatusAction::Pull => source_health::spawn_action(&app, &["pull"], "Pull"),
    }
    Ok(())
}

/// Re-run the update check, the source health check (with a fetch) and the audit now.
/// Each publishes its own event; none of them notifies for findings already announced.
#[tauri::command]
pub async fn check_status_now(app: AppHandle) -> Result<(), String> {
    tokio::join!(
        update_watch::check(&app, false),
        source_health::refresh(&app, true),
        audit::run(&app),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_each_panel_action() {
        assert_eq!(parse_action("update_all"), Some(StatusAction::UpdateAll));
        assert_eq!(parse_action("sync"), Some(StatusAction::Sync));
        assert_eq!(parse_action("collect"), Some(StatusAction::Collect));
        assert_eq!(parse_action("push"), Some(StatusAction::Push));
        assert_eq!(parse_action("pull"), Some(StatusAction::Pull));
    }

    #[test]
    fn rejects_unknown_actions() {
        assert_eq!(parse_action("reset_all_data"), None);
    }
}
