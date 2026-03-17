// Security Audit Service
// Provides structured logging for security-related events
// Feature: Enhanced Project Security Posture - Phase 2

use chrono::{Duration, Utc};
use rusqlite::params;
use std::sync::Arc;

use specforge_lib::utils::database::Database;

// Import from local models (Tauri-specific, not specforge_lib)
// Re-export for external use
pub use crate::local_models::audit::{
    Actor, AuditEvent, AuditFilter, AuditLogRow, AuditStats, Outcome, SecurityEventType,
};

/// Audit service for logging and querying security events
pub struct AuditService {
    db: Arc<Database>,
}

impl AuditService {
    /// Create a new audit service
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Log a security event
    pub fn log(&self, event: &AuditEvent) -> Result<(), String> {
        let actor_type = event.actor.to_string();
        let actor_id = match &event.actor {
            Actor::AIAssistant { session_id } => session_id.clone(),
            _ => None,
        };

        let (outcome_str, outcome_reason) = match &event.outcome {
            Outcome::Success => ("success".to_string(), None),
            Outcome::Failure { reason } => ("failure".to_string(), reason.clone()),
            Outcome::Denied { reason } => ("denied".to_string(), reason.clone()),
        };

        let details_json = event
            .details
            .as_ref()
            .map(|d| serde_json::to_string(d).unwrap_or_default());

        // Clone values for move into closure
        let event_id = event.id.clone();
        let timestamp = event.timestamp.to_rfc3339();
        let event_type = event.event_type.to_string();
        let action = event.action.clone();
        let resource_type = event.resource_type.clone();
        let resource_id = event.resource_id.clone();
        let resource_name = event.resource_name.clone();
        let client_ip = event.client_ip.clone();

        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO security_audit_log (
                    id, timestamp, event_type, actor_type, actor_id,
                    action, resource_type, resource_id, resource_name,
                    outcome, outcome_reason, details, client_ip
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                "#,
                params![
                    event_id,
                    timestamp,
                    event_type,
                    actor_type,
                    actor_id,
                    action,
                    resource_type,
                    resource_id,
                    resource_name,
                    outcome_str,
                    outcome_reason,
                    details_json,
                    client_ip,
                ],
            )
            .map_err(|e| format!("Failed to log audit event: {}", e))?;
            Ok(())
        })
    }

    /// Query audit events with filter
    pub fn query(&self, filter: &AuditFilter) -> Result<Vec<AuditEvent>, String> {
        self.db.with_connection(|conn| {
            let mut sql = String::from(
                r#"
                SELECT id, timestamp, event_type, actor_type, actor_id,
                       action, resource_type, resource_id, resource_name,
                       outcome, outcome_reason, details, client_ip, created_at
                FROM security_audit_log
                WHERE 1=1
                "#,
            );
            let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(from) = &filter.from {
                sql.push_str(" AND timestamp >= ?");
                params_vec.push(Box::new(from.to_rfc3339()));
            }

            if let Some(to) = &filter.to {
                sql.push_str(" AND timestamp < ?");
                params_vec.push(Box::new(to.to_rfc3339()));
            }

            if let Some(event_types) = &filter.event_types {
                if !event_types.is_empty() {
                    let placeholders: Vec<String> =
                        event_types.iter().map(|_| "?".to_string()).collect();
                    sql.push_str(&format!(
                        " AND event_type IN ({})",
                        placeholders.join(", ")
                    ));
                    for et in event_types {
                        params_vec.push(Box::new(et.to_string()));
                    }
                }
            }

            if let Some(actor_type) = &filter.actor_type {
                sql.push_str(" AND actor_type = ?");
                params_vec.push(Box::new(actor_type.clone()));
            }

            if let Some(outcome) = &filter.outcome {
                sql.push_str(" AND outcome = ?");
                params_vec.push(Box::new(outcome.clone()));
            }

            if let Some(resource_type) = &filter.resource_type {
                sql.push_str(" AND resource_type = ?");
                params_vec.push(Box::new(resource_type.clone()));
            }

            if let Some(resource_id) = &filter.resource_id {
                sql.push_str(" AND resource_id = ?");
                params_vec.push(Box::new(resource_id.clone()));
            }

            sql.push_str(" ORDER BY timestamp DESC LIMIT ? OFFSET ?");
            params_vec.push(Box::new(filter.limit as i64));
            params_vec.push(Box::new(filter.offset as i64));

            let params_refs: Vec<&dyn rusqlite::ToSql> =
                params_vec.iter().map(|p| p.as_ref()).collect();

            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| format!("Failed to prepare query: {}", e))?;

            let rows = stmt
                .query_map(params_refs.as_slice(), |row| {
                    Ok(AuditLogRow {
                        id: row.get(0)?,
                        timestamp: row.get(1)?,
                        event_type: row.get(2)?,
                        actor_type: row.get(3)?,
                        actor_id: row.get(4)?,
                        action: row.get(5)?,
                        resource_type: row.get(6)?,
                        resource_id: row.get(7)?,
                        resource_name: row.get(8)?,
                        outcome: row.get(9)?,
                        outcome_reason: row.get(10)?,
                        details: row.get(11)?,
                        client_ip: row.get(12)?,
                        created_at: row.get(13)?,
                    })
                })
                .map_err(|e| format!("Failed to query: {}", e))?;

            let mut events = Vec::new();
            for row in rows {
                let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                let event = AuditEvent::try_from(row)?;
                events.push(event);
            }

            Ok(events)
        })
    }

    /// Get audit statistics for a time period
    pub fn get_stats(&self, days: i64) -> Result<AuditStats, String> {
        let since = Utc::now() - Duration::days(days);
        let since_str = since.to_rfc3339();

        self.db.with_connection(|conn| {
            // Total events
            let total_events: u64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM security_audit_log WHERE timestamp >= ?",
                    params![&since_str],
                    |row| row.get(0),
                )
                .map_err(|e| format!("Failed to count events: {}", e))?;

            // By type
            let mut by_type = std::collections::HashMap::new();
            {
                let mut stmt = conn
                    .prepare(
                        "SELECT event_type, COUNT(*) FROM security_audit_log
                         WHERE timestamp >= ? GROUP BY event_type",
                    )
                    .map_err(|e| format!("Failed to prepare: {}", e))?;

                let rows = stmt
                    .query_map(params![&since_str], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
                    })
                    .map_err(|e| format!("Failed to query: {}", e))?;

                for row in rows {
                    let (k, v) = row.map_err(|e| format!("Failed to read: {}", e))?;
                    by_type.insert(k, v);
                }
            }

            // By outcome
            let mut by_outcome = std::collections::HashMap::new();
            {
                let mut stmt = conn
                    .prepare(
                        "SELECT outcome, COUNT(*) FROM security_audit_log
                         WHERE timestamp >= ? GROUP BY outcome",
                    )
                    .map_err(|e| format!("Failed to prepare: {}", e))?;

                let rows = stmt
                    .query_map(params![&since_str], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
                    })
                    .map_err(|e| format!("Failed to query: {}", e))?;

                for row in rows {
                    let (k, v) = row.map_err(|e| format!("Failed to read: {}", e))?;
                    by_outcome.insert(k, v);
                }
            }

            // By actor
            let mut by_actor = std::collections::HashMap::new();
            {
                let mut stmt = conn
                    .prepare(
                        "SELECT actor_type, COUNT(*) FROM security_audit_log
                         WHERE timestamp >= ? GROUP BY actor_type",
                    )
                    .map_err(|e| format!("Failed to prepare: {}", e))?;

                let rows = stmt
                    .query_map(params![&since_str], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
                    })
                    .map_err(|e| format!("Failed to query: {}", e))?;

                for row in rows {
                    let (k, v) = row.map_err(|e| format!("Failed to read: {}", e))?;
                    by_actor.insert(k, v);
                }
            }

            Ok(AuditStats {
                total_events,
                by_type,
                by_outcome,
                by_actor,
            })
        })
    }

    /// Cleanup old audit logs (retention policy)
    pub fn cleanup(&self, retention_days: i64) -> Result<u64, String> {
        let cutoff = Utc::now() - Duration::days(retention_days);
        let cutoff_str = cutoff.to_rfc3339();

        self.db.with_connection(|conn| {
            let deleted = conn
                .execute(
                    "DELETE FROM security_audit_log WHERE timestamp < ?",
                    params![&cutoff_str],
                )
                .map_err(|e| format!("Failed to cleanup: {}", e))?;

            Ok(deleted as u64)
        })
    }
}

