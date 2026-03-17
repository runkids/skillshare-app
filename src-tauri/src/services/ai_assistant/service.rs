// AI Assistant Service
// Feature: AI Assistant Tab (022-ai-assistant-tab)
//
// Main orchestration service for the AI assistant. Handles:
// - Conversation management
// - Message sending and streaming
// - AI provider integration
// - Tool call processing

use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::RwLock;

use crate::models::ai_assistant::{
    Conversation, ConversationDetail, ConversationListResponse, Message, MessageStatus,
    ProjectContext, SendMessageResponse, SuggestedAction, SuggestionsResponse,
    CreateConversationRequest, ListConversationsRequest,
};
use crate::repositories::AIConversationRepository;
use crate::utils::database::Database;

use super::sanitizer::InputSanitizer;
use super::stream::{StreamManager, StreamContext};
use super::tools::MCPToolHandler;

/// Main AI Assistant service
pub struct AIAssistantService {
    /// Database connection
    db: Database,
    /// Conversation repository
    repo: AIConversationRepository,
    /// Stream manager for active sessions
    stream_manager: Arc<RwLock<StreamManager>>,
    /// Input sanitizer
    sanitizer: InputSanitizer,
    /// Tool handler for MCP operations
    tool_handler: Arc<MCPToolHandler>,
}

impl AIAssistantService {
    /// Create a new AIAssistantService
    pub fn new(db: Database) -> Self {
        let repo = AIConversationRepository::new(db.clone());
        // IMPORTANT: Use with_database to enable path security validation
        let tool_handler = MCPToolHandler::with_database(db.clone());
        Self {
            db,
            repo,
            stream_manager: Arc::new(RwLock::new(StreamManager::new())),
            sanitizer: InputSanitizer::new(),
            tool_handler: Arc::new(tool_handler),
        }
    }

    /// Create with custom dependencies (for testing)
    pub fn with_dependencies(
        db: Database,
        stream_manager: StreamManager,
        sanitizer: InputSanitizer,
        tool_handler: MCPToolHandler,
    ) -> Self {
        let repo = AIConversationRepository::new(db.clone());
        Self {
            db,
            repo,
            stream_manager: Arc::new(RwLock::new(stream_manager)),
            sanitizer,
            tool_handler: Arc::new(tool_handler),
        }
    }

    // =========================================================================
    // Conversation Management
    // =========================================================================

    /// Create a new conversation
    pub fn create_conversation(&self, request: CreateConversationRequest) -> Result<Conversation, String> {
        // Validate project path if provided
        if let Some(ref path) = request.project_path {
            if !std::path::Path::new(path).exists() {
                return Err("INVALID_PROJECT_PATH".to_string());
            }
        }

        // Create conversation - title is set separately via update if needed
        let mut conversation = Conversation::new(
            request.project_path,
            request.provider_id,
        );

        // If title was provided, set it
        if request.title.is_some() {
            conversation.title = request.title;
        }

        self.repo.create_conversation(&conversation)?;

        Ok(conversation)
    }

    /// List conversations with optional filtering
    pub fn list_conversations(&self, request: ListConversationsRequest) -> Result<ConversationListResponse, String> {
        let limit = request.limit.unwrap_or(50);
        let offset = request.offset.unwrap_or(0);
        let order_by = request.order_by.as_deref().unwrap_or("updated");

        self.repo.list_conversations(
            request.project_path.as_deref(),
            limit,
            offset,
            order_by,
        )
    }

    /// Get a conversation with all messages
    pub fn get_conversation(&self, conversation_id: &str) -> Result<ConversationDetail, String> {
        let conversation = self.repo.get_conversation(conversation_id)?
            .ok_or_else(|| "CONVERSATION_NOT_FOUND".to_string())?;

        let messages = self.repo.get_messages(conversation_id)?;

        Ok(ConversationDetail {
            conversation,
            messages,
        })
    }

