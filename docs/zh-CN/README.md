# Skillshare App 文档

欢迎来到 Skillshare App 文档！本指南将帮助您充分利用 Skillshare App。

**其他语言**：[English](../README.md) | [繁體中文](../zh-TW/README.md)

## 快速链接

- [快速入门](./getting-started.md) - 安装、导入项目、运行第一个 script
- [MCP 服务器](./features/mcp-server.md) - 让 AI 工具安全地控制 Skillshare App
- [时间机器](./features/time-machine.md) - 依赖快照、完整性检查、安全洞察
- [安全与隐私](./security-and-privacy.md) - 本地优先、加密、权限模型
- [故障排查](./troubleshooting.md) - 常见问题与快速解决
- [功能指南](#功能) - 各功能深入说明

## 什么是 Skillshare App？

**别再切换终端标签页了。点一下就好。**

Skillshare App 是 Node.js 项目的可视化 DevOps 中心 — 一个 App 搞定脚本执行、Git 管理和部署。最强功能？**你的 AI 助手可以通过 MCP 控制它。**

**专为现代前端工作流程设计：**

- **React、Vue、Next.js、Nuxt** — 一键启动开发服务器、构建与部署
- **npm、pnpm、yarn、bun** — 自动检测包管理器
- **Monorepos** — 原生支持 Nx、Turborepo、Lerna
- **AI 辅助开发** — 生成提交信息、审查代码、分析安全性

**本地优先（Local-first）设计：**

- 项目数据与自动化保留在你的机器上
- 机密信息（token / API key）加密保存（AES-256-GCM）
- MCP 连接具备权限控制（只读 → 需确认 → 全权限）

## Skillshare App 适合谁？

- **前端开发者** — 厌倦了在多个终端窗口间切换
- **Vibe Coders** — 想保持心流状态，而非记忆 CLI 命令
- **团队** — 想要跨项目的一致工作流程
- **AI 优先开发者** — 使用 Claude Code、Codex 或 Gemini CLI

## 主要优势

| 之前 | 之后 |
|------|------|
| `cd project-a && npm run dev` × 5 个标签页 | 点一下，搞定。 |
| 手动部署，复制预览网址 | 一键部署 → 即时链接 |
| 「那个命令怎么打来着？」 | 可视化工作流，零记忆负担 |
| AI 只能读你的代码 | **AI 帮你执行脚本、部署、切换分支** |

## 功能

### 核心功能

| 功能 | 说明 | 文档 |
|------|------|------|
| **项目管理** | 导入、扫描和管理您的项目 | [阅读更多](./features/project-management.md) |
| **一键运行脚本** | 运行 npm/pnpm/yarn 脚本并实时显示输出 | [阅读更多](./features/one-click-scripts.md) |
| **可视化工作流** | 拖放式创建自动化流程 | [阅读更多](./features/visual-workflow.md) |
| **Monorepo 支持** | Nx、Turborepo、Lerna 集成 | [阅读更多](./features/monorepo-support.md) |

### Git 与版本控制

| 功能 | 说明 | 文档 |
|------|------|------|
| **Git 集成** | 可视化 Git 操作，无需 CLI | [阅读更多](./features/git-integration.md) |
| **Worktree 管理** | 使用快速切换器管理 Git worktree | [阅读更多](./features/worktree-management.md) |

### 部署与安全

| 功能 | 说明 | 文档 |
|------|------|------|
| **一键部署** | 部署至 Netlify、Cloudflare、GitHub Pages | [阅读更多](./features/one-click-deploy.md) |
| **安全扫描** | 可视化 npm audit 与漏洞详情 | [阅读更多](./features/security-audit.md) |
| **时间机器** | 依赖快照、差异分析、完整性与安全洞察 | [阅读更多](./features/time-machine.md) |

### AI 与自动化

| 功能 | 说明 | 文档 |
|------|------|------|
| **AI 集成** | 多供应商 AI（OpenAI、Anthropic、Gemini、Ollama） | [阅读更多](./features/ai-integration.md) |
| **MCP 服务器** | 让 Claude Code、Codex、Gemini CLI 控制 Skillshare App | [阅读更多](./features/mcp-server.md) |
| **Webhooks** | 传入/传出 webhook 自动化 | [阅读更多](./features/webhooks.md) |

### 工具与设置

| 功能 | 说明 | 文档 |
|------|------|------|
| **工具链管理** | Volta、Corepack、Node 版本检测 | [阅读更多](./features/toolchain-management.md) |
| **键盘快捷键** | 可自定义的快捷键参考 | [阅读更多](./features/keyboard-shortcuts.md) |

## 支持的技术

### 前端框架

React、Vue、Angular、Svelte、Solid、Next.js、Nuxt、Remix、Astro、Vite

### 包管理器

npm、pnpm、yarn、bun（从 lockfiles 自动检测）

### Monorepo 工具

Nx、Turborepo、Lerna、pnpm workspaces、yarn workspaces

### 部署平台

Netlify、Cloudflare Pages、GitHub Pages、Vercel（即将推出）

### AI 供应商

OpenAI、Anthropic、Google、Ollama、LM Studio

## 系统要求

- **平台**：macOS（Windows 和 Linux 即将推出）
- **Node.js**：18+（用于项目检测）

## 支持

- [GitHub Issues](https://github.com/runkids/skillshare-app/issues) - 错误报告与功能建议
- [Releases](https://github.com/runkids/skillshare-app/releases) - 下载最新版本