// =============================================================================
// Helper functions for common audit events
// =============================================================================

/// Log a webhook trigger event
pub fn log_webhook_trigger(
    service: &AuditService,
    workflow_id: &str,
    workflow_name: &str,
    client_ip: &str,
    success: bool,
    error: Option<&str>,
) {
    let outcome = if success {
        Outcome::Success
    } else {
        Outcome::Failure {
            reason: error.map(|s| s.to_string()),
        }
    };

    let event = AuditEvent::new(
        SecurityEventType::WebhookTrigger,
        Actor::Webhook {
            source_ip: Some(client_ip.to_string()),
        },
        "webhook_trigger",
        outcome,
    )
    .with_resource("workflow", workflow_id, Some(workflow_name.to_string()))
    .with_client_ip(client_ip);

    if let Err(e) = service.log(&event) {
        log::error!("[audit] Failed to log webhook trigger: {}", e);
    }
}

/// Log a rate limit event
pub fn log_rate_limit(service: &AuditService, client_ip: &str, resource: &str) {
    let event = AuditEvent::new(
        SecurityEventType::SecurityAlert,
        Actor::Webhook {
            source_ip: Some(client_ip.to_string()),
        },
        "rate_limit_exceeded",
        Outcome::Denied {
            reason: Some("Rate limit exceeded".to_string()),
        },
    )
    .with_client_ip(client_ip)
    .with_details(serde_json::json!({
        "resource": resource,
        "alert_type": "rate_limit"
    }));

    if let Err(e) = service.log(&event) {
        log::error!("[audit] Failed to log rate limit: {}", e);
    }
}

