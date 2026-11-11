//! The CLI owns the shared config; use its read-only JSON command, never parse YAML here.
use super::cli_manager;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub const POLICY_EVENT: &str = "update-policy-changed";

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePolicy {
    pub enabled: bool,
    pub configured: bool,
    pub env_override: bool,
    #[serde(default = "supported")]
    pub supported: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn supported() -> bool {
    true
}

fn legacy_policy() -> UpdatePolicy {
    let env = std::env::var("SKILLSHARE_AUTO_UPDATE_CHECK").ok();
    let override_value = env.as_deref().and_then(parse_override);
    UpdatePolicy {
        enabled: override_value.unwrap_or(true),
        configured: true,
        env_override: override_value.is_some(),
        supported: false,
        error: None,
    }
}

fn parse_override(value: &str) -> Option<bool> {
    match value.trim() {
        "1" | "t" | "T" | "TRUE" | "true" | "True" => Some(true),
        "0" | "f" | "F" | "FALSE" | "false" | "False" => Some(false),
        _ => None,
    }
}

pub async fn read() -> Result<UpdatePolicy, String> {
    let Some(path) = cli_manager::detect_cli().await else {
        return Ok(legacy_policy());
    };
    let args = vec!["updates".into(), "--json".into()];
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        cli_manager::exec(&path, &args, None),
    )
    .await
    .map_err(|_| "Reading update settings timed out")?;
    match result {
        Ok(json) => {
            serde_json::from_str(&json).map_err(|e| format!("Invalid update settings: {e}"))
        }
        Err(e) if e.to_ascii_lowercase().contains("unknown command") => Ok(legacy_policy()),
        Err(e) => Err(e),
    }
}

pub async fn enabled() -> bool {
    read().await.map(|p| p.enabled).unwrap_or(false)
}

pub async fn refresh(app: &AppHandle) -> Result<UpdatePolicy, String> {
    let policy = read().await.unwrap_or_else(|e| UpdatePolicy {
        enabled: false,
        configured: true,
        env_override: false,
        supported: false,
        error: Some(e),
    });
    // Consumers must see the opt-out before the cache-clearing update event.
    let _ = app.emit(POLICY_EVENT, &policy);
    if !policy.enabled {
        let state = app.state::<super::update_watch::UpdateState>();
        let mut found = state.0.lock().await;
        if found.cli.take().is_some() | found.app.take().is_some() {
            let _ = app.emit(super::update_watch::UPDATES_EVENT, &*found);
        }
    }
    Ok(policy)
}

pub async fn set(app: &AppHandle, enabled: bool) -> Result<UpdatePolicy, String> {
    let policy = read().await?;
    if !policy.supported {
        return Err("Upgrade the CLI to manage automatic update checks.".into());
    }
    if policy.env_override {
        return Err("Controlled by SKILLSHARE_AUTO_UPDATE_CHECK.".into());
    }
    let path = cli_manager::detect_cli().await.ok_or("CLI not found")?;
    let args = vec![
        "updates".into(),
        if enabled { "on" } else { "off" }.into(),
        "--json".into(),
    ];
    tokio::time::timeout(
        Duration::from_secs(10),
        cli_manager::exec(&path, &args, None),
    )
    .await
    .map_err(|_| "Saving update settings timed out")??;
    refresh(app).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_json_preserves_effective_and_saved_preferences() -> Result<(), serde_json::Error> {
        let policy: UpdatePolicy =
            serde_json::from_str(r#"{"enabled":false,"configured":true,"envOverride":true}"#)?;
        assert!(!policy.enabled);
        assert!(policy.configured && policy.env_override && policy.supported);
        Ok(())
    }

    #[test]
    fn overrides_match_cli_boolean_values() {
        for v in ["0", "false", "F", " False "] {
            assert_eq!(parse_override(v), Some(false));
        }
        for v in ["1", "true", "T", " True "] {
            assert_eq!(parse_override(v), Some(true));
        }
        for v in ["", "off", "invalid"] {
            assert_eq!(parse_override(v), None);
        }
    }
}