    /// Update conversation metadata
    pub fn update_conversation(
        &self,
        conversation_id: &str,
        title: Option<String>,
        project_path: Option<Option<String>>,
    ) -> Result<Conversation, String> {
        // Verify conversation exists
        self.repo.get_conversation(conversation_id)?
            .ok_or_else(|| "CONVERSATION_NOT_FOUND".to_string())?;

        // Validate project path if provided
        if let Some(Some(ref path)) = project_path {
            if !std::path::Path::new(path).exists() {
                return Err("INVALID_PROJECT_PATH".to_string());
            }
        }

        self.repo.update_conversation(
            conversation_id,
            title.as_deref(),
            project_path.as_ref().map(|p| p.as_deref()),
        )?;

        // Return updated conversation
        self.repo.get_conversation(conversation_id)?
            .ok_or_else(|| "CONVERSATION_NOT_FOUND".to_string())
    }

    /// Delete a conversation
    pub fn delete_conversation(&self, conversation_id: &str) -> Result<(), String> {
        let deleted = self.repo.delete_conversation(conversation_id)?;
        if !deleted {
            return Err("CONVERSATION_NOT_FOUND".to_string());
        }
        Ok(())
    }

    // =========================================================================
    // Message Handling
    // =========================================================================

    /// Send a message and start streaming response
    pub async fn send_message(
        &self,
        app: AppHandle,
        conversation_id: String,
        content: String,
        project_context: Option<ProjectContext>,
    ) -> Result<SendMessageResponse, String> {
        // Validate conversation exists
        self.repo.get_conversation(&conversation_id)?
            .ok_or_else(|| "CONVERSATION_NOT_FOUND".to_string())?;

        // Sanitize user input
        let sanitized_content = self.sanitizer.sanitize_user_input(&content)
            .map_err(|e| format!("INPUT_VALIDATION_ERROR: {}", e))?;

        // Create user message
        let user_message = Message::user(conversation_id.clone(), sanitized_content.clone());
        self.repo.create_message(&user_message)?;
        self.repo.increment_message_count(&conversation_id)?;

        // Create assistant message (pending)
        let assistant_message = Message::assistant(conversation_id.clone(), String::new());
        self.repo.create_message(&assistant_message)?;
        self.repo.increment_message_count(&conversation_id)?;

        // Create streaming session
        let stream_manager = self.stream_manager.read().await;
        let (session_id, _cancel_rx) = stream_manager.create_session().await;
        drop(stream_manager);

        // Build context for AI
        let _sanitized_context = project_context.map(|ctx| {
            ProjectContext {
                project_name: ctx.project_name,
                project_path: self.sanitizer.sanitize_paths(&ctx.project_path),
                project_type: ctx.project_type,
                package_manager: ctx.package_manager,
                available_scripts: ctx.available_scripts,
            }
        });

        // Start streaming in background task
        let response = SendMessageResponse {
            stream_session_id: session_id.clone(),
            conversation_id: conversation_id.clone(),
            message_id: assistant_message.id.clone(),
        };

        // Clone what we need for the background task
        let repo = AIConversationRepository::new(self.db.clone());
        let stream_manager = self.stream_manager.clone();
        let message_id = assistant_message.id.clone();
        let conv_id = conversation_id.clone();

        // Spawn background task to handle AI response
        tokio::spawn(async move {
            let mut ctx = StreamContext::new(
                session_id.clone(),
                conv_id.clone(),
                message_id.clone(),
                app,
            );

            // Feature 023: Emit thinking status (T024)
            if let Err(e) = ctx.emit_thinking() {
                log::warn!("Failed to emit thinking status: {}", e);
            }

            // Simulate thinking delay
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // TODO: Integrate with actual AI provider
            // For now, emit a placeholder response
            let placeholder_response = format!(
                "I received your message: \"{}\"\n\n\
                The AI assistant is not yet connected to an AI provider. \
                Please configure an AI service in Settings > AI Services.",
                sanitized_content
            );

            // Feature 023: Emit generating status (T025)
            if let Err(e) = ctx.emit_generating(Some("placeholder".to_string())) {
                log::warn!("Failed to emit generating status: {}", e);
            }

            // Simulate streaming by emitting tokens
            for word in placeholder_response.split_whitespace() {
                if let Err(e) = ctx.emit_token(&format!("{} ", word)) {
                    log::error!("Failed to emit token: {}", e);
                    // Feature 023: Emit error status (T028)
                    let _ = ctx.emit_error_status();
                    break;
                }
                // Small delay for visual effect
                tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
            }

            // Update message in database
            let _ = repo.update_message_completion(
                &message_id,
                ctx.get_content(),
                None,
                Some("placeholder"),
                None,
            );

            // Feature 023: Emit complete status with timing (T027)
            if let Err(e) = ctx.emit_complete_status() {
                log::warn!("Failed to emit complete status: {}", e);
            }

            // Emit completion event
            let _ = ctx.emit_complete(0, "placeholder", "stop");

            // Remove session
            let manager = stream_manager.write().await;
            manager.remove_session(&session_id).await;
        });

        Ok(response)
    }

