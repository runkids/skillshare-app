# 安全扫描

可视化 npm audit，含漏洞详情与一键修复。

## 概览

Skillshare App 集成 npm audit 帮助您识别和修复依赖中的安全漏洞。

<!-- TODO: Add screenshot of security audit panel -->

## 运行扫描

### 手动扫描

1. 选择项目
2. 打开**安全**标签
3. 点击**立即扫描**

Skillshare App 运行 `npm audit` 并显示结果。

<!-- TODO: Add gif of running a security scan -->

### 自动提醒

Skillshare App 可以提醒您定期扫描：

1. 前往**设置** → **安全**
2. 启用**扫描提醒**
3. 设置频率（每日、每周、每月）

## 了解结果

### 严重程度

漏洞按严重程度分类：

| 等级 | 颜色 | 说明 |
|------|------|------|
| **Critical** | 红色 | 需要立即处理 |
| **High** | 橙色 | 应尽快修复 |
| **Moderate** | 黄色 | 方便时修复 |
| **Low** | 蓝色 | 风险最小 |
| **Info** | 灰色 | 仅供参考 |

<!-- TODO: Add screenshot of severity badges -->

### 漏洞卡片

每个漏洞显示：

- **包名称**：有漏洞的包
- **严重程度**：Critical、High、Moderate、Low
- **标题**：简短描述
- **路径**：到此包的依赖链
- **可修复**：是否有补丁

## 漏洞详情

点击漏洞查看完整详情：

### 概览
- CVE 识别码（如有）
- CWE 分类
- CVSS 评分
- 受影响版本

### 说明
漏洞及其潜在影响的详细解释。

### 建议
如何修复问题，通常是升级到补丁版本。

### 参考资料
链接到：
- CVE 数据库条目
- GitHub 安全公告
- 包更新日志

<!-- TODO: Add screenshot of vulnerability detail dialog -->

## 修复漏洞

### 一键修复

对于有可用修复的漏洞：

1. 点击漏洞卡片上的**修复**
2. Skillshare App 运行适当的命令：
   - `npm audit fix` 用于安全修复
   - 显示破坏性变更的手动步骤

<!-- TODO: Add gif of one-click fix -->

### 手动修复

对于复杂情况：

1. 查看建议的修复版本
2. 手动更新您的 `package.json`
3. 运行 `npm install`
4. 重新扫描以验证修复

### 破坏性变更

某些修复可能引入破坏性变更。Skillshare App 在以下情况警告您：

- 修复需要主版本升级
- 修复可能影响其他依赖
- 建议手动测试

## 直接依赖 vs. 传递依赖

### 直接依赖

列在您 `package.json` 中的包。您直接控制这些。

### 传递依赖

作为您依赖的依赖安装的包。修复这些可能需要：

- 升级直接依赖
- 等待维护者修复
- 在 `package.json` 中使用 `overrides`

<!-- TODO: Add diagram showing direct vs transitive -->

## 扫描历史

查看过去的扫描：

1. 点击安全标签中的**历史**
2. 查看最近 10 次扫描，包含：
   - 时间戳
   - 发现的总漏洞数
   - 按严重程度分类

跟踪您减少漏洞的进度。

<!-- TODO: Add screenshot of scan history -->

## Monorepo 支持

对于 monorepo，Skillshare App 扫描每个工作区：

1. 点击**扫描所有工作区**
2. 结果按包分组
3. 按工作区名称过滤

<!-- TODO: Add screenshot of monorepo security view -->

## 过滤结果

### 按严重程度

过滤只显示特定严重程度：

- 只显示 Critical 和 High
- 隐藏 Low 和 Info
- 专注于最重要的

### 按包

搜索特定包中的漏洞。

### 按修复状态

- **可修复**：有补丁可用
- **无修复**：等待上游修复

## 导出报告

生成安全报告：

1. 点击**导出报告**
2. 选择格式：
   - JSON（用于 CI/CD）
   - Markdown（用于文档）
   - CSV（用于电子表格）

## 提示

1. **定期扫描**：至少每周运行扫描
2. **优先处理 Critical**：按严重程度排序
3. **更新依赖**：许多漏洞通过更新修复
4. **部署前检查**：每次正式部署前运行扫描
5. **查看传递依赖**：有时需要变更直接依赖来修复传递依赖

