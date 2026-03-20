# Monorepo 支持

Skillshare App 为 Nx、Turborepo、Lerna 和原生工作区等 monorepo 工具提供一流支持。

## 概览

当您导入项目时，会自动检测 Monorepo。Skillshare App 识别：

- 工作区配置
- monorepo 中的所有包
- 构建工具（Nx、Turbo、Lerna）
- 依赖关系

<!-- TODO: Add screenshot of monorepo view -->

## Nx 支持

### 检测

Skillshare App 通过以下方式检测 Nx 项目：
- 根目录中的 `nx.json`
- 依赖中的 `@nx/*` 包

### 功能

**Target 检测**
- 自动发现所有 Nx targets
- Targets 显示为可运行的按钮

**运行 Targets**
```
nx run <project>:<target>
```

点击 target 卡片在特定项目上运行。

<!-- TODO: Add screenshot of Nx targets panel -->

**依赖图**

查看项目依赖图：

1. 点击 Nx 面板中的**显示图表**
2. 打开交互式可视化
3. 点击节点查看依赖

<!-- TODO: Add screenshot of Nx dependency graph -->

**缓存管理**

- 查看缓存状态
- 一键清除 Nx 缓存
- 查看缓存命中率

## Turborepo 支持

### 检测

Skillshare App 通过以下方式检测 Turbo 项目：
- 根目录中的 `turbo.json`
- 依赖中的 `turbo`

### 功能

**Pipeline 检测**

发现并显示 `turbo.json` 中定义的所有 pipelines。

**运行任务**
```
turbo run <task>
```

点击任务在所有包上运行。

<!-- TODO: Add screenshot of Turbo pipeline panel -->

**过滤**

在特定包上运行任务：

1. 在过滤栏中选择包
2. 点击任务运行
3. 仅影响选中的包

**缓存管理**

- 查看远程缓存状态
- 清除本地缓存
- 切换远程缓存

## Lerna 支持

### 检测

Skillshare App 通过以下方式检测 Lerna 项目：
- 根目录中的 `lerna.json`
- 依赖中的 `lerna`

### 功能

- 列出所有包
- 跨包运行命令
- 版本管理支持

## 原生工作区

Skillshare App 支持以下工作区：

| 包管理器 | 配置位置 |
|-----------|----------|
| npm | `package.json` → `workspaces` |
| yarn | `package.json` → `workspaces` |
| pnpm | `pnpm-workspace.yaml` |

### 功能

**包列表**

列出所有工作区包，包含：
- 包名称
- 版本
- 位置
- 可用脚本

<!-- TODO: Add screenshot of workspace packages list -->

**在包中运行**

在特定包中运行脚本：

1. 选择包
2. 查看其脚本
3. 点击运行

**批量操作**

在所有包中运行相同脚本：

1. 选择多个包
2. 选择共同脚本
3. 并行或顺序运行

## 任务快速切换器

快速在您的 monorepo 中查找并运行任务：

1. 按 <kbd>Cmd</kbd> + <kbd>Shift</kbd> + <kbd>P</kbd>
2. 输入任务名称
3. 从过滤结果中选择
4. 按 Enter 运行

<!-- TODO: Add screenshot of task quick switcher -->

## 包过滤栏

按各种条件过滤包：

- **名称**：按包名称搜索
- **路径**：按目录过滤
- **已变更**：仅显示已变更的包
- **私有**：显示/隐藏私有包

<!-- TODO: Add screenshot of package filter bar -->

## 提示

1. **使用依赖图**：在进行变更前了解关系
2. **缓存卡住时清除**：过期缓存可能导致令人困惑的问题
3. **过滤以提高速度**：仅在受影响的包上运行任务
4. **发布前检查**：通过 Skillshare App 使用 Lerna 的版本命令

## 疑难排解

### 任务未显示

- 确保您的配置文件有效（`nx.json`、`turbo.json` 等）
- 尝试刷新项目

### 任务运行缓慢

- 检查缓存是否正确配置
- 验证远程缓存连接（Turbo）

### 包未检测到

- 验证包有有效的 `package.json`
- 检查工作区配置模式
