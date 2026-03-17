// AI Conversation Repository
// Handles all database operations for AI conversations and messages
// Feature: AI Assistant Tab (022-ai-assistant-tab)

use chrono::{DateTime, Utc};
use rusqlite::params;

use crate::models::ai_assistant::{
    Conversation, ConversationSummary, Message, MessageRole, MessageStatus,
    ToolCall, ToolResult, ConversationListResponse,
};
use crate::utils::database::Database;

/// Repository for AI conversation data access
pub struct AIConversationRepository {
    db: Database,
}

impl AIConversationRepository {
    /// Create a new AIConversationRepository
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    // =========================================================================
    // Conversations
    // =========================================================================

    /// Create a new conversation
    pub fn create_conversation(&self, conversation: &Conversation) -> Result<(), String> {
        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO ai_conversations (id, title, project_path, provider_id, message_count, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                params![
                    conversation.id,
                    conversation.title,
                    conversation.project_path,
                    conversation.provider_id,
                    conversation.message_count,
                    conversation.created_at.to_rfc3339(),
                    conversation.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|e| format!("Failed to create conversation: {}", e))?;
            Ok(())
        })
    }

    /// Get a conversation by ID
    pub fn get_conversation(&self, id: &str) -> Result<Option<Conversation>, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT id, title, project_path, provider_id, message_count, created_at, updated_at
                FROM ai_conversations
                WHERE id = ?1
                "#,
                params![id],
                |row| {
                    Ok(ConversationRow {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        project_path: row.get(2)?,
                        provider_id: row.get(3)?,
                        message_count: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                },
            );

            match result {
                Ok(row) => Ok(Some(row.into_conversation()?)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get conversation: {}", e)),
            }
        })
    }

    /// List conversations with pagination
    pub fn list_conversations(
        &self,
        project_path: Option<&str>,
        limit: i64,
        offset: i64,
        order_by: &str,
    ) -> Result<ConversationListResponse, String> {
        self.db.with_connection(|conn| {
            // Build query based on filters
            let order_column = match order_by {
                "created" => "created_at",
                _ => "updated_at",
            };

            let (query, count_query, params_vec): (String, String, Vec<String>) = if let Some(path) = project_path {
                (
                    format!(
                        r#"
                        SELECT c.id, c.title, c.project_path, c.message_count, c.created_at, c.updated_at,
                               (SELECT content FROM ai_messages WHERE conversation_id = c.id ORDER BY created_at DESC LIMIT 1) as last_message
                        FROM ai_conversations c
                        WHERE c.project_path = ?1
                        ORDER BY c.{} DESC
                        LIMIT ?2 OFFSET ?3
                        "#,
                        order_column
                    ),
                    "SELECT COUNT(*) FROM ai_conversations WHERE project_path = ?1".to_string(),
                    vec![path.to_string()],
                )
            } else {
                (
                    format!(
                        r#"
                        SELECT c.id, c.title, c.project_path, c.message_count, c.created_at, c.updated_at,
                               (SELECT content FROM ai_messages WHERE conversation_id = c.id ORDER BY created_at DESC LIMIT 1) as last_message
                        FROM ai_conversations c
                        ORDER BY c.{} DESC
                        LIMIT ?1 OFFSET ?2
                        "#,
                        order_column
                    ),
                    "SELECT COUNT(*) FROM ai_conversations".to_string(),
                    vec![],
                )
            };

            // Get total count
            let total: i64 = if project_path.is_some() {
                conn.query_row(&count_query, params![&params_vec[0]], |row| row.get(0))
                    .map_err(|e| format!("Failed to count conversations: {}", e))?
            } else {
                conn.query_row(&count_query, [], |row| row.get(0))
                    .map_err(|e| format!("Failed to count conversations: {}", e))?
            };

            // Get conversations
            let mut stmt = conn.prepare(&query)
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let mut conversations = Vec::new();

            if project_path.is_some() {
                let rows = stmt.query_map(params![&params_vec[0], limit, offset], |row| {
                    Ok(ConversationSummaryRow {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        project_path: row.get(2)?,
                        message_count: row.get(3)?,
                        created_at: row.get(4)?,
                        updated_at: row.get(5)?,
                        last_message: row.get(6)?,
                    })
                }).map_err(|e| format!("Failed to query conversations: {}", e))?;

                for row in rows {
                    let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                    conversations.push(row.into_summary()?);
                }
            } else {
                let rows = stmt.query_map(params![limit, offset], |row| {
                    Ok(ConversationSummaryRow {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        project_path: row.get(2)?,
                        message_count: row.get(3)?,
                        created_at: row.get(4)?,
                        updated_at: row.get(5)?,
                        last_message: row.get(6)?,
                    })
                }).map_err(|e| format!("Failed to query conversations: {}", e))?;

                for row in rows {
                    let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                    conversations.push(row.into_summary()?);
                }
            }

            let has_more = (offset + limit) < total;

            Ok(ConversationListResponse {
                conversations,
                total,
                has_more,
            })
        })
    }

    /// Update conversation metadata
    pub fn update_conversation(
        &self,
        id: &str,
        title: Option<&str>,
        project_path: Option<Option<&str>>,
    ) -> Result<(), String> {
        self.db.with_connection(|conn| {
            let now = Utc::now().to_rfc3339();

            // Build dynamic update query
            let mut updates = vec!["updated_at = ?"];
            let mut param_count = 1;

            if title.is_some() {
                param_count += 1;
                updates.push("title = ?");
            }
            if project_path.is_some() {
                param_count += 1;
                updates.push("project_path = ?");
            }

            let query = format!(
                "UPDATE ai_conversations SET {} WHERE id = ?{}",
                updates.join(", "),
                param_count + 1
            );

            // Execute with dynamic params
            match (title, project_path) {
                (Some(t), Some(Some(p))) => {
                    conn.execute(&query, params![now, t, p, id])
                }
                (Some(t), Some(None)) => {
                    conn.execute(&query, params![now, t, Option::<&str>::None, id])
                }
                (Some(t), None) => {
                    conn.execute(&query, params![now, t, id])
                }
                (None, Some(Some(p))) => {
                    conn.execute(&query, params![now, p, id])
                }
                (None, Some(None)) => {
                    conn.execute(&query, params![now, Option::<&str>::None, id])
                }
                (None, None) => {
                    conn.execute(&query, params![now, id])
                }
            }.map_err(|e| format!("Failed to update conversation: {}", e))?;

            Ok(())
        })
    }

    /// Update conversation's provider_id
    pub fn update_conversation_service(
        &self,
        id: &str,
        provider_id: Option<&str>,
    ) -> Result<(), String> {
        self.db.with_connection(|conn| {
            let now = Utc::now().to_rfc3339();

            conn.execute(
                "UPDATE ai_conversations SET provider_id = ?1, updated_at = ?2 WHERE id = ?3",
                params![provider_id, now, id],
            )
            .map_err(|e| format!("Failed to update conversation service: {}", e))?;

            Ok(())
        })
    }

    /// Delete a conversation (messages are deleted via CASCADE)
    pub fn delete_conversation(&self, id: &str) -> Result<bool, String> {
        self.db.with_connection(|conn| {
            let rows_affected = conn
                .execute("DELETE FROM ai_conversations WHERE id = ?1", params![id])
                .map_err(|e| format!("Failed to delete conversation: {}", e))?;
            Ok(rows_affected > 0)
        })
    }

    /// Increment message count for a conversation
    pub fn increment_message_count(&self, conversation_id: &str) -> Result<(), String> {
        self.db.with_connection(|conn| {
            let now = Utc::now().to_rfc3339();
            conn.execute(
                r#"
                UPDATE ai_conversations
                SET message_count = message_count + 1, updated_at = ?1
                WHERE id = ?2
                "#,
                params![now, conversation_id],
            )
            .map_err(|e| format!("Failed to increment message count: {}", e))?;
            Ok(())
        })
    }

    // =========================================================================
    // Messages
    // =========================================================================

    /// Create a new message
    pub fn create_message(&self, message: &Message) -> Result<(), String> {
        self.db.with_connection(|conn| {
            let tool_calls_json = message.tool_calls.as_ref().map(|tc| {
                serde_json::to_string(tc).unwrap_or_default()
            });
            let tool_results_json = message.tool_results.as_ref().map(|tr| {
                serde_json::to_string(tr).unwrap_or_default()
            });

            conn.execute(
                r#"
                INSERT INTO ai_messages (id, conversation_id, role, content, tool_calls, tool_results, status, tokens_used, model, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
                params![
                    message.id,
                    message.conversation_id,
                    message.role.to_string(),
                    message.content,
                    tool_calls_json,
                    tool_results_json,
                    message.status.to_string(),
                    message.tokens_used,
                    message.model,
                    message.created_at.to_rfc3339(),
                ],
            )
            .map_err(|e| format!("Failed to create message: {}", e))?;
            Ok(())
        })
    }

    /// Get a message by ID
    pub fn get_message(&self, id: &str) -> Result<Option<Message>, String> {
        self.db.with_connection(|conn| {
            let result = conn.query_row(
                r#"
                SELECT id, conversation_id, role, content, tool_calls, tool_results, status, tokens_used, model, created_at
                FROM ai_messages
                WHERE id = ?1
                "#,
                params![id],
                |row| {
                    Ok(MessageRow {
                        id: row.get(0)?,
                        conversation_id: row.get(1)?,
                        role: row.get(2)?,
                        content: row.get(3)?,
                        tool_calls: row.get(4)?,
                        tool_results: row.get(5)?,
                        status: row.get(6)?,
                        tokens_used: row.get(7)?,
                        model: row.get(8)?,
                        created_at: row.get(9)?,
                    })
                },
            );

            match result {
                Ok(row) => Ok(Some(row.into_message()?)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("Failed to get message: {}", e)),
            }
        })
    }

    /// Get all messages for a conversation
    pub fn get_messages(&self, conversation_id: &str) -> Result<Vec<Message>, String> {
        self.db.with_connection(|conn| {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, conversation_id, role, content, tool_calls, tool_results, status, tokens_used, model, created_at
                    FROM ai_messages
                    WHERE conversation_id = ?1
                    ORDER BY created_at ASC
                    "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let rows = stmt
                .query_map(params![conversation_id], |row| {
                    Ok(MessageRow {
                        id: row.get(0)?,
                        conversation_id: row.get(1)?,
                        role: row.get(2)?,
                        content: row.get(3)?,
                        tool_calls: row.get(4)?,
                        tool_results: row.get(5)?,
                        status: row.get(6)?,
                        tokens_used: row.get(7)?,
                        model: row.get(8)?,
                        created_at: row.get(9)?,
                    })
                })
                .map_err(|e| format!("Failed to query messages: {}", e))?;

            let mut messages = Vec::new();
            for row in rows {
                let row = row.map_err(|e| format!("Failed to read row: {}", e))?;
                messages.push(row.into_message()?);
            }

            Ok(messages)
        })
    }

    /// Update message content (for streaming)
    pub fn update_message_content(&self, id: &str, content: &str) -> Result<(), String> {
        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE ai_messages SET content = ?1 WHERE id = ?2",
                params![content, id],
            )
            .map_err(|e| format!("Failed to update message content: {}", e))?;
            Ok(())
        })
    }

    /// Update message status
    pub fn update_message_status(&self, id: &str, status: MessageStatus) -> Result<(), String> {
        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE ai_messages SET status = ?1 WHERE id = ?2",
                params![status.to_string(), id],
            )
            .map_err(|e| format!("Failed to update message status: {}", e))?;
            Ok(())
        })
    }

    /// Update message with completion data
    /// If content is empty and there are no tool_calls, the message will be deleted
    /// as empty AI messages serve no purpose
    pub fn update_message_completion(
        &self,
        id: &str,
        content: &str,
        tokens_used: Option<i64>,
        model: Option<&str>,
        tool_calls: Option<&Vec<ToolCall>>,
    ) -> Result<(), String> {
        self.db.with_connection(|conn| {
            let content_trimmed = content.trim();
            let has_tool_calls = tool_calls.map(|tc| !tc.is_empty()).unwrap_or(false);

            // If content is empty and no tool_calls, delete the message instead of saving it
            if content_trimmed.is_empty() && !has_tool_calls {
                log::info!("Deleting empty AI message {} (no content, no tool_calls)", id);

                // Get conversation_id before deleting
                let conversation_id: Option<String> = conn
                    .query_row(
                        "SELECT conversation_id FROM ai_messages WHERE id = ?1",
                        params![id],
                        |row| row.get(0),
                    )
                    .ok();

                // Delete the empty message
                conn.execute("DELETE FROM ai_messages WHERE id = ?1", params![id])
                    .map_err(|e| format!("Failed to delete empty message: {}", e))?;

                // Decrement message count for the conversation
                if let Some(conv_id) = conversation_id {
                    conn.execute(
                        "UPDATE ai_conversations SET message_count = message_count - 1 WHERE id = ?1 AND message_count > 0",
                        params![conv_id],
                    )
                    .map_err(|e| format!("Failed to update message count: {}", e))?;
                }

                return Ok(());
            }

            let tool_calls_json = tool_calls.map(|tc| {
                serde_json::to_string(tc).unwrap_or_default()
            });

            conn.execute(
                r#"
                UPDATE ai_messages
                SET content = ?1, status = 'sent', tokens_used = ?2, model = ?3, tool_calls = ?4
                WHERE id = ?5
                "#,
                params![content, tokens_used, model, tool_calls_json, id],
            )
            .map_err(|e| format!("Failed to update message completion: {}", e))?;
            Ok(())
        })
    }

    /// Update tool call status in a message
    pub fn update_tool_call_status(
        &self,
        message_id: &str,
        tool_call_id: &str,
        status: &str,
    ) -> Result<(), String> {
        self.db.with_connection(|conn| {
            // Get current tool_calls
            let tool_calls_json: Option<String> = conn
                .query_row(
                    "SELECT tool_calls FROM ai_messages WHERE id = ?1",
                    params![message_id],
                    |row| row.get(0),
                )
                .map_err(|e| format!("Failed to get message: {}", e))?;

            if let Some(json) = tool_calls_json {
                let mut tool_calls: Vec<ToolCall> = serde_json::from_str(&json)
                    .map_err(|e| format!("Failed to parse tool_calls: {}", e))?;

                // Update the specific tool call status
                for tc in &mut tool_calls {
                    if tc.id == tool_call_id {
                        tc.status = match status {
                            "approved" => crate::models::ai_assistant::ToolCallStatus::Approved,
                            "denied" => crate::models::ai_assistant::ToolCallStatus::Denied,
                            "completed" => crate::models::ai_assistant::ToolCallStatus::Completed,
                            "failed" => crate::models::ai_assistant::ToolCallStatus::Failed,
                            _ => crate::models::ai_assistant::ToolCallStatus::Pending,
                        };
                        break;
                    }
                }

                // Save back
                let updated_json = serde_json::to_string(&tool_calls)
                    .map_err(|e| format!("Failed to serialize tool_calls: {}", e))?;

                conn.execute(
                    "UPDATE ai_messages SET tool_calls = ?1 WHERE id = ?2",
                    params![updated_json, message_id],
                )
                .map_err(|e| format!("Failed to update tool_calls: {}", e))?;
            }

            Ok(())
        })
    }

    /// Add tool results to a message
    pub fn add_tool_results(
        &self,
        message_id: &str,
        results: &[ToolResult],
    ) -> Result<(), String> {
        self.db.with_connection(|conn| {
            // Get current tool_results
            let current_json: Option<String> = conn
                .query_row(
                    "SELECT tool_results FROM ai_messages WHERE id = ?1",
                    params![message_id],
                    |row| row.get(0),
                )
                .map_err(|e| format!("Failed to get message: {}", e))?;

            let mut all_results: Vec<ToolResult> = if let Some(json) = current_json {
                serde_json::from_str(&json).unwrap_or_default()
            } else {
                Vec::new()
            };

            all_results.extend(results.iter().cloned());

            let updated_json = serde_json::to_string(&all_results)
                .map_err(|e| format!("Failed to serialize tool_results: {}", e))?;

            conn.execute(
                "UPDATE ai_messages SET tool_results = ?1 WHERE id = ?2",
                params![updated_json, message_id],
            )
            .map_err(|e| format!("Failed to update tool_results: {}", e))?;

            Ok(())
        })
    }

    /// Update tool calls and results in a message
    pub fn update_message_tool_data(
        &self,
        message_id: &str,
        tool_calls: Option<&Vec<ToolCall>>,
        tool_results: Option<&Vec<ToolResult>>,
    ) -> Result<(), String> {
        self.db.with_connection(|conn| {
            let tool_calls_json = tool_calls.map(|tc| {
                serde_json::to_string(tc).unwrap_or_default()
            });
            let tool_results_json = tool_results.map(|tr| {
                serde_json::to_string(tr).unwrap_or_default()
            });

            conn.execute(
                r#"
                UPDATE ai_messages
                SET tool_calls = COALESCE(?1, tool_calls), tool_results = COALESCE(?2, tool_results)
                WHERE id = ?3
                "#,
                params![tool_calls_json, tool_results_json, message_id],
            )
            .map_err(|e| format!("Failed to update message tool data: {}", e))?;
            Ok(())
        })
    }

    /// Delete messages after a specific message (for regeneration)
    pub fn delete_messages_after(&self, conversation_id: &str, message_id: &str) -> Result<i64, String> {
        self.db.with_connection(|conn| {
            // Get the created_at of the target message
            let created_at: String = conn
                .query_row(
                    "SELECT created_at FROM ai_messages WHERE id = ?1",
                    params![message_id],
                    |row| row.get(0),
                )
                .map_err(|e| format!("Failed to get message timestamp: {}", e))?;

            // Delete messages created after this one (including this one)
            let count = conn
                .execute(
                    r#"
                    DELETE FROM ai_messages
                    WHERE conversation_id = ?1 AND created_at >= ?2
                    "#,
                    params![conversation_id, created_at],
                )
                .map_err(|e| format!("Failed to delete messages: {}", e))?;

            // Update message count
            let new_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM ai_messages WHERE conversation_id = ?1",
                    params![conversation_id],
                    |row| row.get(0),
                )
                .map_err(|e| format!("Failed to count messages: {}", e))?;

            conn.execute(
                "UPDATE ai_conversations SET message_count = ?1, updated_at = ?2 WHERE id = ?3",
                params![new_count, Utc::now().to_rfc3339(), conversation_id],
            )
            .map_err(|e| format!("Failed to update conversation: {}", e))?;

            Ok(count as i64)
        })
    }
}

