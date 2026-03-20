# 疑難排解

常見問題與快速解法。

## 安裝

### Homebrew 安裝失敗

- 先執行 `brew update`，再重試 `brew install --cask skillshare-app`
- 若 tap 不存在：`brew tap runkids/tap`

### macOS 擋住 App（Gatekeeper）

若 macOS 顯示「無法驗證開發者」或類似警告：

1. 打開 **系統設定** → **隱私權與安全性**
2. 找到被封鎖的 App 提示
3. 點擊 **仍要打開**

<!-- TODO: Add screenshot of macOS “Open Anyway” flow. -->

## 匯入專案

### 專案匯入失敗

- 確認資料夾根目錄有 `package.json`
- 確認 Skillshare App 有該資料夾的存取權（系統設定 → 隱私權與安全性）
- 先嘗試匯入較小的 repo 以確認基本流程

### Scripts 沒顯示 / 沒更新

- 確認 `package.json#scripts` 內真的有 scripts
- 在專案上按右鍵並 **Refresh**（若介面提供）
- 若是 monorepo/workspaces，請參考：`docs/zh-TW/features/monorepo-support.md`

## Script / 工作流執行

### 「Command not found」/ Node 版本不對

- 先檢查工具鏈管理：`docs/zh-TW/features/toolchain-management.md`
- 若你用 Volta/Corepack/nvm，請確認 repo 設定一致

### Dev server 起來了但瀏覽器連不到

- 查看終端輸出使用的 port
- 檢查是否有 port 衝突
- 若透過 MCP 啟動 dev server，請確認 MCP 設定中的 **Dev Server Mode**

## MCP 伺服器

### AI 客戶端連不上

- 在 Skillshare App 打開 **Settings → MCP → MCP Integration**，複製產生的設定
- 確認你的 MCP 客戶端指向 Settings 顯示的 `skillshare-mcp` 路徑
- 建議先用 **Read Only** 模式驗證連線

### Claude Desktop / VS Code 設定檔位置搞不清楚

Skillshare App 的 MCP 快速設定區會顯示正確路徑提示。

<!-- TODO: Add screenshot of MCP quick setup section (paths + copy buttons). -->

## 部署

### 部署一開始就失敗

- 在 **Settings → Deploy Accounts** 確認帳號連線
- 確認 build command 與 output 目錄設定正確
- 確認必要的環境變數已設定（並區分 preview / production）

## 還是卡住？

- 先看：`docs/zh-TW/getting-started.md`
- 搜尋既有 issues：https://github.com/runkids/skillshare-app/issues
- 開新 issue 時請提供：
  - macOS 版本
  - Skillshare App 版本
  - 重現步驟
  - 截圖/日誌（請先遮蔽敏感資訊）

