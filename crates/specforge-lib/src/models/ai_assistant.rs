// AI Assistant data models
// Feature: AI Assistant Tab (022-ai-assistant-tab)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Core Entities
// ============================================================================

/// Conversation entity - represents a chat session between user and AI assistant
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    /// Unique identifier (UUID v4)
    pub id: String,
    /// Display title (auto-generated or user-defined)
    pub title: Option<String>,
    /// Associated project path for context
    pub project_path: Option<String>,
    /// AI provider ID used for this conversation
    pub provider_id: Option<String>,
    /// Cached message count for performance
    pub message_count: i64,
    /// When conversation started
    pub created_at: DateTime<Utc>,
    /// When conversation last modified
    pub updated_at: DateTime<Utc>,
}

impl Conversation {
    /// Create a new conversation with only project and provider context
    pub fn new(project_path: Option<String>, provider_id: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            title: None,
            project_path,
            provider_id,
            message_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new conversation with title
    pub fn with_title(title: String, project_path: Option<String>, provider_id: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            title: Some(title),
            project_path,
            provider_id,
            message_count: 0,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Conversation summary for list display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSummary {
    pub id: String,
    pub title: Option<String>,
    pub project_path: Option<String>,
    pub message_count: i64,
    pub last_message_preview: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Message entity - individual message within a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// Unique identifier (UUID v4)
    pub id: String,
    /// Parent conversation ID
    pub conversation_id: String,
    /// Message author role
    pub role: MessageRole,
    /// Message text content
    pub content: String,
    /// Tool calls requested by AI (JSON array)
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Tool execution results (JSON array)
    pub tool_results: Option<Vec<ToolResult>>,
    /// Message delivery status
    pub status: MessageStatus,
    /// Token count (for AI messages)
    pub tokens_used: Option<i64>,
    /// Model used (for AI messages)
    pub model: Option<String>,
    /// When message was created
    pub created_at: DateTime<Utc>,
}

impl Message {
    /// Create a new user message
    pub fn user(conversation_id: String, content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            conversation_id,
            role: MessageRole::User,
            content,
            tool_calls: None,
            tool_results: None,
            status: MessageStatus::Sent,
            tokens_used: None,
            model: None,
            created_at: Utc::now(),
        }
    }

    /// Create a new assistant message (initially pending)
    pub fn assistant(conversation_id: String, initial_content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            conversation_id,
            role: MessageRole::Assistant,
            content: initial_content,
            tool_calls: None,
            tool_results: None,
            status: MessageStatus::Pending,
            tokens_used: None,
            model: None,
            created_at: Utc::now(),
        }
    }

    /// Create a system message
    pub fn system(conversation_id: String, content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            conversation_id,
            role: MessageRole::System,
            content,
            tool_calls: None,
            tool_results: None,
            status: MessageStatus::Sent,
            tokens_used: None,
            model: None,
            created_at: Utc::now(),
        }
    }

    /// Create a tool result message
    pub fn tool_result(conversation_id: String, content: String, results: Vec<ToolResult>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            conversation_id,
            role: MessageRole::Tool,
            content,
            tool_calls: None,
            tool_results: Some(results),
            status: MessageStatus::Sent,
            tokens_used: None,
            model: None,
            created_at: Utc::now(),
        }
    }
}

// ============================================================================
// Enums
// ============================================================================

/// Message author role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
            MessageRole::System => write!(f, "system"),
            MessageRole::Tool => write!(f, "tool"),
        }
    }
}

impl std::str::FromStr for MessageRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" => Ok(MessageRole::User),
            "assistant" => Ok(MessageRole::Assistant),
            "system" => Ok(MessageRole::System),
            "tool" => Ok(MessageRole::Tool),
            _ => Err(format!("Invalid message role: {}", s)),
        }
    }
}

/// Message delivery status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageStatus {
    Pending,
    Sent,
    Error,
}

impl std::fmt::Display for MessageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageStatus::Pending => write!(f, "pending"),
            MessageStatus::Sent => write!(f, "sent"),
            MessageStatus::Error => write!(f, "error"),
        }
    }
}

impl std::str::FromStr for MessageStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(MessageStatus::Pending),
            "sent" => Ok(MessageStatus::Sent),
            "error" => Ok(MessageStatus::Error),
            _ => Err(format!("Invalid message status: {}", s)),
        }
    }
}

/// Tool call status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    Pending,
    Approved,
    Denied,
    Completed,
    Failed,
}

