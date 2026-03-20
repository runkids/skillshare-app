# AI 整合

連接多個 AI 供應商，並在整個 Skillshare App 中使用智慧輔助。

## 概覽

Skillshare App 支援多個 AI 供應商用於智慧功能，如：

- 提交訊息產生
- 程式碼審查分析
- 安全漏洞摘要
- 自訂 AI 提示
- AI CLI 工具整合

## 支援的供應商

| 供應商 | 模型 | 驗證 |
|--------|------|------|
| **OpenAI** | GPT-4o、GPT-4o-mini、o1、o3 | API Key |
| **Anthropic** | Claude 4 Opus、Claude 4 Sonnet、Claude 3.5 Haiku | API Key |
| **Google Gemini** | Gemini 2.0 Flash、Gemini 1.5 Pro（有免費方案） | API Key |
| **Ollama** | Llama、Mistral、Qwen 及任何本地模型 | 本地 |
| **LM Studio** | 任何本地模型 | 本地 |

## 新增 AI 服務

### 雲端供應商（OpenAI、Anthropic、Google）

1. 前往**設定** → **AI 服務**
2. 點擊**新增服務**
3. 選擇供應商
4. 輸入您的 API key
5. 點擊**驗證並儲存**

### 本地供應商（Ollama、LM Studio）

1. 確保 Ollama/LM Studio 在本地執行
2. 前往**設定** → **AI 服務**
3. 點擊**新增服務**
4. 選擇 **Ollama** 或 **LM Studio**
5. 輸入本地 URL：
   - Ollama：`http://127.0.0.1:11434`
   - LM Studio：`http://127.0.0.1:1234/v1`
6. 點擊**連接**

## API Key 安全

API key 使用 AES-256-GCM 加密並安全儲存：

- Key 永遠不會暴露在日誌中
- 靜態加密
- 儲存在系統 keychain（macOS）

## 選擇模型

### 每個服務的模型

每個服務都有可用模型：

1. 點擊服務
2. 點擊**取得模型**
3. 選擇您偏好的預設模型

### 每個功能的模型

為不同任務選擇不同模型：

- 提交訊息：較快的模型（GPT-4o-mini）
- 程式碼審查：更強大的模型（GPT-4o、Claude 4 Sonnet）

## AI 功能

### 提交訊息產生

從您的差異產生有意義的提交訊息：

1. 暫存您的變更
2. 點擊提交表單中的 **AI** 按鈕
3. AI 分析差異並產生訊息
4. 需要時編輯，然後提交

### 程式碼審查分析

AI 驅動的程式碼審查：

1. 暫存您要審查的變更
2. 開啟 AI 審查對話框
3. 選擇範圍（單一檔案或所有暫存變更）
4. AI 分析並提供審查回饋
5. 檢視建議並視需要套用

### 安全漏洞摘要

安全漏洞的白話解釋：

1. 對專案執行安全掃描
2. 點擊漏洞
3. 點擊 **AI 分析** 按鈕
4. AI 以白話解釋漏洞：
   - 問題是什麼
   - 為什麼危險
   - 如何修復
   - 風險評估

### 安全摘要報告

產生所有漏洞的綜合概覽：

1. 安全掃描完成後
2. 點擊**產生 AI 摘要**
3. AI 建立優先順序摘要：
   - 需要立即處理的重大問題
   - 建議的修復順序
   - 依賴更新建議

## AI CLI 整合

Skillshare App 整合 AI CLI 工具以增強功能。

### 支援的 CLI 工具

| CLI | 二進位檔 | 說明 |
|-----|----------|------|
| **Claude Code** | `claude` | Anthropic Claude CLI 用於程式碼輔助 |
| **Codex** | `codex` | OpenAI Codex CLI 用於程式碼產生 |
| **Gemini CLI** | `gemini` | Google Gemini CLI 用於 AI 輔助 |

### 自動偵測

Skillshare App 自動偵測已安裝的 CLI 工具：

1. 前往**設定** → **AI 服務**
2. 檢視 **CLI 工具** 區段
3. 偵測到的工具會顯示版本和驗證狀態

### 執行 AI 指令

1. 前往**設定** → **AI CLI**
2. 選擇已安裝的 CLI 工具
3. 輸入提示
4. 點擊**執行**

輸出即時串流顯示在面板中。

### CLI 執行選項

- **包含差異**：新增暫存的 git diff 作為上下文
- **包含檔案**：新增特定檔案作為上下文
- **自訂上下文**：新增任意文字上下文
- **包含 MCP 上下文**：新增來自 MCP 的專案資訊

## 提示範本

使用範本自訂 AI 產生內容的方式。

### 預設範本

Skillshare App 包含以下範本：

- Git 提交訊息
- Pull request 描述
- 程式碼審查評論
- 文件產生
- 發布說明
- 安全公告
- 自訂提示

