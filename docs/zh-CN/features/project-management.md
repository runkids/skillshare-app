# 项目管理

Skillshare App 自动检测并管理您的 JavaScript/TypeScript 项目，为所有开发任务提供可视化界面。

## 导入项目

### 拖放

添加项目最快的方式是将文件夹拖到 Skillshare App 中。

<!-- TODO: Add gif of drag and drop import -->

### 导入按钮

点击侧边栏的**导入项目**按钮，浏览并选择项目文件夹。

### 要求

- 文件夹必须包含 `package.json` 文件
- Skillshare App 将扫描目录以获取项目元数据

## 自动框架检测

Skillshare App 自动识别您项目的框架和工具：

### 支持的框架

| 框架 | 检测方式 |
|------|----------|
| React | 依赖中包含 `react` |
| Vue | 依赖中包含 `vue` |
| Next.js | 依赖中有 `next` |
| Nuxt | 依赖中有 `nuxt` |
| Remix | 依赖中有 `@remix-run/*` |
| Angular | 依赖中有 `@angular/core` |
| Svelte | 依赖中有 `svelte` |
| Expo | 依赖中有 `expo` |
| React Native | 依赖中有 `react-native` |
| Electron | 依赖中有 `electron` |
| Tauri | 依赖中有 `@tauri-apps/*` |

<!-- TODO: Add screenshot showing framework badges on project cards -->

### UI 库

Skillshare App 也会检测 UI 框架：

- React
- Vue
- Svelte
- Solid
- Preact
- Lit
- Qwik

## 项目信息

对于每个项目，Skillshare App 显示：

### 基本信息
- **项目名称** - 来自 `package.json`
- **版本** - 当前版本号
- **路径** - 项目目录位置
- **框架** - 检测到的框架标签

### 脚本
- `package.json` 中定义的所有脚本
- 显示为可点击的卡片

### 依赖
- 生产依赖数量
- 开发依赖数量
- 同级依赖（如有）

<!-- TODO: Add screenshot of project details panel -->

## 管理项目

### 移除项目

从 Skillshare App 移除项目：

1. 在侧边栏右键点击项目
2. 选择**移除项目**
3. 确认操作

> 注意：这只会从 Skillshare App 移除项目，您的文件不会被删除。

### 删除 node_modules

释放磁盘空间：

1. 右键点击项目
2. 选择**删除 node_modules**
3. 确认删除

这对于清理您目前没有在处理的项目很有用。

<!-- TODO: Add screenshot of context menu with delete node_modules option -->

## 工作区包（Monorepo）

如果您的项目使用工作区（npm、yarn 或 pnpm），Skillshare App 将：

1. 检测工作区配置
2. 列出工作区中的所有包
3. 允许您在单独的包中运行脚本

详见 [Monorepo 支持](./monorepo-support.md)。

## 项目刷新

Skillshare App 监控 `package.json` 的变更。当检测到变更时：

- 脚本自动更新
- 重新计算依赖
- 刷新框架检测

您也可以手动刷新，右键点击项目并选择**刷新**。

## 提示

1. **按文件夹组织**：将相关项目放在同一个父目录中，方便批量导入
2. **使用 worktree**：对于有多个分支的项目，使用 [Worktree 管理](./worktree-management.md) 而非多次 clone
3. **定期清理**：使用「删除 node_modules」功能释放您目前没在使用的项目空间
