# Getting Started

This guide will help you install Skillshare App and get started with your first project.

Skillshare App is an AI-driven `package.json` project manager: import a folder once, then run scripts, manage Git/worktrees, automate workflows, and (optionally) let your AI tool control actions via MCP.

## Installation

### Homebrew (Recommended)

The easiest way to install Skillshare App on macOS:

```bash
brew tap runkids/tap
brew install --cask skillshare-app
```

#### Upgrading

```bash
brew update
brew upgrade --cask skillshare-app
```

### Manual Download

1. Go to the [Releases](https://github.com/runkids/skillshare-app/releases) page
2. Download the latest `.dmg` file
3. Open the DMG and drag Skillshare App to your Applications folder
4. Launch Skillshare App from Applications

## First Launch

When you first open Skillshare App, you'll see an empty project list.

## Importing Your First Project

There are two ways to add a project:

### Method 1: Drag and Drop

Simply drag any folder containing a `package.json` file into the Skillshare App window.

### Method 2: Click to Import

1. Click the **Import Project** button
2. Select a folder containing `package.json`
3. Skillshare App will scan and import the project

## Understanding the Interface

Once you've imported a project, you'll see:

### Main Areas

1. **Sidebar** - Project list and navigation
2. **Script Cards** - All npm scripts as clickable buttons
3. **Terminal Panel** - Live output from running scripts
4. **Status Bar** - Quick actions and system status

<p align="center">
  <img src="./screenshots/scripts.png" width="900" alt="Projects and script cards" />
</p>

## Running Your First Script

1. Click on a project in the sidebar
2. Find the script you want to run (e.g., `dev`, `build`, `test`)
3. Click the script card
4. Watch the output in the terminal panel

<p align="center">
  <img src="./screenshots/run-script.png" width="900" alt="Running an npm script with live output" />
</p>

## Key Shortcuts

| Shortcut | Action |
|----------|--------|
| <kbd>Cmd</kbd> + <kbd>K</kbd> | Quick switch worktrees |
| <kbd>Cmd</kbd> + <kbd>1</kbd> | Projects tab |
| <kbd>Cmd</kbd> + <kbd>2</kbd> | Workflows tab |
| <kbd>Cmd</kbd> + <kbd>,</kbd> | Settings |
| <kbd>Cmd</kbd> + <kbd>/</kbd> | Show all shortcuts |

## Next Steps

Now that you're set up, explore these features:

- [One-Click Scripts](./features/one-click-scripts.md) - Master script execution
- [Visual Workflow](./features/visual-workflow.md) - Automate multi-step tasks
- [Git Integration](./features/git-integration.md) - Visual Git operations
- [One-Click Deploy](./features/one-click-deploy.md) - Deploy with preview links
- [Time Machine](./features/time-machine.md) - Track dependency history and integrity
- [MCP Server](./features/mcp-server.md) - Let AI tools safely run actions for you
- [Security & Privacy](./security-and-privacy.md) - Understand local-first storage and permissions
