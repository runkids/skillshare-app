# Security & Privacy

Skillshare App is designed to be **local-first**: your projects stay on your machine, and AI/MCP features are opt-in and permissioned.

<!-- TODO: Add screenshot of Settings → Security / Permissions (if you have one). -->

## What Skillshare App Stores

### On Your Machine

- Imported project metadata (paths, detected scripts, git/worktree info)
- Workflows, step templates, webhook definitions
- Deployment accounts/config (when enabled)
- AI provider configuration (when enabled)
- MCP server settings + permission rules (when enabled)

### Where It’s Stored (macOS)

Skillshare App stores app data under the OS app data directory (typically):

- `~/Library/Application Support/com.skillshare-app.Skillshare App-macOS/`

> Note: the exact folder name may change between platforms/build variants.

## Secrets & Encryption

- API keys and tokens are encrypted at rest (AES-256-GCM)
- Secret values are not displayed after saving (where applicable)
- Logs and UI output are sanitized to reduce accidental leakage

<!-- TODO: Add screenshot showing “secret” variables / masked tokens UI. -->

## AI Providers (Opt-In)

If you enable an AI provider, requests may be sent to that provider (cloud) or kept local (local models). You control which provider is enabled and used.

Recommendations:

- Use **local models** (Ollama / LM Studio) for sensitive code or private repos
- Use **cloud models** for convenience, larger context, and best reasoning

## MCP Security Model

Skillshare App exposes an MCP server (`skillshare-mcp`) so AI tools can call actions.

### Permission Levels

- **Read Only**: view-only tools (safe default)
- **Execute with Confirm**: actions require your approval
- **Full Access**: actions run without prompts (use only with trusted setups)

### Fine-Grained Tool Permissions

You can allow/confirm/block individual tools (e.g. `run_workflow`, `run_npm_script`, `read_project_file`) depending on your risk tolerance.

### Request Logging

Skillshare App can log MCP requests (tool name, parameters, duration, result) so you can audit what your AI tool did.

<!-- TODO: Add screenshot of MCP logs panel. -->

## No Telemetry by Default

Skillshare App is built to avoid “phone home” analytics by default. Network access is primarily used for:

- AI provider calls (if enabled)
- Deployment providers (if enabled)
- Downloading updates/releases (if enabled)

## Reset / Data Removal

- Remove a project from Skillshare App to forget it (your files are not deleted)
- Disable AI/MCP if you don’t want any integrations
- To fully reset: delete the app data directory for Skillshare App

## Reporting Security Issues

If you discover a security vulnerability, please open a GitHub Issue with a minimal reproduction, or contact the maintainer privately if it’s sensitive.

