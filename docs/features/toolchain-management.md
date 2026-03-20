# Toolchain Management

Detect and manage Node.js versions, package managers, and resolve version conflicts.

## Overview

Skillshare App helps you manage your development toolchain:

- Node.js version detection
- Package manager detection
- Version manager integration (Volta, nvm)
- Conflict resolution

<!-- TODO: Add screenshot of toolchain panel -->

## Version Detection

### Node.js

Skillshare App detects:
- Installed Node.js version
- Project's required version (from `package.json` → `engines`)
- Version manager configuration

### Package Managers

Detected automatically:
- npm
- yarn
- pnpm
- bun

Detection is based on lock files:

| Lock File | Package Manager |
|-----------|-----------------|
| `package-lock.json` | npm |
| `yarn.lock` | yarn |
| `pnpm-lock.yaml` | pnpm |
| `bun.lockb` | bun |

## Version Manager Support

### Volta

Skillshare App integrates deeply with Volta:

**Features:**
- Read Volta config from `package.json`
- Display pinned Node/npm/yarn versions
- Detect Volta installation
- Generate Volta-compatible commands

**Volta Config:**
```json
{
  "volta": {
    "node": "20.10.0",
    "npm": "10.2.3"
  }
}
```

### nvm

Support for nvm configuration:

**Features:**
- Read `.nvmrc` files
- Display required Node version
- Show if current version matches

**.nvmrc:**
```
20.10.0
```

### Corepack

Skillshare App detects Corepack status:

**Features:**
- Check if Corepack is enabled
- Detect package manager versions
- Show Corepack warnings

## Conflict Detection

Skillshare App automatically detects common conflicts:

### Volta + Corepack Conflict

Using both Volta and Corepack can cause issues:

- Both try to manage package managers
- Can lead to version mismatches
- Performance degradation

**Resolution:**
1. Skillshare App shows a warning
2. Choose one: Volta OR Corepack
3. Follow the recommended steps

<!-- TODO: Add screenshot of conflict warning dialog -->

### PNPM_HOME Conflict

When `PNPM_HOME` conflicts with Volta:

**Symptoms:**
- pnpm commands fail
- Version mismatch errors

**Resolution:**
1. Skillshare App identifies the conflict
2. Shows environment variables involved
3. Provides fix commands

### Version Mismatch

When your Node version doesn't match the project:

**Example:**
- Project requires: Node 20.x
- Installed: Node 18.x

**Resolution:**
1. Warning is shown on project card
2. Click to see details
3. Follow upgrade instructions

<!-- TODO: Add screenshot of version mismatch warning -->

## Diagnostics

### Environment Diagnostics

View your complete toolchain:

1. Go to **Settings** → **Toolchain**
2. Click **Run Diagnostics**
3. See:
   - Node.js version and path
   - npm/yarn/pnpm versions
   - Environment variables
   - Version manager status

<!-- TODO: Add screenshot of diagnostics panel -->

### Project Diagnostics

For each project:

1. Click the toolchain icon on a project
2. See project-specific info:
   - Required versions
   - Current versions
   - Compatibility status
   - Conflicts

## Package Manager Preferences

### Setting Default

Choose your preferred package manager:

1. Go to **Settings** → **Toolchain**
2. Select **Default Package Manager**
3. Choose: npm, yarn, pnpm, or bun

### Per-Project Override

Skillshare App respects project-specific settings:
- Lock file determines the package manager
- Volta config takes precedence
- Manual override available

## Version Badges

Project cards show version status:

| Badge | Meaning |
|-------|---------|
| Green | Versions match |
| Yellow | Minor mismatch |
| Red | Major mismatch or conflict |

<!-- TODO: Add screenshot of version badges on project cards -->

## Commands

Skillshare App generates the correct commands based on your toolchain:

### With Volta

```bash
volta run npm install
volta run npm run build
```

### With Corepack

```bash
corepack enable
pnpm install
pnpm run build
```

### Without Version Manager

```bash
npm install
npm run build
```

## Tips

1. **Use Volta OR Corepack**: Don't mix them
2. **Pin versions**: Use Volta or `.nvmrc` for consistency
3. **Check diagnostics**: When something seems wrong
4. **Update regularly**: Keep Node.js and package managers current
5. **Review conflicts**: Address warnings promptly

## Troubleshooting

### Wrong Node Version Used

1. Check version managers (Volta, nvm)
2. Verify PATH order
3. Restart terminal after changes

### Package Manager Not Found

1. Check if it's installed globally
2. Verify Corepack status
3. Check PATH environment variable

### Slow Package Installation

1. Check for Volta + Corepack conflict
2. Verify network connection
3. Try clearing cache
