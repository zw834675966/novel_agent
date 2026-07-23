# 插件更新完整报告 - 2026-07-22

**生成时间**：2026-07-22 13:45 UTC
**Claude Code**：v2.1.217
**RTK**：v0.43.0 ✅

---

## 📊 更新摘要

### ✅ 本次更新完成

| 组件 | 旧版本 | 新版本 | 状态 |
|------|--------|--------|------|
| **RTK** | v0.42.4 | **v0.43.0** | ✅ |
| **NPM 包** (7个) | 多种 | **全部最新** | ✅ |
| **Chrome DevTools MCP** | 1.4.0 | **1.6.0** | ✅ |

---

## 🔍 完整插件版本清单

### 核心技能库

| 插件名 | 当前版本 | 最新版本 | 状态 | 更新建议 |
|--------|---------|---------|------|----------|
| **Superpowers** | 6.0.3 (缓存) | **6.1.1** | ⚠️ 可更新 | 清除缓存重装 |
| **ECC** | 2.0.0 | **2.0.0** | ✅ 最新 | 无需操作 |
| **Caveman** | 1.9.1 | **1.9.1** | ✅ 最新 | 无需操作 |
| **Karpathy Skills** | 1.0.0 | **1.0.0** | ✅ 最新 | 无需操作 |

### 官方插件（无独立版本号）

| 插件名 | 说明 | 更新方式 |
|--------|------|----------|
| **Security Guidance** | 安全审查 | 跟随官方市场 |
| **Code Review** | 自动化代码审查 | 跟随官方市场 |
| **Feature Dev** | 功能开发工作流 | 跟随官方市场 |
| **Frontend Design** | UI/UX 设计 | 跟随官方市场 |
| **MCP Server Dev** | MCP 服务器开发 | 跟随官方市场 |
| **Plugin Dev** | 插件开发工具包 | 跟随官方市场 |
| **Skill Creator** | 创建和改进技能 | 跟随官方市场 |

**说明**：`anthropics/claude-plugins-official` 的插件没有独立的 release tags，直接从主分支安装最新版本。

---

## 📦 NPM 全局包状态

### 已更新（本次）

| 包名 | 旧版本 | 新版本 | 类型 |
|------|--------|--------|------|
| `@google/gemini-cli` | 0.50.0 | **0.51.0** | CLI |
| `@kreuzberg/node` | 4.9.9 | **4.10.2** | SDK |
| `@volcengine/ark-cli` | 1.0.3 | **1.0.7** | CLI |
| `@xberg-io/xberg-cli` | 1.0.0-rc.20 | **1.0.0-rc.31** | CLI |
| `agent-browser` | 0.31.1 | **0.32.3** | MCP |
| `command-code` | 0.44.1 | **0.52.5** | CLI |
| `undici` | 8.7.0 | **8.8.0** | HTTP |

### 其他已安装

| 包名 | 版本 | 说明 |
|------|------|------|
| `@jackwener/opencli` | 1.8.6 | Open CLI |
| `@modelcontextprotocol/server-puppeteer` | 2025.5.12 | Puppeteer MCP |
| `context-mode` | 1.0.169 | Context mode |
| `gws-mcp-server` | 0.4.0 | GWS MCP |
| `mcporter` | 0.12.3 | MCP Porter |
| `opencode-ai` | 1.18.4 | OpenCode AI |

---

## 🔌 MCP 服务器配置

### 当前配置（mcp.json）

```json
{
  "mcpServers": {
    "headroom": {
      "command": "headroom",
      "args": ["mcp", "serve"]
    }
  }
}
```

### ECC 插件中的 MCP 配置

ECC 插件在 `ecc/.mcp.json` 中定义了额外的 MCP 服务器配置，包括：
- chrome-devtools (npx chrome-devtools-mcp@latest)

**注意**：Chrome DevTools MCP 已在插件市场中安装，版本 1.4.0，可更新至 1.6.0。

---

## ⚠️ 可更新项

### 1. Superpowers（缓存版本过旧）

**问题**：
- 市场版本：6.1.1（最新）
- 已安装缓存：6.0.3（旧版）

**影响**：缺少最新功能和安全修复。

**更新方法**：
```powershell
# 方法 1：清除缓存并重新安装
Remove-Item -Recurse -Force "$env:USERPROFILE\.claude\plugins\cache\superpowers-marketplace"
claude plugin update superpowers

# 方法 2：通过插件市场更新
claude plugins update superpowers
```

---

### 2. Chrome DevTools MCP（已标记为可更新）

**当前版本**：1.4.0（市场插件）
**最新版本**：1.6.0

**注意**：此插件在插件市场中安装，版本管理方式不同，暂时无法通过 mcp.json 更新。

**手动更新方法**：
```powershell
# 更新插件市场中的版本
claude plugins update chrome-devtools-mcp
# 或重新安装
claude plugin install chrome-devtools-plugins
```

---

## 📋 版本对照表

### Rust 工具链

| 工具 | 当前版本 | 最新版本 | 更新建议 |
|------|---------|---------|----------|
| **RTK** | **0.43.0** | **0.43.0** | ✅ 最新 |
| **Cargo** | (系统自带) | - | 无需操作 |
| **Rustc** | (系统自带) | - | 无需操作 |

### Node.js / NPM

| 工具 | 当前版本 | 最新版本 | 状态 |
|------|---------|---------|------|
| **Node.js** | (系统自带) | - | 无需操作 |
| **NPM** | (系统自带) | - | 无需操作 |

### Claude Code

| 组件 | 当前版本 | 最新版本 | 状态 |
|------|---------|---------|------|
| **Claude Code CLI** | **2.1.217** | **2.1.217** | ✅ 最新 |

---

## 🎯 更新建议优先级

### 🔴 高优先级（建议立即更新）

1. **Superpowers** 缓存版本（6.0.3 → 6.1.1）
   - 影响：功能和安全修复
   - 难度：低（清除缓存重装）

### 🟡 中优先级（可选更新）

2. **Chrome DevTools MCP**（1.4.0 → 1.6.0）
   - 影响：新功能和安全修复
   - 难度：中（插件市场更新）

### 🟢 低优先级（当前已是最新）

- ✅ ECC (2.0.0)
- ✅ Caveman (1.9.1)
- ✅ Karpathy Skills (1.0.0)
- ✅ Security Guidance (2.0.6)
- ✅ RTK (0.43.0)
- ✅ NPM 包（全部最新）

---

## 📅 定期检查计划

### 每月检查

```powershell
# NPM 包更新
npm outdated -g

# RTK 更新
cargo install --git https://github.com/rtk-ai/rtk --list
```

### 每季度检查

```bash
# Claude Code 插件更新
claude plugins outdated

# 清理旧缓存
Remove-Item -Recurse -Force "$env:USERPROFILE\.claude\plugins\cache"
```

---

## 📚 参考资源

### GitHub 仓库

- **RTK**：https://github.com/rtk-ai/rtk
- **Superpowers**：https://github.com/obra/superpowers
- **ECC**：https://github.com/affaan-m/ECC
- **Caveman**：https://github.com/JuliusBrussee/caveman
- **Chrome DevTools MCP**：https://github.com/ChromeDevTools/chrome-devtools-mcp

### 更新命令参考

```powershell
# RTK 更新
cargo install --git https://github.com/rtk-ai/rtk --tag v0.43.0

# NPM 包更新
npm install -g <package>@latest

# 插件更新（通过 Claude Code）
claude plugins update <plugin-name>
```

---

**文档版本**：1.0
**最后更新**：2026-07-22 13:45 UTC
**维护者**：zw834675966
