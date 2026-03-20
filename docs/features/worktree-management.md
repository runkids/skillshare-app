# Worktree Management

Manage Git worktrees with ease. Work on multiple branches simultaneously without juggling directories.

## What are Worktrees?

Git worktrees let you check out multiple branches at once, each in its own directory. Instead of:

- Stashing changes
- Switching branches
- Losing context

You can have multiple branches open simultaneously.

## Overview

Skillshare App makes worktrees visual and easy to manage:

<p align="center">
  <img src="../screenshots/worktree.png" width="900" alt="Worktree list" />
</p>

<!-- TODO: Add a gif of the Cmd+K quick switcher in action. -->

## Viewing Worktrees

All worktrees are displayed in the Worktree panel showing:

- **Branch name**: The checked-out branch
- **Path**: Directory location
- **Status**: Clean, dirty, or ahead/behind
- **Last opened**: When you last worked on it

## Quick Switcher

The fastest way to switch between worktrees:

1. Press <kbd>Cmd</kbd> + <kbd>K</kbd>
2. Type to filter worktrees
3. Press Enter to switch

<!-- TODO: Add gif of quick switcher in action -->

The quick switcher shows:
- Worktree name
- Branch name
- Recent status
- Keyboard shortcut hints

## Creating Worktrees

### From Existing Branch

1. Click **New Worktree**
2. Select an existing branch
3. Choose the target directory
4. Click **Create**

### With New Branch

1. Click **New Worktree**
2. Toggle **Create new branch**
3. Enter the new branch name
4. Choose the base branch
5. Click **Create**

<!-- TODO: Add screenshot of create worktree dialog -->

## Worktree Templates

Save common worktree configurations as templates for quick creation.

### Creating a Template

1. Click **Templates** in the worktree panel
2. Click **New Template**
3. Configure:
   - Template name
   - Base branch pattern
   - Directory pattern
   - Auto-install dependencies option
4. Save the template

<!-- TODO: Add screenshot of template creation dialog -->

### Using a Template

1. Click **New from Template**
2. Select a template
3. Enter required values (e.g., feature name)
4. Click **Create**

### Example Templates

**Feature Branch**
- Base: `main`
- Branch pattern: `feature/{name}`
- Directory: `../worktrees/feature-{name}`

**Hotfix Branch**
- Base: `main`
- Branch pattern: `hotfix/{name}`
- Directory: `../worktrees/hotfix-{name}`

## Worktree Sessions

Skillshare App tracks your worktree sessions, remembering:

- Open terminals
- Running scripts
- UI state

<p align="center">
  <img src="../screenshots/worktree-session.png" width="900" alt="Worktree sessions" />
</p>

### Resume Session

When switching back to a worktree:

1. Skillshare App detects the previous session
2. Offers to restore your context
3. Reopens terminals and restores state

<!-- TODO: Add screenshot of session restore dialog -->

### Session List

View all sessions:

1. Click **Sessions** in the worktree panel
2. See all saved sessions
3. Click to restore or delete

## Syncing with Main Branch

Keep worktrees up to date with the main branch:

### Check Sync Status

The worktree card shows if you're behind the main branch.

<!-- TODO: Add screenshot showing behind status indicator -->

### Sync Options

1. Click **Sync** on a worktree
2. Choose sync method:
   - **Rebase**: Replay your commits on top of main
   - **Merge**: Create a merge commit

### Handling Conflicts

If conflicts occur during sync:

1. Skillshare App shows conflicted files
2. Resolve conflicts in the Git panel
3. Continue the rebase/merge

## Opening in External Tools

### Open in Editor

Right-click a worktree and select your editor:

- VS Code
- Cursor
- Sublime Text
- Vim
- Custom editor

<!-- TODO: Add screenshot of editor selection menu -->

### Open in Terminal

Open a worktree in your preferred terminal:

- Terminal.app
- iTerm2
- Custom terminal

## Deleting Worktrees

### Safe Delete

1. Right-click a worktree
2. Select **Delete**
3. Skillshare App checks for:
   - Uncommitted changes
   - Unpushed commits
4. Confirm deletion

### Force Delete

If you have uncommitted changes, you can force delete:

1. Check the **Force** option
2. Changes will be lost

> Warning: Force delete cannot be undone.

## Health Check

Skillshare App monitors worktree health:

### Checks Performed

- Branch still exists on remote
- No stale locks
- Directory is accessible
- Git repository is valid

### Fixing Issues

If problems are detected:

1. Click **Fix Issues** on the worktree
2. Skillshare App attempts automatic repair
3. Manual steps are shown if needed

<!-- TODO: Add screenshot of health check warnings -->

## Tips

1. **Use quick switcher**: <kbd>Cmd</kbd> + <kbd>K</kbd> is your friend
2. **Create templates**: Save time on repetitive patterns
3. **Sync regularly**: Avoid large merge conflicts
4. **Clean up old worktrees**: Delete worktrees for merged branches
5. **Use sessions**: Let Skillshare App remember your context

## Common Workflows

### Feature Development

1. Create worktree from template: `feature/{name}`
2. Develop your feature
3. Sync with main periodically
4. Push and create PR
5. Delete worktree after merge

### Code Review

1. Create worktree for the PR branch
2. Review and test
3. Leave comments
4. Delete worktree when done

### Hotfix

1. Create worktree from `main`: `hotfix/{name}`
2. Apply the fix
3. Push directly or create PR
4. Delete worktree after deployment