impl std::fmt::Display for ToolCallStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolCallStatus::Pending => write!(f, "pending"),
            ToolCallStatus::Approved => write!(f, "approved"),
            ToolCallStatus::Denied => write!(f, "denied"),
            ToolCallStatus::Completed => write!(f, "completed"),
            ToolCallStatus::Failed => write!(f, "failed"),
        }
    }
}

// ============================================================================
// Tool Calling
// ============================================================================

/// Tool call requested by AI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    /// Unique call identifier
    pub id: String,
    /// Tool/function name
    pub name: String,
    /// Tool parameters
    pub arguments: serde_json::Value,
    /// Call status
    pub status: ToolCallStatus,
    /// Thought signature for Gemini 2.5+ models (preserves reasoning context)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,
}

impl ToolCall {
    /// Create a new pending tool call
    pub fn new(name: String, arguments: serde_json::Value) -> Self {
        Self {
            id: format!("tc_{}", Uuid::new_v4().to_string().replace("-", "")[..12].to_string()),
            name,
            arguments,
            status: ToolCallStatus::Pending,
            thought_signature: None,
        }
    }

    /// Create a new pending tool call with thought signature (for Gemini 2.5+)
    pub fn new_with_signature(name: String, arguments: serde_json::Value, thought_signature: Option<String>) -> Self {
        Self {
            id: format!("tc_{}", Uuid::new_v4().to_string().replace("-", "")[..12].to_string()),
            name,
            arguments,
            status: ToolCallStatus::Pending,
            thought_signature,
        }
    }
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResult {
    /// Matches ToolCall.id
    pub call_id: String,
    /// Execution success
    pub success: bool,
    /// Formatted output
    pub output: String,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Execution duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl ToolResult {
    /// Create a success result
    pub fn success(call_id: String, output: String, duration_ms: Option<i64>) -> Self {
        Self {
            call_id,
            success: true,
            output,
            error: None,
            duration_ms,
            metadata: None,
        }
    }

    /// Create a failure result
    pub fn failure(call_id: String, error: String) -> Self {
        Self {
            call_id,
            success: false,
            output: error.clone(),
            error: Some(error),
            duration_ms: None,
            metadata: None,
        }
    }
}

// ============================================================================
// Quick Actions / Suggestions
// ============================================================================

/// Quick action execution mode
/// - Instant: Execute tool directly, display result card (no AI, zero tokens)
/// - Smart: Execute tool, then AI summarizes/analyzes (moderate tokens)
/// - Ai: Full AI conversation flow (AI decides tool usage)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum QuickActionMode {
    /// Direct execution + formatted card display (zero token)
    Instant,
    /// Execute tool + AI summary/analysis (moderate token)
    Smart,
    /// Full AI conversation flow (AI decides)
    #[default]
    Ai,
}

/// Tool specification for quick action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickActionTool {
    /// MCP tool name to execute
    pub name: String,
    /// Tool arguments (JSON object)
    #[serde(default)]
    pub args: serde_json::Value,
}

/// Suggested quick action (ephemeral - not persisted)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedAction {
    /// Unique action identifier
    pub id: String,
    /// Display label
    pub label: String,
    /// Text to send when clicked (used for AI mode, or as fallback)
    pub prompt: String,
    /// Lucide icon name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Visual variant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    /// Action category for grouping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Execution mode: instant, smart, or ai
    #[serde(default)]
    pub mode: QuickActionMode,
    /// Tool to execute (for instant/smart modes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<QuickActionTool>,
    /// Hint for AI summarization (smart mode only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_hint: Option<String>,
    /// Whether this action requires a project context to be available (Feature 024)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_project: Option<bool>,
}

// ============================================================================
// Project Context
// ============================================================================

/// Safe project context for AI prompts (sensitive data filtered)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectContext {
    pub project_name: String,
    pub project_path: String,
    pub project_type: String,
    pub package_manager: String,
    pub available_scripts: Vec<String>,
}

// ============================================================================
// Streaming Events
// ============================================================================

