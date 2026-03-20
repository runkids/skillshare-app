# MCP Server

Let AI tools control Skillshare App through the Model Context Protocol (MCP).

## What is MCP?

The Model Context Protocol (MCP) is a standard for AI tools to interact with applications. Skillshare App can act as an MCP server, allowing AI assistants like:

- Claude Code
- Codex CLI
- Gemini CLI

to query and control Skillshare App programmatically.

## Overview

Skillshare App ships a companion MCP server binary, `skillshare-mcp`, and exposes tools that AI assistants can call:

- List projects
- Run scripts
- Execute workflows
- Trigger deployments
- And more

The MCP server uses **stdio transport** (no network port required). Skillshare App can generate ready-to-paste client configs that point at the correct `skillshare-mcp` binary path.

## Enabling MCP Server

1. Go to **Settings** → **MCP** → **MCP Integration**
2. Toggle **Enable MCP Server**
3. Configure the server settings
4. Click **Start Server**

Tip: In the same panel, Skillshare App shows the resolved `skillshare-mcp` path and provides config snippets for common MCP clients.

<p align="center">
  <img src="../screenshots/mcp-setup.png" width="900" alt="MCP Integration settings" />
</p>

## Client Setup (Copy/Paste)

### Claude Code / VS Code (JSON)

Skillshare App generates a config like:

```json
{
  "mcpServers": {
    "skillshare-app": {
      "command": "/Applications/Skillshare App.app/Contents/Resources/bin/skillshare-mcp"
    }
  }
}
```

### Codex CLI (TOML)

Skillshare App also generates:

```toml
[mcp_servers.skillshare-app]
command = "/Applications/Skillshare App.app/Contents/Resources/bin/skillshare-mcp"
```

## Permission Levels

Control what AI tools can do:

### Read Only

AI can only query information:
- List projects
- View workflows
- Check status

Cannot make changes or execute commands.

### Execute with Confirm

AI can request actions, but you must approve:
- A confirmation dialog appears
- You can approve or deny
- Safe for everyday use

### Full Access

AI can execute anything without confirmation:
- Use only with trusted AI tools
- Recommended only for personal automation

## Dev Server Mode

Control how dev server commands (like `npm run dev`) are handled when called via MCP:

### MCP Managed (Default)

- Dev servers run as background processes managed by MCP independently
- Processes can be monitored via `list_background_processes`
- Can be stopped via `stop_background_process`
- **Note**: These processes won't appear in Skillshare App's UI process manager

### UI Integrated (Recommended)

- Processes are tracked in Skillshare App UI via events
- Best of both worlds: AI can start processes, you see them in UI
- Provides better visibility with port tracking and process management
- **Use this if**: You want AI automation with full UI visibility

### Reject with Hint

- MCP will reject dev server commands
- Returns a helpful message suggesting to use Skillshare App UI instead
- Use this if you want all dev servers to be managed through Skillshare App's UI manually

**When to use each mode:**

| Mode | AI Can Start | Visible in UI | Best For |
|------|--------------|---------------|----------|
| MCP Managed | Yes | No | Fully autonomous AI workflows |
| UI Integrated | Yes | Yes | Balanced control & visibility |
| Reject with Hint | No | N/A | Manual-only process management |

## Tool Permissions

Fine-grained control over individual tools:

| Tool | Description | Risk Level |
|------|-------------|------------|
| `list_projects` | List all projects | Low |
| `get_project` | Get project details (scripts, workflows, git info) | Low |
| `read_project_file` | Read file content (security-limited) | Medium |
| `run_npm_script` | Run a package.json script | Medium |
| `run_workflow` | Run a workflow | Medium |
| `run_package_manager_command` | Install/update/audit/add/remove deps | Medium |
| `run_security_scan` | Run an audit scan (optional auto-fix) | Medium |
| `trigger_webhook` | Trigger a configured webhook action | Medium |

