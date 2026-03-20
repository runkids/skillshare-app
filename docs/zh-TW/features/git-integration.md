# Git 整合

無需離開 Skillshare App 即可進行視覺化 Git 操作。暫存、提交、分支、差異比對 — 全部無需 CLI 體操。

## 概覽

Git 面板提供完整的常見 Git 操作介面：

- 檔案暫存與取消暫存
- 建立提交
- 分支管理
- 差異檢視
- Stash 操作

<!-- TODO: Add screenshot of Git panel overview -->

## Git 狀態檢視

### 檔案狀態

狀態檢視按狀態組織顯示所有變更的檔案：

| 區段 | 說明 |
|------|------|
| **已暫存** | 準備提交的檔案 |
| **已修改** | 變更但未暫存的檔案 |
| **未追蹤** | Git 中沒有的新檔案 |
| **衝突** | 有合併衝突的檔案 |

<!-- TODO: Add screenshot of file status sections -->

### 檔案操作

對於每個檔案，您可以：

- **暫存**：新增到暫存區（點擊 `+`）
- **取消暫存**：從暫存區移除（點擊 `-`）
- **捨棄**：還原變更（點擊 `↩`）
- **檢視差異**：查看變更內容

## 暫存檔案

### 暫存單一檔案

點擊任何已修改或未追蹤檔案旁的 `+` 按鈕。

### 全部暫存

點擊**全部暫存**一次新增所有變更。

### 部分暫存

更精細的控制：

1. 點擊檔案檢視其差異
2. 選擇特定區塊進行暫存
3. 僅暫存您需要的部分

<!-- TODO: Add gif of partial staging -->

## 提交

### 建立提交

1. 暫存您的變更
2. 輸入提交訊息
3. 點擊**提交**

<!-- TODO: Add screenshot of commit form -->

### AI 產生訊息

Skillshare App 可以使用 AI 產生提交訊息：

1. 暫存您的變更
2. 點擊訊息輸入旁的 **AI** 按鈕
3. 檢視產生的訊息
4. 需要時編輯，然後提交

詳見 [AI 整合](./ai-integration.md) 進行設定。

<!-- TODO: Add screenshot of AI commit message generation -->

### 提交指引

Skillshare App 顯示字元數並在您的訊息有以下情況時警告：
- 太短（少於 10 個字元）
- 第一行太長（超過 72 個字元）

## 差異檢視器

### 統一檢視

以單欄格式查看變更：
- 紅色行表示刪除
- 綠色行表示新增
- 行號作為參考

<!-- TODO: Add screenshot of unified diff view -->

### 分割檢視

並排比較新舊版本：

1. 點擊差異工具列中的**分割**
2. 左側顯示原始版本
3. 右側顯示您的變更

<!-- TODO: Add screenshot of split diff view -->

### 語法高亮

差異根據檔案類型進行語法高亮，更容易閱讀。

### 大型檔案

Skillshare App 對大型差異使用虛擬化，即使有數千行也能保持效能流暢。

## 分支操作

### 檢視分支

在分支面板中查看所有本地和遠端分支。

<!-- TODO: Add screenshot of branch list -->

### 建立分支

1. 點擊**新增分支**
2. 輸入分支名稱
3. 選擇基底分支
4. 點擊**建立**

### 切換分支

點擊任何分支以切換到它。Skillshare App 會：
- 如果有未提交的變更會警告
- 提供 stash 或捨棄變更的選項

### 刪除分支

1. 右鍵點擊分支
2. 選擇**刪除**
3. 確認刪除

> 注意：您無法刪除目前簽出的分支。

## Stash 操作

### 建立 Stash

1. 進行一些變更
2. 點擊 Git 面板中的 **Stash**
3. 選擇性地新增訊息
4. 您的變更已儲存

### 套用 Stash

1. 檢視 stash 列表
2. 點擊 stash 預覽
3. 點擊**套用**還原變更
4. 選擇保留或刪除 stash

<!-- TODO: Add screenshot of stash list -->

### 刪除 Stash

1. 右鍵點擊 stash
2. 選擇**刪除**
3. 確認刪除

## Pull 和 Push

### Pull 變更

點擊 **Pull** 取得並合併遠端變更。

- Skillshare App 顯示您是否落後於遠端
- 衝突會在狀態檢視中標示

### Push 變更

點擊 **Push** 上傳您的提交。

- 顯示要 push 的提交數量
- 如果遠端有新提交會警告

<!-- TODO: Add screenshot of push/pull buttons with status -->

## 提交歷史

檢視專案的提交歷史：

1. 點擊 Git 面板中的**歷史**
2. 查看最近的提交，包含：
   - 提交訊息
   - 作者
   - 日期
   - SHA

<!-- TODO: Add screenshot of commit history -->

## 提示

1. **經常提交**：小型、專注的提交更容易審查和還原
2. **使用 AI 訊息**：讓 AI 起草，然後精煉
3. **審查差異**：總是檢查您正在提交什麼
4. **切換前 stash**：換分支時不要丟失工作
5. **push 前 pull**：保持最新狀態以避免衝突
