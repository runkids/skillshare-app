# 故障排查

常见问题与快速解决。

## 安装

### Homebrew 安装失败

- 先执行 `brew update`，再重试 `brew install --cask skillshare-app`
- 如果 tap 不存在：`brew tap runkids/tap`

### macOS 阻止 App（Gatekeeper）

如果 macOS 提示应用来自未知开发者：

1. 打开 **系统设置** → **隐私与安全性**
2. 找到被阻止的 App 提示
3. 点击 **仍要打开**

<!-- TODO: Add screenshot of macOS “Open Anyway” flow. -->

## 导入项目

### 项目无法导入

- 确认文件夹根目录包含 `package.json`
- 确认 Skillshare App 具有该文件夹的访问权限（系统设置 → 隐私与安全性）
- 先尝试导入较小的 repo，以验证基本流程

### Scripts 没显示 / 未更新

- 确认 `package.json#scripts` 中确实存在 scripts
- 在项目上右键并 **Refresh**（如界面提供）
- 若是 monorepo/workspaces，请参考：`docs/zh-CN/features/monorepo-support.md`

## Script / 工作流执行

### “Command not found” / Node 版本不对

- 先检查工具链管理：`docs/zh-CN/features/toolchain-management.md`
- 如果你使用 Volta/Corepack/nvm，请确认 repo 配置一致

### Dev server 启动了但浏览器访问不到

- 查看终端输出使用的端口
- 检查是否有端口冲突
- 若通过 MCP 启动 dev server，请确认 MCP 设置里的 **Dev Server Mode**

## MCP 服务器

### AI 客户端无法连接

- 在 Skillshare App 打开 **Settings → MCP → MCP Integration**，复制生成的配置
- 确认你的 MCP 客户端指向 Settings 显示的 `skillshare-mcp` 路径
- 建议先用 **Read Only** 模式验证连接

### Claude Desktop / VS Code 配置路径搞不清楚

Skillshare App 的 MCP 快速设置区域会显示正确路径提示。

<!-- TODO: Add screenshot of MCP quick setup section (paths + copy buttons). -->

## 部署

### 部署一开始就失败

- 在 **Settings → Deploy Accounts** 确认账号连接
- 确认 build command 与 output 目录配置正确
- 确认所需环境变量已设置（并区分 preview / production）

## 还是卡住？

- 先看：`docs/zh-CN/getting-started.md`
- 搜索已有 issues：https://github.com/runkids/skillshare-app/issues
- 新建 issue 时请提供：
  - macOS 版本
  - Skillshare App 版本
  - 复现步骤
  - 截图/日志（请先脱敏）

