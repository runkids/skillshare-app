use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CliMeta {
    pub version: Option<String>,
    pub path: Option<String>,
    pub source: Option<String>,
    pub installed_at: Option<String>,
    pub last_update_check: Option<String>,
    pub preferred_port: Option<u16>,
    pub notify_sync: Option<bool>,
    pub notify_update: Option<bool>,
    /// Sync automatically when the skills source changes; off unless the user opts in.
    pub auto_sync: Option<bool>,
    pub quick_actions_enabled: Option<bool>,
    pub quick_actions_shortcut: Option<String>,
    /// Binary mtime (ms since epoch) when `version` was read; detects upgrades made outside the app.
    pub binary_modified_ms: Option<u64>,
    /// Last versions announced by a notification, so each release is announced once.
    pub notified_cli_version: Option<String>,
    pub notified_app_version: Option<String>,
    /// Skills with updates as of the last notification, so the same set is announced once.
    #[serde(default)]
    pub notified_skill_updates: Vec<String>,
    #[serde(default)]
    pub notified_repository_updates: Vec<String>,
    #[serde(default)]
    pub notified_agent_updates: Vec<String>,
    #[serde(default)]
    pub notified_plugin_updates: Vec<String>,
    /// Source health findings as of the last notification, so each is announced once.
    #[serde(default)]
    pub notified_source_health: Vec<String>,
    /// HIGH/CRITICAL audit findings as of the last notification, so each is announced once.
    #[serde(default)]
    pub notified_audit_findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingStatus {
    pub completed: bool,
    pub cli_ready: bool,
    pub first_project_created: bool,
    pub first_sync_done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub cli_version: Option<String>,
    pub cli_source: Option<String>,
    pub server_running: bool,
    pub server_port: Option<u16>,
    pub onboarding: OnboardingStatus,
}
