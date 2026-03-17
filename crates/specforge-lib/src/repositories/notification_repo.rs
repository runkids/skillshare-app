// Notification Repository
// Handles all database operations for notification history

use chrono::{DateTime, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::utils::database::Database;

/// Notification record stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationRecord {
    pub id: String,
    pub notification_type: String,
    pub category: String,
    pub title: String,
    pub body: String,
    pub is_read: bool,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Response for notification list queries
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationListResponse {
    pub notifications: Vec<NotificationRecord>,
    pub total_count: u32,
    pub unread_count: u32,
}

/// Repository for notification data access
pub struct NotificationRepository {
    db: Database,
}

impl NotificationRepository {
    /// Create a new NotificationRepository
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Create a new notification record
    pub fn create(&self, notification: &NotificationRecord) -> Result<String, String> {
        let metadata_json = notification
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(m).ok())
            .flatten();

        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO notifications (id, notification_type, category, title, body, is_read, metadata, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
                params![
                    notification.id,
                    notification.notification_type,
                    notification.category,
                    notification.title,
                    notification.body,
                    notification.is_read as i32,
                    metadata_json,
                    notification.created_at.to_rfc3339(),
                ],
            )
            .map_err(|e| format!("Failed to create notification: {}", e))?;

            Ok(notification.id.clone())
        })
    }

    /// Get recent notifications with pagination
    pub fn get_recent(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<NotificationListResponse, String> {
        self.db.with_connection(|conn| {
            // Get notifications
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, notification_type, category, title, body, is_read, metadata, created_at
                    FROM notifications
                    ORDER BY created_at DESC
                    LIMIT ?1 OFFSET ?2
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map(params![limit as i64, offset as i64], |row| {
                    Ok(NotificationRow {
                        id: row.get(0)?,
                        notification_type: row.get(1)?,
                        category: row.get(2)?,
                        title: row.get(3)?,
                        body: row.get(4)?,
                        is_read: row.get(5)?,
                        metadata: row.get(6)?,
                        created_at: row.get(7)?,
                    })
                })
                .map_err(|e| format!("Failed to query notifications: {}", e))?;

            let mut notifications = Vec::new();
            for row in rows {
                let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                notifications.push(row.into_record()?);
            }

            // Get total count
            let total_count: u32 = conn
                .query_row("SELECT COUNT(*) FROM notifications", [], |row| row.get(0))
                .map_err(|e| format!("Failed to count notifications: {}", e))?;

            // Get unread count
            let unread_count: u32 = conn
                .query_row(
                    "SELECT COUNT(*) FROM notifications WHERE is_read = 0",
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| format!("Failed to count unread notifications: {}", e))?;

            Ok(NotificationListResponse {
                notifications,
                total_count,
                unread_count,
            })
        })
    }

    /// Get unread notification count
    pub fn get_unread_count(&self) -> Result<u32, String> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM notifications WHERE is_read = 0",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count unread notifications: {}", e))
        })
    }

    /// Mark a notification as read
    pub fn mark_as_read(&self, id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let updated = conn
                .execute(
                    "UPDATE notifications SET is_read = 1 WHERE id = ?1",
                    params![id],
                )
                .map_err(|e| format!("Failed to mark notification as read: {}", e))?;

            Ok(updated > 0)
        })
    }

    /// Mark all notifications as read
    pub fn mark_all_as_read(&self) -> Result<u32, String> {
        self.db.with_connection(|conn| {
            let updated = conn
                .execute("UPDATE notifications SET is_read = 1 WHERE is_read = 0", [])
                .map_err(|e| format!("Failed to mark all notifications as read: {}", e))?;

            Ok(updated as u32)
        })
    }

    /// Delete a notification
    pub fn delete(&self, id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let deleted = conn
                .execute("DELETE FROM notifications WHERE id = ?1", params![id])
                .map_err(|e| format!("Failed to delete notification: {}", e))?;

            Ok(deleted > 0)
        })
    }

    /// Delete notifications older than N days
    pub fn delete_old(&self, days: u32) -> Result<u32, String> {
        self.db.with_connection(|conn| {
            let deleted = conn
                .execute(
                    r#"
                    DELETE FROM notifications
                    WHERE created_at < datetime('now', ?1)
                    "#,
                    params![format!("-{} days", days)],
                )
                .map_err(|e| format!("Failed to delete old notifications: {}", e))?;

            Ok(deleted as u32)
        })
    }

    /// Prune notifications to keep only the most recent N entries
    pub fn prune(&self, keep_count: usize) -> Result<u32, String> {
        self.db.with_connection(|conn| {
            let deleted = conn
                .execute(
                    r#"
                    DELETE FROM notifications
                    WHERE id NOT IN (
                        SELECT id FROM notifications
                        ORDER BY created_at DESC
                        LIMIT ?1
                    )
                    "#,
                    params![keep_count as i64],
                )
                .map_err(|e| format!("Failed to prune notifications: {}", e))?;

            Ok(deleted as u32)
        })
    }

    /// Clear all notifications
    pub fn clear_all(&self) -> Result<u32, String> {
        self.db.with_connection(|conn| {
            let deleted = conn
                .execute("DELETE FROM notifications", [])
                .map_err(|e| format!("Failed to clear notifications: {}", e))?;

            Ok(deleted as u32)
        })
    }
}

/// Internal row structure for mapping database rows
struct NotificationRow {
    id: String,
    notification_type: String,
    category: String,
    title: String,
    body: String,
    is_read: i32,
    metadata: Option<String>,
    created_at: String,
}

impl NotificationRow {
    fn into_record(self) -> Result<NotificationRecord, String> {
        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map_err(|e| format!("Failed to parse created_at: {}", e))?
            .with_timezone(&Utc);

        let metadata = self
            .metadata
            .as_ref()
            .map(|json| serde_json::from_str(json).ok())
            .flatten();

        Ok(NotificationRecord {
            id: self.id,
            notification_type: self.notification_type,
            category: self.category,
            title: self.title,
            body: self.body,
            is_read: self.is_read != 0,
            metadata,
            created_at,
        })
    }
}
