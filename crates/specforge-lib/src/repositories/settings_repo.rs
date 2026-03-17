// Settings Repository
// Handles all database operations for settings (key-value store)

use chrono::Utc;
use rusqlite::params;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::utils::database::Database;
use crate::utils::store::{AppSettings, KeyboardShortcutsSettings, NotificationSettings};

/// Well-known settings keys
pub const KEY_APP_SETTINGS: &str = "app_settings";
pub const KEY_KEYBOARD_SHORTCUTS: &str = "keyboard_shortcuts";
pub const KEY_NOTIFICATION_SETTINGS: &str = "notification_settings";
pub const KEY_PROJECT_SORT_MODE: &str = "project_sort_mode";
pub const KEY_PROJECT_ORDER: &str = "project_order";
pub const KEY_WORKFLOW_SORT_MODE: &str = "workflow_sort_mode";
pub const KEY_WORKFLOW_ORDER: &str = "workflow_order";
pub const KEY_TEMPLATE_PREFERENCES: &str = "template_preferences";

/// View mode for template display
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TemplateViewMode {
    Categories,
    All,
    Favorites,
}

impl Default for TemplateViewMode {
    fn default() -> Self {
        Self::Categories
    }
}

/// Recently used template entry with timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentTemplateEntry {
    pub template_id: String,
    pub used_at: String, // ISO 8601
}

/// Template preferences structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TemplatePreferences {
    pub favorites: Vec<String>,
    pub recently_used: Vec<RecentTemplateEntry>,
    pub collapsed_categories: Vec<String>,
    pub preferred_view: TemplateViewMode,
}

/// Maximum number of recent templates to track
const MAX_RECENT_TEMPLATES: usize = 8;

/// Repository for settings data access
pub struct SettingsRepository {
    db: Database,
}