> Note: Skillshare App’s MCP server is designed to avoid “arbitrary shell execution” by default. Prefer higher-level tools like `run_npm_script`, `run_workflow`, and `run_package_manager_command`.

### Customizing Tool Access

1. Go to **Settings** → **MCP** → **Tool Permissions**
2. For each tool, set:
   - **Allowed**: Can be used
   - **Confirm**: Requires approval
   - **Blocked**: Cannot be used

## AI CLI Integration

### Supported AI CLIs

Skillshare App detects and integrates with:

| CLI | Detection |
|-----|-----------|
| Claude Code | `claude` command |
| Codex CLI | `codex` command |
| Gemini CLI | `gemini` command |

### Running AI Commands

1. Go to **Settings** → **AI CLI**
2. Select an installed CLI
3. Enter a prompt
4. Click **Run**

Output is displayed in the panel.

### Examples

**With Claude Code:**
```
"Deploy my project to Netlify staging"
```

**With Codex:**
```
"Run tests and fix any failures"
```

## MCP Tools Reference

### Project Tools

| Tool | Parameters | Returns |
|------|------------|---------|
| `list_projects` | `query?` | Array of projects |
| `get_project` | `path` | Project details |
| `get_project_dependencies` | `projectPath`, `includeDev?`, `includePeer?` | Dependencies |

### Git & Worktree Tools

| Tool | Parameters | Returns |
|------|------------|---------|
| `list_worktrees` | `projectPath` | Array of worktrees |
| `get_worktree_status` | `worktreePath` | Git status |
| `get_git_diff` | `worktreePath` | Staged changes diff |

### Workflow Tools

| Tool | Parameters | Returns |
|------|------------|---------|
| `list_workflows` | `projectId?` | Array of workflows |
| `get_workflow` | `workflowId` | Workflow details |
| `create_workflow` | `name`, `projectId?`, `description?` | New workflow |
| `add_workflow_step` | `workflowId`, `name`, `command`, `cwd?`, `timeout?` | Step added |
| `update_workflow` | `workflowId`, `name?`, `description?` | Updated workflow |
| `delete_workflow_step` | `workflowId`, `stepId` | Step removed |
| `run_workflow` | `workflowId`, `projectPath?` | Execution result |
| `get_workflow_execution_details` | `executionId`, `includeOutput?` | Execution logs |

### Script & NPM Tools

| Tool | Parameters | Returns |
|------|------------|---------|
| `run_npm_script` | `projectPath`, `scriptName`, `args?`, `timeoutMs?` | Execution result |
| `run_package_manager_command` | `projectPath`, `command`, `packages?`, `flags?`, `timeoutMs?` | Command result |
| `list_step_templates` | `category?`, `query?` | Array of templates |
| `create_step_template` | `name`, `command`, `category?`, `description?` | New template |

**run_package_manager_command** supports these commands:
- `install` / `i` - Install all dependencies
- `update` / `up` - Update dependencies
- `add` - Add packages (requires `packages` parameter)
- `remove` - Remove packages (requires `packages` parameter)
- `ci` - Clean install (frozen lockfile)
- `audit` - Security audit
- `outdated` - Check outdated packages
- `prune` - Remove unused packages
- `dedupe` - Deduplicate dependencies

### Background Process Tools

| Tool | Parameters | Returns |
|------|------------|---------|
| `get_background_process_output` | `processId`, `tailLines?` | Process output |
| `stop_background_process` | `processId`, `force?` | Stop result |
| `list_background_processes` | none | Array of processes |

### MCP Action Tools

| Tool | Parameters | Returns |
|------|------------|---------|
| `list_actions` | `actionType?`, `projectId?`, `enabledOnly?` | Array of actions |
| `get_action` | `actionId` | Action details |
| `run_script` | `actionId`, `args?`, `cwd?`, `env?` | Script result |
| `trigger_webhook` | `actionId`, `payload?`, `variables?` | Webhook result |
| `get_execution_status` | `executionId` | Execution status |
| `list_action_executions` | `actionId?`, `status?`, `actionType?`, `limit?` | Execution history |
| `get_action_permissions` | `actionId?` | Permission config |

