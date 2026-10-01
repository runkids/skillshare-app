use crate::services::audit;
use tauri::State;

#[tauri::command]
pub async fn get_audit_report(
    state: State<'_, audit::AuditState>,
) -> Result<Vec<audit::AuditFinding>, String> {
    Ok(state.0.lock().await.clone())
}
