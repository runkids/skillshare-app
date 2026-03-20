# Monorepo Support

Skillshare App provides first-class support for monorepo tools including Nx, Turborepo, Lerna, and native workspaces.

## Overview

Monorepos are automatically detected when you import a project. Skillshare App identifies:

- Workspace configuration
- All packages in the monorepo
- Build tools (Nx, Turbo, Lerna)
- Dependency relationships

<p align="center">
  <img src="../screenshots/monorepo-support.png" width="900" alt="Monorepo view" />
</p>

<p align="center">
  <img src="../screenshots/dependency-graph.png" width="900" alt="Dependency graph" />
</p>

<!-- TODO: Add a close-up screenshot of the Nx/Turbo panels. -->

## Nx Support

### Detection

Skillshare App detects Nx projects by:
- `nx.json` in the root
- `@nx/*` packages in dependencies

### Features

**Target Detection**
- All Nx targets are automatically discovered
- Targets are displayed as runnable buttons

**Run Targets**
```
nx run <project>:<target>
```

Click a target card to run it on a specific project.

<!-- TODO: Add screenshot of Nx targets panel -->

**Dependency Graph**

View the project dependency graph:

1. Click **Show Graph** in the Nx panel
2. Interactive visualization opens
3. Click nodes to see dependencies

<!-- TODO: Add screenshot of Nx dependency graph -->

**Cache Management**

- View cache status
- Clear Nx cache with one click
- See cache hit rates

## Turborepo Support

### Detection

Skillshare App detects Turbo projects by:
- `turbo.json` in the root
- `turbo` in dependencies

### Features

**Pipeline Detection**

All pipelines defined in `turbo.json` are discovered and displayed.

**Run Tasks**
```
turbo run <task>
```

Click a task to run it across all packages.

<!-- TODO: Add screenshot of Turbo pipeline panel -->

**Filtering**

Run tasks on specific packages:

1. Select packages in the filter bar
2. Click the task to run
3. Only selected packages are affected

**Cache Management**

- View remote cache status
- Clear local cache
- Toggle remote caching

## Lerna Support

### Detection

Skillshare App detects Lerna projects by:
- `lerna.json` in the root
- `lerna` in dependencies

### Features

- List all packages
- Run commands across packages
- Version management support

## Native Workspaces

Skillshare App supports workspaces from:

| Package Manager | Config Location |
|-----------------|-----------------|
| npm | `package.json` → `workspaces` |
| yarn | `package.json` → `workspaces` |
| pnpm | `pnpm-workspace.yaml` |

### Features

**Package List**

All workspace packages are listed with:
- Package name
- Version
- Location
- Available scripts

<!-- TODO: Add screenshot of workspace packages list -->

**Run in Package**

Run scripts in specific packages:

1. Select a package
2. View its scripts
3. Click to run

**Batch Operations**

Run the same script across all packages:

1. Select multiple packages
2. Choose a common script
3. Run in parallel or sequence

## Task Quick Switcher

Quickly find and run tasks across your monorepo:

1. Press <kbd>Cmd</kbd> + <kbd>Shift</kbd> + <kbd>P</kbd>
2. Type the task name
3. Select from filtered results
4. Press Enter to run

<!-- TODO: Add screenshot of task quick switcher -->

## Package Filter Bar

Filter packages by various criteria:

- **Name**: Search by package name
- **Path**: Filter by directory
- **Changed**: Show only changed packages
- **Private**: Show/hide private packages

<!-- TODO: Add screenshot of package filter bar -->

## Tips

1. **Use the dependency graph**: Understand relationships before making changes
2. **Clear cache when stuck**: Stale cache can cause confusing issues
3. **Filter for speed**: Run tasks only on affected packages
4. **Check before publish**: Use Lerna's version commands through Skillshare App

## Troubleshooting

### Tasks not showing

- Ensure your config file is valid (`nx.json`, `turbo.json`, etc.)
- Try refreshing the project

### Slow task execution

- Check if caching is properly configured
- Verify remote cache connection (Turbo)

### Package not detected

- Verify the package has a valid `package.json`
- Check workspace configuration patterns