### AI Assistant Tools

| Tool | Parameters | Returns |
|------|------------|---------|
| `list_ai_providers` | `enabledOnly?` | Array of providers |
| `list_conversations` | `projectPath?`, `limit?`, `searchQuery?` | Conversation list |

### Notification Tools

| Tool | Parameters | Returns |
|------|------------|---------|
| `get_notifications` | `category?`, `unreadOnly?`, `limit?` | Notifications |
| `mark_notifications_read` | `notificationIds?`, `markAll?` | Mark result |

### Security Tools

| Tool | Parameters | Returns |
|------|------------|---------|
| `get_security_scan_results` | `projectPath` | Scan results |
| `run_security_scan` | `projectPath`, `fix?` | Audit output |
| `check_dependency_integrity` | `projectPath`, `referenceSnapshotId?` | Integrity check result |
| `get_security_insights` | `projectPath` | Security overview with risk score |
| `export_security_report` | `projectPath`, `format` | Audit report (json/markdown/html) |

### Time Machine Tools

| Tool | Parameters | Returns |
|------|------------|---------|
| `list_execution_snapshots` | `workflowId`, `limit?` | Array of snapshots |
| `get_snapshot_details` | `snapshotId` | Snapshot with dependencies |
| `compare_snapshots` | `snapshotAId`, `snapshotBId` | Diff result |
| `search_snapshots` | `packageName?`, `projectPath?`, `fromDate?`, `toDate?`, `limit?` | Search results |
| `replay_execution` | `snapshotId`, `option`, `force?` | Replay result |

### Deployment Tools

| Tool | Parameters | Returns |
|------|------------|---------|
| `list_deployments` | `projectPath?`, `platform?`, `status?`, `limit?` | Deployment history |

### File Tools

| Tool | Parameters | Returns |
|------|------------|---------|
| `check_file_exists` | `projectPath`, `paths` | File existence map |
| `search_project_files` | `projectPath`, `pattern`, `maxResults?`, `includeDirectories?` | Matching files |
| `read_project_file` | `projectPath`, `filePath`, `maxLines?`, `startLine?` | File content |

### System Tools

| Tool | Parameters | Returns |
|------|------------|---------|
| `get_environment_info` | `includePaths?`, `projectPath?` | Environment info |

## Logs and Monitoring

### Request Logs

View all MCP requests:

1. Go to **Settings** → **MCP** → **Logs**
2. See:
   - Timestamp
   - Tool called
   - Parameters
   - Result
   - Duration

### Session Tracking

Each AI session is tracked:
- Session ID
- Connected AI tool
- Request count
- Duration

## Security Best Practices

1. **Start with Read Only**: Only escalate when needed
2. **Use confirmation mode**: For sensitive operations
3. **Review logs regularly**: Check what AI tools are doing
4. **Limit tool access**: Disable tools you don't need
5. **Local only**: Don't expose to network unless necessary

## Use Cases

### Automated Workflows

Let AI tools automate repetitive tasks:

```
"Every morning, pull latest changes and run tests for all projects"
```

### Voice-Controlled Development

Pair with voice AI for hands-free coding:

```
"Run the dev server for my blog project"
```

### CI/CD Integration

Use AI tools to manage deployments:

```
"Deploy the latest build to staging after tests pass"
```

## Troubleshooting

### Server Won't Start

- Ensure MCP is enabled in Skillshare App settings
- Make sure the bundled `skillshare-mcp` binary is available (Settings shows the resolved path)
- Check the MCP logs for startup errors

### AI Can't Connect

- Verify your MCP client is pointing at the `skillshare-mcp` command path
- Start with Read Only mode to validate basic connectivity
- Re-check the generated JSON/TOML snippet in Settings (it includes the correct path)

### Commands Failing

- Check tool permissions
- Review the error in logs
- Verify the requested resource exists
