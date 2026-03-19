use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CliMeta {
    pub version: Option<String>,
    pub path: Option<String>,
    pub source: Option<String>,
    pub installed_at: Option<String>,
    pub last_update_check: Option<String>,
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
