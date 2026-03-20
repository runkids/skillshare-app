# 時間機器與安全守門員

Skillshare App 的「時間機器（Time Machine）」會在 lockfile 變更時自動擷取依賴快照，讓你追蹤依賴演進、偵測潛在風險，並且在不同狀態之間比較差異。

## 概覽

時間機器提供：

- **自動快照**：lockfile 變更時自動擷取（含 debounce）
- **手動快照**：需要時一鍵擷取目前狀態
- **安全守門員**：即時偵測可疑套件與 postinstall scripts
- **差異分析**：比較兩個快照的變更內容
- **完整性檢查**：用快照比對目前依賴狀態是否漂移

## 入口

時間機器位於 Project Explorer 的 **Snapshots** 分頁：

```
Project Explorer Tabs:
Scripts | Workspaces | Workflows | Git | Builds | Security | Deploy | Snapshots
                                                                        ↑
```

你也可以在專案標題列使用 **Snapshots** 快速按鈕（青色按鈕）。

## 功能

### 1) 自動擷取快照

當專案的 lockfile 變更時，Skillshare App 會自動：

- 偵測 lockfile 變更（package-lock.json、pnpm-lock.yaml、yarn.lock、bun.lockb）
- 等待 debounce（預設 2 秒）
- 解析 lockfile 並抽取依賴樹
- 偵測 postinstall scripts
- 計算安全分數
- 壓縮並保存快照資料

**觸發來源（Trigger Source）類型：**

- `lockfile_change`：lockfile 變更觸發的自動擷取
- `manual`：使用者於 UI 或 AI Assistant 手動擷取

### 2) 快照時間軸

以時間順序檢視所有快照：

- 依日期範圍或觸發來源過濾
- 一眼看見安全分數
- 標記含 postinstall script 的快照
- 快速進入差異比較

### 3) 依賴差異檢視

比較任意兩個快照可看到：

- 新增 / 移除 / 更新的套件
- 版本變更（含語意化版本分析）
- 新增或變更的 postinstall scripts
- 安全分數的變化

### 4) 安全守門員

自動化安全分析包含：

#### 拼字相似（Typosquatting）偵測

辨識名稱與熱門套件相近的可疑套件：

- `lodahs` vs `lodash`
- `reqeust` vs `request`
- 使用 Levenshtein distance 演算法

#### Postinstall Script 監控

- 追蹤所有含 postinstall script 的套件
- 當出現新的 postinstall script 時提醒
- 顯示不同快照間 script 內容的變更

#### 可疑模式偵測

- 大幅度 major 版本跳躍（例如 1.0.0 → 9.0.0）
- 非預期的版本降級
- 可疑的套件命名樣式

### 5) 完整性檢查

用快照驗證目前依賴狀態：

- 比對目前 lockfile hash 與快照保存的 hash
- 偵測與預期狀態的漂移（drift）
- 找出非預期的變更

### 6) 安全洞察儀表板

專案層級安全概覽：

- 整體風險分數（0-100）
- 依嚴重程度的洞察摘要
- 頻繁更新的套件
- 拼字相似（typosquatting）提醒歷史

### 7) 可搜尋的歷史

跨快照搜尋：

- 依套件名稱或版本
- 依日期範圍
- 依是否含 postinstall script
- 依最低安全分數門檻

## 設定

在 **Settings > Storage** 內可設定時間機器：

### Auto-Watch

控制是否對所有專案啟用 lockfile 自動監控。啟用後，Skillshare App 會監看 lockfile 並在變更時擷取快照。

### Debounce

設定 debounce（預設 2000ms），避免安裝流程中出現短時間大量連續擷取。

## 儲存管理

快照會存放在：

```
~/Library/Application Support/com.skillshare.app/time-machine/snapshots/
```

每個快照包含：

- 壓縮的 lockfile（`.zst`）
- 壓縮的 package.json（`.zst`）
- 依賴樹 JSON
- postinstall 清單

### 保留策略（Retention）

在 Settings > Storage 內設定：

- 每個專案最多保留的快照數量
- 手動清理舊快照
- 清理孤兒儲存檔案

## MCP 工具

時間機器與 MCP 伺服器整合，讓 AI 助手可以呼叫：

| Tool | 說明 |
|------|------|
| `list_snapshots` | 列出專案的快照 |
| `capture_snapshot` | 手動擷取快照 |
| `get_snapshot_details` | 取得完整快照（含依賴） |
| `compare_snapshots` | 比較兩個快照差異 |
| `search_snapshots` | 跨快照搜尋 |
| `check_dependency_integrity` | 檢查是否與最新快照漂移 |
| `get_security_insights` | 取得專案安全洞察 |
| `export_security_report` | 匯出安全報告 |

## AI Assistant 快捷動作

在 AI Assistant 中，時間機器提供快捷動作：

- **Capture Snapshot**：擷取目前依賴狀態
- **View Snapshots**：開啟 Snapshots 分頁
- **Check Integrity**：檢查依賴完整性

## 最佳實務

1. **啟用 Auto-Watch**：重要專案保持自動監控
2. **留意 postinstall**：新出現的 postinstall script 需要特別注意
3. **處理 typosquatting**：遇到可疑套件名請先驗證
4. **定期比較**：大型依賴更新後，挑兩個快照比較變更
5. **定期清理**：透過保留策略控制儲存空間

## 安全分數計算

安全分數（0-100）會綜合考量：

- postinstall script 數量（越多通常風險越高）
- typosquatting 可疑項目
- 已知的漏洞/可疑模式
- 依賴樹深度與複雜度

| 分數區間 | 風險等級 |
|----------|----------|
| 80-100 | 低 |
| 60-79 | 中 |
| 40-59 | 高 |
| 0-39 | 嚴重 |

## 相關功能

- [安全掃描](./security-audit.md)
- [專案管理](./project-management.md)
- [MCP 伺服器](./mcp-server.md)
- [AI 整合](./ai-integration.md)

