# AI 集成

连接多个 AI 供应商，并在整个 Skillshare App 中使用智能辅助。

## 概览

Skillshare App 支持多个 AI 供应商用于智能功能，如：

- 提交信息生成
- 代码审查分析
- 安全漏洞摘要
- 自定义 AI 提示
- AI CLI 工具集成

## 支持的供应商

| 供应商 | 模型 | 验证 |
|--------|------|------|
| **OpenAI** | GPT-4o、GPT-4o-mini、o1、o3 | API Key |
| **Anthropic** | Claude 4 Opus、Claude 4 Sonnet、Claude 3.5 Haiku | API Key |
| **Google Gemini** | Gemini 2.0 Flash、Gemini 1.5 Pro（有免费方案） | API Key |
| **Ollama** | Llama、Mistral、Qwen 及任何本地模型 | 本地 |
| **LM Studio** | 任何本地模型 | 本地 |

## 添加 AI 服务

### 云端供应商（OpenAI、Anthropic、Google）

1. 前往**设置** → **AI 服务**
2. 点击**添加服务**
3. 选择供应商
4. 输入您的 API key
5. 点击**验证并保存**

### 本地供应商（Ollama、LM Studio）

1. 确保 Ollama/LM Studio 在本地运行
2. 前往**设置** → **AI 服务**
3. 点击**添加服务**
4. 选择 **Ollama** 或 **LM Studio**
5. 输入本地 URL：
   - Ollama：`http://127.0.0.1:11434`
   - LM Studio：`http://127.0.0.1:1234/v1`
6. 点击**连接**

## API Key 安全

API key 使用 AES-256-GCM 加密并安全存储：

- Key 永远不会暴露在日志中
- 静态加密
- 存储在系统 keychain（macOS）

## 选择模型

### 每个服务的模型

每个服务都有可用模型：

1. 点击服务
2. 点击**获取模型**
3. 选择您偏好的默认模型

### 每个功能的模型

为不同任务选择不同模型：

- 提交信息：较快的模型（GPT-4o-mini）
- 代码审查：更强大的模型（GPT-4o、Claude 4 Sonnet）

## AI 功能

### 提交信息生成

从您的差异生成有意义的提交信息：

1. 暂存您的变更
2. 点击提交表单中的 **AI** 按钮
3. AI 分析差异并生成信息
4. 需要时编辑，然后提交

### 代码审查分析

AI 驱动的代码审查：

1. 暂存您要审查的变更
2. 打开 AI 审查对话框
3. 选择范围（单一文件或所有暂存变更）
4. AI 分析并提供审查反馈
5. 查看建议并视需要应用

### 安全漏洞摘要

安全漏洞的白话解释：

1. 对项目执行安全扫描
2. 点击漏洞
3. 点击 **AI 分析** 按钮
4. AI 以白话解释漏洞：
   - 问题是什么
   - 为什么危险
   - 如何修复
   - 风险评估

### 安全摘要报告

生成所有漏洞的综合概览：

1. 安全扫描完成后
2. 点击**生成 AI 摘要**
3. AI 创建优先级摘要：
   - 需要立即处理的重大问题
   - 建议的修复顺序
   - 依赖更新建议

## AI CLI 集成

Skillshare App 集成 AI CLI 工具以增强功能。

### 支持的 CLI 工具

| CLI | 二进制文件 | 说明 |
|-----|------------|------|
| **Claude Code** | `claude` | Anthropic Claude CLI 用于代码辅助 |
| **Codex** | `codex` | OpenAI Codex CLI 用于代码生成 |
| **Gemini CLI** | `gemini` | Google Gemini CLI 用于 AI 辅助 |

### 自动检测

Skillshare App 自动检测已安装的 CLI 工具：

1. 前往**设置** → **AI 服务**
2. 查看 **CLI 工具** 部分
3. 检测到的工具会显示版本和验证状态

### 运行 AI 命令

1. 前往**设置** → **AI CLI**
2. 选择已安装的 CLI 工具
3. 输入提示
4. 点击**运行**

输出实时流式显示在面板中。

### CLI 执行选项

- **包含差异**：添加暂存的 git diff 作为上下文
- **包含文件**：添加特定文件作为上下文
- **自定义上下文**：添加任意文本上下文
- **包含 MCP 上下文**：添加来自 MCP 的项目信息

## 提示模板

使用模板自定义 AI 生成内容的方式。

### 默认模板

Skillshare App 包含以下模板：

- Git 提交信息
- Pull request 描述
- 代码审查评论
- 文档生成
- 发布说明
- 安全公告
- 自定义提示

