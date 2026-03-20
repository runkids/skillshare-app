# Webhooks

使用传入和传出 webhook 自动化操作。

## 概览

Skillshare App 支持两种类型的 webhook：

- **传出**：事件发生时发送通知
- **传入**：从外部服务触发工作流

<!-- TODO: Add diagram showing webhook flow -->

## 传出 Webhooks

### 什么是传出 Webhook？

在 Skillshare App 事件发生时发送 HTTP 请求：

- 工作流完成
- 脚本结束
- 部署成功或失败

### 创建传出 Webhook

1. 打开工作流
2. 点击**设置** → **Webhooks**
3. 点击**添加传出 Webhook**
4. 配置：
   - 发送的 URL
   - 触发的事件
   - Payload 模板
5. 保存

<!-- TODO: Add screenshot of outgoing webhook config -->

### Payload 模板

自定义发送的 JSON payload：

```json
{
  "event": "${event_type}",
  "workflow": "${workflow_name}",
  "status": "${status}",
  "timestamp": "${timestamp}",
  "duration": "${duration_ms}",
  "project": "${project_name}"
}
```

### 可用变量

| 变量 | 说明 |
|------|------|
| `${event_type}` | 事件类型 |
| `${workflow_name}` | 工作流名称 |
| `${workflow_id}` | 工作流 ID |
| `${status}` | success/failed |
| `${timestamp}` | ISO 时间戳 |
| `${duration_ms}` | 持续时间（毫秒） |
| `${project_name}` | 项目名称 |
| `${project_path}` | 项目目录 |

### 测试 Webhook

1. 点击 webhook 上的**测试**
2. Skillshare App 发送测试 payload
3. 查看响应状态

## 传入 Webhooks

### 什么是传入 Webhook？

调用时触发 Skillshare App 操作的 URL：

- 启动工作流
- 运行脚本
- 触发部署

### 创建传入 Webhook

1. 打开工作流
2. 点击**设置** → **触发器**
3. 点击**添加 Webhook 触发器**
4. Skillshare App 生成唯一 URL
5. 复制 URL

<!-- TODO: Add screenshot of incoming webhook URL -->

### Webhook URL 格式

```
http://localhost:{port}/webhook/{token}
```

### 验证请求

传入 webhook 使用 token 验证：

- Token 是 URL 的一部分
- 保持 URL 机密
- 如果泄露则重新生成 token

### 启动 Webhook 服务器

1. 前往**设置** → **Webhooks**
2. 启用**传入 Webhook 服务器**
3. 设置端口（默认：7235）
4. 点击**启动**

## 使用案例

### CI/CD 集成

从 CI 触发 Skillshare App 工作流：

**GitHub Actions：**
```yaml
- name: Trigger Skillshare App
  run: |
    curl -X POST https://your-webhook-url
```

**GitLab CI：**
```yaml
trigger_skillshare-app:
  script:
    - curl -X POST $SKILLSHARE_WEBHOOK_URL
```

### Slack 通知

将工作流结果发送到 Slack：

1. 创建 Slack 传入 Webhook
2. 在 Skillshare App 中添加为传出 webhook
3. 自定义 Slack 格式的 payload

**Slack Payload：**
```json
{
  "text": "工作流 '${workflow_name}' ${status}",
  "attachments": [{
    "color": "${status == 'success' ? 'good' : 'danger'}",
    "fields": [
      {"title": "项目", "value": "${project_name}", "short": true},
      {"title": "持续时间", "value": "${duration_ms}ms", "short": true}
    ]
  }]
}
```

### Discord 通知

类似 Slack，发送到 Discord webhook：

**Discord Payload：**
```json
{
  "content": "工作流完成！",
  "embeds": [{
    "title": "${workflow_name}",
    "description": "状态：${status}",
    "color": 5763719
  }]
}
```

### 远程触发

从任何地方启动工作流：

- 手机快捷方式
- 智能家居集成
- 其他应用程序

## Webhook 事件

### 工作流事件

| 事件 | 说明 |
|------|------|
| `workflow.started` | 工作流运行开始 |
| `workflow.completed` | 工作流成功完成 |
| `workflow.failed` | 工作流失败 |
| `workflow.cancelled` | 工作流被取消 |

### 脚本事件

| 事件 | 说明 |
|------|------|
| `script.started` | 脚本运行开始 |
| `script.completed` | 脚本成功完成 |
| `script.failed` | 脚本以错误结束 |

### 部署事件

| 事件 | 说明 |
|------|------|
| `deploy.started` | 部署开始 |
| `deploy.completed` | 部署成功 |
| `deploy.failed` | 部署失败 |

## Webhook 日志

查看 webhook 活动：

1. 前往**设置** → **Webhooks** → **日志**
2. 查看：
   - 时间戳
   - 方向（传入/传出）
   - URL
   - 状态码
   - 响应时间

<!-- TODO: Add screenshot of webhook logs -->

## 安全

### 最佳实践

1. **使用 HTTPS**：对于传出 webhook，优先使用 HTTPS URL
2. **保持 token 机密**：不要公开分享 webhook URL
3. **重新生成 token**：如果 URL 泄露
4. **验证 payload**：对于传入 webhook，验证来源

### 重新生成 Token

如果 webhook URL 泄露：

1. 打开 webhook 设置
2. 点击**重新生成 Token**
3. 使用新 URL 更新所有集成

## 提示

1. **依赖前先测试**：始终测试 webhook 是否有效
2. **检查日志**：查看日志以调试问题
3. **使用模板**：为每个服务自定义 payload
4. **设置超时**：设置合理的超时值
5. **处理失败**：规划 webhook 传送失败的情况