## 疑难排解

### 扫描失败

- 确保 `package-lock.json` 存在
- 先尝试运行 `npm install`
- 检查网络问题

### 误报

某些漏洞可能不影响您的使用：

1. 检查漏洞详情
2. 评估您的代码是否使用受影响的功能
3. 考虑风险是否可接受

### 无法修复漏洞

如果没有可用的修复：

1. 向包维护者提交 issue
2. 考虑有漏洞包的替代方案
3. 如果可能实现绕过方案

## Lockfile 验证

Skillshare App 包含供应链安全验证功能。此功能可在问题发生前检测潜在的安全风险。

### 配置验证

1. 前往**设置** → **安全** → **Lockfile 验证**
2. 启用/禁用验证功能
3. 选择严格等级：
   - **宽松**：仅 Critical 问题
   - **标准**：平衡检测（推荐）
   - **严格**：最大保护

### 验证规则

| 规则 | 说明 |
|------|------|
| **不安全协议** | 检测通过不安全协议解析的包（git://、http://） |
| **非预期 Registry** | 标记来自非白名单 registry 的包 |
| **Manifest 不一致** | 检测 lockfile 与 package.json 不符 |
| **封禁包** | 检测到封禁列表中的包时警示 |
| **缺少 Integrity** | 标记缺少 integrity hash 的包 |
| **Typosquatting 检测** | 识别潜在的名称仿冒攻击 |

### Registry 白名单

管理允许的 registry：

1. 前往**设置** → **安全** → **Lockfile 验证**
2. 添加信任的 registry（如 `https://registry.npmjs.org`）
3. 移除不信任的 registry

### 封禁包

维护封禁列表：

1. 添加要封禁的包名称
2. 提供封禁原因
3. 快照会自动标记这些包

### Typosquatting 检测

Skillshare App 检测三种类型的仿冒：

- **名称相似度**：对热门包进行 Levenshtein 距离分析
- **Scope 混淆**：检测 `@scope/pkg` vs `scope-pkg` 模式
- **同形字攻击**：识别相似的 Unicode 字符

### 验证结果

验证问题会显示在：

- Time Machine 快照的 Security 标签
- 安全标签概览
- 项目仪表板

每个问题显示严重程度（critical、high、medium、low、info）和建议操作。

## 安全审计日志

Skillshare App 维护应用程序中安全相关事件的完整审计日志。

### 访问审计日志

1. 前往**设置** → **安全** → **Security Audit**
2. 使用过滤选项查看事件时间轴
3. 导出日志进行合规性检查或分析

### 事件类型

| 事件类型 | 说明 |
|----------|------|
| **Webhook Trigger** | 外部 webhook 请求及其结果 |
| **Authentication** | 登录尝试、HMAC 签名验证 |
| **Tool Execution** | AI 助手工具调用及结果 |
| **Security Alert** | 速率限制、可疑活动 |
| **Data Access** | 敏感数据访问事件 |
| **Configuration** | 安全相关设置变更 |

### 执行者类型

事件归因于不同的执行者类型：

- **User**：手动用户操作
- **AI Assistant**：AI 助手执行的操作
- **Webhook**：外部 webhook 请求
- **System**：自动化系统操作

### 过滤事件

按以下条件过滤审计日志：

- **时间范围**：过去 24 小时、7 天或 30 天
- **事件类型**：按特定事件类别过滤
- **执行者类型**：按执行操作者过滤
- **结果**：成功、失败或拒绝

### 事件详情

点击事件查看：

- **Event ID**：事件的唯一标识符
- **Resource**：受影响资源的类型和名称
- **Outcome Reason**：事件成功或失败的原因
- **Actor Details**：Session ID、来源 IP
- **Additional Details**：包含额外上下文的 JSON 数据
- **Timestamp**：事件的确切时间

### 导出日志

导出审计日志进行合规性检查或外部分析：

1. 在 Security Audit 面板点击**Export**
2. 日志以 JSON 格式导出
3. 包含所有已过滤的事件

### 保留策略

- 审计日志在 90 天后自动清理
- 每次新增事件时自动执行清理
- 如需合规性要求，请在保留期限前导出日志
