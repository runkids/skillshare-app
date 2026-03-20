# AI Integration

Connect multiple AI providers and use intelligent assistance throughout Skillshare App.

## Overview

Skillshare App supports multiple AI providers for intelligent features like:

- Commit message generation
- Code review analysis
- Security vulnerability summaries
- Custom AI prompts
- AI CLI tool integration

## Supported Providers

| Provider | Models | Auth |
|----------|--------|------|
| **OpenAI** | GPT-4o, GPT-4o-mini, o1, o3 | API Key |
| **Anthropic** | Claude 4 Opus, Claude 4 Sonnet, Claude 3.5 Haiku | API Key |
| **Google Gemini** | Gemini 2.0 Flash, Gemini 1.5 Pro (Free tier available) | API Key |
| **Ollama** | Llama, Mistral, Qwen, and any local model | Local |
| **LM Studio** | Any local model | Local |

## Adding AI Services

### Cloud Providers (OpenAI, Anthropic, Google)

1. Go to **Settings** → **AI Services**
2. Click **Add Service**
3. Select the provider
4. Enter your API key
5. Click **Verify & Save**

### Local Providers (Ollama, LM Studio)

1. Ensure Ollama/LM Studio is running locally
2. Go to **Settings** → **AI Services**
3. Click **Add Service**
4. Select **Ollama** or **LM Studio**
5. Enter the local URL:
   - Ollama: `http://127.0.0.1:11434`
   - LM Studio: `http://127.0.0.1:1234/v1`
6. Click **Connect**

## API Key Security

API keys are encrypted using AES-256-GCM and stored securely:

- Keys are never exposed in logs
- Encrypted at rest
- Stored in your system's keychain (macOS)

## Selecting Models

### Per-Service Models

Each service has available models:

1. Click on a service
2. Click **Fetch Models**
3. Select your preferred default model

### Per-Feature Models

Choose different models for different tasks:

- Commit messages: Faster model (GPT-4o-mini)
- Code review: More capable model (GPT-4o, Claude 3 Sonnet)

## AI Assistant Tab

The AI Assistant provides an interactive chat interface for working with your projects.

### Overview

Access the AI Assistant from the sidebar to:

- Chat with AI about your code and projects
- Execute MCP operations through natural language
- Get contextual suggestions based on your current work
- Manage conversation history

### Starting a Conversation

1. Click the **AI Assistant** tab in the sidebar
2. Type your message in the input area
3. Press **Enter** or click **Send**
4. AI responses stream in real-time

### Quick Actions

Quick action chips appear based on your project context:

| Context | Actions Available |
|---------|------------------|
| Git repository | Generate commit, Review changes |
| Node.js project | Run tests, Build project |
| Any project | Help, Explain this |

Click a chip to send that prompt instantly.

### MCP Operations

The AI can execute Skillshare App operations with your approval:

1. Ask the AI to perform an action (e.g., "run the test script")
2. AI proposes the action with details
3. Click **Approve** to execute or **Deny** to cancel
4. See execution results inline

### Conversation History

Access previous conversations from the sidebar:

- **New Chat**: Start a fresh conversation
- **History**: View past conversations
- **Rename**: Double-click or use menu to rename
- **Delete**: Remove conversations via menu

### Project Context

The AI is aware of your current project:

- Project name and type
- Available scripts
- Package manager
- Git status (if applicable)

This context helps provide relevant suggestions and accurate responses.

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Cmd/Ctrl+B | Toggle sidebar |
| Cmd/Ctrl+N | New chat |
| Cmd/Ctrl+[ | Previous conversation |
| Cmd/Ctrl+] | Next conversation |
| Enter | Send message |
| Shift+Enter | New line |
| Escape | Stop generation / Clear input |

### Sidebar Features

The sidebar provides powerful conversation management:

- **Collapsible**: Toggle with Cmd/Ctrl+B or click the collapse button
- **Search**: Filter conversations by title or content
- **Date Groups**: Conversations organized by Today, Yesterday, This Week, etc.
- **Hover Preview**: In collapsed mode, hover over icons to see conversation details

### Model Selection

Choose different AI models per conversation:

1. Click the model selector in the conversation header
2. Select from your configured AI services
3. The model is saved with the conversation

### Token Usage

Track token usage in real-time:

- Token count shown in the conversation header
- Progress bar indicates usage level
- Warnings at 75% and 90% usage

### Language Support

The AI Assistant responds in the same language as your message. Write in English, Chinese, or any other language, and the AI will reply accordingly.

### Tool Use Requirements

For the AI to execute MCP tools (run scripts, workflows, etc.), you need an AI model that supports function calling:

**Recommended Cloud Models:**
- OpenAI GPT-4o, GPT-4o-mini
- Anthropic Claude 3 Sonnet, Claude 3.5 Haiku
- Google Gemini 2.0 Flash, Gemini 1.5 Pro

**Recommended Local Models (Ollama/LM Studio):**
- Llama 3.1 or newer (7B+)
- Qwen 2.5 or newer (7B+)
- Mistral 7B (with function calling support)

Models without function calling support can still answer questions but cannot execute tools.

## AI Features

### Commit Message Generation

Generate meaningful commit messages from your diffs:

1. Stage your changes
2. Click the **AI** button in the commit form
3. AI analyzes the diff and generates a message
4. Edit if needed, then commit

### Code Review Analysis

AI-powered code review for your changes:

1. Stage changes you want reviewed
2. Open the AI Review dialog
3. Select the scope (file or all staged changes)
4. AI analyzes and provides review feedback
5. Review suggestions and apply as needed

### Security Vulnerability Summaries

Plain-language explanations of security vulnerabilities:

