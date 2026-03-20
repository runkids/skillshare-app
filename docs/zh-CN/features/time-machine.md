# 时间机器与安全守护

Skillshare App 的「时间机器（Time Machine）」会在 lockfile 变更时自动捕获依赖快照，让你追踪依赖演进、检测潜在风险，并在不同状态之间对比差异。

## 概览

时间机器提供：

- **自动快照**：lockfile 变更时自动捕获（含 debounce）
- **手动快照**：需要时一键捕获当前状态
- **安全守护**：实时检测可疑包与 postinstall scripts
- **差异分析**：对比两个快照的变更内容
- **完整性检查**：用快照对比当前依赖状态是否漂移

## 入口

时间机器位于 Project Explorer 的 **Snapshots** 标签页：

```
Project Explorer Tabs:
Scripts | Workspaces | Workflows | Git | Builds | Security | Deploy | Snapshots
                                                                        ↑
```

你也可以在项目标题栏使用 **Snapshots** 快捷按钮（青色按钮）。

## 功能

### 1) 自动捕获快照

当项目的 lockfile 发生变更时，Skillshare App 会自动：

- 检测 lockfile 变更（package-lock.json、pnpm-lock.yaml、yarn.lock、bun.lockb）
- 等待 debounce（默认 2 秒）
- 解析 lockfile 并抽取依赖树
- 检测 postinstall scripts
- 计算安全分数
- 压缩并保存快照数据

**触发来源（Trigger Source）类型：**

- `lockfile_change`：lockfile 变更触发的自动捕获
- `manual`：用户通过 UI 或 AI Assistant 手动捕获

### 2) 快照时间线

按时间顺序查看所有快照：

- 按日期范围或触发来源过滤
- 一眼看到安全分数
- 标记包含 postinstall script 的快照
- 快速进入差异对比

### 3) 依赖差异视图

对比任意两个快照可看到：

- 新增 / 移除 / 更新的包
- 版本变更（含语义化版本分析）
- 新增或变更的 postinstall scripts
- 安全分数变化

### 4) 安全守护

自动化安全分析包括：

#### 拼写相似（Typosquatting）检测

识别名称与热门包相近的可疑包：

- `lodahs` vs `lodash`
- `reqeust` vs `request`
- 使用 Levenshtein distance 算法

#### Postinstall Script 监控

- 跟踪所有包含 postinstall script 的包
- 当出现新的 postinstall script 时提醒
- 展示不同快照间 script 内容的变更

#### 可疑模式检测

- 大幅度 major 版本跳跃（例如 1.0.0 → 9.0.0）
- 非预期版本降级
- 可疑的包命名模式

### 5) 完整性检查

用快照验证当前依赖状态：

- 对比当前 lockfile hash 与快照保存的 hash
- 检测与预期状态的漂移（drift）
- 找出非预期变更

### 6) 安全洞察仪表板

项目级安全概览：

- 总体风险分数（0-100）
- 按严重程度的洞察摘要
- 频繁更新的包
- 拼写相似（typosquatting）提醒历史

### 7) 可搜索历史

跨快照搜索：

- 按包名或版本
- 按日期范围
- 按是否包含 postinstall script
- 按最低安全分数阈值

## 设置

在 **Settings > Storage** 中可配置时间机器：

### Auto-Watch

控制是否对所有项目启用 lockfile 自动监控。启用后，Skillshare App 会监视 lockfile 并在变更时捕获快照。

### Debounce

设置 debounce（默认 2000ms），避免安装过程中短时间内触发大量连续捕获。

## 存储管理

快照存放在：

```
~/Library/Application Support/com.skillshare.app/time-machine/snapshots/
```

每个快照包含：

- 压缩的 lockfile（`.zst`）
- 压缩的 package.json（`.zst`）
- 依赖树 JSON
- postinstall 清单

### 保留策略（Retention）

在 Settings > Storage 中设置：

- 每个项目最多保留的快照数量
- 手动清理旧快照
- 清理孤儿存储文件

## MCP 工具

时间机器与 MCP 服务器集成，让 AI 助手可以调用：

| Tool | 说明 |
|------|------|
| `list_snapshots` | 列出项目的快照 |
| `capture_snapshot` | 手动捕获快照 |
| `get_snapshot_details` | 获取完整快照（含依赖） |
| `compare_snapshots` | 对比两个快照差异 |
| `search_snapshots` | 跨快照搜索 |
| `check_dependency_integrity` | 检查是否与最新快照漂移 |
| `get_security_insights` | 获取项目安全洞察 |
| `export_security_report` | 导出安全报告 |

## AI Assistant 快捷动作

在 AI Assistant 中，时间机器提供快捷动作：

- **Capture Snapshot**：捕获当前依赖状态
- **View Snapshots**：打开 Snapshots 标签页
- **Check Integrity**：检查依赖完整性

## 最佳实践

1. **启用 Auto-Watch**：重要项目保持自动监控
2. **关注 postinstall**：新出现的 postinstall script 需要特别留意
3. **处理 typosquatting**：遇到可疑包名先验证
4. **定期对比**：大型依赖更新后，对比两个快照查看变更
5. **定期清理**：通过保留策略控制存储占用

## 安全分数计算

安全分数（0-100）会综合考虑：

- postinstall script 数量（越多通常风险越高）
- typosquatting 可疑项
- 已知漏洞/可疑模式
- 依赖树深度与复杂度

| 分数区间 | 风险等级 |
|----------|----------|
| 80-100 | 低 |
| 60-79 | 中 |
| 40-59 | 高 |
| 0-39 | 严重 |

## 相关功能

- [安全扫描](./security-audit.md)
- [项目管理](./project-management.md)
- [MCP 服务器](./mcp-server.md)
- [AI 集成](./ai-integration.md)

