// System Prompt Builder for AI Assistant
// Feature: Enhanced AI Chat Experience (023-enhanced-ai-chat)
// Enhancement: AI Precision Improvement (025-ai-workflow-generator)
//
// Constructs structured system prompts that:
// - Define AI's role and capabilities clearly
// - Provide tool usage instructions with examples
// - Include SpecForge feature descriptions
// - Add constraints for off-topic handling
// - Support project-specific context
// - Include session context for precise project/workflow targeting (025)
// - Track created/modified resources during conversation (025)

use crate::models::ai_assistant::{ProjectContext, SessionContext, SessionCreatedResources, ToolDefinition};

/// Builder for constructing structured system prompts
pub struct SystemPromptBuilder {
    /// Role and identity section
    role_section: String,
    /// Capabilities section
    capabilities_section: String,
    /// Tool instructions section
    tool_instructions: String,
    /// Example patterns for tool usage
    examples: Vec<String>,
    /// Constraints and rules
    constraints: Vec<String>,
    /// Optional project context (legacy)
    context: Option<ProjectContext>,
    /// Available tools
    tools: Vec<ToolDefinition>,
    /// Session context for precise targeting (Feature 025)
    session_context: Option<SessionContext>,
    /// Resources created during conversation (Feature 025)
    created_resources: Option<SessionCreatedResources>,
}

impl SystemPromptBuilder {
    /// Create a new SystemPromptBuilder with default sections
    pub fn new() -> Self {
        Self {
            role_section: Self::default_role_section(),
            capabilities_section: Self::default_capabilities_section(),
            tool_instructions: String::new(),
            examples: Self::default_examples(),
            constraints: Self::default_constraints(),
            context: None,
            tools: Vec::new(),
            session_context: None,
            created_resources: None,
        }
    }

    /// Set the role section
    pub fn with_role(mut self, role: String) -> Self {
        self.role_section = role;
        self
    }