1. Run a security scan on your project
2. Click on a vulnerability
3. Click **AI Analysis** button
4. AI explains the vulnerability in plain language:
   - What the issue is
   - Why it's dangerous
   - How to fix it
   - Risk assessment

### Security Summary Report

Generate a comprehensive overview of all vulnerabilities:

1. After a security scan completes
2. Click **Generate AI Summary**
3. AI creates a prioritized summary:
   - Critical issues requiring immediate action
   - Recommended fix order
   - Dependency update suggestions

## AI CLI Integration

Skillshare App integrates with AI CLI tools for enhanced functionality.

### Supported CLI Tools

| CLI | Binary | Description |
|-----|--------|-------------|
| **Claude Code** | `claude` | Anthropic Claude CLI for code assistance |
| **Codex** | `codex` | OpenAI Codex CLI for code generation |
| **Gemini CLI** | `gemini` | Google Gemini CLI for AI assistance |

### Auto-Detection

Skillshare App automatically detects installed CLI tools:

1. Go to **Settings** → **AI Services**
2. View **CLI Tools** section
3. Detected tools show with version and auth status

### Running AI Commands

1. Go to **Settings** → **AI CLI**
2. Select an installed CLI tool
3. Enter a prompt
4. Click **Run**

Output streams in real-time in the panel.

### CLI Execution Options

- **Include Diff**: Add staged git diff as context
- **Include Files**: Add specific files as context
- **Custom Context**: Add arbitrary text context
- **Include MCP Context**: Add project info from MCP

## Prompt Templates

Customize how AI generates content with templates.

### Default Templates

Skillshare App includes templates for:

- Git commit messages
- Pull request descriptions
- Code review comments
- Documentation generation
- Release notes
- Security advisories
- Custom prompts

### Template Categories

| Category | Description | Variables |
|----------|-------------|-----------|
| `git_commit` | Commit message generation | `{diff}` |
| `pull_request` | PR description | `{diff}`, `{commits}`, `{branch}`, `{base_branch}` |
| `code_review` | Code review feedback | `{diff}`, `{file_path}`, `{code}` |
| `documentation` | Doc generation | `{code}`, `{file_path}`, `{function_name}` |
| `release_notes` | Release notes | `{commits}`, `{version}`, `{previous_version}` |
| `security_advisory` | Security analysis | `{vulnerability_json}`, `{project_context}`, `{severity_summary}` |
| `custom` | General purpose | `{input}` |

### Creating Custom Templates

1. Go to **Settings** → **AI Services** → **Templates**
2. Click **New Template**
3. Configure:
   - Name
   - Category
   - Prompt text with variables
   - Output format (for commit messages)
4. Save

### Template Variables

Use variables in your prompts:

| Variable | Description |
|----------|-------------|
| `{diff}` | Git diff content |
| `{code}` | Selected code |
| `{file_path}` | Current file path |
| `{commits}` | Commit history |
| `{branch}` | Current branch name |
| `{base_branch}` | Target branch for PR |
| `{version}` | Release version |
| `{vulnerability_json}` | Vulnerability data as JSON |

### Example Template

**Commit Message Template:**

```
Based on the following git diff, generate a concise commit message
following conventional commit format.

Focus on:
- What changed (not how)
- Why it changed (if apparent)
- Keep the first line under 72 characters

Diff:
{diff}
```

### Commit Message Formats

Choose the output format for commit messages:

- **Conventional Commits**: `type(scope): description`
- **Simple**: Plain descriptive message
- **Custom**: Your own format

### Per-Project Templates

Override templates for specific projects:

1. Open a project
2. Go to **Settings** → **AI**
3. Select template overrides
4. Customize as needed

## AI Execution Modes

Skillshare App supports two execution modes:

### API Mode (Default)

Uses the configured AI service API directly:

- Faster response times
- Token usage tracking
- Works with any provider

### CLI Mode

Uses installed AI CLI tools:

- Richer context support
- Native CLI features
- No API key needed if CLI is authenticated

### Switching Modes

1. Go to **Settings** → **AI Services**
2. Select **Execution Mode** in the overview tab
3. Choose **API** or **CLI**

## Testing Services

### Connection Test

Verify your API key works:

1. Click **Test Connection** on a service
2. Skillshare App sends a simple request
3. Shows success or error details

### Model Probe

Test models without saving a service:

1. Click **Probe Models**
2. Enter provider and endpoint
3. See available models

## Default Service

Set a default AI service:

1. Go to **Settings** → **AI Services**
2. Click the star icon next to a service
3. This service is used when no specific one is selected

## Usage Limits

### Cloud Providers

Be aware of API rate limits and costs:

- OpenAI: Pay-per-token
- Anthropic: Pay-per-token
- Google: Free tier available

### Local Providers

No limits when running locally:

- Ollama: Unlimited
- LM Studio: Unlimited

## Tips

1. **Start with Gemini**: Free tier available, fast and capable for most tasks
2. **Use local models**: For sensitive code, use Ollama with Llama or Qwen
3. **Customize templates**: Better prompts = better results
4. **Test connections**: Verify API keys work before relying on AI features
5. **Monitor costs**: Cloud API calls add up quickly (except Gemini free tier)
6. **Try CLI mode**: For complex tasks, CLI tools often provide better results

## Troubleshooting

### API Key Invalid

- Verify the key is correct
- Check if the key has required permissions
- Ensure billing is active (for cloud providers)

### Slow Responses

- Try a smaller/faster model
- Check your internet connection
- Consider local models for faster responses

### Poor Quality Output

- Review and improve your prompt templates
- Try a more capable model
- Provide more context in templates

### CLI Tool Not Found

- Ensure the CLI is installed globally
- Check if the binary is in your PATH
- Try specifying a custom binary path in settings

### CLI Authentication Failed

- Run the CLI's auth command manually
- Or switch to API mode and use your own API key
