# 快速入门

本指南将帮助您安装 Skillshare App 并开始使用您的第一个项目。

Skillshare App 是一个 AI 驱动的 `package.json` 项目管理工具：导入一次项目文件夹后，就能一键运行 scripts、管理 Git/worktree、创建工作流自动化，并且（可选）通过 MCP 让你的 AI 工具安全地调用动作。

## 安装

### Homebrew（推荐）

在 macOS 上安装 Skillshare App 最简单的方式：

```bash
brew tap runkids/tap
brew install --cask skillshare-app
```

#### 升级

```bash
brew update
brew upgrade --cask skillshare-app
```

### 手动下载

1. 前往 [Releases](https://github.com/runkids/skillshare-app/releases) 页面
2. 下载最新的 `.dmg` 文件
3. 打开 DMG 并将 Skillshare App 拖到应用程序文件夹
4. 从应用程序启动 Skillshare App

## 首次启动

首次打开 Skillshare App 时，您会看到空白的项目列表。

<!-- TODO: Add screenshot of empty project list / welcome screen -->

## 导入您的第一个项目

有两种方式可以添加项目：

### 方法 1：拖放

只需将包含 `package.json` 的文件夹拖到 Skillshare App 窗口中。

<!-- TODO: Add screenshot/gif of drag and drop import -->

### 方法 2：点击导入

1. 点击**导入项目**按钮
2. 选择包含 `package.json` 的文件夹
3. Skillshare App 将扫描并导入项目

## 了解界面

导入项目后，您会看到：

<!-- TODO: Add screenshot of main interface with annotations -->

### 主要区域

1. **侧边栏** - 项目列表与导航
2. **脚本卡片** - 所有 npm 脚本显示为可点击的按钮
3. **终端面板** - 运行中脚本的实时输出
4. **状态栏** - 快捷操作与系统状态

## 运行您的第一个脚本

1. 在侧边栏点击一个项目
2. 找到您要运行的脚本（例如 `dev`、`build`、`test`）
3. 点击脚本卡片
4. 在终端面板查看输出

<!-- TODO: Add gif of running a script -->

## 快捷键

| 快捷键 | 操作 |
|--------|------|
| <kbd>Cmd</kbd> + <kbd>K</kbd> | 快速切换 worktree |
| <kbd>Cmd</kbd> + <kbd>1</kbd> | 项目标签 |
| <kbd>Cmd</kbd> + <kbd>2</kbd> | 工作流标签 |
| <kbd>Cmd</kbd> + <kbd>,</kbd> | 设置 |
| <kbd>Cmd</kbd> + <kbd>/</kbd> | 显示所有快捷键 |

## 下一步

现在您已经设置完成，探索这些功能：

- [一键运行脚本](./features/one-click-scripts.md) - 精通脚本运行
- [可视化工作流](./features/visual-workflow.md) - 自动化多步骤任务
- [Git 集成](./features/git-integration.md) - 可视化 Git 操作
- [一键部署](./features/one-click-deploy.md) - 部署并获取预览链接
- [时间机器](./features/time-machine.md) - 追踪依赖历史与完整性
- [MCP 服务器](./features/mcp-server.md) - 让 AI 工具安全地帮你跑动作
- [安全与隐私](./security-and-privacy.md) - 了解本地优先、加密与权限