// ============================================================================
// Helper structs for row mapping
// ============================================================================

struct ConversationRow {
    id: String,
    title: Option<String>,
    project_path: Option<String>,
    provider_id: Option<String>,
    message_count: i64,
    created_at: String,
    updated_at: String,
}

impl ConversationRow {
    fn into_conversation(self) -> Result<Conversation, String> {
        Ok(Conversation {
            id: self.id,
            title: self.title,
            project_path: self.project_path,
            provider_id: self.provider_id,
            message_count: self.message_count,
            created_at: DateTime::parse_from_rfc3339(&self.created_at)
                .map_err(|e| format!("Invalid created_at: {}", e))?
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&self.updated_at)
                .map_err(|e| format!("Invalid updated_at: {}", e))?
                .with_timezone(&Utc),
        })
    }
}

struct ConversationSummaryRow {
    id: String,
    title: Option<String>,
    project_path: Option<String>,
    message_count: i64,
    created_at: String,
    updated_at: String,
    last_message: Option<String>,
}

impl ConversationSummaryRow {
    fn into_summary(self) -> Result<ConversationSummary, String> {
        // Truncate last message for preview (handle multi-byte UTF-8 characters)
        let last_message_preview = self.last_message.map(|msg| {
            let char_count = msg.chars().count();
            if char_count > 100 {
                let truncated: String = msg.chars().take(100).collect();
                format!("{}...", truncated)
            } else {
                msg
            }
        });

        Ok(ConversationSummary {
            id: self.id,
            title: self.title,
            project_path: self.project_path,
            message_count: self.message_count,
            last_message_preview,
            created_at: DateTime::parse_from_rfc3339(&self.created_at)
                .map_err(|e| format!("Invalid created_at: {}", e))?
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&self.updated_at)
                .map_err(|e| format!("Invalid updated_at: {}", e))?
                .with_timezone(&Utc),
        })
    }
}

