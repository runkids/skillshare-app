use crate::services::source_health;
use tauri::State;

#[tauri::command]
pub async fn get_source_health(
    state: State<'_, source_health::SourceHealthState>,
) -> Result<source_health::SourceHealth, String> {
    Ok(state.0.lock().await.clone())
}
