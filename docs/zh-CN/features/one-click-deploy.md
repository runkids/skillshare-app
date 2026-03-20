# 一键部署

一键将您的项目部署到 Netlify、Cloudflare Pages 或 GitHub Pages。

## 概览

Skillshare App 集成热门托管平台，支持：

- 一键部署
- 即时预览链接
- 环境变量管理
- 多账户支持

<!-- TODO: Add screenshot of deploy panel -->

## 支持的平台

| 平台 | 验证方式 | 功能 |
|------|----------|------|
| **Netlify** | OAuth | 完整集成 |
| **Cloudflare Pages** | API Token | 完整支持 |
| **GitHub Pages** | GitHub Actions | 工作流生成 |

## 连接账户

### Netlify

1. 前往**设置** → **部署账户**
2. 点击**添加账户** → **Netlify**
3. 点击**连接 Netlify**
4. 在浏览器中授权 Skillshare App
5. 账户已连接

<!-- TODO: Add screenshot of Netlify OAuth flow -->

### Cloudflare Pages

1. 前往**设置** → **部署账户**
2. 点击**添加账户** → **Cloudflare**
3. 输入您的 Cloudflare API Token
4. 点击**验证并保存**

创建 API token：
1. 前往 [Cloudflare Dashboard](https://dash.cloudflare.com/profile/api-tokens)
2. 点击**创建 Token**
3. 使用**编辑 Cloudflare Pages** 模板
4. 复制 token

<!-- TODO: Add screenshot of Cloudflare token dialog -->

### 多账户

您可以连接多个账户：
- 多个 Netlify 账户
- 多个 Cloudflare 账户
- 混合不同平台

在设置中为每个平台设置默认账户。

## 构建配置

### 自动检测

Skillshare App 自动检测您的框架并建议：

- 构建命令（例如 `npm run build`）
- 输出目录（例如 `dist`、`.next`、`build`）
- Node.js 版本

<!-- TODO: Add screenshot of auto-detected build config -->

### 支持的框架

| 框架 | 构建命令 | 输出 |
|------|----------|------|
| Vite | `vite build` | `dist` |
| Next.js | `next build` | `.next` |
| Nuxt | `nuxt build` | `.output` |
| Create React App | `react-scripts build` | `build` |
| Remix | `remix build` | `build` |
| Astro | `astro build` | `dist` |

### 自定义配置

覆盖检测的设置：

1. 打开部署面板
2. 点击**编辑配置**
3. 修改：
   - 构建命令
   - 输出目录
   - 安装命令
   - Node 版本

## 环境变量

### 添加变量

1. 打开部署面板中的**环境变量**
2. 点击**添加变量**
3. 输入键和值
4. 选择可见性：
   - **Production**：仅在正式环境
   - **Preview**：仅在预览部署
   - **All**：两种环境

<!-- TODO: Add screenshot of environment variables panel -->

### 机密变量

对于敏感值：

1. 添加时切换**机密**
2. 值会被加密
3. 保存后不再显示在日志或 UI 中

### 从 `.env` 导入

1. 点击**从 .env 导入**
2. 选择您的 `.env` 文件
3. 查看导入的变量
4. 保存到部署配置

## 部署

### 手动部署

1. 选择项目
2. 打开部署面板
3. 选择部署账户
4. 点击**部署**

<!-- TODO: Add gif of deploy process -->

### 部署进度

部署期间，查看：
- 当前状态
- 构建日志
- 任何错误或警告

### 预览链接

成功部署后：
- **正式 URL**：您的上线网站
- **预览 URL**：此次部署的唯一 URL

<!-- TODO: Add screenshot of deploy complete with URLs -->

## 部署历史

查看过去的部署：

1. 打开部署面板
2. 点击**历史**
3. 查看所有部署，包含：
   - 时间戳
   - 状态（成功/失败）
   - 持续时间
   - 提交信息

### 回滚

回滚到之前的部署：

1. 在历史中找到该部署
2. 点击**回滚**
3. 确认操作

## GitHub Pages

GitHub Pages 运作方式不同 — Skillshare App 生成 GitHub Actions 工作流。

### 设置

1. 选择 **GitHub Pages** 作为部署目标
2. 点击**生成工作流**
3. 查看生成的 `.github/workflows/deploy.yml`
4. 提交并 push

### 运作方式

工作流：
1. 在 push 到 main/master 时触发
2. 安装依赖
3. 运行构建命令
4. 部署到 `gh-pages` 分支

<!-- TODO: Add screenshot of generated workflow file -->

## 部署备份

### 导出配置

备份您的部署设置：

1. 前往**设置** → **备份**
2. 点击**导出部署配置**
3. 保存 JSON 文件

### 导入配置

从备份还原：

1. 前往**设置** → **备份**
2. 点击**导入部署配置**
3. 选择您的备份文件
4. 查看并确认

## 提示

1. **使用预览部署**：正式上线前测试变更
2. **先设置环境变量**：避免部署失败
3. **检查构建日志**：快速了解失败原因
4. **使用多账户**：分开个人和工作项目
5. **备份配置**：设置新机器时节省时间

## 疑难排解

### 构建失败

- 检查构建日志的错误
- 验证您的构建命令在本地运行
- 确保所有环境变量已设置

### 部署卡住

- 检查平台状态（Netlify/Cloudflare）
- 取消并重试部署
- 检查是否有大型文件上传

### 缺少环境变量

- 验证变量名称完全匹配
- 检查变量是否设置在正确的环境
- 确保机密正确配置
