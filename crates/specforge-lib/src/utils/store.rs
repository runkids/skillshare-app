// Store helper functions
// Provides utilities for loading and saving data using tauri-plugin-store

use crate::models::{Execution, Project, SecurityScanData, Workflow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Default scan reminder interval in days
fn default_scan_reminder_interval() -> u32 {
    7
}

/// Default project sort mode
fn default_project_sort_mode() -> String {
    String::from("name")
}

/// Default webhook notifications enabled
fn default_webhook_notifications_enabled() -> bool {
    true
}

/// Default workflow sort mode
fn default_workflow_sort_mode() -> String {
    String::from("updated")
}

/// Default global shortcuts enabled
fn default_global_shortcuts_enabled() -> bool {
    true
}

/// Default global toggle shortcut
fn default_global_toggle_shortcut() -> String {
    String::from("cmd+shift+p")
}

/// Default path display format
fn default_path_display_format() -> String {
    String::from("short")
}

/// Default reduce motion setting
fn default_reduce_motion() -> bool {
    false
}

/// Custom shortcut binding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomShortcutBinding {
    /// Shortcut identifier
    pub id: String,
    /// Custom key combination (None = use default)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_key: Option<String>,
    /// Whether this shortcut is enabled
    pub enabled: bool,
}

/// Keyboard shortcuts settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardShortcutsSettings {
    /// Settings version for migration
    #[serde(default = "default_keyboard_shortcuts_version")]
    pub version: u32,
    /// Custom shortcut bindings (keyed by shortcut id)
    #[serde(default)]
    pub custom_bindings: HashMap<String, CustomShortcutBinding>,
    /// Whether global shortcuts are enabled
    #[serde(default = "default_global_shortcuts_enabled")]
    pub global_shortcuts_enabled: bool,
    /// Global shortcut for toggling window visibility
    #[serde(default = "default_global_toggle_shortcut")]
    pub global_toggle_shortcut: String,
}

fn default_keyboard_shortcuts_version() -> u32 {
    1
}

impl Default for KeyboardShortcutsSettings {
    fn default() -> Self {
        Self {
            version: default_keyboard_shortcuts_version(),
            custom_bindings: HashMap::new(),
            global_shortcuts_enabled: default_global_shortcuts_enabled(),
            global_toggle_shortcut: default_global_toggle_shortcut(),
        }
    }
}

// ============================================================================
// Notification Settings
// ============================================================================

/// Default notification enabled
fn default_notification_enabled() -> bool {
    true
}

/// Default notification sound enabled
fn default_notification_sound_enabled() -> bool {
    true
}

/// Default DND start time
fn default_dnd_start_time() -> String {
    String::from("22:00")
}

/// Default DND end time
fn default_dnd_end_time() -> String {
    String::from("08:00")
}

/// Do Not Disturb settings for notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoNotDisturbSettings {
    /// Whether DND is enabled
    #[serde(default)]
    pub enabled: bool,
    /// Start time in 24h format (e.g., "22:00")
    #[serde(default = "default_dnd_start_time")]
    pub start_time: String,
    /// End time in 24h format (e.g., "08:00")
    #[serde(default = "default_dnd_end_time")]
    pub end_time: String,
}

impl Default for DoNotDisturbSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            start_time: default_dnd_start_time(),
            end_time: default_dnd_end_time(),
        }
    }
}

/// Notification categories toggle settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationCategories {
    /// Webhook notifications (incoming triggered, outgoing success/failure)
    #[serde(default = "default_notification_enabled")]
    pub webhooks: bool,
    /// Workflow execution (completed, failed)
    #[serde(default = "default_notification_enabled")]
    pub workflow_execution: bool,
    /// Git operations (push success/failure)
    #[serde(default = "default_notification_enabled")]
    pub git_operations: bool,
    /// Security scan (completed, vulnerabilities found)
    #[serde(default = "default_notification_enabled")]
    pub security_scans: bool,
    /// Deployment (success, failure)
    #[serde(default = "default_notification_enabled")]
    pub deployments: bool,
}

