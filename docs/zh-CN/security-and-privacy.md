# 安全与隐私

Skillshare App 以 **本地优先（local-first）** 为设计核心：你的项目数据留在你的机器上；AI / MCP 功能为可选，并且具备权限控制。

<!-- TODO: Add screenshot of Settings → Security / Permissions (if you have one). -->

## Skillshare App 会存储什么？

### 在你的机器上

- 导入的项目元数据（路径、检测到的 scripts、git/worktree 信息）
- 工作流、步骤模板、webhook 定义
- 部署账号/配置（启用时）
- AI 供应商配置（启用时）
- MCP 服务器配置与权限规则（启用时）

### 存储位置（macOS）

Skillshare App 会把应用数据存放在系统的 app data 目录下（通常是）：

- `~/Library/Application Support/com.skillshare-app.Skillshare App-macOS/`

> 注意：不同平台/打包版本，实际文件夹名称可能会不同。

## 机密与加密

- API key 与 token 会在本地以 AES-256-GCM 加密保存
- 机密值保存后（依界面设计）可能不再以明文显示
- 日志与 UI 输出会做基础的敏感信息遮罩，以降低意外泄露风险

<!-- TODO: Add screenshot showing “secret” variables / masked tokens UI. -->

## AI 供应商（可选）

启用 AI 供应商后，请求可能会发送到云端服务（云端模型）或保留在本机（本地模型）。你可以自行决定启用哪个供应商与使用场景。

建议：

- 对敏感代码或私有 repo，优先使用 **本地模型**（Ollama / LM Studio）
- 需要更大 context 或更强推理时，可使用 **云端模型**

## MCP 安全模型

Skillshare App 提供 MCP 服务器（`skillshare-mcp`），让 AI 工具可以调用动作。

### 权限级别

- **只读**：仅允许查询/读取类型工具（安全默认）
- **执行需确认**：每次执行动作都需要你确认
- **全权限**：动作不再弹确认（仅建议用于可信配置/环境）

### 工具级权限控制

你可以对单个工具进行允许/需确认/阻止（例如 `run_workflow`、`run_npm_script`、`read_project_file`），根据风险偏好调整。

### 请求日志

Skillshare App 可以记录 MCP 请求（工具名、参数、耗时、结果），方便你审计 AI 工具做了什么。

<!-- TODO: Add screenshot of MCP logs panel. -->

## 默认不做遥测

Skillshare App 默认避免加入“回传数据”的分析追踪。网络访问主要用于：

- AI 供应商调用（启用时）
- 部署供应商（启用时）
- 更新/下载发布版本（启用时）

## 重置 / 移除数据

- 从 Skillshare App 移除项目会“忘记”该项目（不会删除你的项目文件）
- 不需要集成时，可关闭 AI / MCP
- 完全重置：删除 Skillshare App 的 app data 数据目录

## 报告安全问题

如果你发现安全漏洞，请在 GitHub Issues 提供最小可复现案例；若内容敏感，建议私下联系维护者。