    /// Cancel an ongoing stream
    pub async fn cancel_stream(&self, session_id: &str) -> Result<(), String> {
        let stream_manager = self.stream_manager.read().await;
        stream_manager.cancel_session(session_id).await
    }

    /// Regenerate a response from a specific message
    pub async fn regenerate_response(
        &self,
        _app: AppHandle,
        conversation_id: String,
        message_id: String,
    ) -> Result<SendMessageResponse, String> {
        // Verify message exists and is from user
        let message = self.repo.get_message(&message_id)?
            .ok_or_else(|| "MESSAGE_NOT_FOUND".to_string())?;

        if message.conversation_id != conversation_id {
            return Err("MESSAGE_NOT_FOUND".to_string());
        }

        // Delete messages after this one
        self.repo.delete_messages_after(&conversation_id, &message_id)?;

        // Create new assistant message
        let assistant_message = Message::assistant(conversation_id.clone(), String::new());
        self.repo.create_message(&assistant_message)?;
        self.repo.increment_message_count(&conversation_id)?;

        // Create streaming session
        let stream_manager = self.stream_manager.read().await;
        let (session_id, _cancel_rx) = stream_manager.create_session().await;
        drop(stream_manager);

        let response = SendMessageResponse {
            stream_session_id: session_id.clone(),
            conversation_id,
            message_id: assistant_message.id.clone(),
        };

        // TODO: Start actual AI streaming

        Ok(response)
    }

    // =========================================================================
    // Suggestions
    // =========================================================================

