use crate::utils::env;
use std::collections::HashMap;

#[tauri::command]
pub async fn get_pty_env() -> Result<HashMap<String, String>, String> {
    Ok(env::build_env_for_child())
}