    /// Set available tools
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = tools;
        self.tool_instructions = Self::build_tool_instructions(&self.tools);
        self
    }

    /// Set project context
    pub fn with_context(mut self, context: Option<ProjectContext>) -> Self {
        self.context = context;
        self
    }

    /// Add an example
    pub fn add_example(mut self, example: String) -> Self {
        self.examples.push(example);
        self
    }

    /// Add a constraint
    pub fn add_constraint(mut self, constraint: String) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Set session context for precise project/workflow targeting (Feature 025)
    pub fn with_session_context(mut self, context: Option<SessionContext>) -> Self {
        self.session_context = context;
        self
    }

    /// Set created resources for in-conversation tracking (Feature 025)
    pub fn with_created_resources(mut self, resources: Option<SessionCreatedResources>) -> Self {
        self.created_resources = resources;
        self
    }

    /// Build the complete system prompt
    pub fn build(&self) -> String {
        let mut sections = Vec::new();

        // PRIORITY: Session context section FIRST (Feature 025)
        // This ensures AI sees the bound project/workflow context before anything else
        if let Some(ref session_ctx) = self.session_context {
            if let Some(session_section) = self.build_session_context_section(session_ctx) {
                sections.push(session_section);
            }
        }

        // Created resources section (Feature 025)
        // Shows resources created/modified during this conversation
        if let Some(ref resources) = self.created_resources {
            if let Some(resources_section) = resources.get_context_summary() {
                sections.push(resources_section);
            }
        }

        // Role section
        sections.push(self.role_section.clone());

        // Capabilities section
        sections.push(self.capabilities_section.clone());

        // Tool instructions (if tools are available)
        if !self.tool_instructions.is_empty() {
            sections.push(self.tool_instructions.clone());
        }

        // Examples section
        if !self.examples.is_empty() {
            let examples_section = format!(
                "## Usage Examples\n\n{}",
                self.examples
                    .iter()
                    .map(|e| format!("- {}", e))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            sections.push(examples_section);
        }

        // Constraints section
        if !self.constraints.is_empty() {
            let constraints_section = format!(
                "## Important Rules\n\n{}",
                self.constraints
                    .iter()
                    .map(|c| format!("- {}", c))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            sections.push(constraints_section);
        }

        // Legacy project context (if available and no session context)
        // Only include if session_context is not set (backward compatibility)
        if self.session_context.is_none() {
            if let Some(ref ctx) = self.context {
                let context_section = format!(
                    "## Current Project Context\n\n\
                    - **Project Name**: {}\n\
                    - **Project Type**: {}\n\
                    - **Package Manager**: {}\n\
                    - **Available Scripts**: {}",
                    ctx.project_name,
                    ctx.project_type,
                    ctx.package_manager,
                    if ctx.available_scripts.is_empty() {
                        "None".to_string()
                    } else {
                        ctx.available_scripts.join(", ")
                    }
                );
                sections.push(context_section);
            }
        }

        sections.join("\n\n")
    }

    /// Build the session context section (Feature 025)
    /// This section appears at the TOP of the prompt for maximum attention
    fn build_session_context_section(&self, ctx: &SessionContext) -> Option<String> {
        // Only build if we have meaningful context
        if !ctx.has_project() && ctx.bound_workflows.is_empty() {
            return None;
        }

        let mut lines = vec![
            "## IMPORTANT: Current Session Context".to_string(),
            String::new(),
            "You are currently assisting with a SPECIFIC project. Always use these IDs unless the user explicitly asks about a different project:".to_string(),
            String::new(),
        ];

        // Project info
        if let Some(ref name) = ctx.project_name {
            lines.push(format!("- **Current Project**: {}", name));
        }
        if let Some(ref id) = ctx.project_id {
            lines.push(format!("- **Project ID**: `{}`", id));
        }
        if let Some(ref path) = ctx.project_path {
            lines.push(format!("- **Project Path**: `{}`", path));
        }
        if let Some(ref project_type) = ctx.project_type {
            lines.push(format!("- **Project Type**: {}", project_type));
        }
        if let Some(ref pm) = ctx.package_manager {
            lines.push(format!("- **Package Manager**: {}", pm));
        }

        // Available scripts
        if !ctx.available_scripts.is_empty() {
            lines.push(format!(
                "- **Available Scripts**: {}",
                ctx.available_scripts.join(", ")
            ));
        }

        // Bound workflows
        if !ctx.bound_workflows.is_empty() {
            lines.push(String::new());
            lines.push("**Workflows in this project:**".to_string());
            for wf in &ctx.bound_workflows {
                lines.push(format!(
                    "- `{}`: {} ({} steps)",
                    wf.id, wf.name, wf.step_count
                ));
            }
        }

        // Active worktree
        if let Some(ref worktree) = ctx.active_worktree {
            lines.push(String::new());
            lines.push(format!(
                "**Active Worktree**: `{}` (branch: {})",
                worktree.path, worktree.branch
            ));
        }

        // Usage instructions
        lines.push(String::new());
        lines.push("### Context Usage Guidelines".to_string());
        lines.push(String::new());
        lines.push("1. **Use current context IDs by default**: When the user asks to \"run the deploy workflow\", use a workflow ID from the list above - do NOT call `list_workflows` first.".to_string());
        lines.push(String::new());
        lines.push("2. **Only query lists when necessary**: Only call `list_projects` or `list_workflows` when:".to_string());
        lines.push("   - User explicitly asks to see all projects/workflows".to_string());
        lines.push("   - User wants to work with a DIFFERENT project than the current one".to_string());
        lines.push("   - Current context has no bound project".to_string());
        lines.push(String::new());
        lines.push("3. **Cross-project operations**:".to_string());
        lines.push("   - ✅ INFO queries (any project): \"What scripts does project X have?\" → use list_project_scripts".to_string());
        lines.push("   - ⚠️ EXECUTION tasks: Stay in current project context".to_string());
        lines.push("   - Example: User asks \"run build in ProjectB\" while in ProjectA → Suggest: \"You're currently in ProjectA. Would you like to switch context or start a new conversation for ProjectB?\"".to_string());

        Some(lines.join("\n"))
    }

    // =========================================================================
    // Default Sections
    // =========================================================================

    fn default_role_section() -> String {
        r#"# Role & Identity

You are an AI assistant integrated into **SpecForge**, a powerful developer tool for managing projects, workflows, and automation tasks on macOS.

Your primary purpose is to help users accomplish development tasks efficiently by leveraging SpecForge's features and tools."#.to_string()
    }

    fn default_capabilities_section() -> String {
        // Feature 023 US2: Enhanced SpecForge feature descriptions (T049)
        // Optimized: Shortened descriptions to ~10 words each
        r#"## Your Capabilities

### Project Management
- **View Projects**: List all projects with type, status, config
- **Script Execution**: Run package.json scripts (build, test, dev)
- **Dependency Check**: Monitor outdated packages and vulnerabilities

### Git Operations
- **Status/Diff**: View changes, staged files, current branch
- **Commit Messages**: Generate meaningful commits from staged changes
- **Worktrees**: Manage multiple working directories and branches

### Workflow Automation
- **Execute Workflows**: Run predefined automation pipelines
- **Webhook Triggers**: Trigger external services
- **Step Templates**: Use and create reusable workflow steps

### Security & Time Machine
- **Snapshots**: Track dependency state changes over time
- **Security Insights**: Detect typosquatting and suspicious patterns

### General Help
- **Explanations**: Understand code, configs, and structures
- **Troubleshooting**: Debug project, script, or workflow issues"#.to_string()
    }

    fn build_tool_instructions(tools: &[ToolDefinition]) -> String {
        if tools.is_empty() {
            return String::new();
        }

        let mut tool_list = Vec::new();
        for tool in tools {
            let confirmation = if tool.requires_confirmation {
                " (requires user confirmation)"
            } else {
                ""
            };
            tool_list.push(format!(
                "- **{}**: {}{}",
                tool.name, tool.description, confirmation
            ));
        }

        format!(
            r#"## Available Tools

You have access to the following tools to perform actions:

{}

### Tool Usage Guidelines

1. **ALWAYS use tools for actions**: When a user asks you to run something, check status, or perform any action, USE THE APPROPRIATE TOOL. Never describe manual steps when a tool can do the job.

2. **Confirmation-required tools**: Some tools (like `run_script`, `run_workflow`) require user confirmation before execution. When you call these tools, the user will see a confirmation dialog.

3. **Read-only tools**: Tools like `get_git_status`, `get_staged_diff`, `list_project_scripts` can be used without confirmation to gather information.

4. **Provide context**: When calling a tool, explain briefly what you're about to do and why."#,
            tool_list.join("\n")
        )
    }

    fn default_examples() -> Vec<String> {
        // Optimized: Reduced from 13 to 8 essential examples (~200 tokens saved)
        vec![
            // Core tool usage patterns
            "\"run build\" → use `run_script` (script_name=\"build\") if in available scripts".to_string(),
            "\"check git status\" → use `get_git_status` tool".to_string(),
            "\"what's staged?\" → use `get_staged_diff` tool".to_string(),
            "\"run deploy workflow\" → use `run_workflow` (workflow_id from list_workflows)".to_string(),
            // Package manager vs scripts distinction
            "`npm audit`, `pnpm outdated` → use `run_package_manager_command`, NOT run_script".to_string(),
            // Interactive elements
            "Use [[navigation:route|Label]] for links, [[action:prompt|Label]] for action buttons".to_string(),
            // Proactive suggestions
            "After build → suggest running tests; After git status with changes → offer commit message".to_string(),
            // Help response
            "\"what can you do?\" → list capabilities: Project Management, Git, Workflows, Security".to_string(),
        ]
    }

    fn default_constraints() -> Vec<String> {
        // Optimized: Consolidated rules, added security constraints
        vec![
            // === SECURITY RULES (Priority 1) ===
            "**NEVER execute destructive operations** - Do NOT run `rm`, `del`, file deletion, format, or any command that permanently destroys data. If requested, explain the risk and decline.".to_string(),
            "**NEVER expose secrets in responses** - Do NOT include API keys, tokens, passwords, or credentials in your output, even if they appear in tool results. Mask them as `[REDACTED]`.".to_string(),
            "**Validate paths** - Only operate on registered project paths. Reject requests targeting system directories or unknown paths.".to_string(),

            // === CORE BEHAVIOR ===
            "**ALWAYS use tools for actions** - Never tell users to manually run commands when a tool exists".to_string(),
            "**Explain before acting** - Briefly describe what you'll do before calling tools".to_string(),
            "**Handle errors gracefully** - Explain failures and suggest alternatives".to_string(),
            "**Respect confirmations** - Wait for user approval on confirmation-required tools".to_string(),

            // === TASK COMPLETION (New) ===
            "**ALWAYS summarize after task completion** - After a tool executes successfully, provide a concise summary: (1) What was done, (2) Key results or output, (3) Any suggested next steps. Example: '✅ Build completed successfully. Output: 42 modules compiled in 3.2s. You may want to run tests next.'".to_string(),
            "**ONE TOOL, ONE RESULT, DONE** - When a tool executes and returns results, summarize once and STOP. Do NOT call the same tool again to 'verify' or 'confirm' - tool results are authoritative.".to_string(),

            // === TOOL USAGE (Consolidated) ===
            "**run_script vs run_package_manager_command**: `run_script` is ONLY for package.json scripts. For `audit`, `outdated`, `install`, use `run_package_manager_command` instead.".to_string(),
            "**Verify before executing** - If unsure about IDs/names, call list tools first (list_workflows, list_projects, list_project_scripts)".to_string(),

            // === UX RULES (Simplified) ===
            "**Stay focused** - For off-topic questions, politely redirect to SpecForge features".to_string(),
            "**Be proactive** - Suggest next actions (e.g., run tests after build, commit message after git status)".to_string(),
            "**Use interactive elements** - Include [[navigation:route|Label]] and [[action:prompt|Label]] when helpful".to_string(),
            "**Keep responses concise** - Use bullet points, avoid information overload".to_string(),
        ]
    }
}

impl Default for SystemPromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a system prompt with optional project context (compatibility function)
pub fn build_system_prompt(project_context: Option<&ProjectContext>) -> String {
    SystemPromptBuilder::new()
        .with_context(project_context.cloned())
        .build()
}

/// Build a complete system prompt with tools and context
pub fn build_system_prompt_with_tools(
    tools: Vec<ToolDefinition>,
    project_context: Option<&ProjectContext>,
) -> String {
    SystemPromptBuilder::new()
        .with_tools(tools)
        .with_context(project_context.cloned())
        .build()
}

/// Build a system prompt with session context and created resources (Feature 025)
/// This is the preferred method for agentic loop integration
pub fn build_system_prompt_with_session_context(
    tools: Vec<ToolDefinition>,
    session_context: Option<&SessionContext>,
    created_resources: Option<&SessionCreatedResources>,
) -> String {
    SystemPromptBuilder::new()
        .with_tools(tools)
        .with_session_context(session_context.cloned())
        .with_created_resources(created_resources.cloned())
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_system_prompt_basic() {
        let prompt = SystemPromptBuilder::new().build();

        assert!(prompt.contains("SpecForge"));
        assert!(prompt.contains("Role & Identity"));
        assert!(prompt.contains("Your Capabilities"));
        assert!(prompt.contains("Important Rules"));
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

        let prompt = SystemPromptBuilder::new()
            .with_context(Some(context))
            .build();

        assert!(prompt.contains("TestApp"));
        assert!(prompt.contains("Node.js"));
        assert!(prompt.contains("pnpm"));
        assert!(prompt.contains("build, test"));
    }

    #[test]
    fn test_build_system_prompt_with_tools() {
        let tools = vec![
            ToolDefinition {
                name: "run_script".to_string(),
                description: "Run a script".to_string(),
                parameters: serde_json::json!({}),
                requires_confirmation: true,
                category: "script".to_string(),
            },
            ToolDefinition {
                name: "get_git_status".to_string(),
                description: "Get git status".to_string(),
                parameters: serde_json::json!({}),
                requires_confirmation: false,
                category: "git".to_string(),
            },
        ];

        let prompt = SystemPromptBuilder::new()
            .with_tools(tools)
            .build();

        assert!(prompt.contains("Available Tools"));
        assert!(prompt.contains("run_script"));
        assert!(prompt.contains("get_git_status"));
        assert!(prompt.contains("requires user confirmation"));
    }

    #[test]
    fn test_constraints_include_tool_usage() {
        let prompt = SystemPromptBuilder::new().build();

        // Should emphasize using tools for actions
        assert!(prompt.contains("ALWAYS use tools for actions"));
    }

    #[test]
    fn test_constraints_include_task_completion_summary() {
        let prompt = SystemPromptBuilder::new().build();

        // Should require summarizing after task completion
        assert!(prompt.contains("ALWAYS summarize after task completion"));
        // Should prevent duplicate tool calls
        assert!(prompt.contains("ONE TOOL, ONE RESULT, DONE"));
    }

    #[test]
    fn test_examples_include_tool_patterns() {
        let prompt = SystemPromptBuilder::new().build();

        // Should include example patterns
        assert!(prompt.contains("run_script"));
        assert!(prompt.contains("get_staged_diff"));
    }

    #[test]
    fn test_off_topic_handling() {
        let prompt = SystemPromptBuilder::new().build();

        // Should include off-topic handling instruction
        assert!(prompt.contains("Stay focused"));
    }

    #[test]
    fn test_interactive_element_instructions() {
        let prompt = SystemPromptBuilder::new().build();

        // Should include interactive element syntax
        assert!(prompt.contains("[[navigation:"));
        assert!(prompt.contains("[[action:"));
    }

    #[test]
    fn test_compatibility_function() {
        // Test backward compatibility with build_system_prompt
        let prompt = build_system_prompt(None);
        assert!(prompt.contains("SpecForge"));

        let context = ProjectContext {
            project_name: "Test".to_string(),
            project_path: "/test".to_string(),
            project_type: "Rust".to_string(),
            package_manager: "cargo".to_string(),
            available_scripts: vec![],
        };
        let prompt_with_context = build_system_prompt(Some(&context));
        assert!(prompt_with_context.contains("Test"));
        assert!(prompt_with_context.contains("Rust"));
    }

    // =========================================================================
    // Feature 025: Session Context Tests
    // =========================================================================

    #[test]
    fn test_session_context_appears_first() {
        use crate::models::ai_assistant::WorkflowSummary;

        let session_ctx = SessionContext {
            project_id: Some("proj_123".to_string()),
            project_name: Some("TestProject".to_string()),
            project_path: Some("/test/path".to_string()),
            project_type: Some("Node.js".to_string()),
            package_manager: Some("pnpm".to_string()),
            available_scripts: vec!["build".to_string(), "test".to_string()],
            bound_workflows: vec![
                WorkflowSummary {
                    id: "wf_001".to_string(),
                    name: "Deploy".to_string(),
                    step_count: 3,
                },
            ],
            active_worktree: None,
        };

        let prompt = SystemPromptBuilder::new()
            .with_session_context(Some(session_ctx))
            .build();

        // Session context should appear BEFORE role section
        let session_pos = prompt.find("IMPORTANT: Current Session Context");
        let role_pos = prompt.find("Role & Identity");

        assert!(session_pos.is_some(), "Session context section should exist");
        assert!(role_pos.is_some(), "Role section should exist");
        assert!(
            session_pos.unwrap() < role_pos.unwrap(),
            "Session context should appear before role section"
        );

        // Check content
        assert!(prompt.contains("TestProject"));
        assert!(prompt.contains("proj_123"));
        assert!(prompt.contains("wf_001"));
        assert!(prompt.contains("Deploy"));
        assert!(prompt.contains("3 steps"));
    }

    #[test]
    fn test_session_context_overrides_legacy_context() {
        let legacy_context = ProjectContext {
            project_name: "LegacyProject".to_string(),
            project_path: "/legacy/path".to_string(),
            project_type: "Python".to_string(),
            package_manager: "pip".to_string(),
            available_scripts: vec![],
        };

        let session_ctx = SessionContext {
            project_id: Some("proj_new".to_string()),
            project_name: Some("NewProject".to_string()),
            project_path: Some("/new/path".to_string()),
            project_type: Some("Node.js".to_string()),
            package_manager: Some("npm".to_string()),
            available_scripts: vec![],
            bound_workflows: vec![],
            active_worktree: None,
        };

        let prompt = SystemPromptBuilder::new()
            .with_context(Some(legacy_context))
            .with_session_context(Some(session_ctx))
            .build();

        // Session context should be used, not legacy
        assert!(prompt.contains("NewProject"));
        assert!(!prompt.contains("LegacyProject"));
    }

    #[test]
    fn test_created_resources_in_prompt() {
        let mut resources = SessionCreatedResources::new();
        resources.add_workflow(
            "wf_new_001".to_string(),
            "Build Pipeline".to_string(),
            None,
            0,
        );
        resources.add_step(
            "wf_new_001".to_string(),
            "Build Pipeline".to_string(),
            "step_1".to_string(),
            "Install Dependencies".to_string(),
            0,
        );

        let prompt = SystemPromptBuilder::new()
            .with_created_resources(Some(resources))
            .build();

        assert!(prompt.contains("Resources Created/Modified in This Conversation"));
        assert!(prompt.contains("wf_new_001"));
        assert!(prompt.contains("Build Pipeline"));
        assert!(prompt.contains("Install Dependencies"));
    }

    #[test]
    fn test_build_system_prompt_with_session_context_function() {
        let session_ctx = SessionContext {
            project_id: Some("proj_func".to_string()),
            project_name: Some("FunctionTest".to_string()),
            project_path: None,
            project_type: None,
            package_manager: None,
            available_scripts: vec![],
            bound_workflows: vec![],
            active_worktree: None,
        };

        let prompt = build_system_prompt_with_session_context(
            vec![],
            Some(&session_ctx),
            None,
        );

        assert!(prompt.contains("FunctionTest"));
        assert!(prompt.contains("proj_func"));
    }

    #[test]
    fn test_empty_session_context_no_section() {
        let empty_ctx = SessionContext::default();

        let prompt = SystemPromptBuilder::new()
            .with_session_context(Some(empty_ctx))
            .build();

        // Empty context should not produce a session section
        assert!(!prompt.contains("IMPORTANT: Current Session Context"));
    }
}
