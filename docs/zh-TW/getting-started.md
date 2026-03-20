# 快速入門

本指南將協助您安裝 Skillshare App 並開始使用您的第一個專案。

Skillshare App 是一個 AI 驅動的 `package.json` 專案管理工具：匯入一次專案資料夾後，就能一鍵執行 scripts、管理 Git/worktree、建立工作流自動化，並且（可選）透過 MCP 讓你的 AI 工具安全地呼叫動作。

## 安裝

### Homebrew（推薦）

在 macOS 上安裝 Skillshare App 最簡單的方式：

```bash
brew tap runkids/tap
brew install --cask skillshare-app
```

#### 升級

```bash
brew update
brew upgrade --cask skillshare-app
```

### 手動下載

1. 前往 [Releases](https://github.com/runkids/skillshare-app/releases) 頁面
2. 下載最新的 `.dmg` 檔案
3. 開啟 DMG 並將 Skillshare App 拖曳至應用程式資料夾
4. 從應用程式啟動 Skillshare App

## 首次啟動

首次開啟 Skillshare App 時，您會看到空白的專案列表。

<!-- TODO: Add screenshot of empty project list / welcome screen -->

## 匯入您的第一個專案

有兩種方式可以新增專案：

### 方法 1：拖放

只需將包含 `package.json` 的資料夾拖曳到 Skillshare App 視窗中。

<!-- TODO: Add screenshot/gif of drag and drop import -->

### 方法 2：點擊匯入

1. 點擊**匯入專案**按鈕
2. 選擇包含 `package.json` 的資料夾
3. Skillshare App 將掃描並匯入專案

## 了解介面

匯入專案後，您會看到：

<!-- TODO: Add screenshot of main interface with annotations -->

### 主要區域

1. **側邊欄** - 專案列表與導覽
2. **腳本卡片** - 所有 npm 腳本顯示為可點擊的按鈕
3. **終端機面板** - 執行中腳本的即時輸出
4. **狀態列** - 快捷操作與系統狀態

## 執行您的第一個腳本

1. 在側邊欄點擊一個專案
2. 找到您要執行的腳本（例如 `dev`、`build`、`test`）
3. 點擊腳本卡片
4. 在終端機面板查看輸出

<!-- TODO: Add gif of running a script -->

## 快捷鍵

| 快捷鍵 | 操作 |
|--------|------|
| <kbd>Cmd</kbd> + <kbd>K</kbd> | 快速切換 worktree |
| <kbd>Cmd</kbd> + <kbd>1</kbd> | 專案分頁 |
| <kbd>Cmd</kbd> + <kbd>2</kbd> | 工作流程分頁 |
| <kbd>Cmd</kbd> + <kbd>,</kbd> | 設定 |
| <kbd>Cmd</kbd> + <kbd>/</kbd> | 顯示所有快捷鍵 |

## 下一步

現在您已經設定完成，探索這些功能：

- [一鍵執行腳本](./features/one-click-scripts.md) - 精通腳本執行
- [視覺化工作流程](./features/visual-workflow.md) - 自動化多步驟任務
- [Git 整合](./features/git-integration.md) - 視覺化 Git 操作
- [一鍵部署](./features/one-click-deploy.md) - 部署並取得預覽連結
- [時間機器](./features/time-machine.md) - 追蹤依賴歷史與完整性
- [MCP 伺服器](./features/mcp-server.md) - 讓 AI 工具安全地幫你跑動作
- [安全與隱私](./security-and-privacy.md) - 了解本機優先、加密與權限