    /// Get suggested actions based on context
    /// Feature 023 US2: Enhanced context-aware suggestions (T053-T056)
    /// Prompts include MCP tool names to guide the AI to use specific tools
    pub fn get_suggestions(
        &self,
        _conversation_id: &str,
        project_path: Option<&str>,
    ) -> Result<SuggestionsResponse, String> {
        let mut suggestions = Vec::new();

        // Default general suggestions
        suggestions.push(SuggestedAction {
            id: "help".to_string(),
            label: "What can you do?".to_string(),
            prompt: "What tools and capabilities do you have? List all available MCP tools.".to_string(),
            icon: Some("HelpCircle".to_string()),
            variant: Some("default".to_string()),
            category: Some("general".to_string()),
            ..Default::default()
        });

        // Project-specific suggestions
        if let Some(path) = project_path {
            let project_path = std::path::Path::new(path);

            // T054: Git operations suggestions (uses get_worktree_status, get_git_diff)
            if project_path.join(".git").exists() {
                suggestions.push(SuggestedAction {
                    id: "git-status".to_string(),
                    label: "Check git status".to_string(),
                    prompt: "Show the git status of this project using get_worktree_status".to_string(),
                    icon: Some("GitBranch".to_string()),
                    variant: Some("default".to_string()),
                    category: Some("git".to_string()),
                    ..Default::default()
                });

                suggestions.push(SuggestedAction {
                    id: "commit".to_string(),
                    label: "Generate commit".to_string(),
                    prompt: "Generate a commit message based on the staged changes (use get_git_diff first)".to_string(),
                    icon: Some("GitCommit".to_string()),
                    variant: Some("primary".to_string()),
                    category: Some("git".to_string()),
                    ..Default::default()
                });

                suggestions.push(SuggestedAction {
                    id: "review".to_string(),
                    label: "Review changes".to_string(),
                    prompt: "Review the staged changes using get_git_diff and suggest improvements".to_string(),
                    icon: Some("FileSearch".to_string()),
                    variant: Some("default".to_string()),
                    category: Some("git".to_string()),
                    ..Default::default()
                });

                suggestions.push(SuggestedAction {
                    id: "diff".to_string(),
                    label: "Show diff".to_string(),
                    prompt: "Show the staged changes diff using get_git_diff".to_string(),
                    icon: Some("FileDiff".to_string()),
                    variant: Some("default".to_string()),
                    category: Some("git".to_string()),
                    ..Default::default()
                });

                suggestions.push(SuggestedAction {
                    id: "list-worktrees".to_string(),
                    label: "List worktrees".to_string(),
                    prompt: "List all git worktrees for this project using list_worktrees".to_string(),
                    icon: Some("GitFork".to_string()),
                    variant: Some("default".to_string()),
                    category: Some("git".to_string()),
                    ..Default::default()
                });
            }

            // T055: Node.js project suggestions (uses get_project, run_npm_script)
            if project_path.join("package.json").exists() {
                suggestions.push(SuggestedAction {
                    id: "list-scripts".to_string(),
                    label: "Project info".to_string(),
                    prompt: "Get project details including available scripts using get_project".to_string(),
                    icon: Some("Info".to_string()),
                    variant: Some("default".to_string()),
                    category: Some("project".to_string()),
                    ..Default::default()
                });

                suggestions.push(SuggestedAction {
                    id: "run-dev".to_string(),
                    label: "Run dev".to_string(),
                    prompt: "Run the dev script using run_npm_script".to_string(),
                    icon: Some("Play".to_string()),
                    variant: Some("primary".to_string()),
                    category: Some("project".to_string()),
                    ..Default::default()
                });

                suggestions.push(SuggestedAction {
                    id: "run-build".to_string(),
                    label: "Build project".to_string(),
                    prompt: "Run the build script using run_npm_script".to_string(),
                    icon: Some("Hammer".to_string()),
                    variant: Some("default".to_string()),
                    category: Some("project".to_string()),
                    ..Default::default()
                });

                suggestions.push(SuggestedAction {
                    id: "run-tests".to_string(),
                    label: "Run tests".to_string(),
                    prompt: "Run the test script using run_npm_script".to_string(),
                    icon: Some("TestTube".to_string()),
                    variant: Some("default".to_string()),
                    category: Some("project".to_string()),
                    ..Default::default()
                });

                suggestions.push(SuggestedAction {
                    id: "run-lint".to_string(),
                    label: "Run lint".to_string(),
                    prompt: "Run the lint script using run_npm_script".to_string(),
                    icon: Some("FileWarning".to_string()),
                    variant: Some("default".to_string()),
                    category: Some("project".to_string()),
                    ..Default::default()
                });
            }

            // T056: Rust project suggestions (uses run_script for cargo commands)
            if project_path.join("Cargo.toml").exists() {
                suggestions.push(SuggestedAction {
                    id: "cargo-check".to_string(),
                    label: "Cargo check".to_string(),
                    prompt: "Run cargo check to verify the code compiles using run_script".to_string(),
                    icon: Some("CheckCircle".to_string()),
                    variant: Some("default".to_string()),
                    category: Some("project".to_string()),
                    ..Default::default()
                });

                suggestions.push(SuggestedAction {
                    id: "cargo-build".to_string(),
                    label: "Cargo build".to_string(),
                    prompt: "Build the Rust project with cargo build using run_script".to_string(),
                    icon: Some("Hammer".to_string()),
                    variant: Some("primary".to_string()),
                    category: Some("project".to_string()),
                    ..Default::default()
                });

                suggestions.push(SuggestedAction {
                    id: "cargo-test".to_string(),
                    label: "Cargo test".to_string(),
                    prompt: "Run the Rust tests with cargo test using run_script".to_string(),
                    icon: Some("TestTube".to_string()),
                    variant: Some("default".to_string()),
                    category: Some("project".to_string()),
                    ..Default::default()
                });

                suggestions.push(SuggestedAction {
                    id: "cargo-clippy".to_string(),
                    label: "Run clippy".to_string(),
                    prompt: "Run clippy to check for linting issues using run_script".to_string(),
                    icon: Some("FileWarning".to_string()),
                    variant: Some("default".to_string()),
                    category: Some("project".to_string()),
                    ..Default::default()
                });
            }

            // T053: Workflow suggestions (uses list_workflows)
            suggestions.push(SuggestedAction {
                id: "list-workflows".to_string(),
                label: "List workflows".to_string(),
                prompt: "List available workflows using list_workflows".to_string(),
                icon: Some("Workflow".to_string()),
                variant: Some("default".to_string()),
                category: Some("workflow".to_string()),
                ..Default::default()
            });
        }

        Ok(SuggestionsResponse { suggestions })
    }

