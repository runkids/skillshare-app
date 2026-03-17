// AI Integration data models
// Feature: AI CLI Integration (020-ai-cli-integration)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Supported AI service providers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AIProvider {
    OpenAI,
    Anthropic,
    Gemini,
    Ollama,
    #[serde(rename = "lm_studio")]
    LMStudio,
}

impl std::fmt::Display for AIProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AIProvider::OpenAI => write!(f, "openai"),
            AIProvider::Anthropic => write!(f, "anthropic"),
            AIProvider::Gemini => write!(f, "gemini"),
            AIProvider::Ollama => write!(f, "ollama"),
            AIProvider::LMStudio => write!(f, "lm_studio"),
        }
    }
}

impl AIProvider {
    /// Returns whether this provider requires an API key
    pub fn requires_api_key(&self) -> bool {
        matches!(self, AIProvider::OpenAI | AIProvider::Anthropic | AIProvider::Gemini)
    }

    /// Returns the default endpoint for this provider
    pub fn default_endpoint(&self) -> &'static str {
        match self {
            AIProvider::OpenAI => "https://api.openai.com/v1",
            AIProvider::Anthropic => "https://api.anthropic.com/v1",
            AIProvider::Gemini => "https://generativelanguage.googleapis.com/v1beta",
            AIProvider::Ollama => "http://127.0.0.1:11434",
            AIProvider::LMStudio => "http://127.0.0.1:1234/v1",
        }
    }

    /// Returns the default model for this provider
    pub fn default_model(&self) -> &'static str {
        match self {
            AIProvider::OpenAI => "gpt-4o-mini",
            AIProvider::Anthropic => "claude-3-haiku-20240307",
            AIProvider::Gemini => "gemini-1.5-flash",
            AIProvider::Ollama => "llama3.2",
            AIProvider::LMStudio => "local-model",
        }
    }

    /// Returns the typical context window size for this provider (in tokens)
    /// Used for dynamic max_tokens calculation
    pub fn context_window(&self) -> u32 {
        match self {
            AIProvider::OpenAI => 128_000,     // GPT-4o supports 128K
            AIProvider::Anthropic => 200_000,  // Claude 3 supports 200K
            AIProvider::Gemini => 1_000_000,   // Gemini 1.5 supports 1M
            AIProvider::Ollama => 8_000,       // Conservative default for local models
            AIProvider::LMStudio => 8_000,     // Conservative default for local models
        }
    }

    /// Returns the maximum output tokens recommended for this provider
    pub fn max_output_tokens(&self) -> u32 {
        match self {
            AIProvider::OpenAI => 16_384,      // GPT-4o max output
            AIProvider::Anthropic => 8_192,    // Claude 3 max output
            AIProvider::Gemini => 8_192,       // Gemini typical max
            AIProvider::Ollama => 4_096,       // Conservative for local
            AIProvider::LMStudio => 4_096,     // Conservative for local
        }
    }
}

/// Template category for different AI use cases
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TemplateCategory {
    /// Git commit message generation
    #[default]
    GitCommit,
    /// Pull request description generation
    PullRequest,
    /// Code review suggestions
    CodeReview,
    /// Documentation generation
    Documentation,
    /// Release notes generation
    ReleaseNotes,
    /// Security vulnerability analysis
    SecurityAdvisory,
    /// Custom/general purpose
    Custom,
}