/// AI Assistant streaming event (sent via Tauri events)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AIAssistantEvent {
    /// Streaming token received
    #[serde(rename = "token")]
    Token {
        #[serde(rename = "streamSessionId")]
        stream_session_id: String,
        #[serde(rename = "conversationId")]
        conversation_id: String,
        #[serde(rename = "messageId")]
        message_id: String,
        token: String,
        #[serde(rename = "isFinal")]
        is_final: bool,
    },
    /// Tool call requested
    #[serde(rename = "tool_call")]
    ToolCall {
        #[serde(rename = "streamSessionId")]
        stream_session_id: String,
        #[serde(rename = "conversationId")]
        conversation_id: String,
        #[serde(rename = "messageId")]
        message_id: String,
        #[serde(rename = "toolCall")]
        tool_call: ToolCall,
    },
    /// Response complete
    #[serde(rename = "complete")]
    Complete {
        #[serde(rename = "streamSessionId")]
        stream_session_id: String,
        #[serde(rename = "conversationId")]
        conversation_id: String,
        #[serde(rename = "messageId")]
        message_id: String,
        #[serde(rename = "fullContent")]
        full_content: String,
        #[serde(rename = "tokensUsed")]
        tokens_used: i64,
        model: String,
        #[serde(rename = "finishReason")]
        finish_reason: String,
    },
    /// Error occurred
    #[serde(rename = "error")]
    Error {
        #[serde(rename = "streamSessionId")]
        stream_session_id: String,
        #[serde(rename = "conversationId")]
        conversation_id: String,
        #[serde(rename = "messageId")]
        message_id: String,
        code: String,
        message: String,
        retryable: bool,
    },
}

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request to create a new conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateConversationRequest {
    pub title: Option<String>,
    pub project_path: Option<String>,
    pub provider_id: Option<String>,
}

/// Request to list conversations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListConversationsRequest {
    pub project_path: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub order_by: Option<String>,
}

/// Response for list conversations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationListResponse {
    pub conversations: Vec<ConversationSummary>,
    pub total: i64,
    pub has_more: bool,
}

/// Response for get conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationDetail {
    pub conversation: Conversation,
    pub messages: Vec<Message>,
}

/// Request to send a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    /// Existing conversation ID (None to create new)
    pub conversation_id: Option<String>,
    /// Message content
    pub content: String,
    /// Project path for context (used when creating new conversation)
    pub project_path: Option<String>,
    /// AI provider ID to use (used when creating new conversation)
    pub provider_id: Option<String>,
    /// Project context for AI prompts
    pub project_context: Option<ProjectContext>,
}

/// Response for send message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageResponse {
    /// Stream session ID for event listening
    pub stream_session_id: String,
    /// Conversation ID (new or existing)
    pub conversation_id: String,
    /// Assistant message ID that will receive streamed content
    pub message_id: String,
}

/// Request to approve a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApproveToolCallRequest {
    pub conversation_id: String,
    pub message_id: String,
    pub tool_call_id: String,
}

/// Request to deny a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DenyToolCallRequest {
    pub conversation_id: String,
    pub message_id: String,
    pub tool_call_id: String,
    pub reason: Option<String>,
}

/// Response for suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionsResponse {
    pub suggestions: Vec<SuggestedAction>,
}

// ============================================================================
// Tool Definitions
// ============================================================================

/// Tool definition for AI providers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub requires_confirmation: bool,
    pub category: String,
}

/// Available tools response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableTools {
    pub tools: Vec<ToolDefinition>,
}

// ============================================================================
// Feature 023: Enhanced AI Chat Experience
// ============================================================================

// ----------------------------------------------------------------------------
// Response Status (T007)
// ----------------------------------------------------------------------------

/// Response processing phase
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ResponsePhase {
    Idle,
    Thinking,
    Generating,
    Tool,
    Complete,
    Error,
}

impl std::fmt::Display for ResponsePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResponsePhase::Idle => write!(f, "idle"),
            ResponsePhase::Thinking => write!(f, "thinking"),
            ResponsePhase::Generating => write!(f, "generating"),
            ResponsePhase::Tool => write!(f, "tool"),
            ResponsePhase::Complete => write!(f, "complete"),
            ResponsePhase::Error => write!(f, "error"),
        }
    }
}

/// Timing breakdown for response
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResponseTiming {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generating_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_ms: Option<u64>,
}

/// Response status tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseStatus {
    pub phase: ResponsePhase,
    pub start_time: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing: Option<ResponseTiming>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Current iteration in agentic loop (1, 2, 3...)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iteration: Option<u32>,
}

impl ResponseStatus {
    /// Create a new idle status
    pub fn idle() -> Self {
        Self {
            phase: ResponsePhase::Idle,
            start_time: Self::now_ms(),
            tool_name: None,
            timing: None,
            model: None,
            iteration: None,
        }
    }

    /// Create a new thinking status
    pub fn thinking() -> Self {
        Self {
            phase: ResponsePhase::Thinking,
            start_time: Self::now_ms(),
            tool_name: None,
            timing: None,
            model: None,
            iteration: None,
        }
    }