    // =========================================================================
    // Tool Handling
    // =========================================================================

    /// Get the tool handler
    pub fn get_tool_handler(&self) -> Arc<MCPToolHandler> {
        self.tool_handler.clone()
    }

    /// Update message status
    pub fn update_message_status(&self, message_id: &str, status: MessageStatus) -> Result<(), String> {
        self.repo.update_message_status(message_id, status)
    }
}

// ============================================================================
// Project Context Builder
// ============================================================================

/// Build project context from a project path
pub struct ProjectContextBuilder;

impl ProjectContextBuilder {
    /// Extract project context from a directory path
    pub fn build_from_path(project_path: &str) -> Result<ProjectContext, String> {
        let path = std::path::Path::new(project_path);

        if !path.exists() {
            return Err("Project path does not exist".to_string());
        }

        let project_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        // Detect project type and package manager
        let (project_type, package_manager, scripts) = Self::detect_project_info(project_path);

        Ok(ProjectContext {
            project_name,
            project_path: project_path.to_string(),
            project_type,
            package_manager,
            available_scripts: scripts,
        })
    }

    /// Detect project type, package manager, and available scripts
    fn detect_project_info(project_path: &str) -> (String, String, Vec<String>) {
        let path = std::path::Path::new(project_path);

        // Check for package.json (Node.js project)
        let package_json_path = path.join("package.json");
        if package_json_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&package_json_path) {
                return Self::parse_node_project(&content, path);
            }
        }

        // Check for Cargo.toml (Rust project)
        if path.join("Cargo.toml").exists() {
            return ("Rust".to_string(), "cargo".to_string(), vec![
                "build".to_string(),
                "test".to_string(),
                "run".to_string(),
            ]);
        }

        // Check for pyproject.toml or requirements.txt (Python project)
        if path.join("pyproject.toml").exists() {
            return ("Python".to_string(), "poetry".to_string(), Vec::new());
        }
        if path.join("requirements.txt").exists() {
            return ("Python".to_string(), "pip".to_string(), Vec::new());
        }

        // Check for go.mod (Go project)
        if path.join("go.mod").exists() {
            return ("Go".to_string(), "go".to_string(), vec![
                "build".to_string(),
                "test".to_string(),
                "run".to_string(),
            ]);
        }

        // Default - unknown project type
        ("Unknown".to_string(), "Unknown".to_string(), Vec::new())
    }

    /// Parse Node.js project info from package.json
    fn parse_node_project(content: &str, path: &std::path::Path) -> (String, String, Vec<String>) {
        let project_type = "Node.js".to_string();

        // Detect package manager from lockfiles
        let package_manager = if path.join("pnpm-lock.yaml").exists() {
            "pnpm".to_string()
        } else if path.join("yarn.lock").exists() {
            "yarn".to_string()
        } else if path.join("bun.lockb").exists() {
            "bun".to_string()
        } else {
            "npm".to_string()
        };

        // Extract scripts from package.json
        let scripts = Self::extract_scripts(content);

        (project_type, package_manager, scripts)
    }

    /// Extract script names from package.json content
    fn extract_scripts(content: &str) -> Vec<String> {
        // Simple JSON parsing for scripts
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(scripts) = json.get("scripts").and_then(|s| s.as_object()) {
                return scripts.keys().cloned().collect();
            }
        }
        Vec::new()
    }

    /// Sanitize project context before sending to AI
    pub fn sanitize_context(context: &ProjectContext) -> ProjectContext {
        let sanitizer = InputSanitizer::new();

        ProjectContext {
            project_name: context.project_name.clone(),
            project_path: sanitizer.sanitize_paths(&context.project_path),
            project_type: context.project_type.clone(),
            package_manager: context.package_manager.clone(),
            available_scripts: context.available_scripts.clone(),
        }
    }
}

