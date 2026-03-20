# 專案管理

Skillshare App 自動偵測並管理您的 JavaScript/TypeScript 專案，為所有開發任務提供視覺化介面。

## 匯入專案

### 拖放

新增專案最快的方式是將資料夾拖曳到 Skillshare App 中。

<!-- TODO: Add gif of drag and drop import -->

### 匯入按鈕

點擊側邊欄的**匯入專案**按鈕，瀏覽並選擇專案資料夾。

### 需求

- 資料夾必須包含 `package.json` 檔案
- Skillshare App 將掃描目錄以取得專案元資料

## 自動框架偵測

Skillshare App 自動識別您專案的框架和工具：

### 支援的框架

| 框架 | 偵測方式 |
|------|----------|
| React | 依賴中包含 `react` |
| Vue | 依賴中包含 `vue` |
| Next.js | 依賴中有 `next` |
| Nuxt | 依賴中有 `nuxt` |
| Remix | 依賴中有 `@remix-run/*` |
| Angular | 依賴中有 `@angular/core` |
| Svelte | 依賴中有 `svelte` |
| Expo | 依賴中有 `expo` |
| React Native | 依賴中有 `react-native` |
| Electron | 依賴中有 `electron` |
| Tauri | 依賴中有 `@tauri-apps/*` |

<!-- TODO: Add screenshot showing framework badges on project cards -->

### UI 函式庫

Skillshare App 也會偵測 UI 框架：

- React
- Vue
- Svelte
- Solid
- Preact
- Lit
- Qwik

## 專案資訊

對於每個專案，Skillshare App 顯示：

### 基本資訊
- **專案名稱** - 來自 `package.json`
- **版本** - 目前版本號
- **路徑** - 專案目錄位置
- **框架** - 偵測到的框架標籤

### 腳本
- `package.json` 中定義的所有腳本
- 顯示為可點擊的卡片

### 依賴
- 生產依賴數量
- 開發依賴數量
- 同級依賴（如有）

<!-- TODO: Add screenshot of project details panel -->

## 管理專案

### 移除專案

從 Skillshare App 移除專案：

1. 在側邊欄右鍵點擊專案
2. 選擇**移除專案**
3. 確認操作

> 注意：這只會從 Skillshare App 移除專案，您的檔案不會被刪除。

### 刪除 node_modules

釋放磁碟空間：

1. 右鍵點擊專案
2. 選擇**刪除 node_modules**
3. 確認刪除

這對於清理您目前沒有在處理的專案很有用。

<!-- TODO: Add screenshot of context menu with delete node_modules option -->

## 工作區套件（Monorepo）

如果您的專案使用工作區（npm、yarn 或 pnpm），Skillshare App 將：

1. 偵測工作區設定
2. 列出工作區中的所有套件
3. 允許您在個別套件中執行腳本

詳見 [Monorepo 支援](./monorepo-support.md)。

## 專案重新整理

Skillshare App 監控 `package.json` 的變更。當偵測到變更時：

- 腳本自動更新
- 重新計算依賴
- 重新整理框架偵測

您也可以手動重新整理，右鍵點擊專案並選擇**重新整理**。

## 提示

1. **按資料夾組織**：將相關專案放在同一個父目錄中，方便批次匯入
2. **使用 worktree**：對於有多個分支的專案，使用 [Worktree 管理](./worktree-management.md) 而非多次 clone
3. **定期清理**：使用「刪除 node_modules」功能釋放您目前沒在使用的專案空間
