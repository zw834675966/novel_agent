# RTK 高级配置与团队最佳实践指南

**基于深度研究** | 版本：RTK v0.42.4 | 日期：2026-07-22
**验证状态**：✅ 遥测已启用 | ✅ 钩子已安装 | ✅ 速查表已创建

---

## 📊 研究摘要（高级配置专项）

### 🔬 已验证的关键发现

| 发现 | 置信度 | 来源 |
|------|--------|------|
| **钩子仅拦截 Bash 命令** | ✅ 高 | 官方 README line 123, hooks/README.md |
| **Read/Grep/Glob 绕过钩子** | ✅ 高 | 官方文档 + 实测验证 |
| **实测节省 96.5%** | ✅ 高 | rtk gain 实测数据 |
| **遥测默认禁用（GDPR）** | ✅ 高 | TELEMETRY.md + 官方文档 |
| **约 70 个子命令被重写** | ✅ 中 | 实测验证 |

### ❌ 被驳斥的常见误解

| 误解 | 实际情况 |
|------|----------|
| ~~60-90% 节省~~ | **96.5% 实测**（4,438 命令） |
| ~~自动重写所有命令~~ | **仅 Bash**，内置工具需手动 |
| ~~0 tokens 开销~~ | **2.4s 平均延迟** |
| ~~v0.43.0 已发布~~ | **当前安装 v0.42.4** |
| ~~工具映射（ls→eza）~~ | **仅代理原生命令** |

---

## 🎯 高级配置选项

### 1. 钩子配置深度解析

#### 1.1 当前配置

**settings.json**（已配置）：
```json
{
  "hooks": {
    "PreToolUse": [{
      "matcher": "Bash",
      "hooks": [{
        "type": "command",
        "command": "rtk hook claude"
      }]
    }]
  }
}
```

**配置解析**：
- `PreToolUse`：在工具使用前触发
- `matcher: "Bash"`：**仅匹配 Bash 命令**
- `command: "rtk hook claude"`：调用 RTK 的 Claude Code 钩子

#### 1.2 重写范围

**自动重写**（约 70 个子命令）：
- ✅ `git status` → `rtk git status`
- ✅ `cargo build` → `rtk cargo build`
- ✅ `cargo test` → `rtk cargo test`
- ✅ `ls -la` → `rtk ls -la`
- ✅ `find *.rs` → `rtk find *.rs`

**不重写**（需手动）：
- ❌ Read 工具
- ❌ Grep 工具
- ❌ Glob 工具
- ❌ 非 Bash 工具（如 Edit, Write）

#### 1.3 配置变体

**仅钩子（无 RTK.md）**：
```powershell
rtk init -g --hook-only
```

**自动补丁（无交互）**：
```powershell
rtk init -g --auto-patch
```

**跳过 settings.json**：
```powershell
rtk init -g --no-patch
# 手动添加 hook 配置
```

---

### 2. 遥测配置（已启用）

#### 2.1 当前状态

```
consent:       yes
consent date:  2026-07-22T05:21:22.675323300+00:00
enabled:       yes
device hash:   (no salt file)
```

**隐私保护**：
- ✅ 匿名数据（无设备标识）
- ✅ GDPR 合规（显式同意）
- ✅ 可随时撤销

#### 2.2 管理命令

```powershell
# 查看状态
rtk telemetry status

# 禁用（随时可执行）
rtk telemetry disable

# 完全删除数据
rtk telemetry forget
```

---

### 3. 选择性命令排除（当前不支持）

**⚠️ 重要限制**：
- ❌ **无白名单配置**：无法指定只优化特定命令
- ❌ **无黑名单配置**：无法排除特定命令
- ✅ **全有或全无**：钩子应用于所有 Bash 命令

**实测验证**：
```
Config shows exclude_commands = []
but only ~70 specific subcommands get rewritten
```

**工作变通**：
如果某些命令不应被重写，继续使用原生 Bash：
```powershell
# 绕过 RTK
rtk proxy git status  # 直接执行，不优化
```

---

## 👥 团队部署最佳实践

### 4.1 统一安装流程

**推荐步骤**（新成员）：

```powershell
# 1. 安装 RTK
cargo install rtk

# 2. 验证安装
rtk --version
rtk gain

# 3. 安装全局钩子
rtk init -g --auto-patch

# 4. 验证钩子
rtk verify

# 5. 测试自动重写
git status
# 检查：rtk gain --history 应显示 rtk git status
```

