# Webhooks

使用傳入和傳出 webhook 自動化操作。

## 概覽

Skillshare App 支援兩種類型的 webhook：

- **傳出**：事件發生時發送通知
- **傳入**：從外部服務觸發工作流程

<!-- TODO: Add diagram showing webhook flow -->

## 傳出 Webhooks

### 什麼是傳出 Webhook？

在 Skillshare App 事件發生時發送 HTTP 請求：

- 工作流程完成
- 腳本結束
- 部署成功或失敗

### 建立傳出 Webhook

1. 開啟工作流程
2. 點擊**設定** → **Webhooks**
3. 點擊**新增傳出 Webhook**
4. 設定：
   - 發送的 URL
   - 觸發的事件
   - Payload 範本
5. 儲存

<!-- TODO: Add screenshot of outgoing webhook config -->

### Payload 範本

自訂發送的 JSON payload：

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

### 可用變數

| 變數 | 說明 |
|------|------|
| `${event_type}` | 事件類型 |
| `${workflow_name}` | 工作流程名稱 |
| `${workflow_id}` | 工作流程 ID |
| `${status}` | success/failed |
| `${timestamp}` | ISO 時間戳記 |
| `${duration_ms}` | 持續時間（毫秒） |
| `${project_name}` | 專案名稱 |
| `${project_path}` | 專案目錄 |

### 測試 Webhook

1. 點擊 webhook 上的**測試**
2. Skillshare App 發送測試 payload
3. 查看回應狀態

## 傳入 Webhooks

### 什麼是傳入 Webhook？

呼叫時觸發 Skillshare App 操作的 URL：

- 啟動工作流程
- 執行腳本
- 觸發部署

### 建立傳入 Webhook

1. 開啟工作流程
2. 點擊**設定** → **觸發器**
3. 點擊**新增 Webhook 觸發器**
4. Skillshare App 產生唯一 URL
5. 複製 URL

<!-- TODO: Add screenshot of incoming webhook URL -->

### Webhook URL 格式

```
http://localhost:{port}/webhook/{token}
```

### 驗證請求

傳入 webhook 使用 token 驗證：

- Token 是 URL 的一部分
- 保持 URL 機密
- 如果洩露則重新產生 token

### 啟動 Webhook 伺服器

1. 前往**設定** → **Webhooks**
2. 啟用**傳入 Webhook 伺服器**
3. 設定連接埠（預設：7235）
4. 點擊**啟動**

## 使用案例

### CI/CD 整合

從 CI 觸發 Skillshare App 工作流程：

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

將工作流程結果發送到 Slack：

1. 建立 Slack 傳入 Webhook
2. 在 Skillshare App 中新增為傳出 webhook
3. 自訂 Slack 格式的 payload

**Slack Payload：**
```json
{
  "text": "工作流程 '${workflow_name}' ${status}",
  "attachments": [{
    "color": "${status == 'success' ? 'good' : 'danger'}",
    "fields": [
      {"title": "專案", "value": "${project_name}", "short": true},
      {"title": "持續時間", "value": "${duration_ms}ms", "short": true}
    ]
  }]
}
```

### Discord 通知

類似 Slack，發送到 Discord webhook：

**Discord Payload：**
```json
{
  "content": "工作流程完成！",
  "embeds": [{
    "title": "${workflow_name}",
    "description": "狀態：${status}",
    "color": 5763719
  }]
}
```

### 遠端觸發

從任何地方啟動工作流程：

- 手機捷徑
- 智慧家居整合
- 其他應用程式

## Webhook 事件

### 工作流程事件

| 事件 | 說明 |
|------|------|
| `workflow.started` | 工作流程執行開始 |
| `workflow.completed` | 工作流程成功完成 |
| `workflow.failed` | 工作流程失敗 |
| `workflow.cancelled` | 工作流程被取消 |

### 腳本事件

| 事件 | 說明 |
|------|------|
| `script.started` | 腳本執行開始 |
| `script.completed` | 腳本成功完成 |
| `script.failed` | 腳本以錯誤結束 |

### 部署事件

| 事件 | 說明 |
|------|------|
| `deploy.started` | 部署開始 |
| `deploy.completed` | 部署成功 |
| `deploy.failed` | 部署失敗 |

## Webhook 日誌

檢視 webhook 活動：

1. 前往**設定** → **Webhooks** → **日誌**
2. 查看：
   - 時間戳記
   - 方向（傳入/傳出）
   - URL
   - 狀態碼
   - 回應時間

<!-- TODO: Add screenshot of webhook logs -->

## 安全

### 最佳實踐

1. **使用 HTTPS**：對於傳出 webhook，優先使用 HTTPS URL
2. **保持 token 機密**：不要公開分享 webhook URL
3. **重新產生 token**：如果 URL 洩露
4. **驗證 payload**：對於傳入 webhook，驗證來源

### 重新產生 Token

如果 webhook URL 洩露：

1. 開啟 webhook 設定
2. 點擊**重新產生 Token**
3. 使用新 URL 更新所有整合

## 提示

1. **依賴前先測試**：始終測試 webhook 是否有效
2. **檢查日誌**：檢視日誌以除錯問題
3. **使用範本**：為每個服務自訂 payload
4. **設定逾時**：設定合理的逾時值
5. **處理失敗**：規劃 webhook 傳送失敗的情況