    /// Create a new thinking status with iteration
    pub fn thinking_with_iter(iteration: u32) -> Self {
        Self {
            phase: ResponsePhase::Thinking,
            start_time: Self::now_ms(),
            tool_name: None,
            timing: None,
            model: None,
            iteration: Some(iteration),
        }
    }

    /// Create a new generating status
    pub fn generating(model: Option<String>) -> Self {
        Self {
            phase: ResponsePhase::Generating,
            start_time: Self::now_ms(),
            tool_name: None,
            timing: None,
            model,
            iteration: None,
        }
    }

    /// Create a new generating status with iteration
    pub fn generating_with_iter(model: Option<String>, iteration: u32) -> Self {
        Self {
            phase: ResponsePhase::Generating,
            start_time: Self::now_ms(),
            tool_name: None,
            timing: None,
            model,
            iteration: Some(iteration),
        }
    }

    /// Create a new tool status
    pub fn tool(tool_name: String) -> Self {
        Self {
            phase: ResponsePhase::Tool,
            start_time: Self::now_ms(),
            tool_name: Some(tool_name),
            timing: None,
            model: None,
            iteration: None,
        }
    }

    /// Create a new tool status with iteration
    pub fn tool_with_iter(tool_name: String, iteration: u32) -> Self {
        Self {
            phase: ResponsePhase::Tool,
            start_time: Self::now_ms(),
            tool_name: Some(tool_name),
            timing: None,
            model: None,
            iteration: Some(iteration),
        }
    }

    /// Create a new complete status with timing and optional model
    pub fn complete_with_model(timing: ResponseTiming, model: Option<String>) -> Self {
        Self {
            phase: ResponsePhase::Complete,
            start_time: Self::now_ms(),
            tool_name: None,
            timing: Some(timing),
            model,
            iteration: None,
        }
    }

    /// Create a new complete status with timing
    pub fn complete(timing: ResponseTiming) -> Self {
        Self::complete_with_model(timing, None)
    }

    /// Create a new error status
    pub fn error() -> Self {
        Self {
            phase: ResponsePhase::Error,
            start_time: Self::now_ms(),
            tool_name: None,
            timing: None,
            model: None,
            iteration: None,
        }
    }

    /// Get current time in milliseconds
    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

// ----------------------------------------------------------------------------
// Interactive Elements (T008)
// ----------------------------------------------------------------------------

/// Interactive element type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InteractiveElementType {
    Navigation,
    Action,
    Entity,
}

/// Interactive UI element embedded in AI response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractiveElement {
    pub id: String,
    #[serde(rename = "type")]
    pub element_type: InteractiveElementType,
    pub label: String,
    pub payload: String,
    pub requires_confirm: bool,
    pub start_index: usize,
    pub end_index: usize,
}

// ----------------------------------------------------------------------------
// Lazy Actions (T009)
// ----------------------------------------------------------------------------

/// Lazy action type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LazyActionType {
    Navigate,
    ExecuteTool,
    Copy,
}

/// Lazy action that executes directly
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LazyAction {
    #[serde(rename = "type")]
    pub action_type: LazyActionType,
    pub payload: String,
}

// ----------------------------------------------------------------------------
// Context Management (T010)
// ----------------------------------------------------------------------------

/// Entity mentioned in conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextEntity {
    #[serde(rename = "type")]
    pub entity_type: String,
    pub id: String,
    pub name: String,
    pub last_mentioned: usize,
}

/// Recent tool call for reference tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentToolCall {
    pub id: String,
    pub name: String,
    pub description: String,
    pub success: bool,
    pub message_index: usize,
}

/// Summarized context for long conversations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationContext {
    pub summary: String,
    pub key_entities: Vec<ContextEntity>,
    pub recent_tool_calls: Vec<RecentToolCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_context: Option<ProjectContext>,
    pub token_count: u32,
}

// ----------------------------------------------------------------------------
// Status Update Event (T011)
// ----------------------------------------------------------------------------

/// Payload for ai:status-update event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusUpdatePayload {
    pub stream_session_id: String,
    pub conversation_id: String,
    pub status: ResponseStatus,
}

// ============================================================================
// Feature 025: Session Context for AI Precision Improvement
// ============================================================================

// ----------------------------------------------------------------------------
// Session Context (Bound at conversation start)
// ----------------------------------------------------------------------------