impl TemplateCategory {
    /// Get the available variables for this category
    pub fn available_variables(&self) -> Vec<&'static str> {
        match self {
            TemplateCategory::GitCommit => vec!["diff"],
            TemplateCategory::PullRequest => vec!["diff", "commits", "branch", "base_branch"],
            TemplateCategory::CodeReview => vec!["diff", "file_path", "code"],
            TemplateCategory::Documentation => vec!["code", "file_path", "function_name"],
            TemplateCategory::ReleaseNotes => vec!["commits", "version", "previous_version"],
            TemplateCategory::SecurityAdvisory => vec!["vulnerability_json", "project_context", "severity_summary"],
            TemplateCategory::Custom => vec!["input"],
        }
    }

    /// Get display name for this category
    pub fn display_name(&self) -> &'static str {
        match self {
            TemplateCategory::GitCommit => "Git Commit",
            TemplateCategory::PullRequest => "Pull Request",
            TemplateCategory::CodeReview => "Code Review",
            TemplateCategory::Documentation => "Documentation",
            TemplateCategory::ReleaseNotes => "Release Notes",
            TemplateCategory::SecurityAdvisory => "Security Advisory",
            TemplateCategory::Custom => "Custom",
        }
    }
}

/// Commit message format types (subset for GitCommit category)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CommitFormat {
    #[default]
    ConventionalCommits,
    Simple,
    Custom,
}

/// AI Provider configuration
/// Stores user-configured AI provider connection information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIProviderConfig {
    /// Unique identifier (UUID v4)
    pub id: String,
    /// User-defined name for this provider
    pub name: String,
    /// AI provider type
    pub provider: AIProvider,
    /// API endpoint URL
    pub endpoint: String,
    /// Selected model name
    pub model: String,
    /// Whether this is the default provider
    #[serde(default)]
    pub is_default: bool,
    /// Whether this provider is enabled
    #[serde(default = "default_true")]
    pub is_enabled: bool,
    /// When this provider was created
    pub created_at: DateTime<Utc>,
    /// When this provider was last updated
    pub updated_at: DateTime<Utc>,
}

fn default_true() -> bool {
    true
}

impl AIProviderConfig {
    /// Create a new AI provider configuration
    pub fn new(name: String, provider: AIProvider, endpoint: String, model: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            provider,
            endpoint,
            model,
            is_default: false,
            is_enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create with default endpoint and model
    pub fn with_defaults(name: String, provider: AIProvider) -> Self {
        Self::new(
            name,
            provider.clone(),
            provider.default_endpoint().to_string(),
            provider.default_model().to_string(),
        )
    }
}

/// Prompt template for AI generation tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptTemplate {
    /// Unique identifier (UUID v4)
    pub id: String,
    /// Template name
    pub name: String,
    /// Template description
    pub description: Option<String>,
    /// Template category (determines available variables)
    #[serde(default)]
    pub category: TemplateCategory,
    /// Prompt content with variable placeholders like {diff}, {code}, etc.
    pub template: String,
    /// Output format type (mainly for GitCommit category)
    #[serde(default)]
    pub output_format: Option<CommitFormat>,
    /// Whether this is the default template for its category
    #[serde(default)]
    pub is_default: bool,
    /// Whether this is a built-in template (cannot be deleted)
    #[serde(default)]
    pub is_builtin: bool,
    /// When this template was created
    pub created_at: DateTime<Utc>,
    /// When this template was last updated
    pub updated_at: DateTime<Utc>,
}

