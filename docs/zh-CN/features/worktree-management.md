# Worktree 管理

轻松管理 Git worktree。同时处理多个分支，无需在目录间切换。

## 什么是 Worktree？

Git worktree 让您可以同时检出多个分支，每个都在自己的目录中。不需要：

- Stash 变更
- 切换分支
- 失去上下文

您可以同时打开多个分支。

## 概览

Skillshare App 让 worktree 可视化且易于管理：

<!-- TODO: Add screenshot of worktree list -->

## 查看 Worktree

所有 worktree 都显示在 Worktree 面板中，包含：

- **分支名称**：检出的分支
- **路径**：目录位置
- **状态**：干净、有变更、或领先/落后
- **最后打开**：上次处理的时间

## 快速切换器

在 worktree 之间切换最快的方式：

1. 按 <kbd>Cmd</kbd> + <kbd>K</kbd>
2. 输入以过滤 worktree
3. 按 Enter 切换

<!-- TODO: Add gif of quick switcher in action -->

快速切换器显示：
- Worktree 名称
- 分支名称
- 最近状态
- 键盘快捷键提示

## 创建 Worktree

### 从现有分支

1. 点击**新建 Worktree**
2. 选择现有分支
3. 选择目标目录
4. 点击**创建**

### 使用新分支

1. 点击**新建 Worktree**
2. 切换**创建新分支**
3. 输入新分支名称
4. 选择基础分支
5. 点击**创建**

<!-- TODO: Add screenshot of create worktree dialog -->

## Worktree 模板

将常用的 worktree 配置保存为模板，快速创建。

### 创建模板

1. 点击 worktree 面板中的**模板**
2. 点击**新建模板**
3. 配置：
   - 模板名称
   - 基础分支模式
   - 目录模式
   - 自动安装依赖选项
4. 保存模板

<!-- TODO: Add screenshot of template creation dialog -->

### 使用模板

1. 点击**从模板新建**
2. 选择模板
3. 输入必要值（例如功能名称）
4. 点击**创建**

### 模板示例

**功能分支**
- 基础：`main`
- 分支模式：`feature/{name}`
- 目录：`../worktrees/feature-{name}`

**修复分支**
- 基础：`main`
- 分支模式：`hotfix/{name}`
- 目录：`../worktrees/hotfix-{name}`

## Worktree 会话

Skillshare App 跟踪您的 worktree 会话，记住：

- 打开的终端
- 运行中的脚本
- UI 状态

### 恢复会话

切换回 worktree 时：

1. Skillshare App 检测到之前的会话
2. 提供恢复您的上下文
3. 重新打开终端并恢复状态

<!-- TODO: Add screenshot of session restore dialog -->

### 会话列表

查看所有会话：

1. 点击 worktree 面板中的**会话**
2. 查看所有已保存的会话
3. 点击恢复或删除

## 与主分支同步

保持 worktree 与主分支同步：

### 检查同步状态

Worktree 卡片显示您是否落后于主分支。

<!-- TODO: Add screenshot showing behind status indicator -->

### 同步选项

1. 点击 worktree 上的**同步**
2. 选择同步方式：
   - **Rebase**：在 main 之上重放您的提交
   - **Merge**：创建合并提交

### 处理冲突

如果同步期间发生冲突：

1. Skillshare App 显示冲突的文件
2. 在 Git 面板中解决冲突
3. 继续 rebase/merge

## 在外部工具中打开

### 在编辑器中打开

右键点击 worktree 并选择您的编辑器：

- VS Code
- Cursor
- Sublime Text
- Vim
- 自定义编辑器

<!-- TODO: Add screenshot of editor selection menu -->

### 在终端中打开

在您偏好的终端中打开 worktree：

- Terminal.app
- iTerm2
- 自定义终端

## 删除 Worktree

### 安全删除

1. 右键点击 worktree
2. 选择**删除**
3. Skillshare App 检查：
   - 未提交的变更
   - 未 push 的提交
4. 确认删除

### 强制删除

如果您有未提交的变更，可以强制删除：

1. 勾选**强制**选项
2. 变更将会丢失

> 警告：强制删除无法恢复。

## 健康检查

Skillshare App 监控 worktree 健康状态：

### 执行的检查

- 分支在远程仍然存在
- 没有过期的锁定
- 目录可访问
- Git 仓库有效

### 修复问题

如果检测到问题：

1. 点击 worktree 上的**修复问题**
2. Skillshare App 尝试自动修复
3. 如果需要会显示手动步骤

<!-- TODO: Add screenshot of health check warnings -->

## 提示

1. **使用快速切换器**：<kbd>Cmd</kbd> + <kbd>K</kbd> 是您的好朋友
2. **创建模板**：节省重复模式的时间
3. **定期同步**：避免大型合并冲突
4. **清理旧 worktree**：删除已合并分支的 worktree
5. **使用会话**：让 Skillshare App 记住您的上下文

## 常见工作流

### 功能开发

1. 从模板创建 worktree：`feature/{name}`
2. 开发您的功能
3. 定期与 main 同步
4. Push 并创建 PR
5. 合并后删除 worktree

### 代码审查

1. 为 PR 分支创建 worktree
2. 审查和测试
3. 留下评论
4. 完成后删除 worktree

### 修复程序

1. 从 `main` 创建 worktree：`hotfix/{name}`
2. 应用修复
3. 直接 push 或创建 PR
4. 部署后删除 worktree