/// Build the system prompt with project context
/// Feature 023: Now uses SystemPromptBuilder for structured prompts
pub fn build_system_prompt(project_context: Option<&ProjectContext>) -> String {
    use super::prompt_builder::SystemPromptBuilder;

    SystemPromptBuilder::new()
        .with_context(project_context.cloned())
        .build()
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
    fn test_create_conversation() {
        let db = setup_test_db();
        let service = AIAssistantService::new(db);

        let request = CreateConversationRequest {
            title: Some("Test Chat".to_string()),
            project_path: None,
            provider_id: None,
        };

        let conversation = service.create_conversation(request).expect("Failed to create conversation");

        assert_eq!(conversation.title, Some("Test Chat".to_string()));
        assert_eq!(conversation.message_count, 0);
    }

    #[test]
    fn test_get_conversation() {
        let db = setup_test_db();
        let service = AIAssistantService::new(db);

        let request = CreateConversationRequest {
            title: Some("Test".to_string()),
            project_path: None,
            provider_id: None,
        };

        let created = service.create_conversation(request).expect("Failed to create");

        let detail = service.get_conversation(&created.id).expect("Failed to get");

        assert_eq!(detail.conversation.id, created.id);
        assert!(detail.messages.is_empty());
    }

    #[test]
    fn test_delete_conversation() {
        let db = setup_test_db();
        let service = AIAssistantService::new(db);

        let request = CreateConversationRequest {
            title: None,
            project_path: None,
            provider_id: None,
        };

        let created = service.create_conversation(request).expect("Failed to create");

        service.delete_conversation(&created.id).expect("Failed to delete");

        let result = service.get_conversation(&created.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_system_prompt_without_context() {
        let prompt = build_system_prompt(None);
        assert!(prompt.contains("SpecForge"));
        // Feature 023: Uses SystemPromptBuilder, no project context section
        assert!(!prompt.contains("Current Project Context"));
    }

    #[test]
    fn test_build_system_prompt_with_context() {
        let context = ProjectContext {
            project_name: "TestApp".to_string(),
            project_path: "/test/path".to_string(),
            project_type: "Node.js".to_string(),
            package_manager: "pnpm".to_string(),
            available_scripts: vec!["build".to_string(), "test".to_string()],
        };

        let prompt = build_system_prompt(Some(&context));

        // Feature 023: Uses SystemPromptBuilder format
        assert!(prompt.contains("SpecForge"));
        assert!(prompt.contains("TestApp"));
        assert!(prompt.contains("Node.js"));
        assert!(prompt.contains("pnpm"));
        assert!(prompt.contains("build, test"));
    }
}
