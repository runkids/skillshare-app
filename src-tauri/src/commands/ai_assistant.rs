// AI Assistant Tauri Commands
// Feature: AI Assistant Tab (022-ai-assistant-tab)
// Enhancement: AI Precision Improvement (025-ai-workflow-generator)
//
// Tauri commands for:
// - Creating and managing conversations
// - Sending messages with streaming responses
// - Cancelling active streams
// - Managing conversation history
// - Session context tracking for precise project/workflow targeting (025)

use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::models::ai_assistant::{
    Conversation, ConversationListResponse, Message, SendMessageRequest, SendMessageResponse,
    MessageStatus, MessageRole, ToolCall, ToolResult, ToolCallStatus, AvailableTools, SuggestionsResponse,
    ProjectContext, SessionContext, SessionCreatedResources, ContextDelta,
};
use crate::models::ai::{ChatMessage, ChatOptions, ChatToolCall};
use crate::repositories::{AIConversationRepository, AIRepository};
use crate::services::ai::{create_provider, AIKeychain};
use crate::services::ai_assistant::{
    StreamManager, StreamContext, MCPToolHandler, ProjectContextBuilder,
    SessionContextBuilder, build_system_prompt_with_session_context,
};
use crate::DatabaseState;

// ============================================================================
// Constants
// ============================================================================

/// Maximum number of tool call iterations to prevent infinite loops
const MAX_TOOL_ITERATIONS: usize = 10;

// ============================================================================
// Conversation Commands
// ============================================================================

/// Create a new conversation
#[tauri::command]
pub async fn ai_assistant_create_conversation(
    db: State<'_, DatabaseState>,
    project_path: Option<String>,
    provider_id: Option<String>,
) -> Result<Conversation, String> {
    let repo = AIConversationRepository::new(db.0.as_ref().clone());
    let ai_repo = AIRepository::new(db.0.as_ref().clone());

    // If no provider_id provided, use the default service
    let effective_provider_id = if provider_id.is_some() {
        provider_id
    } else {
        // Get the default AI service
        ai_repo.get_default_provider()
            .ok()
            .flatten()
            .map(|s| s.id)
    };

    let conversation = Conversation::new(project_path, effective_provider_id);
    repo.create_conversation(&conversation)?;

    Ok(conversation)
}

/// Get a conversation by ID
#[tauri::command]
pub async fn ai_assistant_get_conversation(
    db: State<'_, DatabaseState>,
    conversation_id: String,
) -> Result<Option<Conversation>, String> {
    let repo = AIConversationRepository::new(db.0.as_ref().clone());

    repo.get_conversation(&conversation_id)
}