### 範本類別

| 類別 | 說明 | 變數 |
|------|------|------|
| `git_commit` | 提交訊息產生 | `{diff}` |
| `pull_request` | PR 描述 | `{diff}`、`{commits}`、`{branch}`、`{base_branch}` |
| `code_review` | 程式碼審查回饋 | `{diff}`、`{file_path}`、`{code}` |
| `documentation` | 文件產生 | `{code}`、`{file_path}`、`{function_name}` |
| `release_notes` | 發布說明 | `{commits}`、`{version}`、`{previous_version}` |
| `security_advisory` | 安全分析 | `{vulnerability_json}`、`{project_context}`、`{severity_summary}` |
| `custom` | 通用 | `{input}` |

### 建立自訂範本

1. 前往**設定** → **AI 服務** → **範本**
2. 點擊**新增範本**
3. 設定：
   - 名稱
   - 類別
   - 帶變數的提示文字
   - 輸出格式（用於提交訊息）
4. 儲存

### 範本變數

在提示中使用變數：

| 變數 | 說明 |
|------|------|
| `{diff}` | Git 差異內容 |
| `{code}` | 選中的程式碼 |
| `{file_path}` | 目前檔案路徑 |
| `{commits}` | 提交歷史 |
| `{branch}` | 目前分支名稱 |
| `{base_branch}` | PR 的目標分支 |
| `{version}` | 發布版本 |
| `{vulnerability_json}` | 漏洞資料（JSON 格式） |

### 範本範例

**提交訊息範本：**

```
根據以下 git diff，產生遵循 conventional commit 格式的簡潔提交訊息。

重點：
- 變更了什麼（而非如何）
- 為什麼變更（如果明顯）
- 第一行保持在 72 個字元以下

Diff：
{diff}
```

### 提交訊息格式

選擇提交訊息的輸出格式：

- **Conventional Commits**：`type(scope): description`
- **Simple**：純描述性訊息
- **Custom**：您自己的格式

### 專案範本

為特定專案覆寫範本：

1. 開啟專案
2. 前往**設定** → **AI**
3. 選擇範本覆寫
4. 根據需要自訂

## AI 執行模式

Skillshare App 支援兩種執行模式：

### API 模式（預設）

直接使用設定的 AI 服務 API：

- 更快的回應時間
- Token 使用追蹤
- 適用任何供應商

### CLI 模式

使用已安裝的 AI CLI 工具：

- 更豐富的上下文支援
- 原生 CLI 功能
- 如果 CLI 已驗證則無需 API key

### 切換模式

1. 前往**設定** → **AI 服務**
2. 在概覽分頁選擇**執行模式**
3. 選擇 **API** 或 **CLI**

## 測試服務

### 連接測試

驗證您的 API key 有效：

1. 點擊服務上的**測試連接**
2. Skillshare App 發送簡單請求
3. 顯示成功或錯誤詳情

### 模型探測

不儲存服務即可測試模型：

1. 點擊**探測模型**
2. 輸入供應商和端點
3. 查看可用模型

## 預設服務

設定預設 AI 服務：

1. 前往**設定** → **AI 服務**
2. 點擊服務旁的星號圖示
3. 此服務在未特別指定時使用

## 使用限制

### 雲端供應商

注意 API 速率限制和成本：

- OpenAI：按 token 計費
- Anthropic：按 token 計費
- Google：有免費方案

### 本地供應商

本地執行時無限制：

- Ollama：無限制
- LM Studio：無限制

## 提示

1. **從 Gemini 開始**：有免費方案，快速且能力強大適合大多數任務
2. **使用本地模型**：對於敏感程式碼，使用 Ollama 搭配 Llama 或 Qwen
3. **自訂範本**：更好的提示 = 更好的結果
4. **測試連接**：在依賴 AI 功能前驗證 API key 有效
5. **監控成本**：雲端 API 呼叫會快速累積（Gemini 免費方案除外）
6. **嘗試 CLI 模式**：對於複雜任務，CLI 工具通常提供更好的結果

## 疑難排解

### API Key 無效

- 驗證 key 正確
- 檢查 key 是否有必要權限
- 確保帳單已啟用（雲端供應商）

### 回應緩慢

- 嘗試較小/較快的模型
- 檢查網路連接
- 考慮使用本地模型以獲得更快回應

### 輸出品質差

- 檢視並改進您的提示範本
- 嘗試更強大的模型
- 在範本中提供更多上下文

### CLI 工具找不到

- 確保 CLI 已全域安裝
- 檢查二進位檔是否在您的 PATH 中
- 嘗試在設定中指定自訂二進位檔路徑

### CLI 驗證失敗

- 手動執行 CLI 的驗證指令
- 或切換到 API 模式並使用您自己的 API key