impl SettingsRepository {
    /// Create a new SettingsRepository
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Get a setting value by key
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |row| {
                    let value: String = row.get(0)?;
                    Ok(value)
                },
            );

            match result {
                Ok(json) => {
                    let value: T = serde_json::from_str(&json)
                        .map_err(|e| format!("Failed to parse setting '{}': {}", key, e))?;
                    Ok(Some(value))
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get setting '{}': {}", key, e)),
            }
        })
    }

    /// Get a setting value or return default
    pub fn get_or_default<T: DeserializeOwned + Default>(&self, key: &str) -> Result<T, String> {
        self.get(key).map(|opt| opt.unwrap_or_default())
    }

    /// Set a setting value
    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<(), String> {
        let json = serde_json::to_string(value)
            .map_err(|e| format!("Failed to serialize setting '{}': {}", key, e))?;

        let now = Utc::now().to_rfc3339();

        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
                params![key, json, now],
            )
            .map_err(|e| format!("Failed to save setting '{}': {}", key, e))?;

            Ok(())
        })
    }

    /// Delete a setting
    pub fn delete(&self, key: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let rows_affected = conn
                .execute("DELETE FROM settings WHERE key = ?1", params![key])
                .map_err(|e| format!("Failed to delete setting '{}': {}", key, e))?;

            Ok(rows_affected > 0)
        })
    }

    /// List all setting keys
    pub fn list_keys(&self) -> Result<Vec<String>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare("SELECT key FROM settings ORDER BY key")
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map([], |row| row.get(0))
                .map_err(|e| format!("Failed to query settings: {}", e))?;

            let mut keys = Vec::new();
            for row in rows {
                keys.push(row.map_err(|e| format!("Failed to read row: {}", e))?);
            }

            Ok(keys)
        })
    }

    // =========================================================================
    // Typed convenience methods for common settings
    // =========================================================================

    /// Get app settings
    pub fn get_app_settings(&self) -> Result<AppSettings, String> {
        self.get_or_default(KEY_APP_SETTINGS)
    }

    /// Save app settings
    pub fn save_app_settings(&self, settings: &AppSettings) -> Result<(), String> {
        self.set(KEY_APP_SETTINGS, settings)
    }

    /// Get keyboard shortcuts settings
    pub fn get_keyboard_shortcuts(&self) -> Result<KeyboardShortcutsSettings, String> {
        self.get_or_default(KEY_KEYBOARD_SHORTCUTS)
    }

    /// Save keyboard shortcuts settings
    pub fn save_keyboard_shortcuts(
        &self,
        shortcuts: &KeyboardShortcutsSettings,
    ) -> Result<(), String> {
        self.set(KEY_KEYBOARD_SHORTCUTS, shortcuts)
    }

    /// Get notification settings
    pub fn get_notification_settings(&self) -> Result<NotificationSettings, String> {
        self.get_or_default(KEY_NOTIFICATION_SETTINGS)
    }

    /// Save notification settings
    pub fn save_notification_settings(
        &self,
        settings: &NotificationSettings,
    ) -> Result<(), String> {
        self.set(KEY_NOTIFICATION_SETTINGS, settings)
    }

    /// Get project sort mode
    pub fn get_project_sort_mode(&self) -> Result<String, String> {
        self.get(KEY_PROJECT_SORT_MODE)
            .map(|opt| opt.unwrap_or_else(|| "lastOpened".to_string()))
    }

    /// Set project sort mode
    pub fn set_project_sort_mode(&self, mode: &str) -> Result<(), String> {
        self.set(KEY_PROJECT_SORT_MODE, &mode.to_string())
    }

    /// Get project order (for custom sorting)
    pub fn get_project_order(&self) -> Result<Vec<String>, String> {
        self.get_or_default(KEY_PROJECT_ORDER)
    }

    /// Set project order
    pub fn set_project_order(&self, order: &[String]) -> Result<(), String> {
        self.set(KEY_PROJECT_ORDER, &order)
    }

    /// Get workflow sort mode
    pub fn get_workflow_sort_mode(&self) -> Result<String, String> {
        self.get(KEY_WORKFLOW_SORT_MODE)
            .map(|opt| opt.unwrap_or_else(|| "updated".to_string()))
    }

    /// Set workflow sort mode
    pub fn set_workflow_sort_mode(&self, mode: &str) -> Result<(), String> {
        self.set(KEY_WORKFLOW_SORT_MODE, &mode.to_string())
    }

    /// Get workflow order (for custom sorting)
    pub fn get_workflow_order(&self) -> Result<Vec<String>, String> {
        self.get_or_default(KEY_WORKFLOW_ORDER)
    }

    /// Set workflow order
    pub fn set_workflow_order(&self, order: &[String]) -> Result<(), String> {
        self.set(KEY_WORKFLOW_ORDER, &order)
    }

    // =========================================================================
    // Template Preferences methods
    // =========================================================================

    /// Get template preferences
    pub fn get_template_preferences(&self) -> Result<TemplatePreferences, String> {
        self.get_or_default(KEY_TEMPLATE_PREFERENCES)
    }

    /// Save template preferences
    pub fn save_template_preferences(&self, prefs: &TemplatePreferences) -> Result<(), String> {
        self.set(KEY_TEMPLATE_PREFERENCES, prefs)
    }

    /// Toggle a template favorite
    pub fn toggle_template_favorite(&self, template_id: &str) -> Result<TemplatePreferences, String> {
        let mut prefs = self.get_template_preferences()?;

        if let Some(pos) = prefs.favorites.iter().position(|id| id == template_id) {
            prefs.favorites.remove(pos);
        } else {
            prefs.favorites.push(template_id.to_string());
        }

        self.save_template_preferences(&prefs)?;
        Ok(prefs)
    }

    /// Add a template to favorites
    pub fn add_template_favorite(&self, template_id: &str) -> Result<TemplatePreferences, String> {
        let mut prefs = self.get_template_preferences()?;

        if !prefs.favorites.contains(&template_id.to_string()) {
            prefs.favorites.push(template_id.to_string());
            self.save_template_preferences(&prefs)?;
        }

        Ok(prefs)
    }

    /// Remove a template from favorites
    pub fn remove_template_favorite(&self, template_id: &str) -> Result<TemplatePreferences, String> {
        let mut prefs = self.get_template_preferences()?;
        prefs.favorites.retain(|id| id != template_id);
        self.save_template_preferences(&prefs)?;
        Ok(prefs)
    }

    /// Record template usage (adds to recently used list)
    pub fn record_template_usage(&self, template_id: &str) -> Result<TemplatePreferences, String> {
        let mut prefs = self.get_template_preferences()?;

        // Remove existing entry if present
        prefs.recently_used.retain(|entry| entry.template_id != template_id);

        // Add new entry at the beginning
        let new_entry = RecentTemplateEntry {
            template_id: template_id.to_string(),
            used_at: Utc::now().to_rfc3339(),
        };
        prefs.recently_used.insert(0, new_entry);

        // Keep only MAX_RECENT_TEMPLATES entries
        prefs.recently_used.truncate(MAX_RECENT_TEMPLATES);

        self.save_template_preferences(&prefs)?;
        Ok(prefs)
    }

    /// Clear recently used templates
    pub fn clear_recently_used_templates(&self) -> Result<TemplatePreferences, String> {
        let mut prefs = self.get_template_preferences()?;
        prefs.recently_used.clear();
        self.save_template_preferences(&prefs)?;
        Ok(prefs)
    }

    /// Toggle category collapse state
    pub fn toggle_template_category_collapse(&self, category_id: &str) -> Result<TemplatePreferences, String> {
        let mut prefs = self.get_template_preferences()?;

        if let Some(pos) = prefs.collapsed_categories.iter().position(|id| id == category_id) {
            prefs.collapsed_categories.remove(pos);
        } else {
            prefs.collapsed_categories.push(category_id.to_string());
        }

        self.save_template_preferences(&prefs)?;
        Ok(prefs)
    }

    /// Expand all categories
    pub fn expand_all_template_categories(&self) -> Result<TemplatePreferences, String> {
        let mut prefs = self.get_template_preferences()?;
        prefs.collapsed_categories.clear();
        self.save_template_preferences(&prefs)?;
        Ok(prefs)
    }

    /// Collapse specific categories
    pub fn collapse_template_categories(&self, category_ids: Vec<String>) -> Result<TemplatePreferences, String> {
        let mut prefs = self.get_template_preferences()?;
        prefs.collapsed_categories = category_ids;
        self.save_template_preferences(&prefs)?;
        Ok(prefs)
    }

    /// Set preferred view mode
    pub fn set_template_preferred_view(&self, view: TemplateViewMode) -> Result<TemplatePreferences, String> {
        let mut prefs = self.get_template_preferences()?;
        prefs.preferred_view = view;
        self.save_template_preferences(&prefs)?;
        Ok(prefs)
    }
}
