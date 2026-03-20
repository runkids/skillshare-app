# Skillshare App Documentation

Welcome to the Skillshare App documentation! This guide will help you get the most out of Skillshare App.

## Quick Links

- [Getting Started](./getting-started.md) - Install, import a project, run your first script
- [MCP Server](./features/mcp-server.md) - Let AI tools control Skillshare App safely
- [Time Machine](./features/time-machine.md) - Dependency snapshots, integrity, security insights
- [Security & Privacy](./security-and-privacy.md) - Local-first storage, encryption, permissions
- [Troubleshooting](./troubleshooting.md) - Common issues and fixes
- [Feature Guides](#features) - Deep dives for each feature area

## What is Skillshare App?

**Stop juggling terminal tabs. Start clicking.**

Skillshare App is a visual DevOps hub for Node.js projects — one app to run scripts, manage Git, and deploy. The killer feature? **Your AI assistant can control it via MCP.**

**Built for the modern frontend workflow:**

- **React, Vue, Next.js, Nuxt** — Run dev servers, build, and deploy with one click
- **npm, pnpm, yarn, bun** — Automatic package manager detection
- **Monorepos** — Nx, Turborepo, Lerna native support
- **AI-assisted development** — Generate commits, review code, analyze security

**Local-first by design:**

- Project metadata and automation live on your machine
- Secrets (tokens / API keys) are encrypted at rest (AES-256-GCM)
- MCP access is permissioned (read-only → confirm → full access)

## Who is Skillshare App for?

- **Frontend developers** tired of juggling terminal windows
- **Vibe coders** who want to stay in flow, not memorize CLI commands
- **Teams** who want consistent workflows across projects
- **AI-first developers** using Claude Code, Codex, or Gemini CLI

## Key Benefits

| Before | After |
|--------|-------|
| `cd project-a && npm run dev` × 5 tabs | Click once. Done. |
| Manual deploy, copy preview URL | One-click deploy → instant link |
| "What was that command again?" | Visual workflows, zero memorization |
| AI can only read your code | **AI runs your scripts, deploys, switches branches** |

## Features

Use the docs below based on what you’re trying to do:

### Core Features

| Feature | Description | Documentation |
|---------|-------------|---------------|
| **Project Management** | Import, scan, and manage your projects | [Read more](./features/project-management.md) |
| **One-Click Scripts** | Run npm/pnpm/yarn scripts with live output | [Read more](./features/one-click-scripts.md) |
| **Visual Workflow** | Build automation flows with drag-and-drop | [Read more](./features/visual-workflow.md) |
| **Monorepo Support** | Nx, Turborepo, Lerna integration | [Read more](./features/monorepo-support.md) |

### Git & Version Control

| Feature | Description | Documentation |
|---------|-------------|---------------|
| **Git Integration** | Visual Git operations without CLI | [Read more](./features/git-integration.md) |
| **Worktree Management** | Manage Git worktrees with quick switcher | [Read more](./features/worktree-management.md) |

### Deployment & Security

| Feature | Description | Documentation |
|---------|-------------|---------------|
| **One-Click Deploy** | Deploy to Netlify, Cloudflare, GitHub Pages | [Read more](./features/one-click-deploy.md) |
| **Security Audit** | Visual npm audit with vulnerability details | [Read more](./features/security-audit.md) |
| **Time Machine** | Dependency snapshots, diffs, integrity & security insights | [Read more](./features/time-machine.md) |

### AI & Automation

| Feature | Description | Documentation |
|---------|-------------|---------------|
| **AI Integration** | Multi-provider AI (OpenAI, Anthropic, Gemini, Ollama) | [Read more](./features/ai-integration.md) |
| **MCP Server** | Let Claude Code, Codex, Gemini CLI control Skillshare App | [Read more](./features/mcp-server.md) |
| **Webhooks** | Incoming/outgoing webhook automation | [Read more](./features/webhooks.md) |

### Tools & Settings

| Feature | Description | Documentation |
|---------|-------------|---------------|
| **Toolchain Management** | Volta, Corepack, Node version detection | [Read more](./features/toolchain-management.md) |
| **Keyboard Shortcuts** | Customizable shortcuts reference | [Read more](./features/keyboard-shortcuts.md) |

## Supported Technologies

### Frontend Frameworks

React, Vue, Angular, Svelte, Solid, Next.js, Nuxt, Remix, Astro, Vite

### Package Managers

npm, pnpm, yarn, bun (auto-detected from lockfiles)

### Monorepo Tools

Nx, Turborepo, Lerna, pnpm workspaces, yarn workspaces

### Deployment Platforms

Netlify, Cloudflare Pages, GitHub Pages, Vercel (coming soon)

### AI Providers

OpenAI, Anthropic, Google, Ollama, LM Studio

## System Requirements

- **Platform**: macOS (Windows and Linux coming soon)
- **Node.js**: 18+ (for project detection)

## Support

- [GitHub Issues](https://github.com/runkids/skillshare-app/issues) - Bug reports and feature requests
- [Releases](https://github.com/runkids/skillshare-app/releases) - Download latest version