**文档化要求**：
- ✅ 记录在 `AGENTS.md` 或团队 wiki
- ✅ 包含在新成员 Onboarding 清单
- ✅ 定期检查 `rtk verify` 输出

### 4.2 团队规范

**Read/Grep 使用纪律**：

```markdown
## RTK 使用规范（团队）

### ✅ 允许
- 使用 Read 工具（简单文件读取）
- 使用 Bash 命令（钩子自动优化）

### ⚠️ 必须手动替换
- **遇到 Read 问题** → 使用 `rtk read <file>`
- **代码搜索** → 使用 `rtk grep -rn "<pattern>" src/`
- **文件查找** → 使用 `rtk find <pattern>`
```

**触发条件**（必须切换到 rtk）：
1. Read 返回陈旧内容
2. 需要实时磁盘读取
3. 项目有 30+ 修改文件时
4. 代码审查场景

### 4.3 CI/CD 集成

**GitHub Actions 示例**：

```yaml
name: RTK Verification

on: [push, pull_request]

jobs:
  verify-rtk:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install RTK
        run: cargo install rtk

      - name: Verify RTK Installation
        run: |
          rtk --version
          rtk verify
          rtk gain

      - name: Check Hook Configuration
        run: |
          if [ -f ~/.claude/settings.json ]; then
            grep -q "rtk hook claude" ~/.claude/settings.json
          fi
```

**预提交钩子**：

```bash
#!/bin/bash
# .git/hooks/pre-commit

# 验证 RTK 配置
rtk verify || exit 1

# 检查是否有未优化的命令
rtk discover
```

---

## ⚡ 性能优化策略

### 5.1  Token 节省分析

**当前基准**（2026-07-22）：
| 命令类别 | 使用次数 | 节省 tokens | 平均% |
|---------|---------|------------|-------|
| `rtk grep` | 176 | 95.0M | 34.7% |
| `rtk git diff` | 10 | 328.5K | 56.7% |
| `rtk find` | 58 | 293.4K | 18.9% |
| `rtk rg` | 89 | 115.8K | 14.6% |
| `rtk read` | 132 | 107.6K | 6.3% |

**优化建议**：
1. **优先使用 `rtk grep`**：节省最多（95.0M）
2. **Git 操作自动优化**：钩子已覆盖
3. **Read 工具手动替换**：按需使用 `rtk read`

### 5.2 延迟优化

**实测数据**：
- 平均延迟：2.4s
- 最快：0ms（rtk read）
- 最慢：16.9s（cargo test）

**优化策略**：
- ✅ 使用 `rtk proxy` 调试慢命令
- ✅ 监控 `rtk gain --history` 的 Time 列
- ✅ 对频繁使用的命令考虑替代方案

---

## 🔧 进阶配置场景

### 6.1 多项目管理

**场景**：同时维护多个项目，每个项目需要不同配置

**解决方案**：

```powershell
# 项目级配置（每个项目独立）
cd project-a
rtk init  # 本地 CLAUDE.md

cd project-b
rtk init  # 本地 CLAUDE.md

# 全局钩子共享（settings.json）
# 自动适配所有项目
```

### 6.2 选择性启用/禁用

**临时禁用钩子**：

```powershell
# 备份 settings.json
Copy-Item $env:USERPROFILE\.claude\settings.json "$env:USERPROFILE\.claude\settings.json.bak"

# 移除钩子配置（编辑 settings.json）
# ...

# 恢复钩子
Move-Item "$env:USERPROFILE\.claude\settings.json.bak" $env:USERPROFILE\.claude\settings.json
```

**仅使用元命令（无钩子）**：
```powershell
# 不安装钩子，仅手动使用 rtk 命令
rtk init -g --hook-only --no-patch  # 错误组合
# 正确做法：
rtk init -g --no-patch  # 仅创建 RTK.md
```

### 6.3 Windows 环境特殊配置

**已知问题**：

1. **路径格式**：
   ```powershell
   # Git Bash 路径
   /c/Users/Administrator/.cargo/bin/rtk

   # PowerShell 路径
   C:\Users\Administrator\.cargo\bin\rtk.exe
   ```

2. **Protoc 要求**（Lance 构建）：
   ```powershell
   # 安装 protoc
   choco install protoc

   # 验证路径
   Test-Path "C:\Tools\protoc\bin\protoc.exe"
   ```

3. **Shell 兼容性**：
   - ✅ Git Bash：完全支持
   - ✅ PowerShell 7+：完全支持
   - ⚠️ PowerShell 5.1：部分兼容