/// List all conversations
#[tauri::command]
pub async fn ai_assistant_list_conversations(
    db: State<'_, DatabaseState>,
    project_path: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<ConversationListResponse, String> {
    let repo = AIConversationRepository::new(db.0.as_ref().clone());

    repo.list_conversations(
        project_path.as_deref(),
        limit.unwrap_or(50),
        offset.unwrap_or(0),
        "updated",
    )
}

/// Update a conversation
#[tauri::command]
pub async fn ai_assistant_update_conversation(
    db: State<'_, DatabaseState>,
    conversation_id: String,
    title: Option<String>,
) -> Result<(), String> {
    let repo = AIConversationRepository::new(db.0.as_ref().clone());

    repo.update_conversation(&conversation_id, title.as_deref(), None)
}

/// Update a conversation's AI service
#[tauri::command]
pub async fn ai_assistant_update_conversation_service(
    db: State<'_, DatabaseState>,
    conversation_id: String,
    provider_id: Option<String>,
) -> Result<(), String> {
    let repo = AIConversationRepository::new(db.0.as_ref().clone());

    repo.update_conversation_service(&conversation_id, provider_id.as_deref())
}

/// Update a conversation's project context
/// Feature 024: Context-Aware AI Assistant
#[tauri::command]
pub async fn ai_assistant_update_conversation_context(
    db: State<'_, DatabaseState>,
    conversation_id: String,
    project_path: Option<String>,
) -> Result<(), String> {
    let repo = AIConversationRepository::new(db.0.as_ref().clone());

    // Use Some(Some(path)) to set, Some(None) to clear
    let project_path_update = Some(project_path.as_deref());
    repo.update_conversation(&conversation_id, None, project_path_update)
}

/// Delete a conversation
#[tauri::command]
pub async fn ai_assistant_delete_conversation(
    db: State<'_, DatabaseState>,
    conversation_id: String,
) -> Result<bool, String> {
    let repo = AIConversationRepository::new(db.0.as_ref().clone());

    repo.delete_conversation(&conversation_id)
}

// ============================================================================
// Message Commands
// ============================================================================

/// Send a message and get streaming response
#[tauri::command]
pub async fn ai_assistant_send_message(
    app: AppHandle,
    db: State<'_, DatabaseState>,
    stream_manager: State<'_, StreamManager>,
    request: SendMessageRequest,
) -> Result<SendMessageResponse, String> {
    let conv_repo = AIConversationRepository::new(db.0.as_ref().clone());
    let ai_repo = AIRepository::new(db.0.as_ref().clone());
    let keychain = AIKeychain::new(app.clone());

    // Get AI service config
    let service = if let Some(ref id) = request.provider_id {
        match ai_repo.get_provider(id) {
            Ok(Some(s)) if s.is_enabled => s,
            Ok(Some(_)) => return Err("The specified AI service is disabled".to_string()),
            Ok(None) => {
                // Provider was deleted, fall back to default provider
                log::warn!("AI provider {} not found, falling back to default", id);
                match ai_repo.get_default_provider() {
                    Ok(Some(s)) => s,
                    Ok(None) => return Err("No default AI service configured. Please configure an AI service in Settings.".to_string()),
                    Err(e) => return Err(e),
                }
            },
            Err(e) => return Err(e),
        }
    } else {
        match ai_repo.get_default_provider() {
            Ok(Some(s)) => s,
            Ok(None) => return Err("No default AI service configured. Please configure an AI service in Settings.".to_string()),
            Err(e) => return Err(e),
        }
    };

    // Get API key
    let api_key = match keychain.get_api_key(&service.id) {
        Ok(key) => key,
        Err(e) => {
            log::error!("Failed to get API key for service {}: {}", service.id, e);
            return Err(format!("Failed to retrieve API key: {}", e));
        }
    };

    // Create provider
    let provider = match create_provider(service.clone(), api_key) {
        Ok(p) => p,
        Err(e) => return Err(format!("Failed to create AI provider: {}", e)),
    };

    // Create or get conversation
    let (conversation_id, is_new_conversation) = if let Some(id) = request.conversation_id {
        (id, false)
    } else {
        // Auto-generate title from first message
        let title = generate_conversation_title(&request.content);
        let conversation = Conversation::with_title(
            title,
            request.project_path.clone(),
            Some(service.id.clone()),
        );
        conv_repo.create_conversation(&conversation)?;
        (conversation.id, true)
    };
    let _ = is_new_conversation; // Mark as used (for potential future use)

    // Create user message
    let user_message = Message::user(conversation_id.clone(), request.content.clone());
    conv_repo.create_message(&user_message)?;

    // Create assistant message placeholder
    let assistant_message = Message::assistant(conversation_id.clone(), String::new());
    conv_repo.create_message(&assistant_message)?;

    // Load conversation history for context
    let history = conv_repo.get_messages(&conversation_id).unwrap_or_default();

    // Build ChatMessage array from history
    let mut chat_messages: Vec<ChatMessage> = Vec::new();

    // Build project context from path if not provided directly
    // Feature 024: Auto-generate project context from project_path
    let project_context = if request.project_context.is_some() {
        request.project_context.clone()
    } else if let Some(ref path) = request.project_path {
        ProjectContextBuilder::build_from_path(path).ok()
    } else {
        None
    };

    // Add system message with context
    let system_prompt = build_system_prompt(&project_context);
    chat_messages.push(ChatMessage::system(system_prompt));

    // Add conversation history (excluding the just-created assistant placeholder)
    for msg in history.iter() {
        if msg.id == assistant_message.id {
            continue; // Skip the placeholder
        }
        match msg.role.to_string().as_str() {
            "user" => chat_messages.push(ChatMessage::user(&msg.content)),
            "assistant" => {
                // Include tool calls if present
                if let Some(ref tool_calls) = msg.tool_calls {
                    let chat_tool_calls: Vec<ChatToolCall> = tool_calls.iter().map(|tc| {
                        ChatToolCall {
                            id: tc.id.clone(),
                            tool_type: "function".to_string(),
                            function: crate::models::ai::ChatFunctionCall {
                                name: tc.name.clone(),
                                arguments: tc.arguments.to_string(),
                            },
                            thought_signature: tc.thought_signature.clone(),
                        }
                    }).collect();
                    chat_messages.push(ChatMessage::assistant_with_tool_calls(
                        if msg.content.is_empty() { None } else { Some(msg.content.clone()) },
                        chat_tool_calls,
                    ));
                } else {
                    chat_messages.push(ChatMessage::assistant(&msg.content));
                }
            }
            "tool" => {
                // Include tool results from history
                if let Some(ref results) = msg.tool_results {
                    for result in results {
                        chat_messages.push(ChatMessage::tool_result(&result.call_id, result.output.clone()));
                    }
                }
            }
            _ => {} // Skip system messages in history
        }
    }

    // Create streaming session with info for reconnection support
    let (stream_session_id, cancel_rx) = stream_manager
        .create_session_with_info(
            conversation_id.clone(),
            assistant_message.id.clone(),
        )
        .await;

    // Get stream info reference for syncing during streaming
    let stream_info_ref = stream_manager.get_stream_info_ref();

    // Create stream context for emitting events with reconnection support
    let stream_ctx = StreamContext::with_stream_info(
        stream_session_id.clone(),
        conversation_id.clone(),
        assistant_message.id.clone(),
        app.clone(),
        stream_info_ref,
    );

    // Spawn background task for AI response
    let app_clone = app.clone();
    let assistant_message_id = assistant_message.id.clone();
    let session_id = stream_session_id.clone();
    let model_name = service.model.clone();
    let conversation_id_for_spawn = conversation_id.clone();

    // Get project path for tool context
    let project_path_for_tools = request.project_path.clone();

    // Clone database for the spawned task
    let db_for_tools = db.0.as_ref().clone();

    tokio::spawn(async move {
        let conversation_id = conversation_id_for_spawn;
        let mut ctx = stream_ctx;

        // Initialize tool handler with database for security validation
        let tool_handler = MCPToolHandler::with_database(db_for_tools.clone());
        let tool_definitions = tool_handler.get_chat_tool_definitions(project_path_for_tools.as_deref());

        // Feature 025: Build SessionContext for precise project/workflow targeting
        let session_context: Option<SessionContext> = {
            let builder = SessionContextBuilder::new(db_for_tools.clone());
            builder.build(None, project_path_for_tools.as_deref())
        };

        // Feature 025: Track resources created during this conversation
        let mut created_resources = SessionCreatedResources::new();
        let mut message_index: usize = 0; // Track message index for context delta

        // Chat options with tools
        let options = ChatOptions {
            temperature: Some(0.7),
            max_tokens: Some(4096),
            top_p: None,
            tools: if tool_definitions.is_empty() { None } else { Some(tool_definitions) },
        };

        // Agentic loop - continue until we get a final text response
        let mut messages = chat_messages;
        let mut total_tokens: i64 = 0;
        let mut iteration = 0;
        // Track all tool results for fallback (in case AI returns empty content)
        let mut all_tool_results: Vec<(String, bool, String)> = Vec::new(); // (name, success, output)
        // Track executed tool calls across iterations to prevent infinite loops
        // Key: function_name:arguments, Value: number of times executed
        let mut executed_tool_calls: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        const MAX_SAME_TOOL_EXECUTIONS: usize = 2; // Allow same tool call max 2 times
        // Track seen content to detect repetition loops
        let mut seen_content: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        const MAX_SAME_CONTENT: usize = 2; // If same content appears 2+ times, stop

        loop {
            iteration += 1;
            // Sync iteration counter with StreamContext for accurate status display
            ctx.set_iteration(iteration as u32);

            if iteration > MAX_TOOL_ITERATIONS {
                log::warn!("Agentic loop reached maximum iterations ({})", MAX_TOOL_ITERATIONS);
                // Generate a message informing the user
                let warning_msg = "I've reached the maximum number of tool calls for this response. Please let me know if you'd like me to continue with a new request.";
                let _ = ctx.emit_token(warning_msg);
                break;
            }

            // Check for cancellation
            if cancel_rx.is_closed() {
                return;
            }

            // Call AI provider
            match provider.chat_completion(messages.clone(), options.clone()).await {
                Ok(response) => {
                    total_tokens += response.tokens_used.unwrap_or(0) as i64;

                    // Check for repeated content (AI stuck in a loop)
                    if !response.content.is_empty() {
                        // Use first 200 chars as key to detect similar content
                        let content_key = response.content.chars().take(200).collect::<String>();
                        let count = seen_content.entry(content_key.clone()).or_insert(0);
                        *count += 1;
                        if *count >= MAX_SAME_CONTENT {
                            log::warn!("Detected repeated content - AI may be stuck in a loop");
                            // Don't emit the repeated content, just break
                            if ctx.get_content().is_empty() {
                                let _ = ctx.emit_token("I notice I'm repeating myself. Let me stop here.");
                            }
                            break;
                        }
                    }

                    // Check if AI wants to call tools
                    // Some models may not set finish_reason to ToolCalls, so we check tool_calls presence
                    if let Some(ref tool_calls) = response.tool_calls {
                        if !tool_calls.is_empty() {
                            log::info!("AI requested {} tool calls (finish_reason: {:?})", tool_calls.len(), response.finish_reason);

                            // Get list of valid tool names
                            let valid_tools: std::collections::HashSet<String> = tool_handler
                                .get_available_tools(project_path_for_tools.as_deref())
                                .tools
                                .iter()
                                .map(|t| t.name.clone())
                                .collect();

                            // Filter out invalid tool calls (malformed responses from models that don't support function calling)
                            let valid_tool_calls: Vec<_> = tool_calls.iter()
                                .filter(|tc| {
                                    let is_valid = valid_tools.contains(&tc.function.name);
                                    if !is_valid {
                                        log::warn!(
                                            "Ignoring invalid tool call: '{}' (not in available tools). Model may not support function calling properly.",
                                            tc.function.name
                                        );
                                    }
                                    is_valid
                                })
                                .collect();

                            // If no valid tool calls, treat as regular text response
                            if valid_tool_calls.is_empty() {
                                log::warn!("All tool calls were invalid - treating as text response");
                                // Fall through to text response handling below
                            } else {
                                // Deduplicate tool calls by function name + arguments (within this response)
                                let mut seen_tools: std::collections::HashSet<String> = std::collections::HashSet::new();
                                let unique_tool_calls: Vec<_> = valid_tool_calls.iter()
                                    .filter(|tc| {
                                        let key = format!("{}:{}", tc.function.name, tc.function.arguments);
                                        seen_tools.insert(key)
                                    })
                                    .cloned()
                                    .collect();

                                // Filter out tool calls that have been executed too many times across iterations
                                let filtered_tool_calls: Vec<_> = unique_tool_calls.iter()
                                    .filter(|tc| {
                                        let key = format!("{}:{}", tc.function.name, tc.function.arguments);
                                        let count = executed_tool_calls.get(&key).copied().unwrap_or(0);
                                        if count >= MAX_SAME_TOOL_EXECUTIONS {
                                            log::warn!(
                                                "Skipping repeated tool call '{}' (already executed {} times)",
                                                tc.function.name, count
                                            );
                                            false
                                        } else {
                                            true
                                        }
                                    })
                                    .cloned()
                                    .collect();

                                log::info!("After validation and deduplication: {} tool calls (filtered from {})",
                                    filtered_tool_calls.len(), unique_tool_calls.len());

                                // If all tool calls were filtered out, break the loop
                                if filtered_tool_calls.is_empty() && !unique_tool_calls.is_empty() {
                                    log::warn!("All tool calls were filtered as duplicates - breaking loop");
                                    let warning_msg = "I notice I'm trying to repeat the same action. Let me provide you with the results I already have.";
                                    let _ = ctx.emit_token(warning_msg);
                                    break;
                                }

                                // Replace unique_tool_calls with filtered ones
                                let unique_tool_calls = filtered_tool_calls;

                                // Add assistant message with tool calls to conversation (in-memory for AI context)
                                // Use only valid tool calls, not the original ones
                                let valid_chat_tool_calls: Vec<ChatToolCall> = unique_tool_calls.iter()
                                    .map(|tc| (*tc).clone())
                                    .collect();
                                messages.push(ChatMessage::assistant_with_tool_calls(
                                    if response.content.is_empty() { None } else { Some(response.content.clone()) },
                                    valid_chat_tool_calls,
                                ));

                            // Check if any tool requires confirmation
                            let has_confirmation_required = unique_tool_calls.iter()
                                .any(|tc| tool_handler.requires_confirmation(&tc.function.name));

                            // If all tools can be auto-executed, prepare to save the assistant message with tool_calls to DB
                            // This preserves the tool call history for future context
                            // We'll save with correct statuses AFTER execution
                            let tool_call_msg_id = if !has_confirmation_required {
                                Some(format!("msg_{}_tc_{}", Utc::now().timestamp_millis(), iteration))
                            } else {
                                None
                            };

                            // Track execution results for status updates
                            let mut execution_results: std::collections::HashMap<String, bool> = std::collections::HashMap::new();

                            // Process each unique tool call
                            for tool_call in &unique_tool_calls {
                                // Check if tool requires confirmation
                                if tool_handler.requires_confirmation(&tool_call.function.name) {
                                    // For tools requiring confirmation, stop the loop
                                    // and let the frontend handle approval
                                    log::info!("Tool {} requires confirmation, stopping loop", tool_call.function.name);

                                    // Convert ChatToolCall to internal ToolCall format for storage and event
                                    let internal_tool_call = convert_chat_tool_call_to_internal(tool_call);

                                    // Emit a message about the tool call
                                    let tool_msg = format!(
                                        "I'd like to execute **{}**. This action requires your approval.",
                                        tool_call.function.name
                                    );
                                    let _ = ctx.emit_token(&tool_msg);

                                    // Emit tool_call event to frontend so it shows the approval UI
                                    if let Err(e) = ctx.emit_tool_call(&internal_tool_call) {
                                        log::error!("Failed to emit tool call: {}", e);
                                    }

                                    // Store tool call in the message for persistence
                                    let tool_calls_for_db = vec![internal_tool_call];

                                    // Update message with tool calls
                                    let db_state = app_clone.state::<DatabaseState>();
                                    let repo = AIConversationRepository::new(db_state.0.as_ref().clone());
                                    let _ = repo.update_message_completion(
                                        &assistant_message_id,
                                        ctx.get_content(),
                                        Some(total_tokens),
                                        Some(&model_name),
                                        Some(&tool_calls_for_db),
                                    );

                                    if let Err(e) = ctx.emit_complete(total_tokens, &model_name, "tool_calls") {
                                        log::error!("Failed to emit complete: {}", e);
                                    }

                                    let stream_mgr = app_clone.state::<StreamManager>();
                                    stream_mgr.remove_session(&session_id).await;
                                    return;
                                }

                                // Auto-execute read-only tools
                                let mut internal_tool_call = convert_chat_tool_call_to_internal(tool_call);

                                // Feature 025: Auto-inject session context (project_id, cwd) into tool arguments
                                inject_session_context_to_tool_call(&mut internal_tool_call, session_context.as_ref());

                                // Feature 025: Validate execution context and log warnings if cross-project
                                tool_handler.validate_execution_context(&internal_tool_call, session_context.as_ref());

                                let result = tool_handler.execute_tool_call(&internal_tool_call).await;

                                // Track this tool call as executed (for cross-iteration deduplication)
                                let tool_key = format!("{}:{}", tool_call.function.name, tool_call.function.arguments);
                                *executed_tool_calls.entry(tool_key).or_insert(0) += 1;

                                log::info!(
                                    "Tool {} executed: success={}, output_len={}",
                                    tool_call.function.name,
                                    result.success,
                                    result.output.len()
                                );
                                log::debug!("Tool output: {}", &result.output);

                                // Feature 025: Extract and apply context delta if present
                                if let Some(ref metadata) = result.metadata {
                                    if let Some(delta_value) = metadata.get("context_delta") {
                                        if let Ok(delta) = serde_json::from_value::<ContextDelta>(delta_value.clone()) {
                                            log::info!("Applying context delta: {:?}", delta.delta_type);
                                            created_resources.apply_delta(&delta, message_index);

                                            // Handle batch step creation for create_workflow_with_steps
                                            if let (Some(step_ids), Some(step_names)) = (
                                                metadata.get("created_step_ids").and_then(|v| v.as_array()),
                                                metadata.get("created_step_names").and_then(|v| v.as_array()),
                                            ) {
                                                if let Some(ref wf_id) = delta.resource.workflow_id {
                                                    let wf_name = delta.resource.workflow_name.clone().unwrap_or_default();
                                                    for (i, (id, name)) in step_ids.iter().zip(step_names.iter()).enumerate() {
                                                        if let (Some(id_str), Some(name_str)) = (id.as_str(), name.as_str()) {
                                                            created_resources.add_step(
                                                                wf_id.clone(),
                                                                wf_name.clone(),
                                                                id_str.to_string(),
                                                                name_str.to_string(),
                                                                i as i32,
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Track execution result for status update
                                execution_results.insert(tool_call.id.clone(), result.success);

                                // Add tool result to messages for AI context (output is already a string)
                                messages.push(ChatMessage::tool_result(&tool_call.id, result.output.clone()));

                                // Track tool result for fallback (in case AI returns empty content)
                                all_tool_results.push((
                                    tool_call.function.name.clone(),
                                    result.success,
                                    result.output.clone(),
                                ));

                                // Save tool result to database for history persistence
                                {
                                    let tool_result = ToolResult {
                                        call_id: tool_call.id.clone(),
                                        success: result.success,
                                        output: result.output.clone(),
                                        error: if result.success { None } else { Some(result.output.clone()) },
                                        duration_ms: None,
                                        metadata: result.metadata.clone(), // Feature 025: Preserve metadata
                                    };
                                    let tool_message = Message::tool_result(
                                        conversation_id.clone(),
                                        String::new(), // content is empty for tool result messages
                                        vec![tool_result],
                                    );
                                    let db_state = app_clone.state::<DatabaseState>();
                                    let repo = AIConversationRepository::new(db_state.0.as_ref().clone());
                                    if let Err(e) = repo.create_message(&tool_message) {
                                        log::error!("Failed to save tool result message: {}", e);
                                    }
                                }
                            }

                            // After all auto-executed tools, save the assistant message with correct statuses
                            if let Some(ref msg_id) = tool_call_msg_id {
                                // Create tool calls with correct statuses based on execution results
                                let internal_tool_calls: Vec<ToolCall> = unique_tool_calls.iter()
                                    .map(|tc| {
                                        let success = execution_results.get(&tc.id).copied().unwrap_or(false);
                                        ToolCall {
                                            id: tc.id.clone(),
                                            name: tc.function.name.clone(),
                                            arguments: serde_json::from_str(&tc.function.arguments)
                                                .unwrap_or(serde_json::json!({})),
                                            status: if success { ToolCallStatus::Completed } else { ToolCallStatus::Failed },
                                            thought_signature: tc.thought_signature.clone(),
                                        }
                                    })
                                    .collect();

                                // Create and save the assistant message with tool_calls
                                let tool_call_msg = Message {
                                    id: msg_id.clone(),
                                    conversation_id: conversation_id.clone(),
                                    role: MessageRole::Assistant,
                                    content: response.content.clone(),
                                    tool_calls: Some(internal_tool_calls),
                                    tool_results: None,
                                    status: MessageStatus::Sent,
                                    tokens_used: response.tokens_used.map(|t| t as i64),
                                    model: Some(model_name.clone()),
                                    created_at: Utc::now(),
                                };

                                let db_state = app_clone.state::<DatabaseState>();
                                let repo = AIConversationRepository::new(db_state.0.as_ref().clone());
                                if let Err(e) = repo.create_message(&tool_call_msg) {
                                    log::error!("Failed to save assistant tool call message: {}", e);
                                }
                            }

                            // Feature 025: Update system prompt with created resources for next iteration
                            // This ensures AI sees newly created workflow/step IDs in subsequent calls
                            if !created_resources.is_empty() {
                                let updated_prompt = build_system_prompt_with_session_context(
                                    tool_handler.get_chat_tool_definitions(project_path_for_tools.as_deref())
                                        .into_iter()
                                        .map(|d| crate::models::ai_assistant::ToolDefinition {
                                            name: d.function.name.clone(),
                                            description: d.function.description.clone(),
                                            parameters: d.function.parameters.clone(),
                                            requires_confirmation: tool_handler.requires_confirmation(&d.function.name),
                                            category: String::new(),
                                        })
                                        .collect(),
                                    session_context.as_ref(),
                                    Some(&created_resources),
                                );

                                // Update the system message (first message in array)
                                if let Some(system_msg) = messages.first_mut() {
                                    if system_msg.role == "system" {
                                        *system_msg = ChatMessage::system(updated_prompt);
                                        log::debug!("Updated system prompt with created resources");
                                    }
                                }
                            }

                            // Increment message index for context delta tracking
                            message_index += 1;

                            // Continue the loop to get AI's response after tool execution
                            continue;
                            } // end else (valid tool calls)
                        } // end if !tool_calls.is_empty()
                    } // end if let Some(tool_calls)

                    // No tool calls or final response - emit content
                    let mut content = response.content.clone();

                    log::info!(
                        "AI response: content_len={}, finish_reason={:?}, tokens={}",
                        content.len(),
                        response.finish_reason,
                        response.tokens_used.unwrap_or(0)
                    );

                    // If content is empty but we have tool results, use them as fallback
                    // This handles models that don't properly respond after tool execution
                    if content.is_empty() && !all_tool_results.is_empty() {
                        log::info!("AI returned empty content, generating fallback summary for {} tool(s)", all_tool_results.len());
                        content = generate_tool_results_summary(&all_tool_results);
                    } else if content.is_empty() {
                        log::warn!("AI returned empty content with no tool results");
                    }

                    let chars: Vec<char> = content.chars().collect();

                    for (i, chunk) in chars.chunks(5).enumerate() {
                        if cancel_rx.is_closed() {
                            break;
                        }

                        let token: String = chunk.iter().collect();
                        if let Err(e) = ctx.emit_token(&token) {
                            log::error!("Failed to emit token: {}", e);
                            break;
                        }

                        // Small delay to create streaming effect
                        if i % 5 == 0 {
                            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
                        }
                    }

                    let finish_reason = response.finish_reason
                        .map(|r| format!("{:?}", r).to_lowercase())
                        .unwrap_or_else(|| "stop".to_string());

                    if let Err(e) = ctx.emit_complete(total_tokens, &model_name, &finish_reason) {
                        log::error!("Failed to emit complete: {}", e);
                    }

                    // Update message in database
                    let db_state = app_clone.state::<DatabaseState>();
                    let repo = AIConversationRepository::new(db_state.0.as_ref().clone());
                    let _ = repo.update_message_completion(
                        &assistant_message_id,
                        ctx.get_content(),
                        Some(total_tokens),
                        Some(&model_name),
                        None,
                    );

                    // Exit the loop - we got a final text response
                    break;
                }
                Err(e) => {
                    log::error!("AI chat completion failed: {}", e);

                    // Emit error event
                    if let Err(emit_err) = ctx.emit_error(
                        "AI_PROVIDER_ERROR",
                        &format!("Failed to get AI response: {}", e),
                        true, // retryable
                    ) {
                        log::error!("Failed to emit error: {}", emit_err);
                    }

                    // Update message status to error
                    let db_state = app_clone.state::<DatabaseState>();
                    let repo = AIConversationRepository::new(db_state.0.as_ref().clone());
                    let _ = repo.update_message_status(&assistant_message_id, MessageStatus::Error);

                    break;
                }
            }
        }

        // Remove session
        let stream_mgr = app_clone.state::<StreamManager>();
        stream_mgr.remove_session(&session_id).await;
    });

    Ok(SendMessageResponse {
        stream_session_id,
        conversation_id,
        message_id: assistant_message.id,
    })
}

/// Generate a conversation title from the first user message
fn generate_conversation_title(message: &str) -> String {
    let trimmed = message.trim();

    // If message is short enough, use it directly
    if trimmed.len() <= 50 {
        return trimmed.to_string();
    }

    // Find a good breaking point (space, punctuation)
    let max_len = 47; // Leave room for "..."

    // UTF-8 safe truncation: find valid character boundary
    let truncate_at = trimmed
        .char_indices()
        .take_while(|(i, _)| *i < max_len)
        .last()
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(trimmed.len().min(max_len));
    let truncated = &trimmed[..truncate_at];

    // Try to find a word boundary (rfind returns byte index of ASCII space, which is safe)
    if let Some(last_space) = truncated.rfind(' ') {
        if last_space > 20 {
            return format!("{}...", &truncated[..last_space]);
        }
    }

    // Fall back to character truncation
    format!("{}...", truncated)
}

/// Build system prompt with project context and available tools
fn build_system_prompt(project_context: &Option<ProjectContext>) -> String {
    let mut prompt = String::from(
r#"You are an AI assistant integrated into SpecForge, a development workflow management tool.

## Your Capabilities
- Running project scripts and workflows
- Generating commit messages
- Reviewing code changes
- Answering questions about projects
- Getting git status and staged changes

## Available Tools
You have access to these MCP tools. When you need to perform an action, tell the user which tool you would use and wait for their confirmation.

### Execution Tools (Require User Approval)
1. **run_script**: Run a script ONLY if it's defined in the project's package.json
   - Parameters: script_name (required), project_path (required)
   - IMPORTANT: script_name MUST be one of the available scripts listed below
   - Example: "I'll use run_script to execute the 'build' script"
   - DO NOT use this for package manager commands like 'audit', 'outdated', 'install' - use run_package_manager_command instead

2. **run_package_manager_command**: Run package manager commands directly (audit, outdated, install, update, etc.)
   - Parameters: command (required), project_path (required), args (optional array)
   - Supported commands: audit, outdated, install, update, prune, dedupe, why, list, info
   - Example: "I'll use run_package_manager_command with command='audit' to check for vulnerabilities"
   - Use this for security audits, dependency checks, and package management

3. **run_workflow**: Execute a SpecForge workflow
   - Parameters: workflow_id (required)
   - Example: "I'll use run_workflow to run the deployment workflow"

4. **trigger_webhook**: Trigger a configured webhook
   - Parameters: webhook_id (required), payload (optional)

### Read-Only Tools (Auto-Approved)
5. **get_git_status**: Get current git status of a repository
   - Parameters: project_path (required)

6. **get_staged_diff**: Get diff of staged changes
   - Parameters: project_path (required)

7. **list_project_scripts**: List available scripts from package.json
   - Parameters: project_path (required)

## Important Clarifications

### What run_script CAN do:
- Run scripts defined in package.json (e.g., "build", "test", "dev", "lint")
- These are custom scripts the project has set up

### What run_script CANNOT do:
- Run package manager commands like `audit`, `outdated`, `install`
- For these commands, use **run_package_manager_command** instead

### What run_package_manager_command CAN do:
- Run built-in package manager commands: audit, outdated, install, update, prune, dedupe, why, list, info
- Security audits: Use command='audit' to check for vulnerabilities
- Dependency checks: Use command='outdated' to see outdated packages
- Package operations: install, update, prune, etc.

## Guidelines
- **Always respond in the same language as the user's message** (e.g., if user writes in Chinese, respond in Chinese; if in English, respond in English)
- Be helpful, concise, and provide actionable responses
- When you want to run a tool, clearly state which tool and parameters you'll use
- For execution tools, wait for user approval before proceeding
- CRITICAL: Only use run_script with script names from the available scripts list
- If asked to do something outside these tools, explain what's possible and suggest alternatives
- Do NOT make up commands or tools that don't exist
- Do NOT use run_script with script names that aren't in the available scripts list
- Format code examples in code blocks
"#
    );

    if let Some(ctx) = project_context {
        prompt.push_str("\n## Current Project Context\n");
        prompt.push_str(&format!("- **Project**: {}\n", ctx.project_name));
        prompt.push_str(&format!("- **Path**: {}\n", ctx.project_path));
        prompt.push_str(&format!("- **Type**: {}\n", ctx.project_type));
        prompt.push_str(&format!("- **Package Manager**: {}\n", ctx.package_manager));
        if !ctx.available_scripts.is_empty() {
            prompt.push_str(&format!(
                "- **Available Scripts** (ONLY these can be used with run_script): {}\n",
                ctx.available_scripts.join(", ")
            ));
            prompt.push_str("\nIMPORTANT: When user asks to run something, check if it's in the available scripts list above.\n");
            prompt.push_str("If not, explain that it's not a defined script and suggest alternatives.\n");
        }
        prompt.push_str("\nUse this project path when calling tools that require project_path.\n");
    }

    prompt
}

/// Cancel an active stream
#[tauri::command]
pub async fn ai_assistant_cancel_stream(
    stream_manager: State<'_, StreamManager>,
    session_id: String,
) -> Result<(), String> {
    stream_manager.cancel_session(&session_id).await
}

/// Response for stream reconnection
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamResumeResponse {
    pub stream_session_id: String,
    pub conversation_id: String,
    pub message_id: String,
    pub accumulated_content: String,
    pub status: String,
    pub model: Option<String>,
    pub is_active: bool,
}

/// Get active stream for a conversation (for reconnection after page switch)
#[tauri::command]
pub async fn ai_assistant_get_active_stream(
    stream_manager: State<'_, StreamManager>,
    conversation_id: String,
) -> Result<Option<StreamResumeResponse>, String> {
    let result = stream_manager
        .get_active_stream_for_conversation(&conversation_id)
        .await;

    match result {
        Some((session_id, info)) => Ok(Some(StreamResumeResponse {
            stream_session_id: session_id,
            conversation_id: info.conversation_id,
            message_id: info.message_id,
            accumulated_content: info.accumulated_content,
            status: info.status,
            model: info.model,
            is_active: true,
        })),
        None => Ok(None),
    }
}

/// Get messages for a conversation
#[tauri::command]
pub async fn ai_assistant_get_messages(
    db: State<'_, DatabaseState>,
    conversation_id: String,
) -> Result<Vec<Message>, String> {
    let repo = AIConversationRepository::new(db.0.as_ref().clone());

    repo.get_messages(&conversation_id)
}

// ============================================================================
// Tool Call Commands
// ============================================================================

/// Get available tools for AI
#[tauri::command]
pub async fn ai_assistant_get_tools(
    db: State<'_, DatabaseState>,
    project_path: Option<String>,
) -> Result<AvailableTools, String> {
    let handler = MCPToolHandler::with_database(db.0.as_ref().clone());
    Ok(handler.get_available_tools(project_path.as_deref()))
}

/// Approve a tool call and execute it
#[tauri::command]
pub async fn ai_assistant_approve_tool_call(
    app: AppHandle,
    db: State<'_, DatabaseState>,
    conversation_id: String,
    message_id: String,
    tool_call_id: String,
) -> Result<ToolResult, String> {
    let start_time = std::time::Instant::now();
    let repo = AIConversationRepository::new(db.0.as_ref().clone());
    let action_repo = crate::repositories::MCPActionRepository::new(db.0.as_ref().clone());
    // Use with_database for path security validation
    let handler = MCPToolHandler::with_database(db.0.as_ref().clone());

    // Feature 025: Build session context from conversation for auto-injection
    let session_context = {
        let conversation = repo.get_conversation(&conversation_id)?;
        if let Some(conv) = conversation {
            let session_builder = SessionContextBuilder::new(db.0.as_ref().clone());
            session_builder.build(None, conv.project_path.as_deref())
        } else {
            None
        }
    };

    // Get the message with tool calls
    let message = repo.get_message(&message_id)?
        .ok_or_else(|| "MESSAGE_NOT_FOUND".to_string())?;

    // Find the tool call
    let tool_calls = message.tool_calls
        .ok_or_else(|| "NO_TOOL_CALLS".to_string())?;

    let original_tool_call = tool_calls.iter()
        .find(|tc| tc.id == tool_call_id)
        .ok_or_else(|| "TOOL_CALL_NOT_FOUND".to_string())?;

    // Feature 025: Clone and inject session context into tool call arguments
    let mut tool_call = original_tool_call.clone();
    inject_session_context_to_tool_call(&mut tool_call, session_context.as_ref());

    // Validate the tool call
    handler.validate_tool_call(&tool_call)?;

    // Determine action type for history recording
    let action_type = match tool_call.name.as_str() {
        "run_workflow" | "create_workflow" | "add_workflow_step" =>
            crate::models::mcp_action::MCPActionType::Workflow,
        "run_script" | "run_npm_script" =>
            crate::models::mcp_action::MCPActionType::Script,
        "trigger_webhook" =>
            crate::models::mcp_action::MCPActionType::Webhook,
        _ => crate::models::mcp_action::MCPActionType::Script,
    };

    // Create execution record for history (T066: record all AI tool executions)
    let execution = action_repo.create_execution(
        None, // action_id - AI tools don't have pre-defined action IDs
        action_type.clone(),
        tool_call.name.clone(),
        Some("ai-assistant".to_string()), // source_client
        Some(tool_call.arguments.clone()), // parameters
        crate::models::mcp_action::ExecutionStatus::Running,
    ).ok(); // Don't fail the tool execution if history recording fails

    let execution_id = execution.as_ref().map(|e| e.id.clone());

    // Special handling for tools that need AppHandle or special context
    let result = match tool_call.name.as_str() {
        "run_workflow" => {
            execute_workflow_tool(&app, &db, &tool_call, &conversation_id).await
        }
        "trigger_webhook" => {
            execute_webhook_tool(&app, &db, &tool_call).await
        }
        _ => {
            // Execute other tools via MCPToolHandler
            handler.execute_confirmed_tool_call(&tool_call).await
        }
    };

    // Calculate duration
    let duration_ms = start_time.elapsed().as_millis() as i64;

    // Update execution record with result
    if let Some(exec_id) = execution_id {
        let status = if result.success {
            crate::models::mcp_action::ExecutionStatus::Completed
        } else {
            crate::models::mcp_action::ExecutionStatus::Failed
        };
        let result_json = Some(serde_json::json!({
            "success": result.success,
            "output_preview": result.output.chars().take(500).collect::<String>(),
            "duration_ms": duration_ms,
        }));
        let error_msg = if result.success { None } else { result.error.clone() };
        let _ = action_repo.update_execution_status(&exec_id, status, result_json, error_msg);
    }

    // Update tool call status in the message
    let updated_tool_calls: Vec<ToolCall> = tool_calls.iter()
        .map(|tc| {
            if tc.id == tool_call_id {
                ToolCall {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    arguments: tc.arguments.clone(),
                    status: if result.success {
                        ToolCallStatus::Completed
                    } else {
                        ToolCallStatus::Failed
                    },
                    thought_signature: tc.thought_signature.clone(),
                }
            } else {
                tc.clone()
            }
        })
        .collect();

    // Store tool result in message
    let tool_results = vec![result.clone()];
    let _ = repo.update_message_tool_data(
        &message_id,
        Some(&updated_tool_calls),
        Some(&tool_results),
    );

    Ok(result)
}

/// Execute run_workflow tool by calling the actual workflow execution
async fn execute_workflow_tool(
    app: &AppHandle,
    db: &State<'_, DatabaseState>,
    tool_call: &ToolCall,
    _conversation_id: &str,
) -> ToolResult {
    use crate::commands::workflow::execute_workflow_internal;

    let workflow_id = match tool_call.arguments.get("workflow_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return ToolResult::failure(
            tool_call.id.clone(),
            "Missing required parameter: workflow_id".to_string(),
        ),
    };

    // Get workflow name for better response message
    let workflow_repo = crate::repositories::WorkflowRepository::new(db.0.as_ref().clone());
    let workflow_name = workflow_repo.get(&workflow_id)
        .ok()
        .flatten()
        .map(|w| w.name.clone())
        .unwrap_or_else(|| workflow_id.clone());

    // Execute the workflow using the real execution function
    match execute_workflow_internal(
        app.clone(),
        db.0.as_ref().clone(),
        workflow_id.clone(),
        None, // parent_execution_id
        None, // parent_node_id
    ).await {
        Ok(execution_id) => {
            let output_json = serde_json::json!({
                "success": true,
                "workflow_id": workflow_id,
                "workflow_name": workflow_name,
                "execution_id": execution_id,
                "message": format!("Workflow '{}' started successfully. Execution ID: {}", workflow_name, execution_id),
                "note": "Check the Workflows panel for real-time execution status. A notification will be sent when complete."
            });
            ToolResult::success(
                tool_call.id.clone(),
                serde_json::to_string_pretty(&output_json).unwrap_or_default(),
                None,
            )
        }
        Err(e) => {
            ToolResult::failure(
                tool_call.id.clone(),
                format!("Failed to execute workflow '{}': {}", workflow_name, e),
            )
        }
    }
}

/// Execute trigger_webhook tool by calling the actual webhook execution
async fn execute_webhook_tool(
    app: &AppHandle,
    db: &State<'_, DatabaseState>,
    tool_call: &ToolCall,
) -> ToolResult {
    use crate::services::mcp_action::create_executor;
    use crate::models::mcp_action::MCPActionType;
    use crate::services::notification::{send_notification, NotificationType};

    let action_repo = crate::repositories::MCPActionRepository::new(db.0.as_ref().clone());

    // Get webhook_id (could be an action ID or workflow webhook token ID)
    let webhook_id = match tool_call.arguments.get("webhook_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return ToolResult::failure(
            tool_call.id.clone(),
            "Missing required parameter: webhook_id".to_string(),
        ),
    };

    // Get optional payload
    let payload = tool_call.arguments.get("payload").cloned();

    // Try to find as MCP action first
    match action_repo.get_action(&webhook_id) {
        Ok(Some(action)) if action.action_type == MCPActionType::Webhook => {
            // Found as MCP webhook action
            if !action.is_enabled {
                return ToolResult::failure(
                    tool_call.id.clone(),
                    format!("Webhook action '{}' is disabled", action.name),
                );
            }

            // Build execution parameters
            let mut exec_params = serde_json::json!({
                "config": action.config
            });

            if let Some(p) = payload.as_ref() {
                exec_params["payload"] = p.clone();
            }

            // Execute the webhook
            let executor = create_executor(MCPActionType::Webhook);
            let start_time = std::time::Instant::now();

            match executor.execute(exec_params).await {
                Ok(result_value) => {
                    let duration_ms = start_time.elapsed().as_millis() as u64;

                    // Send success notification
                    let url = action.config.get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let _ = send_notification(
                        app,
                        NotificationType::WebhookOutgoingSuccess {
                            workflow_name: action.name.clone(),
                            url,
                        },
                    );

                    let output_json = serde_json::json!({
                        "success": true,
                        "action_name": action.name,
                        "webhook_id": webhook_id,
                        "result": result_value,
                        "duration_ms": duration_ms,
                    });
                    ToolResult::success(
                        tool_call.id.clone(),
                        serde_json::to_string_pretty(&output_json).unwrap_or_default(),
                        Some(duration_ms as i64),
                    )
                }
                Err(e) => {
                    // Send failure notification
                    let _ = send_notification(
                        app,
                        NotificationType::WebhookOutgoingFailure {
                            workflow_name: action.name.clone(),
                            error: e.clone(),
                        },
                    );

                    ToolResult::failure(
                        tool_call.id.clone(),
                        format!("Webhook execution failed: {}", e),
                    )
                }
            }
        }
        Ok(Some(_)) => {
            ToolResult::failure(
                tool_call.id.clone(),
                format!("Action '{}' is not a webhook action", webhook_id),
            )
        }
        Ok(None) => {
            ToolResult::failure(
                tool_call.id.clone(),
                format!("Webhook action not found: {}", webhook_id),
            )
        }
        Err(e) => {
            ToolResult::failure(
                tool_call.id.clone(),
                format!("Error looking up webhook: {}", e),
            )
        }
    }
}

/// Deny a tool call
#[tauri::command]
pub async fn ai_assistant_deny_tool_call(
    db: State<'_, DatabaseState>,
    _conversation_id: String,
    message_id: String,
    tool_call_id: String,
    reason: Option<String>,
) -> Result<(), String> {
    let repo = AIConversationRepository::new(db.0.as_ref().clone());

    // Get the message with tool calls
    let message = repo.get_message(&message_id)?
        .ok_or_else(|| "MESSAGE_NOT_FOUND".to_string())?;

    // Find and update the tool call
    let tool_calls = message.tool_calls
        .ok_or_else(|| "NO_TOOL_CALLS".to_string())?;

    let updated_tool_calls: Vec<ToolCall> = tool_calls.iter()
        .map(|tc| {
            if tc.id == tool_call_id {
                ToolCall {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    arguments: tc.arguments.clone(),
                    status: ToolCallStatus::Denied,
                    thought_signature: tc.thought_signature.clone(),
                }
            } else {
                tc.clone()
            }
        })
        .collect();

    // Create a denial result
    let denial_result = ToolResult::failure(
        tool_call_id,
        reason.unwrap_or_else(|| "User denied the action".to_string()),
    );

    // Update message with denied status
    let _ = repo.update_message_tool_data(
        &message_id,
        Some(&updated_tool_calls),
        Some(&vec![denial_result]),
    );

    Ok(())
}

/// Stop an executing tool call process
/// This kills the background process associated with the tool call
#[tauri::command]
pub async fn ai_assistant_stop_tool_execution(
    tool_call_id: String,
) -> Result<(), String> {
    use crate::services::ai_assistant::PROCESS_MANAGER;

    log::info!("Stopping tool execution: {}", tool_call_id);

    // Try to stop the process
    PROCESS_MANAGER.stop_process(&tool_call_id).await?;

    log::info!("Tool execution stopped: {}", tool_call_id);
    Ok(())
}

/// Continue AI conversation after tool call approval
/// This resumes the agentic loop with the tool result in context
#[tauri::command]
pub async fn ai_assistant_continue_after_tool(
    app: AppHandle,
    db: State<'_, DatabaseState>,
    stream_manager: State<'_, StreamManager>,
    conversation_id: String,
    project_path: Option<String>,
    provider_id: Option<String>,
) -> Result<SendMessageResponse, String> {
    let conv_repo = AIConversationRepository::new(db.0.as_ref().clone());
    let ai_repo = AIRepository::new(db.0.as_ref().clone());
    let keychain = AIKeychain::new(app.clone());

    // Verify conversation exists
    let conversation = conv_repo.get_conversation(&conversation_id)?
        .ok_or_else(|| "Conversation not found".to_string())?;

    // Get AI provider
    let effective_provider_id = provider_id
        .or(conversation.provider_id.clone())
        .ok_or_else(|| "No AI provider configured for this conversation".to_string())?;

    let service = ai_repo.get_provider(&effective_provider_id)?
        .ok_or_else(|| "AI provider not found".to_string())?;

    // Get API key
    let api_key = keychain.get_api_key(&service.id)
        .map_err(|e| format!("Failed to retrieve API key: {}", e))?;

    // Create provider
    let provider = create_provider(service.clone(), api_key)
        .map_err(|e| format!("Failed to create AI provider: {}", e))?;

    // Load conversation history
    let history = conv_repo.get_messages(&conversation_id).unwrap_or_default();

    // Build project context if available
    let project_context = if let Some(ref path) = project_path {
        ProjectContextBuilder::build_from_path(path).ok()
    } else {
        None
    };

    // Build ChatMessage array from history
    let mut chat_messages: Vec<ChatMessage> = Vec::new();

    // Add system message with context
    let system_prompt = build_system_prompt(&project_context);
    chat_messages.push(ChatMessage::system(system_prompt));

    // Add conversation history
    for msg in history.iter() {
        match msg.role.to_string().as_str() {
            "user" => chat_messages.push(ChatMessage::user(&msg.content)),
            "assistant" => {
                if let Some(ref tool_calls) = msg.tool_calls {
                    let chat_tool_calls: Vec<ChatToolCall> = tool_calls.iter().map(|tc| {
                        ChatToolCall {
                            id: tc.id.clone(),
                            tool_type: "function".to_string(),
                            function: crate::models::ai::ChatFunctionCall {
                                name: tc.name.clone(),
                                arguments: tc.arguments.to_string(),
                            },
                            thought_signature: tc.thought_signature.clone(),
                        }
                    }).collect();
                    chat_messages.push(ChatMessage::assistant_with_tool_calls(
                        if msg.content.is_empty() { None } else { Some(msg.content.clone()) },
                        chat_tool_calls,
                    ));
                } else {
                    chat_messages.push(ChatMessage::assistant(&msg.content));
                }
            }
            "tool" => {
                if let Some(ref results) = msg.tool_results {
                    for result in results {
                        chat_messages.push(ChatMessage::tool_result(&result.call_id, result.output.clone()));
                    }
                }
            }
            _ => {}
        }
    }

    // Create new assistant message placeholder for continuation response
    let assistant_message = Message::assistant(conversation_id.clone(), String::new());
    conv_repo.create_message(&assistant_message)?;

    // Create streaming session
    let (stream_session_id, cancel_rx) = stream_manager.create_session().await;

    // Create stream context
    let stream_ctx = StreamContext::new(
        stream_session_id.clone(),
        conversation_id.clone(),
        assistant_message.id.clone(),
        app.clone(),
    );

    // Spawn background task for AI response
    let app_clone = app.clone();
    let assistant_message_id = assistant_message.id.clone();
    let session_id = stream_session_id.clone();
    let model_name = service.model.clone();
    let conversation_id_for_spawn = conversation_id.clone();
    let project_path_for_tools = project_path.clone();
    let db_for_tools = db.0.as_ref().clone();

    tokio::spawn(async move {
        let conversation_id = conversation_id_for_spawn;
        let mut ctx = stream_ctx;

        // Initialize tool handler
        let tool_handler = MCPToolHandler::with_database(db_for_tools);
        let tool_definitions = tool_handler.get_chat_tool_definitions(project_path_for_tools.as_deref());

        // Chat options with tools
        let options = ChatOptions {
            temperature: Some(0.7),
            max_tokens: Some(4096),
            top_p: None,
            tools: if tool_definitions.is_empty() { None } else { Some(tool_definitions) },
        };

        // Agentic loop - continue until we get a final text response
        let mut messages = chat_messages;
        let mut total_tokens: i64 = 0;
        let mut iteration = 0;
        // Track all tool results for fallback (in case AI returns empty content)
        let mut all_tool_results: Vec<(String, bool, String)> = Vec::new();
        // Track executed tool calls across iterations to prevent infinite loops
        let mut executed_tool_calls: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        const MAX_SAME_TOOL_EXECUTIONS: usize = 2;
        // Track seen content to detect repetition loops
        let mut seen_content: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        const MAX_SAME_CONTENT: usize = 2;

        loop {
            iteration += 1;
            // Sync iteration counter with StreamContext for accurate status display
            ctx.set_iteration(iteration as u32);

            if iteration > MAX_TOOL_ITERATIONS {
                log::warn!("Continuation loop reached maximum iterations ({})", MAX_TOOL_ITERATIONS);
                // Generate a message informing the user
                let warning_msg = "I've reached the maximum number of tool calls for this response. Please let me know if you'd like me to continue with a new request.";
                let _ = ctx.emit_token(warning_msg);
                break;
            }

            if cancel_rx.is_closed() {
                log::info!("Continuation stream cancelled");
                break;
            }

            // Emit status update
            let _ = ctx.emit_generating(Some(model_name.clone()));

            match provider.chat_completion(messages.clone(), options.clone()).await {
                Ok(response) => {
                    total_tokens += response.tokens_used.unwrap_or(0) as i64;

                    // Check for repeated content (AI stuck in a loop)
                    if !response.content.is_empty() {
                        let content_key = response.content.chars().take(200).collect::<String>();
                        let count = seen_content.entry(content_key.clone()).or_insert(0);
                        *count += 1;
                        if *count >= MAX_SAME_CONTENT {
                            log::warn!("Detected repeated content in continuation - AI may be stuck in a loop");
                            if ctx.get_content().is_empty() {
                                let _ = ctx.emit_token("I notice I'm repeating myself. Let me stop here.");
                            }
                            break;
                        }
                    }

                    // Check for tool calls
                    if let Some(ref tool_calls) = response.tool_calls {
                        if !tool_calls.is_empty() {
                            // Emit the assistant message content if any
                            if !response.content.is_empty() {
                                let _ = ctx.emit_token(&response.content);
                            }

                            // Filter out tool calls that have been executed too many times
                            let filtered_tool_calls: Vec<&ChatToolCall> = tool_calls.iter()
                                .filter(|tc| {
                                    let key = format!("{}:{}", tc.function.name, tc.function.arguments);
                                    let count = executed_tool_calls.get(&key).copied().unwrap_or(0);
                                    if count >= MAX_SAME_TOOL_EXECUTIONS {
                                        log::warn!(
                                            "Skipping repeated tool call '{}' (already executed {} times)",
                                            tc.function.name, count
                                        );
                                        false
                                    } else {
                                        true
                                    }
                                })
                                .collect();

                            // If all tool calls were filtered out, break the loop
                            if filtered_tool_calls.is_empty() {
                                log::warn!("All tool calls were filtered as duplicates - breaking continuation loop");
                                let warning_msg = "I notice I'm trying to repeat the same action. Let me provide you with the results I already have.";
                                let _ = ctx.emit_token(warning_msg);
                                break;
                            }

                            // Process tool calls (deduplication within this response)
                            let mut unique_tool_calls: Vec<&ChatToolCall> = Vec::new();
                            let mut seen_ids = std::collections::HashSet::new();
                            for tool_call in filtered_tool_calls.iter() {
                                if seen_ids.insert(tool_call.id.clone()) {
                                    unique_tool_calls.push(tool_call);
                                }
                            }

                            let mut execution_results: std::collections::HashMap<String, bool> = std::collections::HashMap::new();

                            for tool_call in unique_tool_calls.iter() {
                                let internal_tool_call = convert_chat_tool_call_to_internal(tool_call);

                                // Check if tool requires confirmation
                                if tool_handler.requires_confirmation(&tool_call.function.name) {
                                    // Emit tool call event and stop - wait for user approval
                                    let tool_msg = format!(
                                        "\n\nI'd like to execute **{}**. This action requires your approval.",
                                        tool_call.function.name
                                    );
                                    let _ = ctx.emit_token(&tool_msg);

                                    if let Err(e) = ctx.emit_tool_call(&internal_tool_call) {
                                        log::error!("Failed to emit tool call: {}", e);
                                    }

                                    // Store tool call in message
                                    let tool_calls_for_db = vec![internal_tool_call];
                                    let db_state = app_clone.state::<DatabaseState>();
                                    let repo = AIConversationRepository::new(db_state.0.as_ref().clone());
                                    let _ = repo.update_message_completion(
                                        &assistant_message_id,
                                        ctx.get_content(),
                                        Some(total_tokens),
                                        Some(&model_name),
                                        Some(&tool_calls_for_db),
                                    );

                                    if let Err(e) = ctx.emit_complete(total_tokens, &model_name, "tool_calls") {
                                        log::error!("Failed to emit complete: {}", e);
                                    }

                                    let stream_mgr = app_clone.state::<StreamManager>();
                                    stream_mgr.remove_session(&session_id).await;
                                    return;
                                }

                                // Auto-execute read-only tools
                                let result = tool_handler.execute_tool_call(&internal_tool_call).await;
                                execution_results.insert(tool_call.id.clone(), result.success);

                                // Track this tool call as executed (for cross-iteration deduplication)
                                let tool_key = format!("{}:{}", tool_call.function.name, tool_call.function.arguments);
                                *executed_tool_calls.entry(tool_key).or_insert(0) += 1;

                                // Add tool result to messages
                                messages.push(ChatMessage::tool_result(&tool_call.id, result.output.clone()));

                                // Track tool result for fallback
                                all_tool_results.push((
                                    tool_call.function.name.clone(),
                                    result.success,
                                    result.output.clone(),
                                ));

                                // Save tool result to database
                                {
                                    let tool_result = ToolResult {
                                        call_id: tool_call.id.clone(),
                                        success: result.success,
                                        output: result.output.clone(),
                                        error: if result.success { None } else { Some(result.output.clone()) },
                                        duration_ms: None,
                                        metadata: None,
                                    };
                                    let tool_message = Message::tool_result(
                                        conversation_id.clone(),
                                        String::new(),
                                        vec![tool_result],
                                    );
                                    let db_state = app_clone.state::<DatabaseState>();
                                    let repo = AIConversationRepository::new(db_state.0.as_ref().clone());
                                    if let Err(e) = repo.create_message(&tool_message) {
                                        log::error!("Failed to save tool result message: {}", e);
                                    }
                                }
                            }

                            // Continue the loop to get AI's response
                            continue;
                        }
                    }

                    // No tool calls or final response - emit content
                    let mut content = response.content.clone();

                    // Fallback if content is empty but we have tool results
                    if content.is_empty() && !all_tool_results.is_empty() {
                        log::info!("AI returned empty content, generating fallback summary for {} tool(s)", all_tool_results.len());
                        content = generate_tool_results_summary(&all_tool_results);
                    } else if content.is_empty() {
                        log::warn!("AI returned empty content with no tool results");
                    }

                    // Stream the content
                    let chars: Vec<char> = content.chars().collect();
                    for (i, chunk) in chars.chunks(5).enumerate() {
                        if cancel_rx.is_closed() {
                            break;
                        }
                        let token: String = chunk.iter().collect();
                        if let Err(e) = ctx.emit_token(&token) {
                            log::error!("Failed to emit token: {}", e);
                            break;
                        }
                        if i % 5 == 0 {
                            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
                        }
                    }

                    let finish_reason = response.finish_reason
                        .map(|r| format!("{:?}", r).to_lowercase())
                        .unwrap_or_else(|| "stop".to_string());

                    if let Err(e) = ctx.emit_complete(total_tokens, &model_name, &finish_reason) {
                        log::error!("Failed to emit complete: {}", e);
                    }

                    // Update message in database
                    let db_state = app_clone.state::<DatabaseState>();
                    let repo = AIConversationRepository::new(db_state.0.as_ref().clone());
                    let _ = repo.update_message_completion(
                        &assistant_message_id,
                        ctx.get_content(),
                        Some(total_tokens),
                        Some(&model_name),
                        None,
                    );

                    break;
                }
                Err(e) => {
                    log::error!("AI continuation failed: {}", e);
                    if let Err(emit_err) = ctx.emit_error("AI_PROVIDER_ERROR", &e.to_string(), true) {
                        log::error!("Failed to emit error: {}", emit_err);
                    }
                    break;
                }
            }
        }

        // Clean up session
        let stream_mgr = app_clone.state::<StreamManager>();
        stream_mgr.remove_session(&session_id).await;
    });

    Ok(SendMessageResponse {
        stream_session_id,
        conversation_id,
        message_id: assistant_message.id,
    })
}

/// Get suggestions for the current context
///
/// Quick Action Modes:
/// - Instant: Execute tool directly, display result card (zero tokens)
/// - Smart: Execute tool, then AI summarizes (moderate tokens)
/// - Ai: Full AI conversation flow (AI decides tool usage)
#[tauri::command]
pub async fn ai_assistant_get_suggestions(
    _conversation_id: Option<String>,
    project_path: Option<String>,
) -> Result<SuggestionsResponse, String> {
    use crate::models::ai_assistant::{QuickActionMode, QuickActionTool, SuggestedAction};

    let mut suggestions = Vec::new();

    // Project-specific suggestions based on actual MCP tools
    if let Some(ref path) = project_path {
        let project_path_obj = std::path::Path::new(path);

        // Git suggestions - Smart mode (AI provides analysis)
        if project_path_obj.join(".git").exists() {
            suggestions.push(SuggestedAction {
                id: "git-status".to_string(),
                label: "Git Status".to_string(),
                prompt: "Show me the git status of this project".to_string(),
                icon: Some("GitBranch".to_string()),
                variant: Some("default".to_string()),
                category: Some("git".to_string()),
                mode: QuickActionMode::Smart,
                tool: Some(QuickActionTool {
                    name: "get_git_status".to_string(),
                    args: serde_json::json!({ "project_path": path }),
                }),
                summary_hint: Some("Summarize the git status. Highlight any uncommitted changes or issues.".to_string()),
                requires_project: Some(true), // Feature 024
            });

            suggestions.push(SuggestedAction {
                id: "staged-diff".to_string(),
                label: "Staged Diff".to_string(),
                prompt: "Show me the staged changes for review".to_string(),
                icon: Some("FileDiff".to_string()),
                variant: Some("default".to_string()),
                category: Some("git".to_string()),
                mode: QuickActionMode::Smart,
                tool: Some(QuickActionTool {
                    name: "get_staged_diff".to_string(),
                    args: serde_json::json!({ "project_path": path }),
                }),
                summary_hint: Some("Review the staged changes. Provide a brief summary and any suggestions.".to_string()),
                requires_project: Some(true), // Feature 024
            });

            suggestions.push(SuggestedAction {
                id: "worktrees".to_string(),
                label: "Worktrees".to_string(),
                prompt: "List all git worktrees".to_string(),
                icon: Some("GitFork".to_string()),
                variant: Some("default".to_string()),
                category: Some("git".to_string()),
                mode: QuickActionMode::Instant,
                tool: Some(QuickActionTool {
                    name: "list_worktrees".to_string(),
                    args: serde_json::json!({ "project_path": path }),
                }),
                summary_hint: None,
                requires_project: Some(true), // Feature 024
            });
        }

        // Node.js project suggestions
        if project_path_obj.join("package.json").exists() {
            // Scripts - Instant (just list them)
            suggestions.push(SuggestedAction {
                id: "scripts".to_string(),
                label: "Scripts".to_string(),
                prompt: "Show available npm scripts".to_string(),
                icon: Some("Terminal".to_string()),
                variant: Some("default".to_string()),
                category: Some("project".to_string()),
                mode: QuickActionMode::Instant,
                tool: Some(QuickActionTool {
                    name: "list_project_scripts".to_string(),
                    args: serde_json::json!({ "project_path": path }),
                }),
                summary_hint: None,
                requires_project: Some(true), // Feature 024
            });

            // Run scripts - AI mode (needs confirmation and status reporting)
            suggestions.push(SuggestedAction {
                id: "run-dev".to_string(),
                label: "npm dev".to_string(),
                prompt: "Run the dev script for this project".to_string(),
                icon: Some("Play".to_string()),
                variant: Some("primary".to_string()),
                category: Some("project".to_string()),
                mode: QuickActionMode::Ai,
                tool: None,
                summary_hint: None,
                requires_project: Some(true), // Feature 024
            });

            suggestions.push(SuggestedAction {
                id: "run-build".to_string(),
                label: "npm build".to_string(),
                prompt: "Run the build script for this project".to_string(),
                icon: Some("Hammer".to_string()),
                variant: Some("default".to_string()),
                category: Some("project".to_string()),
                mode: QuickActionMode::Ai,
                tool: None,
                summary_hint: None,
                requires_project: Some(true), // Feature 024
            });

            suggestions.push(SuggestedAction {
                id: "run-test".to_string(),
                label: "npm test".to_string(),
                prompt: "Run the test script for this project".to_string(),
                icon: Some("TestTube".to_string()),
                variant: Some("default".to_string()),
                category: Some("project".to_string()),
                mode: QuickActionMode::Ai,
                tool: None,
                summary_hint: None,
                requires_project: Some(true), // Feature 024
            });

            // Security - Smart mode (AI analyzes vulnerabilities)
            suggestions.push(SuggestedAction {
                id: "security-scan".to_string(),
                label: "Security Scan".to_string(),
                prompt: "Run a security scan on this project".to_string(),
                icon: Some("Shield".to_string()),
                variant: Some("warning".to_string()),
                category: Some("security".to_string()),
                mode: QuickActionMode::Smart,
                tool: Some(QuickActionTool {
                    name: "run_security_scan".to_string(),
                    args: serde_json::json!({ "project_path": path }),
                }),
                summary_hint: Some("Analyze the security scan results. List vulnerabilities by severity and provide remediation suggestions.".to_string()),
                requires_project: Some(true), // Feature 024
            });

            suggestions.push(SuggestedAction {
                id: "view-vulnerabilities".to_string(),
                label: "Vulnerabilities".to_string(),
                prompt: "Show security vulnerabilities".to_string(),
                icon: Some("AlertTriangle".to_string()),
                variant: Some("default".to_string()),
                category: Some("security".to_string()),
                mode: QuickActionMode::Smart,
                tool: Some(QuickActionTool {
                    name: "get_security_scan_results".to_string(),
                    args: serde_json::json!({ "project_path": path }),
                }),
                summary_hint: Some("Summarize the vulnerabilities found. Group by severity and suggest fixes.".to_string()),
                requires_project: Some(true), // Feature 024
            });

            // Dependencies - Instant (just display the list)
            suggestions.push(SuggestedAction {
                id: "dependencies".to_string(),
                label: "Dependencies".to_string(),
                prompt: "Show project dependencies".to_string(),
                icon: Some("Package".to_string()),
                variant: Some("default".to_string()),
                category: Some("project".to_string()),
                mode: QuickActionMode::Instant,
                tool: Some(QuickActionTool {
                    name: "get_project_dependencies".to_string(),
                    args: serde_json::json!({ "project_path": path }),
                }),
                summary_hint: None,
                requires_project: Some(true), // Feature 024
            });

            // Time Machine actions - Only show if project has a lockfile
            let has_lockfile = project_path_obj.join("pnpm-lock.yaml").exists()
                || project_path_obj.join("package-lock.json").exists()
                || project_path_obj.join("yarn.lock").exists()
                || project_path_obj.join("bun.lockb").exists();

            if has_lockfile {
                // Time Machine - Capture snapshot
                suggestions.push(SuggestedAction {
                    id: "capture-snapshot".to_string(),
                    label: "Capture Snapshot".to_string(),
                    prompt: "Capture a snapshot of the current dependency state".to_string(),
                    icon: Some("Camera".to_string()),
                    variant: Some("default".to_string()),
                    category: Some("project".to_string()),
                    mode: QuickActionMode::Instant,
                    tool: Some(QuickActionTool {
                        name: "capture_snapshot".to_string(),
                        args: serde_json::json!({ "project_path": path }),
                    }),
                    summary_hint: None,
                    requires_project: Some(true),
                });

                // Time Machine - View snapshots
                suggestions.push(SuggestedAction {
                    id: "view-snapshots".to_string(),
                    label: "View Snapshots".to_string(),
                    prompt: "Show Time Machine snapshots for this project".to_string(),
                    icon: Some("History".to_string()),
                    variant: Some("default".to_string()),
                    category: Some("project".to_string()),
                    mode: QuickActionMode::Smart,
                    tool: Some(QuickActionTool {
                        name: "list_snapshots".to_string(),
                        args: serde_json::json!({ "project_path": path }),
                    }),
                    summary_hint: Some("Summarize the snapshot history. Highlight any significant dependency changes.".to_string()),
                    requires_project: Some(true),
                });

                // Time Machine - Check integrity
                suggestions.push(SuggestedAction {
                    id: "check-integrity".to_string(),
                    label: "Check Integrity".to_string(),
                    prompt: "Check dependency integrity against the latest snapshot".to_string(),
                    icon: Some("ShieldCheck".to_string()),
                    variant: Some("default".to_string()),
                    category: Some("security".to_string()),
                    mode: QuickActionMode::Smart,
                    tool: Some(QuickActionTool {
                        name: "check_dependency_integrity".to_string(),
                        args: serde_json::json!({ "project_path": path }),
                    }),
                    summary_hint: Some("Analyze the integrity check results. Report any mismatches or potential security concerns.".to_string()),
                    requires_project: Some(true),
                });
            }

            // File search - AI mode (needs user input)
            suggestions.push(SuggestedAction {
                id: "search-files".to_string(),
                label: "Search Files".to_string(),
                prompt: "Search for files in this project. What pattern should I search for?".to_string(),
                icon: Some("Search".to_string()),
                variant: Some("default".to_string()),
                category: Some("project".to_string()),
                mode: QuickActionMode::Ai,
                tool: None,
                summary_hint: None,
                requires_project: Some(true), // Feature 024
            });
        }
    }

    // Process management - Instant (pure status query) - Global actions
    suggestions.push(SuggestedAction {
        id: "list-processes".to_string(),
        label: "Processes".to_string(),
        prompt: "Show running background processes".to_string(),
        icon: Some("Activity".to_string()),
        variant: Some("default".to_string()),
        category: Some("process".to_string()),
        mode: QuickActionMode::Instant,
        tool: Some(QuickActionTool {
            name: "list_background_processes".to_string(),
            args: serde_json::json!({}),
        }),
        summary_hint: None,
        requires_project: Some(false), // Feature 024: Global action
    });

    suggestions.push(SuggestedAction {
        id: "stop-process".to_string(),
        label: "Stop Process".to_string(),
        prompt: "Show me the running background processes so I can stop one".to_string(),
        icon: Some("StopCircle".to_string()),
        variant: Some("warning".to_string()),
        category: Some("process".to_string()),
        mode: QuickActionMode::Ai,
        tool: None,
        summary_hint: None,
        requires_project: Some(false), // Feature 024: Global action
    });

    // System - Instant (pure status query) - Global actions
    suggestions.push(SuggestedAction {
        id: "environment".to_string(),
        label: "Environment".to_string(),
        prompt: "Show environment info".to_string(),
        icon: Some("Settings".to_string()),
        variant: Some("default".to_string()),
        category: Some("system".to_string()),
        mode: QuickActionMode::Instant,
        tool: Some(QuickActionTool {
            name: "get_environment_info".to_string(),
            args: serde_json::json!({}),
        }),
        summary_hint: None,
        requires_project: Some(false), // Feature 024: Global action
    });

    suggestions.push(SuggestedAction {
        id: "notifications".to_string(),
        label: "Notifications".to_string(),
        prompt: "Show notifications".to_string(),
        icon: Some("Bell".to_string()),
        variant: Some("default".to_string()),
        category: Some("system".to_string()),
        mode: QuickActionMode::Instant,
        tool: Some(QuickActionTool {
            name: "get_notifications".to_string(),
            args: serde_json::json!({}),
        }),
        summary_hint: None,
        requires_project: Some(false), // Feature 024: Global action
    });

    // Default suggestions when no project context
    let has_project_suggestions = suggestions
        .iter()
        .any(|s| s.category.as_deref() == Some("project") || s.category.as_deref() == Some("git"));

    if !has_project_suggestions {
        suggestions.insert(
            0,
            SuggestedAction {
                id: "list-projects".to_string(),
                label: "Projects".to_string(),
                prompt: "List all registered projects".to_string(),
                icon: Some("FolderOpen".to_string()),
                variant: Some("primary".to_string()),
                category: Some("project".to_string()),
                mode: QuickActionMode::Instant,
                tool: Some(QuickActionTool {
                    name: "list_projects".to_string(),
                    args: serde_json::json!({}),
                }),
                summary_hint: None,
                requires_project: Some(false), // Feature 024: Global action
            },
        );

        suggestions.insert(
            1,
            SuggestedAction {
                id: "list-workflows".to_string(),
                label: "Workflows".to_string(),
                prompt: "List all available workflows".to_string(),
                icon: Some("Workflow".to_string()),
                variant: Some("default".to_string()),
                category: Some("workflow".to_string()),
                mode: QuickActionMode::Instant,
                tool: Some(QuickActionTool {
                    name: "list_workflows".to_string(),
                    args: serde_json::json!({}),
                }),
                summary_hint: None,
                requires_project: Some(false), // Feature 024: Global action
            },
        );

        suggestions.insert(
            2,
            SuggestedAction {
                id: "list-actions".to_string(),
                label: "Actions".to_string(),
                prompt: "List all MCP actions".to_string(),
                icon: Some("Zap".to_string()),
                variant: Some("default".to_string()),
                category: Some("workflow".to_string()),
                mode: QuickActionMode::Instant,
                tool: Some(QuickActionTool {
                    name: "list_actions".to_string(),
                    args: serde_json::json!({}),
                }),
                summary_hint: None,
                requires_project: Some(false), // Feature 024: Global action
            },
        );
    }

    Ok(SuggestionsResponse { suggestions })
}

/// Execute a tool directly without AI interpretation (for Lazy Actions)
/// This is a simplified execution path that doesn't require a conversation context
#[tauri::command]
pub async fn ai_assistant_execute_tool_direct(
    app: AppHandle,
    db: State<'_, DatabaseState>,
    tool_name: String,
    tool_args: serde_json::Value,
) -> Result<ToolResult, String> {
    use crate::models::ai_assistant::{ToolCall, ToolCallStatus};
    use crate::commands::script::{ScriptExecutionState, ExecutionStatus};

    log::info!("[AI Tool Direct] Executing tool: {} with args: {:?}", tool_name, tool_args);

    // Special handling for list_background_processes - directly access ScriptExecutionState
    if tool_name == "list_background_processes" {
        log::info!("[AI Tool Direct] Special handling for list_background_processes");
        let state = app.state::<ScriptExecutionState>();
        let executions = state.executions.read().await;

        let processes: Vec<serde_json::Value> = executions
            .values()
            .map(|exec| {
                serde_json::json!({
                    "execution_id": exec.execution_id,
                    "script_name": exec.script_name,
                    "project_path": exec.project_path,
                    "project_name": exec.project_name,
                    "started_at": exec.started_at_iso,
                    "status": match exec.status {
                        ExecutionStatus::Running => "running",
                        ExecutionStatus::Completed => "completed",
                        ExecutionStatus::Failed => "failed",
                        ExecutionStatus::Cancelled => "cancelled",
                    },
                    "exit_code": exec.exit_code,
                    "completed_at": exec.completed_at,
                    "elapsed_ms": exec.started_at.elapsed().as_millis() as u64,
                })
            })
            .collect();

        let result = serde_json::json!({
            "message": if processes.is_empty() {
                "No background processes are currently running"
            } else {
                "Found running processes"
            },
            "count": processes.len(),
            "processes": processes
        });

        return Ok(ToolResult {
            call_id: format!("direct_{}", uuid::Uuid::new_v4()),
            success: true,
            output: serde_json::to_string_pretty(&result).unwrap_or_default(),
            error: None,
            duration_ms: None,
            metadata: None,
        });
    }

    // Special handling for get_environment_info - use get_environment_diagnostics
    if tool_name == "get_environment_info" {
        log::info!("[AI Tool Direct] Special handling for get_environment_info");
        match crate::commands::toolchain::get_environment_diagnostics(None).await {
            Ok(diagnostics) => {
                let result = serde_json::json!({
                    "volta": {
                        "available": diagnostics.volta.available,
                        "version": diagnostics.volta.version,
                        "path": diagnostics.volta.path,
                        "shim_path": diagnostics.volta.shim_path,
                    },
                    "corepack": {
                        "available": diagnostics.corepack.available,
                        "enabled": diagnostics.corepack.enabled,
                        "version": diagnostics.corepack.version,
                    },
                    "system_node": {
                        "version": diagnostics.system_node.version,
                        "path": diagnostics.system_node.path,
                    },
                    "package_managers": {
                        "npm": diagnostics.package_managers.npm,
                        "pnpm": diagnostics.package_managers.pnpm,
                        "yarn": diagnostics.package_managers.yarn,
                    },
                    "path_analysis": {
                        "volta_first": diagnostics.path_analysis.volta_first,
                        "corepack_first": diagnostics.path_analysis.corepack_first,
                    }
                });

                return Ok(ToolResult {
                    call_id: format!("direct_{}", uuid::Uuid::new_v4()),
                    success: true,
                    output: serde_json::to_string_pretty(&result).unwrap_or_default(),
                    error: None,
                    duration_ms: None,
                    metadata: None,
                });
            }
            Err(e) => {
                return Ok(ToolResult {
                    call_id: format!("direct_{}", uuid::Uuid::new_v4()),
                    success: false,
                    output: String::new(),
                    error: Some(e),
                    duration_ms: None,
                    metadata: None,
                });
            }
        }
    }

    // Special handling for get_notifications - return empty for now
    if tool_name == "get_notifications" {
        log::info!("[AI Tool Direct] Special handling for get_notifications");
        let result = serde_json::json!({
            "message": "No pending notifications",
            "notifications": []
        });

        return Ok(ToolResult {
            call_id: format!("direct_{}", uuid::Uuid::new_v4()),
            success: true,
            output: serde_json::to_string_pretty(&result).unwrap_or_default(),
            error: None,
            duration_ms: None,
            metadata: None,
        });
    }

    // Special handling for security tools - require project context
    if tool_name == "run_security_scan" || tool_name == "get_security_scan_results" {
        log::info!("[AI Tool Direct] Special handling for security tool: {}", tool_name);
        let result = serde_json::json!({
            "message": "Security scanning requires a project context. Please select a project first or use the Security panel in the Projects tab.",
            "hint": "Navigate to Projects tab > select a project > Security panel to run security scans."
        });

        return Ok(ToolResult {
            call_id: format!("direct_{}", uuid::Uuid::new_v4()),
            success: true,
            output: serde_json::to_string_pretty(&result).unwrap_or_default(),
            error: None,
            duration_ms: None,
            metadata: None,
        });
    }

    // Special handling for get_project_dependencies - require project context
    if tool_name == "get_project_dependencies" {
        log::info!("[AI Tool Direct] Special handling for get_project_dependencies");
        let result = serde_json::json!({
            "message": "Dependency listing requires a project context. Please select a project first.",
            "hint": "Navigate to Projects tab > select a project to view its dependencies."
        });

        return Ok(ToolResult {
            call_id: format!("direct_{}", uuid::Uuid::new_v4()),
            success: true,
            output: serde_json::to_string_pretty(&result).unwrap_or_default(),
            error: None,
            duration_ms: None,
            metadata: None,
        });
    }

    // Create tool handler
    let handler = MCPToolHandler::with_database(db.0.as_ref().clone());

    // Create a ToolCall structure
    let tool_call = ToolCall {
        id: format!("direct_{}", uuid::Uuid::new_v4()),
        name: tool_name.clone(),
        arguments: tool_args,
        status: ToolCallStatus::Pending,
        thought_signature: None,
    };

    // Validate the tool exists
    let available_tools = handler.get_available_tools(None);
    let tool_def = available_tools
        .tools
        .iter()
        .find(|t| t.name == tool_name)
        .ok_or_else(|| format!("Unknown tool: {}", tool_name))?;

    // Check if tool requires confirmation - Lazy Actions should not require confirmation
    if tool_def.requires_confirmation {
        return Err(format!(
            "Tool '{}' requires confirmation and cannot be executed directly. Use the chat interface instead.",
            tool_name
        ));
    }

    // Execute the tool
    let result = handler.execute_tool_call(&tool_call).await;

    Ok(result)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert ChatToolCall (from AI response) to internal ToolCall format
fn convert_chat_tool_call_to_internal(chat_tool_call: &ChatToolCall) -> ToolCall {
    // Parse the arguments JSON string
    let arguments: serde_json::Value = serde_json::from_str(&chat_tool_call.function.arguments)
        .unwrap_or(serde_json::json!({}));

    // Preserve the original ID from the AI response
    ToolCall {
        id: chat_tool_call.id.clone(),
        name: chat_tool_call.function.name.clone(),
        arguments,
        status: ToolCallStatus::Pending,
        thought_signature: chat_tool_call.thought_signature.clone(),
    }
}

/// Special marker for global workflows (not bound to any project)
const GLOBAL_WORKFLOW_MARKER: &str = "__GLOBAL__";

/// Inject session context into tool call arguments
/// Feature 025: Auto-fill project_id and cwd from session context when not provided
///
/// This ensures:
/// - create_workflow/create_workflow_with_steps automatically bind to current project
/// - Steps cwd defaults to project_path when not specified
/// - Use project_id="__GLOBAL__" to explicitly create a global workflow
fn inject_session_context_to_tool_call(
    tool_call: &mut ToolCall,
    session_context: Option<&SessionContext>,
) {
    let session_ctx = match session_context {
        Some(ctx) if ctx.has_project() => ctx,
        _ => return, // No session context or no project bound
    };

    let tool_name = tool_call.name.as_str();

    // Inject project_id for workflow creation tools
    if matches!(tool_name, "create_workflow" | "create_workflow_with_steps") {
        let existing_project_id = tool_call.arguments
            .get("project_id")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string());

        match existing_project_id.as_deref() {
            // If __GLOBAL__ is specified, convert to empty string (no project binding)
            Some(GLOBAL_WORKFLOW_MARKER) => {
                if let Some(obj) = tool_call.arguments.as_object_mut() {
                    obj.insert("project_id".to_string(), serde_json::json!(""));
                    log::info!(
                        "[Session Context] User requested global workflow (no project binding) for {}",
                        tool_name
                    );
                }
            }
            // If project_id is already set (non-empty, non-global), keep it
            Some(pid) if !pid.is_empty() => {
                log::info!(
                    "[Session Context] Using provided project_id '{}' for {}",
                    pid,
                    tool_name
                );
            }
            // If project_id is empty or not provided, inject from session
            _ => {
                if let Some(ref project_id) = session_ctx.project_id {
                    if let Some(obj) = tool_call.arguments.as_object_mut() {
                        obj.insert("project_id".to_string(), serde_json::json!(project_id));
                        log::info!(
                            "[Session Context] Auto-injected project_id '{}' into {}",
                            project_id,
                            tool_name
                        );
                    }
                }
            }
        }
    }

    // Inject cwd for step creation tools (single step)
    if matches!(tool_name, "add_workflow_step") {
        let has_cwd = tool_call.arguments
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

        if !has_cwd {
            if let Some(ref project_path) = session_ctx.project_path {
                if let Some(obj) = tool_call.arguments.as_object_mut() {
                    obj.insert("cwd".to_string(), serde_json::json!(project_path));
                    log::info!(
                        "[Session Context] Auto-injected cwd '{}' into {}",
                        project_path,
                        tool_name
                    );
                }
            }
        }
    }

    // Inject cwd for batch step creation tools
    if matches!(tool_name, "add_workflow_steps" | "create_workflow_with_steps") {
        if let Some(ref project_path) = session_ctx.project_path {
            if let Some(steps) = tool_call.arguments.get_mut("steps").and_then(|v| v.as_array_mut()) {
                for step in steps.iter_mut() {
                    let has_cwd = step
                        .get("cwd")
                        .and_then(|v| v.as_str())
                        .map(|s| !s.trim().is_empty())
                        .unwrap_or(false);

                    if !has_cwd {
                        if let Some(obj) = step.as_object_mut() {
                            obj.insert("cwd".to_string(), serde_json::json!(project_path));
                        }
                    }
                }
                log::info!(
                    "[Session Context] Auto-injected cwd '{}' into {} steps",
                    project_path,
                    tool_name
                );
            }
        }
    }
}

/// Generate a human-readable summary of tool execution results
/// Used as fallback when AI returns empty content after tool execution
fn generate_tool_results_summary(results: &[(String, bool, String)]) -> String {
    let mut summary = String::from("Here are the results from the executed tools:\n\n");

    for (name, success, output) in results {
        if *success {
            // Try to parse as JSON for better formatting
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
                // Format JSON nicely
                let formatted = serde_json::to_string_pretty(&json)
                    .unwrap_or_else(|_| output.clone());
                summary.push_str(&format!("**{}** completed successfully:\n```json\n{}\n```\n\n", name, formatted));
            } else {
                summary.push_str(&format!("**{}** completed successfully:\n```\n{}\n```\n\n", name, output));
            }
        } else {
            summary.push_str(&format!("**{}** failed:\n```\n{}\n```\n\n", name, output));
        }
    }

    summary.push_str("Is there anything specific you'd like me to help you with based on these results?");
    summary
}

// ============================================================================
// Interactive Element Commands (Feature 023 - US3)
// ============================================================================

use crate::models::ai_assistant::{InteractiveElement, LazyAction, LazyActionType};
use crate::services::ai_assistant::{parse_interactive_elements, get_clean_content};

/// Parse interactive elements from AI response content (T066)
/// Returns the list of interactive elements found in the content
#[tauri::command]
pub fn ai_assistant_parse_interactive(content: String) -> ParseInteractiveResponse {
    let elements = parse_interactive_elements(&content);
    let clean_content = get_clean_content(&content);

    ParseInteractiveResponse {
        elements,
        clean_content,
    }
}

/// Response type for parse_interactive command
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseInteractiveResponse {
    /// Parsed interactive elements
    pub elements: Vec<InteractiveElement>,
    /// Content with markers stripped (labels only)
    pub clean_content: String,
}

/// Execute a lazy action (T067)
/// Handles navigation, tool execution, and clipboard copy actions
#[tauri::command]
pub async fn ai_assistant_execute_lazy_action(
    app: AppHandle,
    action: LazyAction,
) -> Result<LazyActionResult, String> {
    match action.action_type {
        LazyActionType::Navigate => {
            // Navigation is handled by frontend via emit
            app.emit("ai:navigate", &action.payload)
                .map_err(|e| format!("Failed to emit navigation: {}", e))?;

            Ok(LazyActionResult {
                success: true,
                message: Some(format!("Navigating to {}", action.payload)),
                data: None,
            })
        }
        LazyActionType::ExecuteTool => {
            // Tool execution requires confirmation - emit event for frontend to handle
            app.emit("ai:execute-tool-request", &action.payload)
                .map_err(|e| format!("Failed to emit tool request: {}", e))?;

            Ok(LazyActionResult {
                success: true,
                message: Some("Tool execution requested".to_string()),
                data: Some(action.payload),
            })
        }
        LazyActionType::Copy => {
            // Copy to clipboard - handled by frontend
            app.emit("ai:copy-to-clipboard", &action.payload)
                .map_err(|e| format!("Failed to emit copy request: {}", e))?;

            Ok(LazyActionResult {
                success: true,
                message: Some("Copied to clipboard".to_string()),
                data: None,
            })
        }
    }
}

/// Result type for lazy action execution
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LazyActionResult {
    /// Whether the action was successful
    pub success: bool,
    /// Optional message
    pub message: Option<String>,
    /// Optional data returned by the action
    pub data: Option<String>,
}

// ============================================================================
// Autocomplete Commands (Feature 023 - US5)
// ============================================================================

/// Autocomplete suggestion
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutocompleteSuggestion {
    /// Suggested text to insert
    pub text: String,
    /// Source of the suggestion
    pub source: AutocompleteSource,
    /// Display label (may be truncated)
    pub label: String,
    /// Optional icon hint
    pub icon: Option<String>,
}

/// Source of autocomplete suggestion
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutocompleteSource {
    /// From recent prompts
    RecentPrompt,
    /// From tool description
    ToolDescription,
    /// From conversation context
    Context,
}

/// Get autocomplete suggestions for input (T109-T112)
#[tauri::command]
pub async fn ai_assistant_get_autocomplete(
    db: State<'_, DatabaseState>,
    conversation_id: String,
    input: String,
    limit: Option<usize>,
) -> Result<Vec<AutocompleteSuggestion>, String> {
    let limit = limit.unwrap_or(5);
    let input_lower = input.to_lowercase();

    if input_lower.len() < 2 {
        return Ok(vec![]);
    }

    let mut suggestions: Vec<AutocompleteSuggestion> = Vec::new();

    // T110: Recent prompts matching
    let repo = AIConversationRepository::new(db.0.as_ref().clone());
    if let Ok(messages) = repo.get_messages(&conversation_id) {
        for msg in messages.iter().rev() {
            if msg.role == crate::models::ai_assistant::MessageRole::User {
                let content_lower = msg.content.to_lowercase();
                if content_lower.contains(&input_lower) && msg.content != input {
                    suggestions.push(AutocompleteSuggestion {
                        text: msg.content.clone(),
                        source: AutocompleteSource::RecentPrompt,
                        label: if msg.content.len() > 50 {
                            // UTF-8 safe truncation: find valid character boundary
                            let truncate_at = msg.content
                                .char_indices()
                                .take_while(|(i, _)| *i < 50)
                                .last()
                                .map(|(i, c)| i + c.len_utf8())
                                .unwrap_or(msg.content.len().min(50));
                            format!("{}...", &msg.content[..truncate_at])
                        } else {
                            msg.content.clone()
                        },
                        icon: Some("clock".to_string()),
                    });

                    if suggestions.len() >= limit {
                        break;
                    }
                }
            }
        }
    }

    // T111: Tool description matching
    let tool_suggestions = get_tool_suggestions(&input_lower);
    for suggestion in tool_suggestions {
        if suggestions.len() >= limit {
            break;
        }
        suggestions.push(suggestion);
    }

    // T112: Context-based suggestions
    if suggestions.len() < limit {
        let context_suggestions = get_context_suggestions(&input_lower);
        for suggestion in context_suggestions {
            if suggestions.len() >= limit {
                break;
            }
            suggestions.push(suggestion);
        }
    }

    Ok(suggestions)
}

/// Get tool-based autocomplete suggestions (T111)
fn get_tool_suggestions(input: &str) -> Vec<AutocompleteSuggestion> {
    let mut suggestions = Vec::new();

    // Common tool-related prompts
    let tool_prompts = vec![
        ("list", "List all projects using list_projects", "folder"),
        ("show", "Show git status using get_worktree_status", "git-branch"),
        ("run", "Run npm script using run_npm_script", "play"),
        ("workflow", "List workflows using list_workflows", "workflow"),
        ("create", "Create a new workflow using create_workflow", "plus"),
        ("status", "Get git status using get_worktree_status", "git-branch"),
    ];

    for (keyword, prompt, icon) in tool_prompts {
        if keyword.contains(input) || input.contains(keyword) {
            suggestions.push(AutocompleteSuggestion {
                text: prompt.to_string(),
                source: AutocompleteSource::ToolDescription,
                label: prompt.to_string(),
                icon: Some(icon.to_string()),
            });
        }
    }

    suggestions
}

/// Get context-based autocomplete suggestions (T112)
fn get_context_suggestions(input: &str) -> Vec<AutocompleteSuggestion> {
    let mut suggestions = Vec::new();

    // Common follow-up patterns
    let follow_ups = vec![
        ("again", "Run the same command again"),
        ("more", "Show more details"),
        ("explain", "Explain the last result"),
        ("fix", "Fix the issue mentioned above"),
        ("continue", "Continue with the previous task"),
    ];

    for (keyword, suggestion) in follow_ups {
        if keyword.contains(input) || input.contains(keyword) {
            suggestions.push(AutocompleteSuggestion {
                text: suggestion.to_string(),
                source: AutocompleteSource::Context,
                label: suggestion.to_string(),
                icon: Some("message-circle".to_string()),
            });
        }
    }

    suggestions
}

/// Context summary result
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSummaryResult {
    /// Summary text
    pub summary: String,
    /// Key entities extracted
    pub key_entities: Vec<String>,
    /// Recent tool calls
    pub recent_tool_calls: Vec<crate::services::ai_assistant::ToolCallSummary>,
    /// Number of messages summarized
    pub messages_summarized: usize,
}

/// Summarize conversation context (T114)
#[tauri::command]
pub async fn ai_assistant_summarize_context(
    db: State<'_, DatabaseState>,
    conversation_id: String,
) -> Result<ContextSummaryResult, String> {
    use crate::services::ai_assistant::{ContextManager, ContextConfig};

    let repo = AIConversationRepository::new(db.0.as_ref().clone());
    let messages = repo.get_messages(&conversation_id)?;

    let config = ContextConfig {
        max_messages: 50,
        recent_to_keep: 10,
        summarization_threshold: 15,
    };
    let manager = ContextManager::new(config);

    let prepared = manager.prepare_context(&messages);
    let summary = if prepared.was_summarized {
        prepared.system_context.unwrap_or_default()
    } else {
        manager.summarize_messages(&messages)
    };

    Ok(ContextSummaryResult {
        summary,
        key_entities: prepared.key_entities,
        recent_tool_calls: prepared.recent_tool_calls,
        messages_summarized: messages.len(),
    })
}

// ============================================================================
// Background Process Commands
// Feature: AI Assistant Background Script Execution
// ============================================================================

use crate::services::ai_assistant::background_process::{
    BACKGROUND_PROCESS_MANAGER, BackgroundProcessInfo,
};

/// Request to spawn a background process
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpawnBackgroundProcessRequest {
    /// Display name for the process
    pub name: String,
    /// Command to execute
    pub command: String,
    /// Command arguments
    pub args: Vec<String>,
    /// Working directory
    pub cwd: String,
    /// Project path for context
    pub project_path: String,
    /// Project name for display (reserved for future use)
    #[allow(dead_code)]
    pub project_name: Option<String>,
    /// Associated conversation ID
    pub conversation_id: Option<String>,
    /// Associated message ID
    pub message_id: Option<String>,
    /// Environment variables to add (reserved for future use)
    #[allow(dead_code)]
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}

/// Response for spawn background process
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpawnBackgroundProcessResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for stop background process
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StopBackgroundProcessResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Spawn a background process for AI Assistant
/// This allows running long-running scripts (like dev servers) without blocking the conversation
#[tauri::command]
pub async fn ai_assistant_spawn_background_process(
    app: AppHandle,
    request: SpawnBackgroundProcessRequest,
) -> Result<SpawnBackgroundProcessResponse, String> {
    log::info!(
        "[AI Background] Spawning process: {} {} in {}",
        request.command,
        request.args.join(" "),
        request.cwd
    );

    // Ensure app handle is set for event emission
    BACKGROUND_PROCESS_MANAGER.set_app_handle(app).await;

    // Start the background process
    match BACKGROUND_PROCESS_MANAGER
        .start_process(
            request.name,
            request.command,
            request.args,
            request.cwd,
            request.project_path,
            None, // success_pattern - not used from frontend currently
            None, // success_timeout_ms
            request.conversation_id,
            request.message_id, // Using message_id as tool_call_id
            None, // log_entry_id - spawned from frontend, no AI Activity log needed
        )
        .await
    {
        Ok(info) => {
            log::info!("[AI Background] Process spawned with ID: {}", info.id);
            Ok(SpawnBackgroundProcessResponse {
                success: true,
                process_id: Some(info.id),
                error: None,
            })
        }
        Err(e) => {
            log::error!("[AI Background] Failed to spawn process: {}", e);
            Ok(SpawnBackgroundProcessResponse {
                success: false,
                process_id: None,
                error: Some(e),
            })
        }
    }
}

/// Stop a background process
#[tauri::command]
pub async fn ai_assistant_stop_background_process(
    process_id: String,
) -> Result<StopBackgroundProcessResponse, String> {
    log::info!("[AI Background] Stopping process: {}", process_id);

    match BACKGROUND_PROCESS_MANAGER.stop_process(&process_id, false).await {
        Ok(()) => {
            log::info!("[AI Background] Process {} stopped", process_id);
            Ok(StopBackgroundProcessResponse {
                success: true,
                error: None,
            })
        }
        Err(e) => {
            log::error!("[AI Background] Failed to stop process {}: {}", process_id, e);
            Ok(StopBackgroundProcessResponse {
                success: false,
                error: Some(e),
            })
        }
    }
}

/// List all background processes
#[tauri::command]
pub async fn ai_assistant_list_background_processes() -> Result<Vec<BackgroundProcessInfo>, String> {
    Ok(BACKGROUND_PROCESS_MANAGER.list_processes().await)
}

/// Get a specific background process by ID
#[tauri::command]
pub async fn ai_assistant_get_background_process(
    process_id: String,
) -> Result<Option<BackgroundProcessInfo>, String> {
    match BACKGROUND_PROCESS_MANAGER.get_process(&process_id).await {
        Ok(info) => Ok(Some(info)),
        Err(_) => Ok(None),
    }
}
