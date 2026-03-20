# 工具链管理

检测和管理 Node.js 版本、包管理器，并解决版本冲突。

## 概览

Skillshare App 帮助您管理开发工具链：

- Node.js 版本检测
- 包管理器检测
- 版本管理器集成（Volta、nvm）
- 冲突解决

<!-- TODO: Add screenshot of toolchain panel -->

## 版本检测

### Node.js

Skillshare App 检测：
- 已安装的 Node.js 版本
- 项目需要的版本（来自 `package.json` → `engines`）
- 版本管理器配置

### 包管理器

自动检测：
- npm
- yarn
- pnpm
- bun

基于 lock 文件检测：

| Lock 文件 | 包管理器 |
|-----------|----------|
| `package-lock.json` | npm |
| `yarn.lock` | yarn |
| `pnpm-lock.yaml` | pnpm |
| `bun.lockb` | bun |

## 版本管理器支持

### Volta

Skillshare App 与 Volta 深度集成：

**功能：**
- 从 `package.json` 读取 Volta 配置
- 显示钉选的 Node/npm/yarn 版本
- 检测 Volta 安装
- 生成 Volta 兼容命令

**Volta 配置：**
```json
{
  "volta": {
    "node": "20.10.0",
    "npm": "10.2.3"
  }
}
```

### nvm

支持 nvm 配置：

**功能：**
- 读取 `.nvmrc` 文件
- 显示需要的 Node 版本
- 显示当前版本是否匹配

**.nvmrc：**
```
20.10.0
```

### Corepack

Skillshare App 检测 Corepack 状态：

**功能：**
- 检查 Corepack 是否启用
- 检测包管理器版本
- 显示 Corepack 警告

## 冲突检测

Skillshare App 自动检测常见冲突：

### Volta + Corepack 冲突

同时使用 Volta 和 Corepack 可能导致问题：

- 两者都试图管理包管理器
- 可能导致版本不匹配
- 性能下降

**解决方案：**
1. Skillshare App 显示警告
2. 选择其一：Volta 或 Corepack
3. 按照建议的步骤

<!-- TODO: Add screenshot of conflict warning dialog -->

### PNPM_HOME 冲突

当 `PNPM_HOME` 与 Volta 冲突时：

**症状：**
- pnpm 命令失败
- 版本不匹配错误

**解决方案：**
1. Skillshare App 识别冲突
2. 显示涉及的环境变量
3. 提供修复命令

### 版本不匹配

当您的 Node 版本与项目不匹配时：

**示例：**
- 项目需要：Node 20.x
- 已安装：Node 18.x

**解决方案：**
1. 项目卡片上显示警告
2. 点击查看详情
3. 按照升级指示

<!-- TODO: Add screenshot of version mismatch warning -->

## 诊断

### 环境诊断

查看您完整的工具链：

1. 前往**设置** → **工具链**
2. 点击**运行诊断**
3. 查看：
   - Node.js 版本和路径
   - npm/yarn/pnpm 版本
   - 环境变量
   - 版本管理器状态

<!-- TODO: Add screenshot of diagnostics panel -->

### 项目诊断

对于每个项目：

1. 点击项目上的工具链图标
2. 查看项目特定信息：
   - 需要的版本
   - 当前版本
   - 兼容性状态
   - 冲突

## 包管理器偏好

### 设置默认

选择您偏好的包管理器：

1. 前往**设置** → **工具链**
2. 选择**默认包管理器**
3. 选择：npm、yarn、pnpm 或 bun

### 每个项目覆盖

Skillshare App 尊重项目特定设置：
- Lock 文件决定包管理器
- Volta 配置优先
- 可手动覆盖

## 版本徽章

项目卡片显示版本状态：

| 徽章 | 意义 |
|------|------|
| 绿色 | 版本匹配 |
| 黄色 | 小版本不匹配 |
| 红色 | 大版本不匹配或冲突 |

<!-- TODO: Add screenshot of version badges on project cards -->

## 命令

Skillshare App 根据您的工具链生成正确的命令：

### 使用 Volta

```bash
volta run npm install
volta run npm run build
```

### 使用 Corepack

```bash
corepack enable
pnpm install
pnpm run build
```

### 不使用版本管理器

```bash
npm install
npm run build
```

## 提示

1. **使用 Volta 或 Corepack**：不要混用
2. **钉选版本**：使用 Volta 或 `.nvmrc` 保持一致性
3. **检查诊断**：当事情看起来不对时
4. **定期更新**：保持 Node.js 和包管理器最新
5. **查看冲突**：及时处理警告

## 疑难排解

### 使用错误的 Node 版本

1. 检查版本管理器（Volta、nvm）
2. 验证 PATH 顺序
3. 更改后重新启动终端

### 找不到包管理器

1. 检查是否全局安装
2. 验证 Corepack 状态
3. 检查 PATH 环境变量

### 包安装缓慢

1. 检查 Volta + Corepack 冲突
2. 验证网络连接
3. 尝试清除缓存