---

## 📊 监控与维护

### 7.1 定期检查清单

**每日**：
```powershell
# 钩子是否生效
git status  # 应自动优化
```

**每周**：
```powershell
# 查看节省统计
rtk gain

# 记录指标：
# - Total commands（总命令数）
# - Tokens saved（节省 tokens）
# - Efficiency（效率百分比）
```

**每月**：
```powershell
# 发现遗漏机会
rtk discover

# 验证安装
rtk verify

# 检查更新
cargo install --list | Select-String "rtk"
```

### 7.2 警报阈值

**需要调查的情况**：
- ⚠️ 效率 < 95%
- ⚠️ 命令数连续 3 天无增长
- ⚠️ `rtk verify` 出现失败
- ⚠️ `rtk gain --history` 无新条目

---

## 🐛 故障排除

### 8.1 钩子未生效

**症状**：`git status` 不在 `rtk gain --history` 中

**诊断流程**：

```powershell
# 1. 检查钩子状态
rtk init --show

# 2. 验证 settings.json
Get-Content $env:USERPROFILE\.claude\settings.json

# 3. 完整验证
rtk verify

# 4. 检查 Claude Code 重启
# 关闭所有 Claude Code 窗口，重新启动
```

**修复步骤**：
```powershell
# 重新安装
rtk init -g --force

# 或手动修复
# 编辑 settings.json，确保 hook 配置正确
```

### 8.2 Read 工具陈旧问题

**症状**：文件内容与磁盘状态不符

**解决方案**：
```powershell
# 方案 1：rtk read（实时读取）
rtk read <file>
rtk read -n <file>  # 带行号

# 方案 2：Bash 原生
cat <file>
Get-Content <file>

# 方案 3：grep 验证
rtk grep -n "<pattern>" <file>
```

### 8.3 遥测问题

**状态不一致**：
```powershell
# 强制刷新状态
rtk telemetry status

# 如果状态异常，重新启用
rtk telemetry disable
rtk telemetry enable
```

---

## 📚 参考资源

### 官方文档
- **GitHub**：https://github.com/rtk-ai/rtk
- **README**：本地 `~/.cargo/git/checkouts/rtk-*/README.md`
- **钩子文档**：`hooks/README.md` (line 123, 316)
- **遥测政策**：`TELEMETRY.md`

### 项目文档
- **基础计划书**：`docs/superpowers/plans/2026-07-22-rtk-best-practices-config.md`
- **速查表**：`C:\Users\Administrator\.claude\RTK-QUICKREF.md`
- **AI 维护手册**：`docs/superpowers/plans/2026-07-20-ai-maintenance-playbook.md`

### 命令参考
```powershell
# 帮助
rtk --help
rtk <command> --help

# 验证
rtk verify
rtk init --show

# 配置
rtk config --create
rtk config --show

# 遥测
rtk telemetry status
rtk telemetry enable/disable/forget
```

---

## 🚀 实施路线图（已完成 ✅）

| 阶段 | 状态 | 完成时间 | 关键成果 |
|------|------|----------|----------|
| **阶段 1**：验证状态 | ✅ | 2026-07-22 | 基线数据建立 |
| **阶段 2**：安装钩子 | ✅ | 2026-07-22 | 154/154 tests passed |
| **阶段 3**：遥测配置 | ✅ | 2026-07-22 | consent: yes, enabled: yes |
| **阶段 4**：命令纪律 | ✅ | 2026-07-22 | RTK-QUICKREF.md 创建 |
| **阶段 5**：监控优化 | 🔄 | Ongoing | 待执行 |

---

## 🎯 关键要点

### 核心原则（必须遵守）

1. **Bash 命令自动优化**：钩子已安装，无需手动替换
2. **Read/Grep 必须手动**：`rtk read`, `rtk grep`（钩子不拦截）
3. **遇到 Read 问题立即切换**：rtk read 或原生 Bash
4. **定期监控**：每周 `rtk gain`，每月 `rtk discover`

### 团队协作规范

- ✅ **新成员 Onboarding**：必须包含 RTK 安装步骤
- ✅ **代码审查**：检查是否使用了 `rtk read/grep`
- ✅ **文档维护**：定期更新 RTK-QUICKREF.md
- ✅ **性能监控**：跟踪团队整体节省数据

---

**文档版本**：2.0（高级配置版）
**最后更新**：2026-07-22
**维护者**：zw834675966
**状态**：✅ 已启用并运行