/// Log a tool execution event
pub fn log_tool_execution(
    service: &AuditService,
    session_id: &str,
    tool_name: &str,
    success: bool,
    error: Option<&str>,
) {
    let outcome = if success {
        Outcome::Success
    } else {
        Outcome::Failure {
            reason: error.map(|s| s.to_string()),
        }
    };

    let event = AuditEvent::new(
        SecurityEventType::ToolExecution,
        Actor::AIAssistant {
            session_id: Some(session_id.to_string()),
        },
        format!("tool_execute:{}", tool_name),
        outcome,
    )
    .with_details(serde_json::json!({
        "tool_name": tool_name
    }));

    if let Err(e) = service.log(&event) {
        log::error!("[audit] Failed to log tool execution: {}", e);
    }
}

/// Log an authentication event
pub fn log_auth_event(
    service: &AuditService,
    method: &str,
    success: bool,
    client_ip: Option<&str>,
    reason: Option<&str>,
) {
    let outcome = if success {
        Outcome::Success
    } else {
        Outcome::Denied {
            reason: reason.map(|s| s.to_string()),
        }
    };

    let mut event = AuditEvent::new(
        SecurityEventType::Authentication,
        Actor::Webhook {
            source_ip: client_ip.map(|s| s.to_string()),
        },
        format!("auth:{}", method),
        outcome,
    )
    .with_details(serde_json::json!({
        "method": method
    }));

    if let Some(ip) = client_ip {
        event = event.with_client_ip(ip);
    }

    if let Err(e) = service.log(&event) {
        log::error!("[audit] Failed to log auth event: {}", e);
    }
}

// =============================================================================
// Standalone audit logging functions (no service required)
// These functions create a temporary AuditService for one-off logging
// =============================================================================

/// Log a webhook trigger event (standalone)
pub fn audit_webhook_trigger(
    db: Arc<Database>,
    workflow_id: &str,
    workflow_name: &str,
    client_ip: &str,
    success: bool,
    error: Option<&str>,
) {
    let service = AuditService::new(db);
    log_webhook_trigger(&service, workflow_id, workflow_name, client_ip, success, error);
}

/// Log a rate limit event (standalone)
pub fn audit_rate_limit(db: Arc<Database>, client_ip: &str, resource: &str) {
    let service = AuditService::new(db);
    log_rate_limit(&service, client_ip, resource);
}

/// Log an authentication event (standalone)
pub fn audit_auth_event(
    db: Arc<Database>,
    method: &str,
    success: bool,
    client_ip: Option<&str>,
    reason: Option<&str>,
) {
    let service = AuditService::new(db);
    log_auth_event(&service, method, success, client_ip, reason);
}

/// Log a tool execution event (standalone)
pub fn audit_tool_execution(
    db: Arc<Database>,
    session_id: &str,
    tool_name: &str,
    success: bool,
    error: Option<&str>,
) {
    let service = AuditService::new(db);
    log_tool_execution(&service, session_id, tool_name, success, error);
}