/// Summary of a workflow for session context
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowSummary {
    pub id: String,
    pub name: String,
    /// Number of steps in the workflow
    pub step_count: usize,
}

/// Summary of a worktree for session context
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeSummary {
    pub path: String,
    pub branch: String,
}

/// Session-level context binding for AI Assistant
/// Ensures AI operations stay within the intended project scope
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionContext {
    /// Unique project ID from database
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    /// Project name for display
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    /// Absolute project path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    /// Project type (Node.js, Rust, Python, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_type: Option<String>,
    /// Package manager
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_manager: Option<String>,
    /// Available scripts from package.json or equivalent
    #[serde(default)]
    pub available_scripts: Vec<String>,
    /// Workflows associated with this project (project-specific + global)
    #[serde(default)]
    pub bound_workflows: Vec<WorkflowSummary>,
    /// Active worktree (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_worktree: Option<WorktreeSummary>,
}

impl SessionContext {
    /// Create a new empty session context
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if this context has a bound project
    pub fn has_project(&self) -> bool {
        self.project_id.is_some() || self.project_path.is_some()
    }

    /// Check if a workflow ID is bound to this session
    pub fn is_workflow_bound(&self, workflow_id: &str) -> bool {
        self.bound_workflows.iter().any(|w| w.id == workflow_id)
    }

    /// Get workflow by ID if bound
    pub fn get_workflow(&self, workflow_id: &str) -> Option<&WorkflowSummary> {
        self.bound_workflows.iter().find(|w| w.id == workflow_id)
    }
}

// ----------------------------------------------------------------------------
// Context Delta (Returned by tools that create/modify resources)
// ----------------------------------------------------------------------------

/// Type of context delta
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextDeltaType {
    /// A new workflow was created
    WorkflowCreated,
    /// A step was added to a workflow
    StepAdded,
    /// A step was removed from a workflow
    StepRemoved,
    /// A workflow was deleted
    WorkflowDeleted,
    /// Multiple steps were added (batch operation)
    StepsAdded,
}

/// Resource information in a context delta
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextDeltaResource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_order: Option<i32>,
    /// For batch operations, list of step IDs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_ids: Option<Vec<String>>,
    /// For batch operations, list of step names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_names: Option<Vec<String>>,
}

/// Context delta returned by tools that create/modify resources
/// Used to update SessionCreatedResources during agentic loop
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextDelta {
    /// Type of delta
    pub delta_type: ContextDeltaType,
    /// Affected resource info
    pub resource: ContextDeltaResource,
}

impl ContextDelta {
    /// Create a workflow created delta
    pub fn workflow_created(
        workflow_id: String,
        workflow_name: String,
        project_id: Option<String>,
    ) -> Self {
        Self {
            delta_type: ContextDeltaType::WorkflowCreated,
            resource: ContextDeltaResource {
                workflow_id: Some(workflow_id),
                workflow_name: Some(workflow_name),
                project_id,
                ..Default::default()
            },
        }
    }

    /// Create a step added delta
    pub fn step_added(
        workflow_id: String,
        workflow_name: String,
        project_id: Option<String>,
        step_id: String,
        step_name: String,
        step_order: i32,
    ) -> Self {
        Self {
            delta_type: ContextDeltaType::StepAdded,
            resource: ContextDeltaResource {
                workflow_id: Some(workflow_id),
                workflow_name: Some(workflow_name),
                project_id,
                step_id: Some(step_id),
                step_name: Some(step_name),
                step_order: Some(step_order),
                ..Default::default()
            },
        }
    }

    /// Create a steps added (batch) delta
    pub fn steps_added(
        workflow_id: String,
        workflow_name: String,
        project_id: Option<String>,
        step_ids: Vec<String>,
        step_names: Vec<String>,
    ) -> Self {
        Self {
            delta_type: ContextDeltaType::StepsAdded,
            resource: ContextDeltaResource {
                workflow_id: Some(workflow_id),
                workflow_name: Some(workflow_name),
                project_id,
                step_ids: Some(step_ids),
                step_names: Some(step_names),
                ..Default::default()
            },
        }
    }

    /// Create a step removed delta
    pub fn step_removed(
        workflow_id: String,
        workflow_name: String,
        step_id: String,
    ) -> Self {
        Self {
            delta_type: ContextDeltaType::StepRemoved,
            resource: ContextDeltaResource {
                workflow_id: Some(workflow_id),
                workflow_name: Some(workflow_name),
                step_id: Some(step_id),
                ..Default::default()
            },
        }
    }

