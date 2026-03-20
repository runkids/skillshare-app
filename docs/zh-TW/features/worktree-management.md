# Worktree 管理

輕鬆管理 Git worktree。同時處理多個分支，無需在目錄間切換。

## 什麼是 Worktree？

Git worktree 讓您可以同時簽出多個分支，每個都在自己的目錄中。不需要：

- Stash 變更
- 切換分支
- 失去上下文

您可以同時開啟多個分支。

## 概覽

Skillshare App 讓 worktree 視覺化且易於管理：

<!-- TODO: Add screenshot of worktree list -->

## 檢視 Worktree

所有 worktree 都顯示在 Worktree 面板中，包含：

- **分支名稱**：簽出的分支
- **路徑**：目錄位置
- **狀態**：乾淨、有變更、或領先/落後
- **最後開啟**：上次處理的時間

## 快速切換器

在 worktree 之間切換最快的方式：

1. 按 <kbd>Cmd</kbd> + <kbd>K</kbd>
2. 輸入以過濾 worktree
3. 按 Enter 切換

<!-- TODO: Add gif of quick switcher in action -->

快速切換器顯示：
- Worktree 名稱
- 分支名稱
- 最近狀態
- 鍵盤快捷鍵提示

## 建立 Worktree

### 從現有分支

1. 點擊**新增 Worktree**
2. 選擇現有分支
3. 選擇目標目錄
4. 點擊**建立**

### 使用新分支

1. 點擊**新增 Worktree**
2. 切換**建立新分支**
3. 輸入新分支名稱
4. 選擇基底分支
5. 點擊**建立**

<!-- TODO: Add screenshot of create worktree dialog -->

## Worktree 範本

將常用的 worktree 設定儲存為範本，快速建立。

### 建立範本

1. 點擊 worktree 面板中的**範本**
2. 點擊**新增範本**
3. 設定：
   - 範本名稱
   - 基底分支模式
   - 目錄模式
   - 自動安裝依賴選項
4. 儲存範本

<!-- TODO: Add screenshot of template creation dialog -->

### 使用範本

1. 點擊**從範本新增**
2. 選擇範本
3. 輸入必要值（例如功能名稱）
4. 點擊**建立**

### 範本範例

**功能分支**
- 基底：`main`
- 分支模式：`feature/{name}`
- 目錄：`../worktrees/feature-{name}`

**修補分支**
- 基底：`main`
- 分支模式：`hotfix/{name}`
- 目錄：`../worktrees/hotfix-{name}`

## Worktree 工作階段

Skillshare App 追蹤您的 worktree 工作階段，記住：

- 開啟的終端機
- 執行中的腳本
- UI 狀態

### 恢復工作階段

切換回 worktree 時：

1. Skillshare App 偵測到之前的工作階段
2. 提供恢復您的上下文
3. 重新開啟終端機並恢復狀態

<!-- TODO: Add screenshot of session restore dialog -->

### 工作階段列表

檢視所有工作階段：

1. 點擊 worktree 面板中的**工作階段**
2. 查看所有已儲存的工作階段
3. 點擊恢復或刪除

## 與主分支同步

保持 worktree 與主分支同步：

### 檢查同步狀態

Worktree 卡片顯示您是否落後於主分支。

<!-- TODO: Add screenshot showing behind status indicator -->

### 同步選項

1. 點擊 worktree 上的**同步**
2. 選擇同步方式：
   - **Rebase**：在 main 之上重播您的提交
   - **Merge**：建立合併提交

### 處理衝突

如果同步期間發生衝突：

1. Skillshare App 顯示衝突的檔案
2. 在 Git 面板中解決衝突
3. 繼續 rebase/merge

## 在外部工具中開啟

### 在編輯器中開啟

右鍵點擊 worktree 並選擇您的編輯器：

- VS Code
- Cursor
- Sublime Text
- Vim
- 自訂編輯器

<!-- TODO: Add screenshot of editor selection menu -->

### 在終端機中開啟

在您偏好的終端機中開啟 worktree：

- Terminal.app
- iTerm2
- 自訂終端機

## 刪除 Worktree

### 安全刪除

1. 右鍵點擊 worktree
2. 選擇**刪除**
3. Skillshare App 檢查：
   - 未提交的變更
   - 未 push 的提交
4. 確認刪除

### 強制刪除

如果您有未提交的變更，可以強制刪除：

1. 勾選**強制**選項
2. 變更將會遺失

> 警告：強制刪除無法復原。

## 健康檢查

Skillshare App 監控 worktree 健康狀態：

### 執行的檢查

- 分支在遠端仍然存在
- 沒有過期的鎖定
- 目錄可存取
- Git 儲存庫有效

### 修復問題

如果偵測到問題：

1. 點擊 worktree 上的**修復問題**
2. Skillshare App 嘗試自動修復
3. 如果需要會顯示手動步驟

<!-- TODO: Add screenshot of health check warnings -->

## 提示

1. **使用快速切換器**：<kbd>Cmd</kbd> + <kbd>K</kbd> 是您的好朋友
2. **建立範本**：節省重複模式的時間
3. **定期同步**：避免大型合併衝突
4. **清理舊 worktree**：刪除已合併分支的 worktree
5. **使用工作階段**：讓 Skillshare App 記住您的上下文

## 常見工作流程

### 功能開發

1. 從範本建立 worktree：`feature/{name}`
2. 開發您的功能
3. 定期與 main 同步
4. Push 並建立 PR
5. 合併後刪除 worktree

### 程式碼審查

1. 為 PR 分支建立 worktree
2. 審查和測試
3. 留下評論
4. 完成後刪除 worktree

### 修補程式

1. 從 `main` 建立 worktree：`hotfix/{name}`
2. 套用修正
3. 直接 push 或建立 PR
4. 部署後刪除 worktree
