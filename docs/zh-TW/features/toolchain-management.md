# 工具鏈管理

偵測和管理 Node.js 版本、套件管理器，並解決版本衝突。

## 概覽

Skillshare App 幫助您管理開發工具鏈：

- Node.js 版本偵測
- 套件管理器偵測
- 版本管理器整合（Volta、nvm）
- 衝突解決

<!-- TODO: Add screenshot of toolchain panel -->

## 版本偵測

### Node.js

Skillshare App 偵測：
- 已安裝的 Node.js 版本
- 專案需要的版本（來自 `package.json` → `engines`）
- 版本管理器設定

### 套件管理器

自動偵測：
- npm
- yarn
- pnpm
- bun

基於 lock 檔案偵測：

| Lock 檔案 | 套件管理器 |
|-----------|-----------|
| `package-lock.json` | npm |
| `yarn.lock` | yarn |
| `pnpm-lock.yaml` | pnpm |
| `bun.lockb` | bun |

## 版本管理器支援

### Volta

Skillshare App 與 Volta 深度整合：

**功能：**
- 從 `package.json` 讀取 Volta 設定
- 顯示釘選的 Node/npm/yarn 版本
- 偵測 Volta 安裝
- 產生 Volta 相容指令

**Volta 設定：**
```json
{
  "volta": {
    "node": "20.10.0",
    "npm": "10.2.3"
  }
}
```

### nvm

支援 nvm 設定：

**功能：**
- 讀取 `.nvmrc` 檔案
- 顯示需要的 Node 版本
- 顯示目前版本是否匹配

**.nvmrc：**
```
20.10.0
```

### Corepack

Skillshare App 偵測 Corepack 狀態：

**功能：**
- 檢查 Corepack 是否啟用
- 偵測套件管理器版本
- 顯示 Corepack 警告

## 衝突偵測

Skillshare App 自動偵測常見衝突：

### Volta + Corepack 衝突

同時使用 Volta 和 Corepack 可能導致問題：

- 兩者都試圖管理套件管理器
- 可能導致版本不匹配
- 效能下降

**解決方案：**
1. Skillshare App 顯示警告
2. 選擇其一：Volta 或 Corepack
3. 按照建議的步驟

<!-- TODO: Add screenshot of conflict warning dialog -->

### PNPM_HOME 衝突

當 `PNPM_HOME` 與 Volta 衝突時：

**症狀：**
- pnpm 指令失敗
- 版本不匹配錯誤

**解決方案：**
1. Skillshare App 識別衝突
2. 顯示涉及的環境變數
3. 提供修復指令

### 版本不匹配

當您的 Node 版本與專案不匹配時：

**範例：**
- 專案需要：Node 20.x
- 已安裝：Node 18.x

**解決方案：**
1. 專案卡片上顯示警告
2. 點擊查看詳情
3. 按照升級指示

<!-- TODO: Add screenshot of version mismatch warning -->

## 診斷

### 環境診斷

檢視您完整的工具鏈：

1. 前往**設定** → **工具鏈**
2. 點擊**執行診斷**
3. 查看：
   - Node.js 版本和路徑
   - npm/yarn/pnpm 版本
   - 環境變數
   - 版本管理器狀態

<!-- TODO: Add screenshot of diagnostics panel -->

### 專案診斷

對於每個專案：

1. 點擊專案上的工具鏈圖示
2. 查看專案特定資訊：
   - 需要的版本
   - 目前版本
   - 相容性狀態
   - 衝突

## 套件管理器偏好

### 設定預設

選擇您偏好的套件管理器：

1. 前往**設定** → **工具鏈**
2. 選擇**預設套件管理器**
3. 選擇：npm、yarn、pnpm 或 bun

### 每個專案覆寫

Skillshare App 尊重專案特定設定：
- Lock 檔案決定套件管理器
- Volta 設定優先
- 可手動覆寫

## 版本徽章

專案卡片顯示版本狀態：

| 徽章 | 意義 |
|------|------|
| 綠色 | 版本匹配 |
| 黃色 | 小版本不匹配 |
| 紅色 | 大版本不匹配或衝突 |

<!-- TODO: Add screenshot of version badges on project cards -->

## 指令

Skillshare App 根據您的工具鏈產生正確的指令：

### 使用 Volta

```bash
volta run npm install
volta run npm run build
```

### 使用 Corepack

```bash
corepack enable
pnpm install
pnpm run build
```

### 不使用版本管理器

```bash
npm install
npm run build
```

## 提示

1. **使用 Volta 或 Corepack**：不要混用
2. **釘選版本**：使用 Volta 或 `.nvmrc` 保持一致性
3. **檢查診斷**：當事情看起來不對時
4. **定期更新**：保持 Node.js 和套件管理器最新
5. **檢視衝突**：及時處理警告

## 疑難排解

### 使用錯誤的 Node 版本

1. 檢查版本管理器（Volta、nvm）
2. 驗證 PATH 順序
3. 變更後重新啟動終端機

### 找不到套件管理器

1. 檢查是否全域安裝
2. 驗證 Corepack 狀態
3. 檢查 PATH 環境變數

### 套件安裝緩慢

1. 檢查 Volta + Corepack 衝突
2. 驗證網路連接
3. 嘗試清除快取
