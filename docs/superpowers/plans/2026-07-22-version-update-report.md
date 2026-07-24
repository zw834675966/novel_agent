# 版本更新报告 - 2026-07-22

**更新时间**：2026-07-22 13:35 UTC
**执行人**：zw834675966

---

## 📊 更新摘要

### ✅ 已更新

| 组件 | 旧版本 | 新版本 | 更新方式 |
|------|--------|--------|----------|
| **RTK (Rust Token Killer)** | v0.42.4 | **v0.43.0** ✅ | cargo install --git |
| **npm 全局包 (7个)** | 多种 | **全部最新** ✅ | npm install -g |

---

## 🔧 RTK 更新详情

### 版本升级

**v0.42.4 → v0.43.0** (2026-06-28 发布)

**安装方式**：
```powershell
# 卸载错误包（Rust Type Kit v0.1.0）
cargo uninstall rtk

# 从正确仓库安装 v0.43.0
cargo install --git https://github.com/rtk-ai/rtk --tag v0.43.0
```

**编译时间**：4 分 26 秒
**安装路径**：`C:\Users\Administrator\.cargo\bin\rtk.exe`

**验证结果**：
```powershell
rtk --version      # ✅ rtk 0.43.0
rtk verify         # ✅ 154/154 tests passed
rtk init --show    # ✅ 所有检查项 [ok]
```

**注意**：
- ⚠️ crates.io 上的 `rtk` 包（v0.1.0）是 **Rust Type Kit**，不是 RTK (Rust Token Killer)
- ✅ 必须从 GitHub 安装：`https://github.com/rtk-ai/rtk`

---

## 📦 NPM 包更新详情

### 更新的包列表

| 包名 | 旧版本 | 新版本 | 类型 |
|------|--------|--------|------|
| `@google/gemini-cli` | 0.50.0 | **0.51.0** | CLI 工具 |
| `@kreuzberg/node` | 4.9.9 | **4.10.2** | SDK |
| `@volcengine/ark-cli` | 1.0.3 | **1.0.7** | CLI 工具 |
| `@xberg-io/xberg-cli` | 1.0.0-rc.20 | **1.0.0-rc.31** | CLI 工具 |
| `agent-browser` | 0.31.1 | **0.32.3** | MCP Server |
| `command-code` | 0.44.1 | **0.52.5** | CLI 工具 |
| `undici` | 8.7.0 | **8.8.0** | HTTP 客户端 |

**更新命令**：
```powershell
npm install -g @google/gemini-cli@latest @kreuzberg/node@latest @volcengine/ark-cli@latest @xberg-io/xberg-cli@latest agent-browser@latest command-code@latest undici@latest
```

---

## 🔌 Claude Code 插件版本快照

### 核心插件（当前版本）

| 插件名 | 版本 | 最后更新 | 状态 |
|--------|------|----------|------|
| **Claude Code** | 2.1.217 | 2026-07-22 | ✅ 最新 |
| **Superpowers** | 6.1.1 | 2026-07-02 | ✅ 最新（缓存 v6.0.3） |
| **ECC** | 2.0.0 | 2026-06-10 | ✅ 最新 |
| **Caveman** | 1.9.1 | 2024-07-03 | ✅ 最新 |
| **Karpathy Skills** | 1.0.0 | 2026-06-30 | ✅ 最新 |
| **Chrome DevTools MCP** | 1.4.0 | - | ⚠️ 可更新至 1.6.0 |

### MCP 服务器

| MCP 服务器 | 配置版本 | 可用版本 | 状态 |
|-----------|---------|---------|------|
| **headroom** | local | - | ✅ 运行中 |
| **chrome-devtools** | 1.4.0 | **1.6.0** | ⚠️ 可更新 |
| **puppeteer** | 2025.5.12 | - | ✅ 运行中 |

---

## ⚠️ 已知问题

### 1. Chrome DevTools MCP 可更新

