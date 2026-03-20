# Monorepo 支援

Skillshare App 為 Nx、Turborepo、Lerna 和原生工作區等 monorepo 工具提供一流支援。

## 概覽

當您匯入專案時，會自動偵測 Monorepo。Skillshare App 識別：

- 工作區設定
- monorepo 中的所有套件
- 建置工具（Nx、Turbo、Lerna）
- 依賴關係

<!-- TODO: Add screenshot of monorepo view -->

## Nx 支援

### 偵測

Skillshare App 透過以下方式偵測 Nx 專案：
- 根目錄中的 `nx.json`
- 依賴中的 `@nx/*` 套件

### 功能

**Target 偵測**
- 自動發現所有 Nx targets
- Targets 顯示為可執行的按鈕

**執行 Targets**
```
nx run <project>:<target>
```

點擊 target 卡片在特定專案上執行。

<!-- TODO: Add screenshot of Nx targets panel -->

**依賴圖**

檢視專案依賴圖：

1. 點擊 Nx 面板中的**顯示圖表**
2. 開啟互動式視覺化
3. 點擊節點查看依賴

<!-- TODO: Add screenshot of Nx dependency graph -->

**快取管理**

- 檢視快取狀態
- 一鍵清除 Nx 快取
- 查看快取命中率

## Turborepo 支援

### 偵測

Skillshare App 透過以下方式偵測 Turbo 專案：
- 根目錄中的 `turbo.json`
- 依賴中的 `turbo`

### 功能

**Pipeline 偵測**

發現並顯示 `turbo.json` 中定義的所有 pipelines。

**執行任務**
```
turbo run <task>
```

點擊任務在所有套件上執行。

<!-- TODO: Add screenshot of Turbo pipeline panel -->

**過濾**

在特定套件上執行任務：

1. 在過濾列中選擇套件
2. 點擊任務執行
3. 僅影響選中的套件

**快取管理**

- 檢視遠端快取狀態
- 清除本地快取
- 切換遠端快取

## Lerna 支援

### 偵測

Skillshare App 透過以下方式偵測 Lerna 專案：
- 根目錄中的 `lerna.json`
- 依賴中的 `lerna`

### 功能

- 列出所有套件
- 跨套件執行指令
- 版本管理支援

## 原生工作區

Skillshare App 支援以下工作區：

| 套件管理器 | 設定位置 |
|-----------|----------|
| npm | `package.json` → `workspaces` |
| yarn | `package.json` → `workspaces` |
| pnpm | `pnpm-workspace.yaml` |

### 功能

**套件列表**

列出所有工作區套件，包含：
- 套件名稱
- 版本
- 位置
- 可用腳本

<!-- TODO: Add screenshot of workspace packages list -->

**在套件中執行**

在特定套件中執行腳本：

1. 選擇套件
2. 檢視其腳本
3. 點擊執行

**批次操作**

在所有套件中執行相同腳本：

1. 選擇多個套件
2. 選擇共同腳本
3. 平行或循序執行

## 任務快速切換器

快速在您的 monorepo 中尋找並執行任務：

1. 按 <kbd>Cmd</kbd> + <kbd>Shift</kbd> + <kbd>P</kbd>
2. 輸入任務名稱
3. 從過濾結果中選擇
4. 按 Enter 執行

<!-- TODO: Add screenshot of task quick switcher -->

## 套件過濾列

按各種條件過濾套件：

- **名稱**：按套件名稱搜尋
- **路徑**：按目錄過濾
- **已變更**：僅顯示已變更的套件
- **私有**：顯示/隱藏私有套件

<!-- TODO: Add screenshot of package filter bar -->

## 提示

1. **使用依賴圖**：在進行變更前了解關係
2. **快取卡住時清除**：過期快取可能導致令人困惑的問題
3. **過濾以提高速度**：僅在受影響的套件上執行任務
4. **發布前檢查**：透過 Skillshare App 使用 Lerna 的版本指令

## 疑難排解

### 任務未顯示

- 確保您的設定檔有效（`nx.json`、`turbo.json` 等）
- 嘗試重新整理專案

### 任務執行緩慢

- 檢查快取是否正確設定
- 驗證遠端快取連接（Turbo）

### 套件未偵測到

- 驗證套件有有效的 `package.json`
- 檢查工作區設定模式
