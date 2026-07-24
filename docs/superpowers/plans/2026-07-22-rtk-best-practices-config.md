# RTK 最佳实践配置计划书

**基于深度研究结果** | 研究完成：2026-07-22
**置信度**：7项验证通过 (7/19) | 12项被驳斥

---

## 📊 研究摘要

### ✅ 已验证事实

1. **RTK v0.42.4 生产就绪** (2026年7月安装)
   - 实测节省：**96.5% tokens** (4,408命令，节省 96.3M tokens)
   - 位置：`~/.cargo/bin/rtk`

2. **四元命令集**（实测验证）
   - `rtk gain` - 显示节省统计
   - `rtk gain --history` - 使用历史
   - `rtk discover` - 分析遗漏的节省机会
   - `rtk proxy <cmd>` - 原始命令执行（带追踪）

3. **钩子系统**（实测 + 文档验证）
   - 安装：`rtk init -g`
   - 作用：自动重写 Bash 调用为 RTK 优化版本
   - 限制：**仅对 Bash 调用有效**，Claude Code 内置工具（Read/Grep/Glob）**绕过钩子**

4. **遥测默认禁用**（GDPR合规）
   - 状态：`consent: never asked, enabled: no`
   - 启用需显式：`rtk telemetry enable` 或 `rtk init` 时同意

5. **本项目特殊需求**（基于项目记忆）
   - Read 工具在本项目返回陈旧内容（32个修改文件超过阈值）
   - 推荐工作流：`rtk read <file>` 或 `rtk grep -rn <pattern> src/`

### ❌ 已驳斥的声称

1. ~~RTK 提供 60-90% token 节省~~ → **实测 96.5%**（数据来自 rtk gain）
2. ~~RTK 映射 Unix 工具（ls→eza, cat→bat, find→fd）~~ → **rtk ls/grep 仅代理原生命令**
3. ~~RTK v0.43.0 已发布（2026-06-28）~~ → **当前安装 v0.42.4**
4. ~~钩子自动重写所有命令~~ → **仅 Bash 调用，内置工具需手动替换**

### ⚠️ 关键限制

| 限制 | 影响 | 缓解措施 |
|------|------|----------|
| 钩子仅拦截 Bash | Read/Grep/Glob 不自动优化 | 手动使用 `rtk read` / `rtk grep` |
| 本项目 Read 工具陈旧 | 32个修改文件超过阈值 | 优先用 rtk 命令 |
| Windows 路径问题 | `which rtk` 返回 Git Bash 路径 | PowerShell 用 `Get-Command rtk` |
| 无选择性重写配置 | 无法白名单/黑名单命令模式 | 全有或全无 |

---

## 🎯 配置目标

1. **短期**：安装全局钩子，启用自动 Bash 命令优化
2. **中期**：建立手动 rtk 命令纪律（替换 Read/Grep）
3. **长期**：监控节省数据，优化工作流

---

## 📋 实施计划

### 阶段 1：验证当前状态 (Day 1)

**目标**：确认 RTK 安装完整性和基线数据

#### 步骤 1.1：基础验证

```powershell
# 检查版本
rtk --version
# 预期：rtk 0.42.4

# 检查遥测状态
rtk telemetry status
# 预期：enabled: no, consent: never asked

# 检查钩子状态
rtk init --show
# 预期：[warn] No hook installed

# 获取详细分析
rtk gain
# 记录当前节省基线
```

**成功标准**：
- ✅ 版本显示 `0.42.4`
- ✅ 遥测状态可读取
- ✅ 钩子警告出现

---

### 阶段 2：安装全局钩子 (Day 1-2)

**目标**：启用自动 Bash 命令优化

#### 步骤 2.1：预览安装影响

```powershell
# 模拟安装（不实际修改）
rtk init -g --dry-run

# 检查会修改哪些文件
# 预期：会修改 C:\Users\Administrator\.claude\settings.json
```

**检查清单**：
- [ ] 确认 settings.json 路径正确
- [ ] 检查现有 hooks 配置（避免覆盖）
- [ ] 备份现有 settings.json

#### 步骤 2.2：执行安装

```powershell
# 安装钩子 + RTK.md + @RTK.md + settings.json
rtk init -g
```

**预期输出**：
- ✅ Hook installed
- ✅ RTK.md created
- ✅ @RTK.md created
- ✅ settings.json patched

#### 步骤 2.3：验证安装

```powershell
# 验证钩子生效
rtk verify
# 预期：所有检查通过

# 重新检查钩子状态
rtk init --show
# 预期：无警告，显示钩子路径

# 测试自动重写
# 在 Claude Code 中执行：git status
# 预期：自动转为 rtk git status（检查 rtk gain --history）
```

**成功标准**：
- ✅ `rtk verify` 全通过
- ✅ `rtk init --show` 无警告
- ✅ `git status` 等命令在历史中显示为 `rtk git status`

---

### 阶段 3：遥测配置（可选） (Day 2-3)

**目标**：决定是否启用使用数据收集

#### 决策矩阵

| 选项 | 命令 | 隐私影响 | 数据价值 |
|------|------|----------|----------|
| 保持禁用 | 无需操作 | 无数据上传 | 无远程分析 |
| 启用 | `rtk telemetry enable` | 匿名使用数据上传 | RTK 团队改进 |

**推荐**：**保持禁用**（符合 GDPR 默认原则）