    /// Create a workflow deleted delta
    pub fn workflow_deleted(workflow_id: String) -> Self {
        Self {
            delta_type: ContextDeltaType::WorkflowDeleted,
            resource: ContextDeltaResource {
                workflow_id: Some(workflow_id),
                ..Default::default()
            },
        }
    }
}

// ----------------------------------------------------------------------------
// Session Created Resources (Tracked during conversation)
// ----------------------------------------------------------------------------

/// A step created during this conversation session
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedStep {
    pub id: String,
    pub name: String,
    pub workflow_id: String,
    pub order: i32,
}

/// A workflow created during this conversation session
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedWorkflow {
    pub id: String,
    pub name: String,
    pub project_id: Option<String>,
    pub steps: Vec<CreatedStep>,
    /// Message index when this workflow was created
    pub created_at_message_index: usize,
}

/// Modification to an existing workflow during this session
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowModification {
    pub workflow_id: String,
    pub workflow_name: String,
    pub added_steps: Vec<CreatedStep>,
    pub removed_step_ids: Vec<String>,
}

/// Resources created/modified during the conversation
/// This is tracked in-memory and injected into system prompt
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCreatedResources {
    /// Workflows created in this session
    pub workflows: Vec<CreatedWorkflow>,
    /// Modifications to existing workflows
    pub workflow_modifications: Vec<WorkflowModification>,
    /// Last update timestamp (milliseconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<i64>,
}

impl SessionCreatedResources {
    /// Create a new empty resources tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any resources have been created/modified
    pub fn is_empty(&self) -> bool {
        self.workflows.is_empty() && self.workflow_modifications.is_empty()
    }

    /// Add a newly created workflow
    pub fn add_workflow(
        &mut self,
        id: String,
        name: String,
        project_id: Option<String>,
        message_index: usize,
    ) {
        // Check if already exists (idempotency)
        if self.workflows.iter().any(|w| w.id == id) {
            return;
        }
        self.workflows.push(CreatedWorkflow {
            id,
            name,
            project_id,
            steps: Vec::new(),
            created_at_message_index: message_index,
        });
        self.touch();
    }

    /// Add a step to a workflow (newly created or existing)
    pub fn add_step(
        &mut self,
        workflow_id: String,
        workflow_name: String,
        step_id: String,
        step_name: String,
        order: i32,
    ) {
        let step = CreatedStep {
            id: step_id.clone(),
            name: step_name,
            workflow_id: workflow_id.clone(),
            order,
        };

        // First, check if this is a workflow we created in this session
        if let Some(wf) = self.workflows.iter_mut().find(|w| w.id == workflow_id) {
            if !wf.steps.iter().any(|s| s.id == step_id) {
                wf.steps.push(step);
            }
            self.touch();
            return;
        }

        // Otherwise, it's a modification to an existing workflow
        if let Some(mod_entry) = self
            .workflow_modifications
            .iter_mut()
            .find(|m| m.workflow_id == workflow_id)
        {
            if !mod_entry.added_steps.iter().any(|s| s.id == step_id) {
                mod_entry.added_steps.push(step);
            }
        } else {
            self.workflow_modifications.push(WorkflowModification {
                workflow_id,
                workflow_name,
                added_steps: vec![step],
                removed_step_ids: Vec::new(),
            });
        }
        self.touch();
    }

    /// Add multiple steps to a workflow (batch operation)
    pub fn add_steps(
        &mut self,
        workflow_id: String,
        workflow_name: String,
        steps: Vec<(String, String, i32)>, // (id, name, order)
    ) {
        for (step_id, step_name, order) in steps {
            self.add_step(
                workflow_id.clone(),
                workflow_name.clone(),
                step_id,
                step_name,
                order,
            );
        }
    }

    /// Record a step removal
    pub fn remove_step(&mut self, workflow_id: String, workflow_name: String, step_id: String) {
        // If it's a workflow we created, remove from there
        if let Some(wf) = self.workflows.iter_mut().find(|w| w.id == workflow_id) {
            wf.steps.retain(|s| s.id != step_id);
            self.touch();
            return;
        }

        // Otherwise track as modification
        if let Some(mod_entry) = self
            .workflow_modifications
            .iter_mut()
            .find(|m| m.workflow_id == workflow_id)
        {
            // Remove from added_steps if it was added in this session
            mod_entry.added_steps.retain(|s| s.id != step_id);
            // Track removal
            if !mod_entry.removed_step_ids.contains(&step_id) {
                mod_entry.removed_step_ids.push(step_id);
            }
        } else {
            self.workflow_modifications.push(WorkflowModification {
                workflow_id,
                workflow_name,
                added_steps: Vec::new(),
                removed_step_ids: vec![step_id],
            });
        }
        self.touch();
    }

