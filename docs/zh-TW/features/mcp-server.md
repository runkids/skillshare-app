# MCP 伺服器

讓 AI 工具透過 Model Context Protocol（MCP）控制 Skillshare App。

## 什麼是 MCP？

Model Context Protocol（MCP）是 AI 工具與應用程式互動的標準。Skillshare App 可以作為 MCP 伺服器，允許 AI 助手如：

- Claude Code
- Codex CLI
- Gemini CLI

以程式化方式查詢和控制 Skillshare App。

## 概覽

啟用後，Skillshare App 公開 AI 助手可以呼叫的工具：

- 列出專案
- 執行腳本
- 執行工作流程
- 觸發部署
- 還有更多

<!-- TODO: Add diagram of MCP architecture -->

## 啟用 MCP 伺服器

1. 前往**設定** → **MCP**
2. 切換**啟用 MCP 伺服器**
3. 設定伺服器設定
4. 點擊**啟動伺服器**

<!-- TODO: Add screenshot of MCP settings panel -->

## 伺服器設定

### 連接埠

預設：`7234`

如果連接埠已被使用請變更。

### 主機

預設：`localhost`

基於安全考量，預設只允許本地連接。

## 權限等級

控制 AI 工具可以做什麼：

### 唯讀

AI 只能查詢資訊：
- 列出專案
- 檢視工作流程
- 檢查狀態

無法進行變更或執行指令。

### 需確認執行

AI 可以請求操作，但您必須核准：
- 出現確認對話框
- 您可以核准或拒絕
- 日常使用安全

### 完整存取

AI 可以無需確認執行任何操作：
- 僅與信任的 AI 工具使用
- 僅建議用於個人自動化

<!-- TODO: Add screenshot of permission level selector -->

## 工具權限

對個別工具的細粒度控制：

| 工具 | 說明 | 風險等級 |
|------|------|----------|
| `list_projects` | 列出所有專案 | 低 |
| `get_project` | 取得專案詳情（scripts、workflows、git） | 低 |
| `read_project_file` | 讀取檔案內容（有安全限制） | 中 |
| `run_npm_script` | 執行 package.json script | 中 |
| `run_workflow` | 執行工作流程 | 中 |
| `run_package_manager_command` | 安裝/更新/稽核/新增/移除依賴 | 中 |
| `run_security_scan` | 執行安全稽核（可選擇自動修復） | 中 |
| `trigger_webhook` | 觸發已設定的 webhook action | 中 |

> 提醒：Skillshare App 的 MCP 設計上會避免「任意 shell 執行」的預設暴露，建議優先使用 `run_npm_script` / `run_workflow` / `run_package_manager_command` 等高階工具。

### 自訂工具存取

1. 前往**設定** → **MCP** → **工具權限**
2. 對每個工具設定：
   - **允許**：可以使用
   - **確認**：需要核准
   - **封鎖**：無法使用

<!-- TODO: Add screenshot of tool permission matrix -->

## AI CLI 整合

### 支援的 AI CLI

Skillshare App 偵測並整合：

| CLI | 偵測 |
|-----|------|
| Claude Code | `claude` 指令 |
| Codex CLI | `codex` 指令 |
| Gemini CLI | `gemini` 指令 |

### 執行 AI 指令

1. 前往**設定** → **AI CLI**
2. 選擇已安裝的 CLI
3. 輸入提示
4. 點擊**執行**

輸出顯示在面板中。

<!-- TODO: Add screenshot of AI CLI panel -->

### 範例

**使用 Claude Code：**
```
"將我的專案部署到 Netlify staging"
```

**使用 Codex：**
```
"執行測試並修復任何失敗"
```

## MCP 工具參考

Skillshare App 透過 `skillshare-mcp` 提供多個工具給 AI 呼叫。工具清單會隨版本調整，建議以 App 內的 **Settings → MCP → Tool Permissions** 為準。

常見工具（節錄）：

| 工具 | 用途 |
|------|------|
| `list_projects` | 列出已註冊的專案 |
| `get_project` | 取得專案詳情（scripts、workflows、git） |
| `run_npm_script` | 執行 npm/yarn/pnpm script |
| `run_workflow` | 執行工作流程 |
| `read_project_file` | 讀取檔案內容（有安全限制） |

完整工具與參數請參考：`docs/features/mcp-server.md`（英文）或 App 內的工具清單。

## 日誌與監控

### 請求日誌

檢視所有 MCP 請求：

1. 前往**設定** → **MCP** → **日誌**
2. 查看：
   - 時間戳記
   - 呼叫的工具
   - 參數
   - 結果
   - 持續時間

<!-- TODO: Add screenshot of MCP logs -->

### 工作階段追蹤

追蹤每個 AI 工作階段：
- 工作階段 ID
- 連接的 AI 工具
- 請求數量
- 持續時間

## 安全最佳實踐

1. **從唯讀開始**：僅在需要時提升
2. **使用確認模式**：用於敏感操作
3. **定期檢視日誌**：檢查 AI 工具在做什麼
4. **限制工具存取**：停用不需要的工具
5. **僅限本地**：除非必要，不要暴露到網路

## 使用案例

### 自動化工作流程

讓 AI 工具自動化重複任務：

```
"每天早上拉取最新變更並為所有專案執行測試"
```

### 語音控制開發

與語音 AI 配對進行免手操作程式設計：

```
"為我的部落格專案執行開發伺服器"
```

### CI/CD 整合

使用 AI 工具管理部署：

```
"測試通過後將最新建置部署到 staging"
```

## 疑難排解

### 伺服器無法啟動

- 檢查連接埠是否被使用
- 嘗試不同的連接埠
- 確保 Skillshare App 有網路權限

### AI 無法連接

- 驗證伺服器正在執行
- 檢查連接埠號碼
- 確保防火牆允許本地連接

### 指令失敗

- 檢查工具權限
- 檢視日誌中的錯誤
- 驗證請求的資源存在