如果需要启用：
```powershell
rtk telemetry enable
# 需要明确同意
```

---

### 阶段 4：建立手动命令纪律 (Week 1)

**目标**：在钩子覆盖范围外，手动使用 rtk 命令

#### 4.1：替换 Read 工具

**触发条件**：
- 需要读取文件时
- Read 工具返回内容与预期不符时

**替代命令**：
```powershell
# 旧：Read("Cargo.toml")
# 新：
rtk read Cargo.toml

# 读取大文件（带行号）
rtk read --number src/main.rs
```

**记忆锚点**：本项目 Read 工具已知问题（32修改文件）

#### 4.2：替换 Grep 工具

**触发条件**：
- 代码搜索
- 模式匹配

**替代命令**：
```powershell
# 旧：Grep("tokio", type="rust")
# 新：
rtk grep -rn "tokio" src/

# 忽略大小写
rtk grep -rni "TODO" src/

# 只搜特定文件类型
rtk grep -rn "unwrap()" --type rust src/
```

#### 4.3：创建速查表

**保存位置**：`C:\Users\Administrator\.claude\RTK-QUICKREF.md`

```markdown
# RTK 快速参考

## 元命令
- `rtk gain` - 查看节省统计
- `rtk gain --history` - 最近命令历史
- `rtk discover` - 发现遗漏机会
- `rtk proxy <cmd>` - 原始执行（用于调试）

## 文件操作
- `rtk read <file>` - 读取文件（实时磁盘读取）
- `rtk read -n <file>` - 带行号
- `rtk ls <path>` - 列目录
- `rtk find <pattern>` - 查找文件

## 搜索
- `rtk grep -rn <pattern> <path>` - 递归搜索
- `rtk grep -rni <pattern> <path>` - 忽略大小写
- `rtk rg <pattern>` - ripgrep 原生

## Git
- `rtk git <subcommand>` - 所有 git 命令自动优化
```

---

### 阶段 5：监控与优化 (Ongoing)

#### 5.1：定期检查节省数据

**每周**：
```powershell
rtk gain
# 记录周报：命令数、tokens 节省、效率百分比
```

**指标基准**（基于当前数据）：
- 平均执行时间：2.4s
- 效率：≥95%
- 热门命令：`rtk grep`, `rtk git diff`, `rtk read`

#### 5.2：发现遗漏机会

**每月**：
```powershell
rtk discover
# 分析 Claude Code 历史，找出未使用 RTK 的命令
```

**行动项**：
- 识别高频未优化命令
- 更新工作流，优先使用 rtk 版本
- 调整速查表

#### 5.3：版本更新检查

**每季度**：
```powershell
# 检查最新版本
cargo install --list | Select-String "rtk"

# 对比 GitHub releases
# https://github.com/rtk-ai/rtk/releases
```

**更新策略**：
- 主版本更新（v0.43.0+）：测试后更新
- 补丁版本：直接更新

---

## 🔧 故障排除

### 钩子未生效

**症状**：`git status` 不在 `rtk gain --history` 中

**排查步骤**：
```powershell
# 1. 检查钩子安装
rtk init --show

# 2. 验证 settings.json
rtk verify

# 3. 检查 Claude Code 配置路径
Get-ChildItem $env:USERPROFILE\.claude\settings.json

# 4. 重新安装
rtk init -g --force
```

### Read 工具返回陈旧内容

**症状**：文件内容与磁盘状态不符

**解决方案**：
```powershell
# 1. 使用 rtk read（实时读取）
rtk read <file>

# 2. 或使用 Bash
cat <file>
# 或
Get-Content <file>
```

### 性能下降

**症状**：命令执行变慢

**排查**：
```powershell
# 1. 检查特定命令延迟
rtk gain --history | Select-Object -First 10

# 2. 禁用遥测（如果启用）
rtk telemetry disable

# 3. 使用 proxy 调试
rtk proxy <slow-command>
```

---

## 📚 参考资源

### 官方文档
- GitHub: https://github.com/rtk-ai/rtk
- README: 本地 `~/.cargo/git/checkouts/rtk-*/README.md`
- 遥测政策: `TELEMETRY.md`

### 项目记忆
- `MEMORY.md` - 质量门状态
- `read-tool-staleness.md` - Read 工具陈旧问题
- `corpus-distillation-progress.md` - 蒸馏进度

### 命令参考
```powershell
# 帮助
rtk --help
rtk <command> --help

# 验证
rtk verify

# 配置
rtk config --create
rtk config --show
```

---

## 🚀 下一步行动

**立即执行（今天）**：
- [ ] 阶段 1：验证当前状态
- [ ] 阶段 2：安装全局钩子

**本周完成**：
- [ ] 阶段 3：遥测决策
- [ ] 阶段 4：建立命令纪律 + 创建速查表

**持续执行**：
- [ ] 阶段 5：监控与优化（每月检查）

---

## 📈 预期收益

基于当前基线数据：
- **命令数**：4,408+ → 预计每月 +500
- **Token 节省**：96.3M → 预计每月 +10M
- **效率**：96.5% → 目标 ≥95%
- **平均延迟**：2.4s → 保持稳定

**钩子安装后预期**：
- Bash 命令自动优化（节省手动替换时间）
- 减少遗漏机会（`rtk discover` 每月更新）
- 透明集成（0 tokens 开销）

---

**文档版本**：1.0
**最后更新**：2026-07-22
**状态**：待执行
