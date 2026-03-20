# Git 集成

无需离开 Skillshare App 即可进行可视化 Git 操作。暂存、提交、分支、差异对比 — 全部无需 CLI 体操。

## 概览

Git 面板提供完整的常见 Git 操作界面：

- 文件暂存与取消暂存
- 创建提交
- 分支管理
- 差异查看
- Stash 操作

<!-- TODO: Add screenshot of Git panel overview -->

## Git 状态视图

### 文件状态

状态视图按状态组织显示所有变更的文件：

| 区域 | 说明 |
|------|------|
| **已暂存** | 准备提交的文件 |
| **已修改** | 变更但未暂存的文件 |
| **未跟踪** | Git 中没有的新文件 |
| **冲突** | 有合并冲突的文件 |

<!-- TODO: Add screenshot of file status sections -->

### 文件操作

对于每个文件，您可以：

- **暂存**：添加到暂存区（点击 `+`）
- **取消暂存**：从暂存区移除（点击 `-`）
- **丢弃**：还原变更（点击 `↩`）
- **查看差异**：查看变更内容

## 暂存文件

### 暂存单个文件

点击任何已修改或未跟踪文件旁的 `+` 按钮。

### 全部暂存

点击**全部暂存**一次添加所有变更。

### 部分暂存

更精细的控制：

1. 点击文件查看其差异
2. 选择特定区块进行暂存
3. 仅暂存您需要的部分

<!-- TODO: Add gif of partial staging -->

## 提交

### 创建提交

1. 暂存您的变更
2. 输入提交信息
3. 点击**提交**

<!-- TODO: Add screenshot of commit form -->

### AI 生成信息

Skillshare App 可以使用 AI 生成提交信息：

1. 暂存您的变更
2. 点击信息输入旁的 **AI** 按钮
3. 查看生成的信息
4. 需要时编辑，然后提交

详见 [AI 集成](./ai-integration.md) 进行设置。

<!-- TODO: Add screenshot of AI commit message generation -->

### 提交指南

Skillshare App 显示字符数并在您的信息有以下情况时警告：
- 太短（少于 10 个字符）
- 第一行太长（超过 72 个字符）

## 差异查看器

### 统一视图

以单列格式查看变更：
- 红色行表示删除
- 绿色行表示添加
- 行号作为参考

<!-- TODO: Add screenshot of unified diff view -->

### 分割视图

并排比较新旧版本：

1. 点击差异工具栏中的**分割**
2. 左侧显示原始版本
3. 右侧显示您的变更

<!-- TODO: Add screenshot of split diff view -->

### 语法高亮

差异根据文件类型进行语法高亮，更容易阅读。

### 大型文件

Skillshare App 对大型差异使用虚拟化，即使有数千行也能保持性能流畅。

## 分支操作

### 查看分支

在分支面板中查看所有本地和远程分支。

<!-- TODO: Add screenshot of branch list -->

### 创建分支

1. 点击**新建分支**
2. 输入分支名称
3. 选择基础分支
4. 点击**创建**

### 切换分支

点击任何分支以切换到它。Skillshare App 会：
- 如果有未提交的变更会警告
- 提供 stash 或丢弃变更的选项

### 删除分支

1. 右键点击分支
2. 选择**删除**
3. 确认删除

> 注意：您无法删除当前检出的分支。

## Stash 操作

### 创建 Stash

1. 进行一些变更
2. 点击 Git 面板中的 **Stash**
3. 可选地添加信息
4. 您的变更已保存

### 应用 Stash

1. 查看 stash 列表
2. 点击 stash 预览
3. 点击**应用**还原变更
4. 选择保留或删除 stash

<!-- TODO: Add screenshot of stash list -->

### 删除 Stash

1. 右键点击 stash
2. 选择**删除**
3. 确认删除

## Pull 和 Push

### Pull 变更

点击 **Pull** 获取并合并远程变更。

- Skillshare App 显示您是否落后于远程
- 冲突会在状态视图中标示

### Push 变更

点击 **Push** 上传您的提交。

- 显示要 push 的提交数量
- 如果远程有新提交会警告

<!-- TODO: Add screenshot of push/pull buttons with status -->

## 提交历史

查看项目的提交历史：

1. 点击 Git 面板中的**历史**
2. 查看最近的提交，包含：
   - 提交信息
   - 作者
   - 日期
   - SHA

<!-- TODO: Add screenshot of commit history -->

## 提示

1. **经常提交**：小型、专注的提交更容易审查和还原
2. **使用 AI 信息**：让 AI 起草，然后精炼
3. **审查差异**：总是检查您正在提交什么
4. **切换前 stash**：换分支时不要丢失工作
5. **push 前 pull**：保持最新状态以避免冲突
