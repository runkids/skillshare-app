# One-Click Scripts

Turn your npm scripts into clickable buttons with live terminal output.

## Overview

Every script in your `package.json` becomes a clickable card in Skillshare App. No more typing commands — just click and watch.

<p align="center">
  <img src="../screenshots/scripts.png" width="900" alt="Script cards" />
</p>

<!-- TODO: Add a short gif of clicking a script card and watching output. -->

## Running Scripts

### Basic Execution

1. Select a project from the sidebar
2. Find the script card (e.g., `dev`, `build`, `test`)
3. Click the card to run

The terminal panel opens automatically, showing live output.

<!-- TODO: Add gif of clicking a script and seeing output -->

### Script States

| State | Indicator | Description |
|-------|-----------|-------------|
| Ready | Default | Script is ready to run |
| Running | Spinner | Script is currently executing |
| Success | Green check | Script completed successfully |
| Failed | Red X | Script exited with error |

## Terminal Features

### PTY (Pseudo-Terminal)

Skillshare App uses PTY for true terminal emulation:

- **Colors**: Full ANSI color support
- **Interactive**: Supports interactive prompts
- **Cursor**: Proper cursor movement
- **Unicode**: Full unicode support

### Output Buffer

- Maximum buffer size: 1MB
- Older output is automatically trimmed
- Clear terminal with the clear button

<!-- TODO: Add screenshot of terminal with colorful output -->

## Stopping Scripts

### Single Script

Click the **Stop** button on a running script card, or press the stop icon in the terminal panel.

### All Scripts

Use the **Stop All** button in the status bar to terminate all running scripts at once.

## Port Management

Skillshare App automatically detects ports used by your scripts.

### Port Conflict Detection

If a script tries to use a port that's already in use:

1. Skillshare App detects the conflict
2. Shows which process is using the port
3. Offers to kill the conflicting process

<!-- TODO: Add screenshot of port conflict dialog -->

### Check Ports

Before running dev servers, you can:

1. Right-click a project
2. Select **Check Ports**
3. See which ports are currently in use

### Kill Ports

To free up specific ports:

1. Right-click a project
2. Select **Kill Ports**
3. Enter the port numbers to kill
4. Confirm the action

## Script Categories

Scripts are automatically organized by common patterns:

| Category | Scripts |
|----------|---------|
| Development | `dev`, `start`, `serve` |
| Build | `build`, `compile` |
| Testing | `test`, `test:*`, `jest`, `vitest` |
| Linting | `lint`, `eslint`, `prettier` |
| Other | Everything else |

<!-- TODO: Add screenshot showing script categories -->

## Environment Variables

Scripts run with your system's environment variables. To customize:

1. Create a `.env` file in your project root
2. Skillshare App will load these variables automatically

## Running Multiple Scripts

You can run multiple scripts simultaneously:

1. Click the first script to start it
2. Click additional scripts while others are running
3. Each script gets its own terminal tab

<!-- TODO: Add screenshot of multiple terminal tabs -->

## Tips

1. **Use keyboard shortcuts**: Learn the shortcuts for frequently used scripts
2. **Watch the status bar**: It shows the count of running scripts
3. **Check port conflicts**: Run "Check Ports" before starting dev servers
4. **Clear terminal regularly**: Keep the output clean for easier debugging
