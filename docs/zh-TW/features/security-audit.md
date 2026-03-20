# 安全掃描

視覺化 npm audit，含漏洞詳情與一鍵修復。

## 概覽

Skillshare App 整合 npm audit 幫助您識別和修復依賴中的安全漏洞。

<!-- TODO: Add screenshot of security audit panel -->

## 執行掃描

### 手動掃描

1. 選擇專案
2. 開啟**安全**分頁
3. 點擊**立即掃描**

Skillshare App 執行 `npm audit` 並顯示結果。

<!-- TODO: Add gif of running a security scan -->

### 自動提醒

Skillshare App 可以提醒您定期掃描：

1. 前往**設定** → **安全**
2. 啟用**掃描提醒**
3. 設定頻率（每日、每週、每月）

## 了解結果

### 嚴重程度

漏洞按嚴重程度分類：

| 等級 | 顏色 | 說明 |
|------|------|------|
| **Critical** | 紅色 | 需要立即處理 |
| **High** | 橙色 | 應盡快修復 |
| **Moderate** | 黃色 | 方便時修復 |
| **Low** | 藍色 | 風險最小 |
| **Info** | 灰色 | 僅供參考 |

<!-- TODO: Add screenshot of severity badges -->

### 漏洞卡片

每個漏洞顯示：

- **套件名稱**：有漏洞的套件
- **嚴重程度**：Critical、High、Moderate、Low
- **標題**：簡短描述
- **路徑**：到此套件的依賴鏈
- **可修復**：是否有修補程式

## 漏洞詳情

點擊漏洞查看完整詳情：

### 概覽
- CVE 識別碼（如有）
- CWE 分類
- CVSS 評分
- 受影響版本

### 說明
漏洞及其潛在影響的詳細解釋。

### 建議
如何修復問題，通常是升級到修補版本。

### 參考資料
連結到：
- CVE 資料庫條目
- GitHub 安全公告
- 套件更新日誌

<!-- TODO: Add screenshot of vulnerability detail dialog -->

## 修復漏洞

### 一鍵修復

對於有可用修復的漏洞：

1. 點擊漏洞卡片上的**修復**
2. Skillshare App 執行適當的指令：
   - `npm audit fix` 用於安全修復
   - 顯示破壞性變更的手動步驟

<!-- TODO: Add gif of one-click fix -->

### 手動修復

對於複雜情況：

1. 檢視建議的修復版本
2. 手動更新您的 `package.json`
3. 執行 `npm install`
4. 重新掃描以驗證修復

### 破壞性變更

某些修復可能引入破壞性變更。Skillshare App 在以下情況警告您：

- 修復需要主版本升級
- 修復可能影響其他依賴
- 建議手動測試

## 直接依賴 vs. 傳遞依賴

### 直接依賴

列在您 `package.json` 中的套件。您直接控制這些。

### 傳遞依賴

作為您依賴的依賴安裝的套件。修復這些可能需要：

- 升級直接依賴
- 等待維護者修復
- 在 `package.json` 中使用 `overrides`

<!-- TODO: Add diagram showing direct vs transitive -->

## 掃描歷史

檢視過去的掃描：

1. 點擊安全分頁中的**歷史**
2. 查看最近 10 次掃描，包含：
   - 時間戳記
   - 發現的總漏洞數
   - 按嚴重程度分類

追蹤您減少漏洞的進度。

<!-- TODO: Add screenshot of scan history -->

## Monorepo 支援

對於 monorepo，Skillshare App 掃描每個工作區：

1. 點擊**掃描所有工作區**
2. 結果按套件分組
3. 按工作區名稱過濾

<!-- TODO: Add screenshot of monorepo security view -->

## 過濾結果

### 按嚴重程度

過濾只顯示特定嚴重程度：

- 只顯示 Critical 和 High
- 隱藏 Low 和 Info
- 專注於最重要的

### 按套件

搜尋特定套件中的漏洞。

### 按修復狀態

- **可修復**：有修補程式可用
- **無修復**：等待上游修復

## 匯出報告

產生安全報告：

1. 點擊**匯出報告**
2. 選擇格式：
   - JSON（用於 CI/CD）
   - Markdown（用於文檔）
   - CSV（用於試算表）

## 提示

1. **定期掃描**：至少每週執行掃描
2. **優先處理 Critical**：按嚴重程度排序
3. **更新依賴**：許多漏洞透過更新修復
4. **部署前檢查**：每次正式部署前執行掃描
5. **檢視傳遞依賴**：有時需要變更直接依賴來修復傳遞依賴

