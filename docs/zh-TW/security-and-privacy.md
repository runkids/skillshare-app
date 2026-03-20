# 安全與隱私

Skillshare App 以 **本機優先（local-first）** 為設計核心：你的專案資料留在你的機器上；AI / MCP 功能為可選，且具備權限控管。

<!-- TODO: Add screenshot of Settings → Security / Permissions (if you have one). -->

## Skillshare App 會儲存什麼？

### 在你的機器上

- 匯入的專案中繼資料（路徑、偵測到的 scripts、git/worktree 資訊）
- 工作流、步驟模板、webhook 定義
- 部署帳號/設定（啟用時）
- AI 供應商設定（啟用時）
- MCP 伺服器設定與權限規則（啟用時）

### 儲存位置（macOS）

Skillshare App 會把應用資料存放在系統的 app data 目錄下（通常是）：

- `~/Library/Application Support/com.skillshare-app.Skillshare App-macOS/`

> 注意：不同平台/打包版本，實際資料夾名稱可能會不同。

## 祕密與加密

- API key 與 token 會在本機以 AES-256-GCM 加密保存
- 祕密值儲存後（依畫面設計）可能不再以明文顯示
- 日誌與 UI 輸出會做基本的敏感資訊遮罩，以降低意外外洩風險

<!-- TODO: Add screenshot showing “secret” variables / masked tokens UI. -->

## AI 供應商（可選）

啟用 AI 供應商後，請求可能會送到雲端服務（雲端模型）或留在本機（本機模型）。你可以自行決定啟用哪個供應商與使用情境。

建議：

- 對敏感程式碼或私有 repo，優先使用 **本機模型**（Ollama / LM Studio）
- 需要更大 context 或更強推理時，可使用 **雲端模型**

## MCP 安全模型

Skillshare App 會提供 MCP 伺服器（`skillshare-mcp`），讓 AI 工具可以呼叫動作。

### 權限等級

- **唯讀**：只允許查詢/讀取類型的工具（安全預設）
- **執行需確認**：每次執行動作都要你確認
- **全權限**：動作不再跳確認（僅建議用在可信的設定/環境）

### 工具級權限控管

你可以針對單一工具允許/需確認/封鎖（例如 `run_workflow`、`run_npm_script`、`read_project_file`），依你的風險偏好調整。

### 請求紀錄

Skillshare App 可以記錄 MCP 請求（工具名稱、參數、耗時、結果），方便你回溯 AI 工具做了什麼。

<!-- TODO: Add screenshot of MCP logs panel. -->

## 預設不做遙測

Skillshare App 預設避免加入「回傳資料」的分析追蹤。網路連線主要用於：

- AI 供應商呼叫（啟用時）
- 部署供應商（啟用時）
- 更新/下載發佈版本（啟用時）

## 重置 / 移除資料

- 從 Skillshare App 移除專案會「忘記」該專案（不會刪除你的專案檔案）
- 不需要整合時，可關閉 AI / MCP
- 完全重置：刪除 Skillshare App 的 app data 資料夾

## 回報安全問題

若你發現安全漏洞，請在 GitHub Issues 提供最小可重現案例；若內容敏感，建議私下聯絡維護者。