### 模板类别

| 类别 | 说明 | 变量 |
|------|------|------|
| `git_commit` | 提交信息生成 | `{diff}` |
| `pull_request` | PR 描述 | `{diff}`、`{commits}`、`{branch}`、`{base_branch}` |
| `code_review` | 代码审查反馈 | `{diff}`、`{file_path}`、`{code}` |
| `documentation` | 文档生成 | `{code}`、`{file_path}`、`{function_name}` |
| `release_notes` | 发布说明 | `{commits}`、`{version}`、`{previous_version}` |
| `security_advisory` | 安全分析 | `{vulnerability_json}`、`{project_context}`、`{severity_summary}` |
| `custom` | 通用 | `{input}` |

### 创建自定义模板

1. 前往**设置** → **AI 服务** → **模板**
2. 点击**新建模板**
3. 配置：
   - 名称
   - 类别
   - 带变量的提示文本
   - 输出格式（用于提交信息）
4. 保存

### 模板变量

在提示中使用变量：

| 变量 | 说明 |
|------|------|
| `{diff}` | Git 差异内容 |
| `{code}` | 选中的代码 |
| `{file_path}` | 当前文件路径 |
| `{commits}` | 提交历史 |
| `{branch}` | 当前分支名称 |
| `{base_branch}` | PR 的目标分支 |
| `{version}` | 发布版本 |
| `{vulnerability_json}` | 漏洞数据（JSON 格式） |

### 模板示例

**提交信息模板：**

```
根据以下 git diff，生成遵循 conventional commit 格式的简洁提交信息。

重点：
- 变更了什么（而非如何）
- 为什么变更（如果明显）
- 第一行保持在 72 个字符以下

Diff：
{diff}
```

### 提交信息格式

选择提交信息的输出格式：

- **Conventional Commits**：`type(scope): description`
- **Simple**：纯描述性信息
- **Custom**：您自己的格式

### 项目模板

为特定项目覆盖模板：

1. 打开项目
2. 前往**设置** → **AI**
3. 选择模板覆盖
4. 根据需要自定义

## AI 执行模式

Skillshare App 支持两种执行模式：

### API 模式（默认）

直接使用配置的 AI 服务 API：

- 更快的响应时间
- Token 使用追踪
- 适用任何供应商

### CLI 模式

使用已安装的 AI CLI 工具：

- 更丰富的上下文支持
- 原生 CLI 功能
- 如果 CLI 已验证则无需 API key

### 切换模式

1. 前往**设置** → **AI 服务**
2. 在概览选项卡选择**执行模式**
3. 选择 **API** 或 **CLI**

## 测试服务

### 连接测试

验证您的 API key 有效：

1. 点击服务上的**测试连接**
2. Skillshare App 发送简单请求
3. 显示成功或错误详情

### 模型探测

不保存服务即可测试模型：

1. 点击**探测模型**
2. 输入供应商和端点
3. 查看可用模型

## 默认服务

设置默认 AI 服务：

1. 前往**设置** → **AI 服务**
2. 点击服务旁的星号图标
3. 此服务在未特别指定时使用

## 使用限制

### 云端供应商

注意 API 速率限制和成本：

- OpenAI：按 token 计费
- Anthropic：按 token 计费
- Google：有免费方案

### 本地供应商

本地运行时无限制：

- Ollama：无限制
- LM Studio：无限制

## 提示

1. **从 Gemini 开始**：有免费方案，快速且能力强大适合大多数任务
2. **使用本地模型**：对于敏感代码，使用 Ollama 搭配 Llama 或 Qwen
3. **自定义模板**：更好的提示 = 更好的结果
4. **测试连接**：在依赖 AI 功能前验证 API key 有效
5. **监控成本**：云端 API 调用会快速累积（Gemini 免费方案除外）
6. **尝试 CLI 模式**：对于复杂任务，CLI 工具通常提供更好的结果

## 疑难排解

### API Key 无效

- 验证 key 正确
- 检查 key 是否有必要权限
- 确保账单已启用（云端供应商）

### 响应缓慢

- 尝试较小/较快的模型
- 检查网络连接
- 考虑使用本地模型以获得更快响应

### 输出质量差

- 查看并改进您的提示模板
- 尝试更强大的模型
- 在模板中提供更多上下文

### CLI 工具找不到

- 确保 CLI 已全局安装
- 检查二进制文件是否在您的 PATH 中
- 尝试在设置中指定自定义二进制文件路径

### CLI 验证失败

- 手动运行 CLI 的验证命令
- 或切换到 API 模式并使用您自己的 API key
