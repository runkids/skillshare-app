//! The re-check behind the title bar status panel's Check now. The panel's other actions
//! open Web UI pages, so the user starts every change from the CLI's own screen.

use crate::services::{audit, source_health, update_watch};
use tauri::AppHandle;

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