impl Default for NotificationCategories {
    fn default() -> Self {
        Self {
            webhooks: default_notification_enabled(),
            workflow_execution: default_notification_enabled(),
            git_operations: default_notification_enabled(),
            security_scans: default_notification_enabled(),
            deployments: default_notification_enabled(),
        }
    }
}

/// Notification settings stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationSettings {
    /// Master toggle for all notifications
    #[serde(default = "default_notification_enabled")]
    pub enabled: bool,
    /// Play sound with notifications
    #[serde(default = "default_notification_sound_enabled")]
    pub sound_enabled: bool,
    /// Category-specific toggles
    #[serde(default)]
    pub categories: NotificationCategories,
    /// Do Not Disturb settings
    #[serde(default)]
    pub do_not_disturb: DoNotDisturbSettings,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: default_notification_enabled(),
            sound_enabled: default_notification_sound_enabled(),
            categories: NotificationCategories::default(),
            do_not_disturb: DoNotDisturbSettings::default(),
        }
    }
}

/// Application settings stored in settings.json
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub default_timeout: u64,
    pub sidebar_width: u32,
    pub terminal_height: u32,
    pub theme: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_workflow_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_project_id: Option<String>,
    /// Security scan reminder interval in days (default: 7)
    #[serde(default = "default_scan_reminder_interval")]
    pub scan_reminder_interval_days: u32,
    /// Project sort mode: "name" | "lastOpened" | "created" | "custom"
    #[serde(default = "default_project_sort_mode")]
    pub project_sort_mode: String,
    /// Project order for custom sorting (array of project IDs)
    #[serde(default)]
    pub project_order: Vec<String>,
    /// Whether to show desktop notifications for webhook events (default: true)
    #[serde(default = "default_webhook_notifications_enabled")]
    pub webhook_notifications_enabled: bool,
    /// Workflow sort mode: "name" | "updated" | "created" | "custom"
    #[serde(default = "default_workflow_sort_mode")]
    pub workflow_sort_mode: String,
    /// Workflow order for custom sorting (array of workflow IDs)
    #[serde(default)]
    pub workflow_order: Vec<String>,
    /// Keyboard shortcuts settings
    #[serde(default)]
    pub keyboard_shortcuts: KeyboardShortcutsSettings,
    /// Path display format: "short" (with ~/...) | "full" (complete path)
    #[serde(default = "default_path_display_format")]
    pub path_display_format: String,
    /// Reduce motion setting for accessibility
    #[serde(default = "default_reduce_motion")]
    pub reduce_motion: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_timeout: 600000,
            sidebar_width: 240,
            terminal_height: 200,
            theme: String::from("dark"),
            last_workflow_id: None,
            last_project_id: None,
            scan_reminder_interval_days: default_scan_reminder_interval(),
            project_sort_mode: default_project_sort_mode(),
            project_order: Vec::new(),
            webhook_notifications_enabled: default_webhook_notifications_enabled(),
            workflow_sort_mode: default_workflow_sort_mode(),
            workflow_order: Vec::new(),
            keyboard_shortcuts: KeyboardShortcutsSettings::default(),
            path_display_format: default_path_display_format(),
            reduce_motion: default_reduce_motion(),
        }
    }
}

/// Complete store schema
/// Note: Uses `#[serde(default)]` on all fields to gracefully handle unknown fields (e.g., mcp_server_config)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreData {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub projects: Vec<Project>,
    #[serde(default)]
    pub workflows: Vec<Workflow>,
    #[serde(default)]
    pub running_executions: HashMap<String, Execution>,
    #[serde(default)]
    pub settings: AppSettings,
    /// Security scan data per project (keyed by project ID)
    #[serde(default)]
    pub security_scans: HashMap<String, SecurityScanData>,
}

impl Default for StoreData {
    fn default() -> Self {
        Self {
            version: String::from("2.0.0"),
            projects: Vec::new(),
            workflows: Vec::new(),
            running_executions: HashMap::new(),
            settings: AppSettings::default(),
            security_scans: HashMap::new(),
        }
    }
}