**当前版本**：1.4.0（市场插件）
**最新版本**：1.6.0（2026-07-14 发布）

**配置位置**：`mcp.json`
```json
{
  "mcpServers": {
    "chrome-devtools": {
      "command": "npx",
      "args": ["chrome-devtools-mcp@1.4.0"]  // ⚠️ 可改为 @1.6.0
    }
  }
}
```

**更新方法**：
```powershell
# 编辑 mcp.json
notepad $env:USERPROFILE\.claude\mcp.json

# 或手动更新
$config = Get-Content $env:USERPROFILE\.claude\mcp.json | ConvertFrom-Json
$config.mcpServers.chrome-devtools.args[0] = "chrome-devtools-mcp@1.6.0"
$config | ConvertTo-Json | Set-Content $env:USERPROFILE\.claude\mcp.json
```

**注意**：Chrome DevTools MCP 通过 `npx` 运行，每次启动时会自动下载指定版本。

---

### 2. RTK 包冲突

**问题**：crates.io 上的 `rtk` 包（v0.1.0）是 **Rust Type Kit**，与 RTK (Rust Token Killer) 冲突

**解决方案**：
- ✅ 已卸载错误包
- ✅ 从 Git 仓库安装正确版本
- ⚠️ 未来更新必须使用：`cargo install --git https://github.com/rtk-ai/rtk --tag v0.43.0`

---

## 📋 更新检查清单

### RTK 更新 ✅

- [x] 卸载错误包（rtk v0.1.0）
- [x] 从 GitHub 安装 v0.43.0
- [x] 验证版本（0.43.0）
- [x] 验证钩子（154/154 tests passed）
- [x] 验证配置（settings.json）
- [ ] 更新文档中的版本号（待完成）

### NPM 包更新 ✅

- [x] @google/gemini-cli (0.50.0 → 0.51.0)
- [x] @kreuzberg/node (4.9.9 → 4.10.2)
- [x] @volcengine/ark-cli (1.0.3 → 1.0.7)
- [x] @xberg-io/xberg-cli (rc.20 → rc.31)
- [x] agent-browser (0.31.1 → 0.32.3)
- [x] command-code (0.44.1 → 0.52.5)
- [x] undici (8.7.0 → 8.8.0)

### 插件状态检查 ✅

- [x] Superpowers (6.1.1)
- [x] ECC (2.0.0)
- [x] Caveman (1.9.1)
- [x] Karpathy Skills (1.0.0)
- [ ] Chrome DevTools MCP (1.4.0 → 1.6.0)（可选）

---

## 🚀 下一步行动

### 立即执行

1. **更新文档版本号**
   - [ ] 更新 `RTK.md` 中的版本说明
   - [ ] 更新 `docs/superpowers/plans/*-rtk-*.md` 中的版本引用
   - [ ] 更新 `MEMORY.md` 中的版本信息

2. **可选更新**
   - [ ] 更新 Chrome DevTools MCP 至 1.6.0
   - [ ] 验证更新后功能正常

3. **定期检查**
   - [ ] 每月检查 RTK 更新（GitHub releases）
   - [ ] 每月检查 npm 包更新（`npm outdated -g`）
   - [ ] 每季度检查插件更新（市场）

---

## 📈 版本历史

| 日期 | 组件 | 变更 |
|------|------|------|
| 2026-07-22 | RTK | v0.42.4 → v0.43.0 |
| 2026-07-22 | npm (7 pkgs) | 全部更新至最新 |
| 2026-07-08 | Caveman | 安装（v1.9.1） |
| 2026-07-02 | Superpowers | 市场更新至 v6.1.1 |
| 2026-06-30 | Karpathy, ECC | 安装（v1.0.0, v2.0.0） |
| 2026-07-22 | Claude Code | v2.1.217 |

---

**文档版本**：1.0
**最后更新**：2026-07-22 13:35 UTC
**维护者**：zw834675966
