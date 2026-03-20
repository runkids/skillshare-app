# Project Management

Skillshare App automatically detects and manages your JavaScript/TypeScript projects, providing a visual interface for all your development tasks.

## Importing Projects

### Drag and Drop

The quickest way to add a project is to drag a folder into Skillshare App.

<p align="center">
  <img src="../screenshots/scripts.png" width="900" alt="Projects list and script cards" />
</p>

<!-- TODO: Add gif of drag-and-drop import. -->

### Import Button

Click the **Import Project** button in the sidebar to browse and select a project folder.

### Requirements

- The folder must contain a `package.json` file
- Skillshare App will scan the directory for project metadata

## Automatic Framework Detection

Skillshare App automatically identifies your project's framework and tools:

### Supported Frameworks

| Framework | Detection Method |
|-----------|-----------------|
| React | Dependencies containing `react` |
| Vue | Dependencies containing `vue` |
| Next.js | `next` in dependencies |
| Nuxt | `nuxt` in dependencies |
| Remix | `@remix-run/*` in dependencies |
| Angular | `@angular/core` in dependencies |
| Svelte | `svelte` in dependencies |
| Expo | `expo` in dependencies |
| React Native | `react-native` in dependencies |
| Electron | `electron` in dependencies |
| Tauri | `@tauri-apps/*` in dependencies |

<!-- TODO: Add screenshot showing framework badges on project cards -->

### UI Libraries

Skillshare App also detects UI frameworks:

- React
- Vue
- Svelte
- Solid
- Preact
- Lit
- Qwik

## Project Information

For each project, Skillshare App displays:

### Basic Info
- **Project name** - From `package.json`
- **Version** - Current version number
- **Path** - Project directory location
- **Framework** - Detected framework badge

### Scripts
- All scripts defined in `package.json`
- Displayed as clickable cards

### Dependencies
- Production dependencies count
- Development dependencies count
- Peer dependencies (if any)

<!-- TODO: Add screenshot of project details panel -->

## Managing Projects

### Remove a Project

To remove a project from Skillshare App:

1. Right-click on the project in the sidebar
2. Select **Remove Project**
3. Confirm the action

> Note: This only removes the project from Skillshare App. Your files are not deleted.

### Delete node_modules

To free up disk space:

1. Right-click on the project
2. Select **Delete node_modules**
3. Confirm the deletion

This is useful for cleaning up projects you're not actively working on.

<!-- TODO: Add screenshot of context menu with delete node_modules option -->

## Workspace Packages (Monorepo)

If your project uses workspaces (npm, yarn, or pnpm), Skillshare App will:

1. Detect the workspace configuration
2. List all packages in the workspace
3. Allow you to run scripts in individual packages

See [Monorepo Support](./monorepo-support.md) for more details.

## Project Refresh

Skillshare App watches for changes to `package.json`. When changes are detected:

- Scripts are automatically updated
- Dependencies are recalculated
- Framework detection is refreshed

You can also manually refresh by right-clicking a project and selecting **Refresh**.

## Tips

1. **Organize by folders**: Keep related projects in the same parent directory for easy batch import
2. **Use worktrees**: For projects with multiple branches, use [Worktree Management](./worktree-management.md) instead of cloning multiple times
3. **Clean regularly**: Use the "Delete node_modules" feature to free up space on projects you're not actively using