    /// Remove a workflow (if it was created in this session)
    pub fn remove_workflow(&mut self, workflow_id: &str) {
        self.workflows.retain(|w| w.id != workflow_id);
        self.touch();
    }

    /// Apply a context delta to update resources
    pub fn apply_delta(&mut self, delta: &ContextDelta, message_index: usize) {
        match delta.delta_type {
            ContextDeltaType::WorkflowCreated => {
                if let (Some(id), Some(name)) = (
                    &delta.resource.workflow_id,
                    &delta.resource.workflow_name,
                ) {
                    self.add_workflow(
                        id.clone(),
                        name.clone(),
                        delta.resource.project_id.clone(),
                        message_index,
                    );
                }
            }
            ContextDeltaType::StepAdded => {
                if let (Some(wf_id), Some(wf_name), Some(step_id), Some(step_name)) = (
                    &delta.resource.workflow_id,
                    &delta.resource.workflow_name,
                    &delta.resource.step_id,
                    &delta.resource.step_name,
                ) {
                    self.add_step(
                        wf_id.clone(),
                        wf_name.clone(),
                        step_id.clone(),
                        step_name.clone(),
                        delta.resource.step_order.unwrap_or(0),
                    );
                }
            }
            ContextDeltaType::StepsAdded => {
                if let (Some(wf_id), Some(wf_name), Some(step_ids), Some(step_names)) = (
                    &delta.resource.workflow_id,
                    &delta.resource.workflow_name,
                    &delta.resource.step_ids,
                    &delta.resource.step_names,
                ) {
                    let steps: Vec<(String, String, i32)> = step_ids
                        .iter()
                        .zip(step_names.iter())
                        .enumerate()
                        .map(|(i, (id, name))| (id.clone(), name.clone(), i as i32))
                        .collect();
                    self.add_steps(wf_id.clone(), wf_name.clone(), steps);
                }
            }
            ContextDeltaType::StepRemoved => {
                if let (Some(wf_id), Some(wf_name), Some(step_id)) = (
                    &delta.resource.workflow_id,
                    &delta.resource.workflow_name,
                    &delta.resource.step_id,
                ) {
                    self.remove_step(wf_id.clone(), wf_name.clone(), step_id.clone());
                }
            }
            ContextDeltaType::WorkflowDeleted => {
                if let Some(wf_id) = &delta.resource.workflow_id {
                    self.remove_workflow(wf_id);
                }
            }
        }
    }

    /// Get summary for system prompt injection
    pub fn get_context_summary(&self) -> Option<String> {
        if self.is_empty() {
            return None;
        }

        let mut lines = vec![
            "## Resources Created/Modified in This Conversation".to_string(),
            String::new(),
        ];

        if !self.workflows.is_empty() {
            lines.push("**Newly Created Workflows:**".to_string());
            for wf in &self.workflows {
                lines.push(format!(
                    "- `{}`: {} ({} steps)",
                    wf.id,
                    wf.name,
                    wf.steps.len()
                ));
                for step in &wf.steps {
                    lines.push(format!("  - Step `{}`: {}", step.id, step.name));
                }
            }
            lines.push(String::new());
        }

        if !self.workflow_modifications.is_empty() {
            lines.push("**Modified Workflows:**".to_string());
            for mod_entry in &self.workflow_modifications {
                let added = mod_entry.added_steps.len();
                let removed = mod_entry.removed_step_ids.len();
                lines.push(format!(
                    "- `{}`: {} (+{} steps, -{} steps)",
                    mod_entry.workflow_id, mod_entry.workflow_name, added, removed
                ));
                for step in &mod_entry.added_steps {
                    lines.push(format!("  - Added step `{}`: {}", step.id, step.name));
                }
            }
            lines.push(String::new());
        }

        lines.push(
            "Use these IDs when referencing resources created in this conversation.".to_string(),
        );

        Some(lines.join("\n"))
    }

    fn touch(&mut self) {
        self.last_updated = Some(chrono::Utc::now().timestamp_millis());
    }
}

#[cfg(test)]
mod session_context_tests {
    use super::*;

    #[test]
    fn test_session_context_has_project() {
        let mut ctx = SessionContext::new();
        assert!(!ctx.has_project());

        ctx.project_id = Some("proj_123".to_string());
        assert!(ctx.has_project());
    }

