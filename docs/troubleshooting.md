# Troubleshooting

Common issues and quick fixes.

## Installation

### Homebrew install fails

- Run `brew update` then retry `brew install --cask skillshare-app`
- If the tap is missing: `brew tap runkids/tap`

### macOS blocks the app (Gatekeeper)

If macOS warns the app is from an unidentified developer:

1. Open **System Settings** → **Privacy & Security**
2. Find the blocked app message
3. Click **Open Anyway**

<!-- TODO: Add screenshot of macOS “Open Anyway” flow. -->

## Project Import

### Project won’t import

- Ensure the folder contains a `package.json` at its root
- Make sure Skillshare App has permission to access that folder (System Settings → Privacy & Security)
- Try importing a smaller repo first to validate basic behavior

### Scripts don’t show up / outdated

- Confirm scripts exist under `package.json#scripts`
- Right-click the project and **Refresh** (if available)
- If using workspaces/monorepos, check the Monorepo docs: `docs/features/monorepo-support.md`

## Script / Workflow Execution

### “Command not found” / wrong Node version

- Check **Toolchain Management**: `docs/features/toolchain-management.md`
- If using Volta/Corepack/nvm, ensure your repo is configured consistently

### Dev server starts but you can’t reach it

- Verify which port is used in the terminal output
- Check for port conflicts with another process
- If using MCP to start dev servers, confirm the **Dev Server Mode** in MCP settings

## MCP Server

### AI client can’t connect

- In Skillshare App, open **Settings → MCP → MCP Integration** and copy the generated config
- Ensure your MCP client points to the exact `skillshare-mcp` path shown in Settings
- Start with **Read Only** mode to validate connectivity

### Claude Desktop / VS Code config path confusion

Skillshare App shows the correct file path hints in the MCP quick setup UI.

<!-- TODO: Add screenshot of MCP quick setup section (paths + copy buttons). -->

## Deploy

### Deploy fails immediately

- Verify account connection under **Settings → Deploy Accounts**
- Confirm the build command and output directory are correct
- Ensure required environment variables are set (and set to the right environment: preview vs production)

## Still Stuck?

- Check `docs/getting-started.md`
- Search existing issues: https://github.com/runkids/skillshare-app/issues
- Open a new issue with:
  - macOS version
  - Skillshare App version
  - Steps to reproduce
  - Screenshots/logs (redact secrets)

