# MCP 服务器

让 AI 工具通过 Model Context Protocol（MCP）控制 Skillshare App。

## 什么是 MCP？

Model Context Protocol（MCP）是 AI 工具与应用程序交互的标准。Skillshare App 可以作为 MCP 服务器，允许 AI 助手如：

- Claude Code
- Codex CLI
- Gemini CLI

以编程方式查询和控制 Skillshare App。

## 概览

启用后，Skillshare App 公开 AI 助手可以调用的工具：

- 列出项目
- 运行脚本
- 运行工作流
- 触发部署
- 还有更多

<!-- TODO: Add diagram of MCP architecture -->

## 启用 MCP 服务器

1. 前往**设置** → **MCP**
2. 切换**启用 MCP 服务器**
3. 配置服务器设置
4. 点击**启动服务器**

<!-- TODO: Add screenshot of MCP settings panel -->

## 服务器配置

### 端口

默认：`7234`

如果端口已被使用请更改。

### 主机

默认：`localhost`

基于安全考虑，默认只允许本地连接。

## 权限等级

控制 AI 工具可以做什么：

### 只读

AI 只能查询信息：
- 列出项目
- 查看工作流
- 检查状态

无法进行更改或运行命令。

### 需确认运行

AI 可以请求操作，但您必须批准：
- 出现确认对话框
- 您可以批准或拒绝
- 日常使用安全

### 完整访问

AI 可以无需确认运行任何操作：
- 仅与信任的 AI 工具使用
- 仅建议用于个人自动化

<!-- TODO: Add screenshot of permission level selector -->

## 工具权限

对单独工具的细粒度控制：

| 工具 | 说明 | 风险等级 |
|------|------|----------|
| `list_projects` | 列出所有项目 | 低 |
| `get_project` | 获取项目详情（scripts、workflows、git） | 低 |
| `read_project_file` | 读取文件内容（有安全限制） | 中 |
| `run_npm_script` | 运行 package.json script | 中 |
| `run_workflow` | 运行工作流 | 中 |
| `run_package_manager_command` | 安装/更新/审计/新增/移除依赖 | 中 |
| `run_security_scan` | 运行安全审计（可选自动修复） | 中 |
| `trigger_webhook` | 触发已配置的 webhook action | 中 |

> 提示：Skillshare App 的 MCP 设计上会避免默认暴露「任意 shell 执行」，建议优先使用 `run_npm_script` / `run_workflow` / `run_package_manager_command` 等高阶工具。

### 自定义工具访问

1. 前往**设置** → **MCP** → **工具权限**
2. 对每个工具设置：
   - **允许**：可以使用
   - **确认**：需要批准
   - **封锁**：无法使用

<!-- TODO: Add screenshot of tool permission matrix -->

## AI CLI 集成

### 支持的 AI CLI

Skillshare App 检测并集成：

| CLI | 检测 |
|-----|------|
| Claude Code | `claude` 命令 |
| Codex CLI | `codex` 命令 |
| Gemini CLI | `gemini` 命令 |

### 运行 AI 命令

1. 前往**设置** → **AI CLI**
2. 选择已安装的 CLI
3. 输入提示
4. 点击**运行**

输出显示在面板中。

<!-- TODO: Add screenshot of AI CLI panel -->

### 示例

**使用 Claude Code：**
```
"将我的项目部署到 Netlify staging"
```

**使用 Codex：**
```
"运行测试并修复任何失败"
```

## MCP 工具参考

Skillshare App 通过 `skillshare-mcp` 提供多个工具供 AI 调用。工具清单会随版本调整，建议以 App 内的 **Settings → MCP → Tool Permissions** 为准。

常用工具（节选）：

| 工具 | 用途 |
|------|------|
| `list_projects` | 列出已注册的项目 |
| `get_project` | 获取项目详情（scripts、workflows、git） |
| `run_npm_script` | 运行 npm/yarn/pnpm script |
| `run_workflow` | 运行工作流 |
| `read_project_file` | 读取文件内容（有安全限制） |

完整工具与参数请参考：`docs/features/mcp-server.md`（英文）或 App 内的工具清单。

## 日志与监控

### 请求日志

查看所有 MCP 请求：

1. 前往**设置** → **MCP** → **日志**
2. 查看：
   - 时间戳
   - 调用的工具
   - 参数
   - 结果
   - 持续时间

<!-- TODO: Add screenshot of MCP logs -->

### 会话跟踪

跟踪每个 AI 会话：
- 会话 ID
- 连接的 AI 工具
- 请求数量
- 持续时间

## 安全最佳实践

1. **从只读开始**：仅在需要时提升
2. **使用确认模式**：用于敏感操作
3. **定期查看日志**：检查 AI 工具在做什么
4. **限制工具访问**：停用不需要的工具
5. **仅限本地**：除非必要，不要暴露到网络

## 使用案例

### 自动化工作流

让 AI 工具自动化重复任务：

```
"每天早上拉取最新变更并为所有项目运行测试"
```

### 语音控制开发

与语音 AI 配对进行免手操作编程：

```
"为我的博客项目运行开发服务器"
```

### CI/CD 集成

使用 AI 工具管理部署：

```
"测试通过后将最新构建部署到 staging"
```

## 疑难排解

### 服务器无法启动

- 检查端口是否被使用
- 尝试不同的端口
- 确保 Skillshare App 有网络权限

### AI 无法连接

- 验证服务器正在运行
- 检查端口号
- 确保防火墙允许本地连接

### 命令失败

- 检查工具权限
- 查看日志中的错误
- 验证请求的资源存在