impl PromptTemplate {
    /// Create a new prompt template
    pub fn new(
        name: String,
        description: Option<String>,
        category: TemplateCategory,
        template: String,
        output_format: Option<CommitFormat>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            category,
            template,
            output_format,
            is_default: false,
            is_builtin: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get the built-in Conventional Commits template
    pub fn builtin_git_conventional() -> Self {
        Self {
            id: "builtin-git-conventional".to_string(),
            name: "Conventional Commits".to_string(),
            description: Some("Standard Conventional Commits format".to_string()),
            category: TemplateCategory::GitCommit,
            template: r#"Generate a Git commit message following Conventional Commits format.

Format: <type>(<scope>): <description>

Types: feat|fix|docs|style|refactor|test|chore

Changes:
{diff}

IMPORTANT: Output ONLY the commit message. No thinking, no explanation, no XML tags, no markdown code blocks. Just the plain commit message text."#.to_string(),
            output_format: Some(CommitFormat::ConventionalCommits),
            is_default: true,
            is_builtin: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Get the built-in Simple commit template
    pub fn builtin_git_simple() -> Self {
        Self {
            id: "builtin-git-simple".to_string(),
            name: "Simple Commit".to_string(),
            description: Some("Concise single-line description".to_string()),
            category: TemplateCategory::GitCommit,
            template: r#"Generate a concise one-line commit message for these changes:

{diff}

IMPORTANT: Output ONLY the commit message. No thinking, no explanation, no XML tags, no markdown code blocks. Just the plain commit message text (one line)."#.to_string(),
            output_format: Some(CommitFormat::Simple),
            is_default: false,
            is_builtin: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Get the built-in PR description template
    pub fn builtin_pr_description() -> Self {
        Self {
            id: "builtin-pr-description".to_string(),
            name: "PR Description".to_string(),
            description: Some("Generate pull request description".to_string()),
            category: TemplateCategory::PullRequest,
            template: r#"You are a professional developer writing a pull request description.
Based on the following information, generate a clear and informative PR description.

Branch: {branch}
Base branch: {base_branch}

Commits:
{commits}

Code changes:
{diff}

Generate a PR description with:
1. A brief summary (1-2 sentences)
2. Key changes (bullet points)
3. Any breaking changes or migration notes if applicable

Use markdown formatting."#.to_string(),
            output_format: None,
            is_default: true,
            is_builtin: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Get the built-in code review template
    pub fn builtin_code_review() -> Self {
        Self {
            id: "builtin-code-review".to_string(),
            name: "Code Review".to_string(),
            description: Some("Review code changes with comprehensive security analysis".to_string()),
            category: TemplateCategory::CodeReview,
            template: r#"You are a senior security-focused developer conducting a comprehensive code review.
Review the following code changes and provide constructive feedback.

File: {file_path}

Changes:
{diff}

## Review Categories

### 1. Code Quality & Readability
- Code structure and organization
- Naming conventions and clarity
- Error handling completeness
- Code duplication or redundancy

### 2. Potential Bugs
- Logic errors or edge cases
- Null/undefined handling
- Race conditions or async issues
- Resource leaks (memory, file handles, connections)

### 3. Performance
- Algorithm efficiency
- Unnecessary iterations or computations
- Memory usage concerns
- Database query optimization

### 4. Security Analysis (CRITICAL)
Check thoroughly for these vulnerabilities:

**Injection Attacks:**
- SQL Injection: Unsanitized input in SQL queries
- Command Injection: Shell command execution with user input
- XSS (Cross-Site Scripting): Unescaped user content in HTML/JS output
- LDAP/XPath Injection: Unsanitized input in directory queries
- Template Injection: User input in template engines

**Authentication & Authorization:**
- Hardcoded credentials, API keys, passwords, or secrets
- Weak authentication mechanisms
- Missing or improper authorization checks
- Session management issues
- JWT validation flaws

**Data Exposure:**
- Sensitive data in logs (passwords, tokens, PII)
- Secrets committed to code (API keys, connection strings)
- Information leakage in error messages
- Unencrypted sensitive data storage or transmission

**Other Security Issues:**
- Path Traversal: File operations with user-controlled paths
- SSRF (Server-Side Request Forgery): User-controlled URLs in server requests
- CSRF (Cross-Site Request Forgery): Missing CSRF protection
- Insecure Deserialization: Deserializing untrusted data
- XML External Entity (XXE): XML parsing without proper configuration
- Improper input validation or sanitization
- Insecure cryptographic practices
- Missing security headers or CORS misconfiguration

### 5. Suggestions for Improvement
- Better approaches or patterns
- Modern language features to utilize
- Refactoring opportunities

## Output Format
For each finding, specify:
- **Severity**: Critical / High / Medium / Low / Info
- **Location**: Line number or code section
- **Issue**: Clear description of the problem
- **Recommendation**: How to fix it

Prioritize security issues. Be specific and actionable. Use markdown formatting."#.to_string(),
            output_format: None,
            is_default: true,
            is_builtin: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Get the built-in release notes template
    pub fn builtin_release_notes() -> Self {
        Self {
            id: "builtin-release-notes".to_string(),
            name: "Release Notes".to_string(),
            description: Some("Generate release notes from commits".to_string()),
            category: TemplateCategory::ReleaseNotes,
            template: r#"You are a technical writer preparing release notes.
Generate release notes based on the following commits.

Version: {version}
Previous version: {previous_version}

Commits:
{commits}

Generate release notes with sections:
- **New Features** - New functionality added
- **Bug Fixes** - Issues that were fixed
- **Improvements** - Enhancements to existing features
- **Breaking Changes** - Changes that require user action

Use markdown formatting. Be concise but informative."#.to_string(),
            output_format: None,
            is_default: true,
            is_builtin: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Get the built-in security advisory template
    pub fn builtin_security_advisory() -> Self {
        Self {
            id: "builtin-security-advisory".to_string(),
            name: "Security Advisory".to_string(),
            description: Some("Analyze security vulnerabilities and provide remediation guidance".to_string()),
            category: TemplateCategory::SecurityAdvisory,
            template: r#"You are a security advisor analyzing package vulnerabilities for a software project.

## Project Context
{project_context}

## Vulnerability Details
{vulnerability_json}

## Severity Summary
{severity_summary}

Provide a comprehensive security analysis with the following sections:

### 1. Risk Assessment
- Overall risk level (Critical/High/Medium/Low)
- Business impact analysis
- Likelihood of exploitation
- Attack vectors and prerequisites

### 2. Priority Actions
- Immediate steps to mitigate critical issues
- Short-term remediation tasks
- Long-term security improvements

### 3. Remediation Strategy
- Specific upgrade paths for affected packages
- Alternative packages if upgrades are not available
- Workarounds for vulnerabilities without fixes
- Code changes if applicable

### 4. Breaking Change Warnings
- Potential compatibility issues with suggested fixes
- Migration notes for major version upgrades
- Testing recommendations

Be specific, actionable, and prioritize by severity. Use markdown formatting."#.to_string(),
            output_format: None,
            is_default: true,
            is_builtin: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Get all built-in templates
    pub fn all_builtins() -> Vec<Self> {
        vec![
            Self::builtin_git_conventional(),
            Self::builtin_git_simple(),
            Self::builtin_pr_description(),
            Self::builtin_code_review(),
            Self::builtin_release_notes(),
            Self::builtin_security_advisory(),
        ]
    }

    /// Get built-in templates for a specific category
    pub fn builtins_for_category(category: &TemplateCategory) -> Vec<Self> {
        Self::all_builtins()
            .into_iter()
            .filter(|t| &t.category == category)
            .collect()
    }

    /// Get available variables for this template's category
    pub fn available_variables(&self) -> Vec<&'static str> {
        self.category.available_variables()
    }

    /// Validate that the template contains required variables for its category
    pub fn validate_variables(&self) -> Result<(), String> {
        let available = self.available_variables();

        // Check if at least one variable is used
        let has_variable = available.iter().any(|var| {
            self.template.contains(&format!("{{{}}}", var))
        });

        if !has_variable && !available.is_empty() {
            return Err(format!(
                "Template must contain at least one of: {}",
                available.iter().map(|v| format!("{{{}}}", v)).collect::<Vec<_>>().join(", ")
            ));
        }

        Ok(())
    }

    /// Render the template with the given variables
    pub fn render(&self, variables: &std::collections::HashMap<String, String>) -> String {
        let mut result = self.template.clone();
        for (key, value) in variables {
            result = result.replace(&format!("{{{}}}", key), value);
        }
        result
    }

    /// Render with a single diff (backward compatible helper)
    pub fn render_with_diff(&self, diff: &str) -> String {
        let mut vars = std::collections::HashMap::new();
        vars.insert("diff".to_string(), diff.to_string());
        self.render(&vars)
    }
}

/// Project-level AI settings override
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAISettings {
    /// Project path (used as key)
    pub project_path: String,
    /// Preferred AI provider ID for this project
    pub preferred_provider_id: Option<String>,
    /// Preferred prompt template ID for this project
    pub preferred_template_id: Option<String>,
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatToolDefinition {
    /// Tool type (always "function" for now)
    #[serde(rename = "type")]
    pub tool_type: String,
    /// Function definition
    pub function: ChatFunctionDefinition,
}

/// Function definition within a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatFunctionDefinition {
    /// Function name
    pub name: String,
    /// Description of what the function does
    pub description: String,
    /// JSON Schema for parameters
    pub parameters: serde_json::Value,
}

impl ChatToolDefinition {
    /// Create a new function tool definition
    pub fn function(name: impl Into<String>, description: impl Into<String>, parameters: serde_json::Value) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: ChatFunctionDefinition {
                name: name.into(),
                description: description.into(),
                parameters,
            },
        }
    }
}

/// Tool call made by the AI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatToolCall {
    /// Unique ID for this tool call
    pub id: String,
    /// Type of tool (always "function" for now)
    #[serde(rename = "type")]
    pub tool_type: String,
    /// Function call details
    pub function: ChatFunctionCall,
    /// Thought signature for Gemini 2.5+ models (preserves reasoning context)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,
}

/// Function call within a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatFunctionCall {
    /// Function name
    pub name: String,
    /// JSON string of arguments
    pub arguments: String,
}

/// Chat message for AI completion
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    /// Role: "system", "user", "assistant", or "tool"
    pub role: String,
    /// Message content (can be null for assistant messages with tool_calls)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Tool calls made by assistant (for assistant messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ChatToolCall>>,
    /// Tool call ID this message is responding to (for tool messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create an assistant message with tool calls (content may be empty)
    pub fn assistant_with_tool_calls(content: Option<String>, tool_calls: Vec<ChatToolCall>) -> Self {
        Self {
            role: "assistant".to_string(),
            content,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    /// Create a tool result message
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

/// Options for chat completion requests
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChatOptions {
    /// Temperature (0.0 - 2.0)
    pub temperature: Option<f32>,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// Top-p sampling
    pub top_p: Option<f32>,
    /// Tool definitions for function calling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ChatToolDefinition>>,
}

/// Reason why the AI stopped generating
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// Normal completion
    Stop,
    /// Hit max_tokens limit (response may be truncated)
    Length,
    /// Content filtered
    ContentFilter,
    /// Tool/function call
    ToolCalls,
    /// Unknown or not provided
    Unknown,
}

/// Response from chat completion
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatResponse {
    /// Generated content (may be empty if tool_calls present)
    pub content: String,
    /// Tokens used (if available)
    pub tokens_used: Option<u32>,
    /// Model used
    pub model: String,
    /// Why the model stopped generating
    #[serde(default)]
    pub finish_reason: Option<FinishReason>,
    /// Tool calls requested by the AI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ChatToolCall>>,
}

/// Result from AI commit message generation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateResult {
    /// Generated commit message
    pub message: String,
    /// Tokens used (if available)
    pub tokens_used: Option<u32>,
}

/// Result from AI connection test
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestConnectionResult {
    /// Whether the connection was successful
    pub success: bool,
    /// Response latency in milliseconds
    pub latency_ms: Option<u64>,
    /// Available models (for Ollama/LMStudio)
    pub models: Option<Vec<String>>,
    /// Error message if failed
    pub error: Option<String>,
}

/// Model information (for Ollama/LMStudio)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    /// Model name
    pub name: String,
    /// Model size in bytes
    pub size: Option<u64>,
    /// Last modified time
    pub modified_at: Option<String>,
}

/// Request to add a new AI provider
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddProviderRequest {
    pub name: String,
    pub provider: AIProvider,
    pub endpoint: String,
    pub model: String,
    /// API key (only for cloud providers)
    pub api_key: Option<String>,
}

/// Request to update an AI provider
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProviderRequest {
    pub id: String,
    pub name: Option<String>,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub is_enabled: Option<bool>,
    /// API key (if provided, will be updated)
    pub api_key: Option<String>,
}

/// Request to add a new prompt template
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddTemplateRequest {
    pub name: String,
    pub description: Option<String>,
    pub category: TemplateCategory,
    pub template: String,
    /// Output format (mainly for GitCommit category)
    pub output_format: Option<CommitFormat>,
}

/// Request to update a prompt template
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTemplateRequest {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub template: Option<String>,
    pub output_format: Option<CommitFormat>,
}

/// Request to generate a commit message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateCommitMessageRequest {
    pub project_path: String,
    /// Provider ID (if not specified, use default)
    pub provider_id: Option<String>,
    /// Template ID (if not specified, use default)
    pub template_id: Option<String>,
}

/// Request to generate a code review
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateCodeReviewRequest {
    pub project_path: String,
    /// File path relative to repository root
    pub file_path: String,
    /// Whether to review staged or unstaged diff
    pub staged: bool,
    /// Provider ID (if not specified, use default)
    pub provider_id: Option<String>,
    /// Template ID (if not specified, use default code review template)
    pub template_id: Option<String>,
}

/// Result from AI code review generation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateCodeReviewResult {
    /// Generated review content (markdown)
    pub review: String,
    /// Tokens used (if available)
    pub tokens_used: Option<u32>,
    /// Whether the response was truncated due to token limit
    #[serde(default)]
    pub is_truncated: bool,
}

/// Request to generate a review of all staged changes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateStagedReviewRequest {
    pub project_path: String,
    /// Provider ID (if not specified, use default)
    pub provider_id: Option<String>,
    /// Template ID (if not specified, use default code review template)
    pub template_id: Option<String>,
}

/// Request to update project AI settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectSettingsRequest {
    pub project_path: String,
    /// Preferred provider ID (null to clear)
    pub preferred_provider_id: Option<String>,
    /// Preferred template ID (null to clear)
    pub preferred_template_id: Option<String>,
}

/// Request to generate security analysis for a single vulnerability
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateSecurityAnalysisRequest {
    /// Project path
    pub project_path: String,
    /// Project name for context
    pub project_name: String,
    /// Package manager (npm, pnpm, yarn, bun)
    pub package_manager: String,
    /// Vulnerability data as JSON
    pub vulnerability: serde_json::Value,
    /// Provider ID (if not specified, use default)
    pub provider_id: Option<String>,
    /// Template ID (if not specified, use default security advisory template)
    pub template_id: Option<String>,
}

/// Request to generate security summary for all vulnerabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateSecuritySummaryRequest {
    /// Project path
    pub project_path: String,
    /// Project name for context
    pub project_name: String,
    /// Package manager (npm, pnpm, yarn, bun)
    pub package_manager: String,
    /// All vulnerabilities as JSON array
    pub vulnerabilities: Vec<serde_json::Value>,
    /// Vulnerability summary counts
    pub summary: serde_json::Value,
    /// Provider ID (if not specified, use default)
    pub provider_id: Option<String>,
    /// Template ID (if not specified, use default security advisory template)
    pub template_id: Option<String>,
}

/// Result from AI security analysis generation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateSecurityAnalysisResult {
    /// Generated analysis content (markdown)
    pub analysis: String,
    /// Tokens used (if available)
    pub tokens_used: Option<u32>,
    /// Whether the response was truncated due to token limit
    #[serde(default)]
    pub is_truncated: bool,
}
