# Time Machine & Security Guardian

Skillshare App's Time Machine feature automatically captures dependency snapshots when your lockfile changes, enabling you to track dependency evolution, detect security issues, and compare states over time.

## Overview

The Time Machine provides:
- **Automatic Snapshots**: Capture dependency state when lockfile changes (debounced)
- **Manual Snapshots**: Capture current state on-demand
- **Security Guardian**: Real-time detection of suspicious packages and postinstall scripts
- **Diff Analysis**: Compare snapshots to track what changed between captures
- **Integrity Checking**: Verify dependency integrity against snapshots

## Access

Time Machine is accessed via the **Snapshots** tab in Project Explorer:

```
Project Explorer Tabs:
Scripts | Workspaces | Workflows | Git | Builds | Security | Deploy | Snapshots
                                                                        ↑
```

You can also use the **Snapshots** quick-access button in the project header (cyan-colored button).

## Features

### 1. Automatic Snapshot Capture

When a project's lockfile changes, Skillshare App automatically:
- Detects lockfile modification (package-lock.json, pnpm-lock.yaml, yarn.lock, bun.lockb)
- Waits for debounce period (2 seconds by default)
- Parses lockfile and extracts dependency tree
- Detects postinstall scripts
- Calculates security score
- Compresses and stores snapshot data

**Trigger Source Types:**
- `lockfile_change` - Automatic capture when lockfile changes
- `manual` - User-initiated capture via UI or AI Assistant

### 2. Snapshot Timeline

View all snapshots for a project in chronological order:
- Filter by date range or trigger source
- See security scores at a glance
- Identify snapshots with postinstall scripts
- Quick access to diff comparison

### 3. Dependency Diff View

Compare any two snapshots to see:
- Added/removed/updated packages
- Version changes with semantic versioning analysis
- New or changed postinstall scripts
- Security score changes

### 4. Security Guardian

Automated security analysis includes:

#### Typosquatting Detection
Identifies packages with names similar to popular packages:
- `lodahs` vs `lodash`
- `reqeust` vs `request`
- Uses Levenshtein distance algorithm

#### Postinstall Script Monitoring
- Tracks all packages with postinstall scripts
- Alerts when new postinstall scripts appear
- Shows script content changes between snapshots

#### Suspicious Pattern Detection
- Major version jumps (e.g., 1.0.0 → 9.0.0)
- Unexpected version downgrades
- Suspicious package naming patterns

### 5. Integrity Checking

Verify current dependency state against snapshots:
- Compare current lockfile hash with captured hash
- Detect drift from expected state
- Identify unexpected changes

### 6. Security Insights Dashboard

Project-level security overview:
- Overall risk score (0-100)
- Insight summary by severity
- Frequently updated packages
- Typosquatting alerts history

### 7. Searchable History

Search across all snapshots:
- By package name or version
- Filter by date range
- Filter by postinstall presence
- Filter by minimum security score

## Settings

Configure Time Machine in **Settings > Storage**:

### Auto-Watch
Toggle automatic lockfile monitoring for all projects. When enabled, Skillshare App watches lockfiles and captures snapshots on changes.

### Debounce
Set the debounce period (default: 2000ms) to prevent rapid successive captures during install operations.

## Storage Management

Snapshots are stored in:
```
~/Library/Application Support/com.skillshare.app/time-machine/snapshots/
```

Each snapshot includes:
- Compressed lockfile (.zst)
- Compressed package.json (.zst)
- Dependency tree JSON
- Postinstall manifest

### Retention Settings

Configure snapshot retention in Settings > Storage:
- Set maximum snapshots per project
- Manually prune old snapshots
- Cleanup orphaned storage files

## MCP Tools

Time Machine integrates with the MCP server, providing AI assistants access to:

| Tool | Description |
|------|-------------|
| `list_snapshots` | List snapshots for a project |
| `capture_snapshot` | Manually capture a snapshot |
| `get_snapshot_details` | Get full snapshot with dependencies |
| `compare_snapshots` | Diff two snapshots |
| `search_snapshots` | Search across all snapshots |
| `check_dependency_integrity` | Check for drift from latest snapshot |
| `get_security_insights` | Get project security overview |
| `export_security_report` | Export audit report |

## AI Assistant Quick Actions

In the AI Assistant, Time Machine quick actions are available:
- **Capture Snapshot** - Capture current dependency state
- **View Snapshots** - Open Snapshots tab
- **Check Integrity** - Verify dependency integrity

## Best Practices

1. **Enable Auto-Watch**: Keep automatic monitoring enabled for important projects
2. **Monitor Postinstall**: Pay attention to new postinstall scripts
3. **Investigate Typosquatting**: Always verify suspicious package names
4. **Regular Comparison**: Compare snapshots after major dependency updates
5. **Prune Regularly**: Keep storage usage reasonable with retention settings

## Security Score Calculation

The security score (0-100) considers:
- Number of postinstall scripts (higher = riskier)
- Typosquatting suspects
- Known vulnerability patterns
- Dependency tree depth and complexity

| Score Range | Risk Level |
|-------------|------------|
| 80-100 | Low |
| 60-79 | Medium |
| 40-59 | High |
| 0-39 | Critical |

## Related Features

- [Security Audit](./security-audit.md)
- [Project Management](./project-management.md)
- [MCP Server](./mcp-server.md)
- [AI Integration](./ai-integration.md)
