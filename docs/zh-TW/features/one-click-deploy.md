# 一鍵部署

一鍵將您的專案部署到 Netlify、Cloudflare Pages 或 GitHub Pages。

## 概覽

Skillshare App 整合熱門託管平台，支援：

- 一鍵部署
- 即時預覽連結
- 環境變數管理
- 多帳戶支援

<!-- TODO: Add screenshot of deploy panel -->

## 支援的平台

| 平台 | 驗證方式 | 功能 |
|------|----------|------|
| **Netlify** | OAuth | 完整整合 |
| **Cloudflare Pages** | API Token | 完整支援 |
| **GitHub Pages** | GitHub Actions | 工作流程產生 |

## 連接帳戶

### Netlify

1. 前往**設定** → **部署帳戶**
2. 點擊**新增帳戶** → **Netlify**
3. 點擊**連接 Netlify**
4. 在瀏覽器中授權 Skillshare App
5. 帳戶已連接

<!-- TODO: Add screenshot of Netlify OAuth flow -->

### Cloudflare Pages

1. 前往**設定** → **部署帳戶**
2. 點擊**新增帳戶** → **Cloudflare**
3. 輸入您的 Cloudflare API Token
4. 點擊**驗證並儲存**

建立 API token：
1. 前往 [Cloudflare Dashboard](https://dash.cloudflare.com/profile/api-tokens)
2. 點擊**建立 Token**
3. 使用**編輯 Cloudflare Pages** 範本
4. 複製 token

<!-- TODO: Add screenshot of Cloudflare token dialog -->

### 多帳戶

您可以連接多個帳戶：
- 多個 Netlify 帳戶
- 多個 Cloudflare 帳戶
- 混合不同平台

在設定中為每個平台設定預設帳戶。

## 建置設定

### 自動偵測

Skillshare App 自動偵測您的框架並建議：

- 建置指令（例如 `npm run build`）
- 輸出目錄（例如 `dist`、`.next`、`build`）
- Node.js 版本

<!-- TODO: Add screenshot of auto-detected build config -->

### 支援的框架

| 框架 | 建置指令 | 輸出 |
|------|----------|------|
| Vite | `vite build` | `dist` |
| Next.js | `next build` | `.next` |
| Nuxt | `nuxt build` | `.output` |
| Create React App | `react-scripts build` | `build` |
| Remix | `remix build` | `build` |
| Astro | `astro build` | `dist` |

### 自訂設定

覆寫偵測的設定：

1. 開啟部署面板
2. 點擊**編輯設定**
3. 修改：
   - 建置指令
   - 輸出目錄
   - 安裝指令
   - Node 版本

## 環境變數

### 新增變數

1. 開啟部署面板中的**環境變數**
2. 點擊**新增變數**
3. 輸入鍵和值
4. 選擇可見性：
   - **Production**：僅在正式環境
   - **Preview**：僅在預覽部署
   - **All**：兩種環境

<!-- TODO: Add screenshot of environment variables panel -->

### 機密變數

對於敏感值：

1. 新增時切換**機密**
2. 值會被加密
3. 儲存後不再顯示在日誌或 UI 中

### 從 `.env` 匯入

1. 點擊**從 .env 匯入**
2. 選擇您的 `.env` 檔案
3. 檢視匯入的變數
4. 儲存到部署設定

## 部署

### 手動部署

1. 選擇專案
2. 開啟部署面板
3. 選擇部署帳戶
4. 點擊**部署**

<!-- TODO: Add gif of deploy process -->

### 部署進度

部署期間，查看：
- 目前狀態
- 建置日誌
- 任何錯誤或警告

### 預覽連結

成功部署後：
- **正式 URL**：您的上線網站
- **預覽 URL**：此次部署的唯一 URL

<!-- TODO: Add screenshot of deploy complete with URLs -->

## 部署歷史

檢視過去的部署：

1. 開啟部署面板
2. 點擊**歷史**
3. 查看所有部署，包含：
   - 時間戳記
   - 狀態（成功/失敗）
   - 持續時間
   - 提交資訊

### 回滾

回滾到之前的部署：

1. 在歷史中找到該部署
2. 點擊**回滾**
3. 確認操作

## GitHub Pages

GitHub Pages 運作方式不同 — Skillshare App 產生 GitHub Actions 工作流程。

### 設定

1. 選擇 **GitHub Pages** 作為部署目標
2. 點擊**產生工作流程**
3. 檢視產生的 `.github/workflows/deploy.yml`
4. 提交並 push

### 運作方式

工作流程：
1. 在 push 到 main/master 時觸發
2. 安裝依賴
3. 執行建置指令
4. 部署到 `gh-pages` 分支

<!-- TODO: Add screenshot of generated workflow file -->

## 部署備份

### 匯出設定

備份您的部署設定：

1. 前往**設定** → **備份**
2. 點擊**匯出部署設定**
3. 儲存 JSON 檔案

### 匯入設定

從備份還原：

1. 前往**設定** → **備份**
2. 點擊**匯入部署設定**
3. 選擇您的備份檔案
4. 檢視並確認

## 提示

1. **使用預覽部署**：正式上線前測試變更
2. **先設定環境變數**：避免部署失敗
3. **檢查建置日誌**：快速了解失敗原因
4. **使用多帳戶**：分開個人和工作專案
5. **備份設定**：設定新機器時節省時間

## 疑難排解

### 建置失敗

- 檢查建置日誌的錯誤
- 驗證您的建置指令在本地運作
- 確保所有環境變數已設定

### 部署卡住

- 檢查平台狀態（Netlify/Cloudflare）
- 取消並重試部署
- 檢查是否有大型檔案上傳

### 缺少環境變數

- 驗證變數名稱完全匹配
- 檢查變數是否設定在正確的環境
- 確保機密正確設定