struct MessageRow {
    id: String,
    conversation_id: String,
    role: String,
    content: String,
    tool_calls: Option<String>,
    tool_results: Option<String>,
    status: String,
    tokens_used: Option<i64>,
    model: Option<String>,
    created_at: String,
}

impl MessageRow {
    fn into_message(self) -> Result<Message, String> {
        let role: MessageRole = self.role.parse()
            .map_err(|e: String| format!("Invalid role: {}", e))?;
        let status: MessageStatus = self.status.parse()
            .map_err(|e: String| format!("Invalid status: {}", e))?;

        let tool_calls: Option<Vec<ToolCall>> = self.tool_calls
            .map(|json| serde_json::from_str(&json))
            .transpose()
            .map_err(|e| format!("Invalid tool_calls JSON: {}", e))?;

        let tool_results: Option<Vec<ToolResult>> = self.tool_results
            .map(|json| serde_json::from_str(&json))
            .transpose()
            .map_err(|e| format!("Invalid tool_results JSON: {}", e))?;

        Ok(Message {
            id: self.id,
            conversation_id: self.conversation_id,
            role,
            content: self.content,
            tool_calls,
            tool_results,
            status,
            tokens_used: self.tokens_used,
            model: self.model,
            created_at: DateTime::parse_from_rfc3339(&self.created_at)
                .map_err(|e| format!("Invalid created_at: {}", e))?
                .with_timezone(&Utc),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::database::Database;

    fn setup_test_db() -> Database {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        // Keep the dir alive by leaking it for the duration of the test
        std::mem::forget(dir);
        Database::new(db_path).expect("Failed to create test database")
    }

    #[test]
    fn test_create_and_get_conversation() {
        let db = setup_test_db();
        let repo = AIConversationRepository::new(db);

        // Conversation::new takes (project_path, provider_id)
        let mut conversation = Conversation::new(
            Some("/test/path".to_string()),
            None,
        );
        conversation.title = Some("Test Conversation".to_string());

        repo.create_conversation(&conversation).expect("Failed to create conversation");

        let fetched = repo.get_conversation(&conversation.id)
            .expect("Failed to get conversation")
            .expect("Conversation not found");

        assert_eq!(fetched.id, conversation.id);
        assert_eq!(fetched.title, Some("Test Conversation".to_string()));
        assert_eq!(fetched.project_path, Some("/test/path".to_string()));
    }

    #[test]
    fn test_create_and_get_message() {
        let db = setup_test_db();
        let repo = AIConversationRepository::new(db);

        let conversation = Conversation::new(None, None);
        repo.create_conversation(&conversation).expect("Failed to create conversation");

        let message = Message::user(conversation.id.clone(), "Hello, AI!".to_string());
        repo.create_message(&message).expect("Failed to create message");
        repo.increment_message_count(&conversation.id).expect("Failed to increment count");

        let fetched = repo.get_message(&message.id)
            .expect("Failed to get message")
            .expect("Message not found");

        assert_eq!(fetched.id, message.id);
        assert_eq!(fetched.content, "Hello, AI!");
        assert_eq!(fetched.role, MessageRole::User);
    }

    #[test]
    fn test_list_conversations() {
        let db = setup_test_db();
        let repo = AIConversationRepository::new(db);

        // Create multiple conversations
        for _i in 0..5 {
            let conversation = Conversation::new(None, None);
            repo.create_conversation(&conversation).expect("Failed to create conversation");
        }

        let result = repo.list_conversations(None, 10, 0, "updated")
            .expect("Failed to list conversations");

        assert_eq!(result.conversations.len(), 5);
        assert_eq!(result.total, 5);
        assert!(!result.has_more);
    }

    #[test]
    fn test_delete_conversation_cascades_messages() {
        let db = setup_test_db();
        let repo = AIConversationRepository::new(db);

        let conversation = Conversation::new(None, None);
        repo.create_conversation(&conversation).expect("Failed to create conversation");

        let message = Message::user(conversation.id.clone(), "Test message".to_string());
        repo.create_message(&message).expect("Failed to create message");

        // Delete conversation
        let deleted = repo.delete_conversation(&conversation.id)
            .expect("Failed to delete conversation");
        assert!(deleted);

        // Verify message is also deleted (CASCADE)
        let msg = repo.get_message(&message.id).expect("Failed to check message");
        assert!(msg.is_none());
    }
}