## 疑難排解

### 掃描失敗

- 確保 `package-lock.json` 存在
- 先嘗試執行 `npm install`
- 檢查網路問題

### 誤報

某些漏洞可能不影響您的使用：

1. 檢查漏洞詳情
2. 評估您的程式碼是否使用受影響的功能
3. 考慮風險是否可接受

### 無法修復漏洞

如果沒有可用的修復：

1. 向套件維護者提交 issue
2. 考慮有漏洞套件的替代方案
3. 如果可能實作繞過方案

## Lockfile 驗證

Skillshare App 包含供應鏈安全驗證功能。此功能可在問題發生前偵測潛在的安全風險。

### 設定驗證

1. 前往**設定** → **安全** → **Lockfile 驗證**
2. 啟用/停用驗證功能
3. 選擇嚴格等級：
   - **寬鬆**：僅 Critical 問題
   - **標準**：平衡檢測（建議）
   - **嚴格**：最大保護

### 驗證規則

| 規則 | 說明 |
|------|------|
| **不安全協定** | 偵測透過不安全協定解析的套件（git://、http://） |
| **非預期 Registry** | 標記來自非白名單 registry 的套件 |
| **Manifest 不一致** | 偵測 lockfile 與 package.json 不符 |
| **封鎖套件** | 偵測到封鎖清單中的套件時警示 |
| **缺少 Integrity** | 標記缺少 integrity hash 的套件 |
| **Typosquatting 偵測** | 識別潛在的名稱仿冒攻擊 |

### Registry 白名單

管理允許的 registry：

1. 前往**設定** → **安全** → **Lockfile 驗證**
2. 新增信任的 registry（如 `https://registry.npmjs.org`）
3. 移除不信任的 registry

### 封鎖套件

維護封鎖清單：

1. 新增要封鎖的套件名稱
2. 提供封鎖原因
3. 快照會自動標記這些套件

### Typosquatting 偵測

Skillshare App 偵測三種類型的仿冒：

- **名稱相似度**：對熱門套件進行 Levenshtein 距離分析
- **Scope 混淆**：偵測 `@scope/pkg` vs `scope-pkg` 模式
- **同形字攻擊**：識別相似的 Unicode 字元

### 驗證結果

驗證問題會顯示在：

- Time Machine 快照的 Security 分頁
- 安全分頁總覽
- 專案儀表板

每個問題顯示嚴重程度（critical、high、medium、low、info）和建議操作。

## 安全稽核日誌

Skillshare App 維護應用程式中安全相關事件的完整稽核日誌。

### 存取稽核日誌

1. 前往**設定** → **安全** → **Security Audit**
2. 使用過濾選項查看事件時間軸
3. 匯出日誌進行合規性檢查或分析

### 事件類型

| 事件類型 | 說明 |
|----------|------|
| **Webhook Trigger** | 外部 webhook 請求及其結果 |
| **Authentication** | 登入嘗試、HMAC 簽名驗證 |
| **Tool Execution** | AI 助手工具呼叫及結果 |
| **Security Alert** | 速率限制、可疑活動 |
| **Data Access** | 敏感資料存取事件 |
| **Configuration** | 安全相關設定變更 |

### 執行者類型

事件歸因於不同的執行者類型：

- **User**：手動使用者操作
- **AI Assistant**：AI 助手執行的操作
- **Webhook**：外部 webhook 請求
- **System**：自動化系統操作

### 過濾事件

依以下條件過濾稽核日誌：

- **時間範圍**：過去 24 小時、7 天或 30 天
- **事件類型**：依特定事件類別過濾
- **執行者類型**：依執行操作者過濾
- **結果**：成功、失敗或拒絕

### 事件詳情

點擊事件查看：

- **Event ID**：事件的唯一識別碼
- **Resource**：受影響資源的類型和名稱
- **Outcome Reason**：事件成功或失敗的原因
- **Actor Details**：Session ID、來源 IP
- **Additional Details**：包含額外上下文的 JSON 資料
- **Timestamp**：事件的確切時間

### 匯出日誌

匯出稽核日誌進行合規性檢查或外部分析：

1. 在 Security Audit 面板點擊**Export**
2. 日誌以 JSON 格式匯出
3. 包含所有已過濾的事件

### 保留政策

- 稽核日誌在 90 天後自動清理
- 每次新增事件時自動執行清理
- 如需合規性要求，請在保留期限前匯出日誌