    #[test]
    fn test_session_context_is_workflow_bound() {
        let mut ctx = SessionContext::new();
        ctx.bound_workflows.push(WorkflowSummary {
            id: "wf_001".to_string(),
            name: "Test Workflow".to_string(),
            step_count: 3,
        });

        assert!(ctx.is_workflow_bound("wf_001"));
        assert!(!ctx.is_workflow_bound("wf_999"));
    }

    #[test]
    fn test_context_delta_creation() {
        let delta = ContextDelta::workflow_created(
            "wf_new".to_string(),
            "New Workflow".to_string(),
            Some("proj_123".to_string()),
        );
        assert_eq!(delta.delta_type, ContextDeltaType::WorkflowCreated);
        assert_eq!(delta.resource.workflow_id, Some("wf_new".to_string()));
    }

    #[test]
    fn test_session_created_resources_add_workflow() {
        let mut resources = SessionCreatedResources::new();
        assert!(resources.is_empty());

        resources.add_workflow(
            "wf_001".to_string(),
            "Test".to_string(),
            None,
            0,
        );
        assert!(!resources.is_empty());
        assert_eq!(resources.workflows.len(), 1);

        // Idempotency - adding same workflow again should not duplicate
        resources.add_workflow(
            "wf_001".to_string(),
            "Test".to_string(),
            None,
            1,
        );
        assert_eq!(resources.workflows.len(), 1);
    }

    #[test]
    fn test_session_created_resources_add_step_to_new_workflow() {
        let mut resources = SessionCreatedResources::new();
        resources.add_workflow("wf_001".to_string(), "Test".to_string(), None, 0);

        resources.add_step(
            "wf_001".to_string(),
            "Test".to_string(),
            "step_001".to_string(),
            "First Step".to_string(),
            0,
        );

        assert_eq!(resources.workflows[0].steps.len(), 1);
        assert_eq!(resources.workflows[0].steps[0].name, "First Step");
    }

    #[test]
    fn test_session_created_resources_add_step_to_existing_workflow() {
        let mut resources = SessionCreatedResources::new();

        // Add step to a workflow that was NOT created in this session
        resources.add_step(
            "wf_existing".to_string(),
            "Existing Workflow".to_string(),
            "step_001".to_string(),
            "New Step".to_string(),
            0,
        );

        assert!(resources.workflows.is_empty());
        assert_eq!(resources.workflow_modifications.len(), 1);
        assert_eq!(resources.workflow_modifications[0].added_steps.len(), 1);
    }

    #[test]
    fn test_session_created_resources_remove_step() {
        let mut resources = SessionCreatedResources::new();
        resources.add_workflow("wf_001".to_string(), "Test".to_string(), None, 0);
        resources.add_step(
            "wf_001".to_string(),
            "Test".to_string(),
            "step_001".to_string(),
            "Step 1".to_string(),
            0,
        );

        resources.remove_step(
            "wf_001".to_string(),
            "Test".to_string(),
            "step_001".to_string(),
        );

        assert_eq!(resources.workflows[0].steps.len(), 0);
    }

    #[test]
    fn test_session_created_resources_apply_delta() {
        let mut resources = SessionCreatedResources::new();

        let delta = ContextDelta::workflow_created(
            "wf_new".to_string(),
            "New Workflow".to_string(),
            None,
        );
        resources.apply_delta(&delta, 0);
        assert_eq!(resources.workflows.len(), 1);

        let delta = ContextDelta::step_added(
            "wf_new".to_string(),
            "New Workflow".to_string(),
            None,
            "step_1".to_string(),
            "Build".to_string(),
            0,
        );
        resources.apply_delta(&delta, 1);
        assert_eq!(resources.workflows[0].steps.len(), 1);
    }

    #[test]
    fn test_session_created_resources_get_context_summary() {
        let mut resources = SessionCreatedResources::new();

        // Empty resources should return None
        assert!(resources.get_context_summary().is_none());

        // Add some resources
        resources.add_workflow("wf_001".to_string(), "Build Workflow".to_string(), None, 0);
        resources.add_step(
            "wf_001".to_string(),
            "Build Workflow".to_string(),
            "step_1".to_string(),
            "Install".to_string(),
            0,
        );

        let summary = resources.get_context_summary().unwrap();
        assert!(summary.contains("Build Workflow"));
        assert!(summary.contains("wf_001"));
        assert!(summary.contains("Install"));
    }
}
