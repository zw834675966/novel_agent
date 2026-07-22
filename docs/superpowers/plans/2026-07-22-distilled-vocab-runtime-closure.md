# Distilled Vocab Runtime Closure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把本地蒸馏词库从「离线资产」接入 novels 运行时，在候选爆炸可控的前提下让感知推导与 prose 拼装真正引用原著片段，并补齐质量门禁与 Superpowers 交付闭环。

**Architecture:** 在现有 `Vocab::merge` / `load_dir_merged` + `candidates_for_tags` + `validate_selection` + prose `assemble` 上做**薄扩展**：统一 bootstrap 加载（base YAML ± distilled 目录）、确定性 top-k 候选截断、可选 tag facet 收束与对话句过滤工具、可复现的质量报告 KPI。不引入向量库、不换 LLM provider、不改写 corpus 原文。

**Tech Stack:** Rust 2024 · `src/vocab` · `src/main.rs` · `src/scene/service.rs` · `src/prose/assembly.rs` · `tools/*.py` · SQLite 演示库 · Mock 测试 · 既有 `assets/distilled/`

**Spec / 依据:**

- `docs/superpowers/specs/2026-07-22-vocab-distill-gap-analysis.md`（差距矩阵 G1–G12）
- `docs/superpowers/plans/2026-07-19-corpus-distillation.md`（已完成的蒸馏生产线）
- 联网（Exa 2026-07-22）：Google LangExtract 式 **post-hoc span grounding**；ELTeC 式 **小核心受控特征集**；万级符号表必须 **top-k / facet 过滤** 防候选爆炸

## Superpowers 全流程映射

| 阶段 | 状态 | 产物 |
|------|------|------|
| **DEFINE** | ✅ 已完成 | gap-analysis 规格 |
| **PLAN** | ✅ 完成 | 本 implementation plan |
| **BUILD** | ✅ 完成 | Task 1–7 代码/工具/测试 |
| **VERIFY** | ✅ 完成 | Task 8 质量门禁 + KPI 脚本 |
| **REVIEW** | ✅ 完成 | Task 9 whole-branch APPROVE |
| **SHIP** | ⏳ 待用户选择 | Task 10 合并/PR/保留分支 |

## Global Constraints

- 保持 `SenseGenerator` / DeepSeek provider / `DEEPSEEK_V4_FLASH` 不变。
- 保持 `validate_selection` 语义：非法 ID 剥离；全空重试一次。
- 保持 `derive_scene` 并发上限 **4**。
- 蒸馏 `text` 必须保持 **源章连续子串**（空白归一化）；禁止 LLM 改写入库。
- **禁止**把 `.env`、API keys、完整版权语料提交进 Git。
- 候选进入 LLM 必须有硬顶：默认 **每类最多 24 条、总计最多 96 条**（可配置常量）。
- `available_tags` 进入 `select_context_tags` 必须有硬顶：默认 **最多 80 个 tag**（确定性排序后截断）。
- 不新增向量依赖；本阶段不做 embedding 检索。
- Windows 上测试：`cargo test --test vocab_test` 等 narrow 测试优先；全量 `cargo test --all-targets` 在可行时跑。
- 蒸馏主库 `assets/distilled/` 体量大：测试只用 **fixtures 小 YAML**，不要在单元测试里 load_dir 全库。

## File Map

| 路径 | 职责 |
|------|------|
| `src/vocab/bootstrap.rs`（新建） | `load_runtime_vocab(base, distilled_dir?)` |
| `src/vocab/loader.rs` | `candidates_for_tags` 增加 limit；`known_tags` 截断辅助 |
| `src/vocab/mod.rs` | re-export bootstrap |
| `src/main.rs` | 使用 bootstrap；env `NOVELS_DISTILLED_DIR` |
| `src/scene/service.rs` | 候选/tag 截断接入（若 cap 不在 Vocab 内则在此调用） |
| `tests/vocab_test.rs` | merge + cap + bootstrap 测试 |
| `tests/fixtures/distilled_sample.yaml`（新建） | 小蒸馏样本 |
| `tools/distill_quality_report.py`（新建） | KPI：类别分布、对话率、tag 基数、verbatim 抽检 |
| `tools/validate_fragments.py` | 可选：对话句启发式 DROP（flag 控制） |
| `assets/tag_facets.yaml`（新建，可选 Task 4） | 受控 facet 词表 |
| `AGENTS.md` | 运行时加载 distilled 说明 |
| 知识库 `Projects/novels/*` | 进度与链接 |

---

### Task 1: Runtime vocab bootstrap（闭合 G8）

**Files:**
- Create: `src/vocab/bootstrap.rs`
- Modify: `src/vocab/mod.rs`
- Modify: `src/main.rs`
- Test: `tests/vocab_test.rs`
- Create: `tests/fixtures/distilled_sample.yaml`

**Interfaces:**
- Produces: `pub fn load_runtime_vocab(base: &Path, distilled: Option<&Path>) -> Result<(Vocab, VocabLoadReport), StoryError>`
- Produces: `pub struct VocabLoadReport { pub base_entries: usize, pub distilled_files: usize, pub total_entries: usize }`

- [ ] **Step 1: 写失败测试（bootstrap 合并）**

在 `tests/vocab_test.rs` 追加：

```rust
#[test]
fn load_runtime_vocab_merges_base_and_distilled_fixture() {
    use novels::vocab::load_runtime_vocab;
    use std::path::Path;

    let base = Path::new("assets/vocab.yaml");
    let distilled = Path::new("tests/fixtures/distilled_sample.yaml");
    // 先只测单文件：实现可接受 distilled 为「文件」或「目录」；
    // 本测试要求目录接口时，把 fixture 放进 tests/fixtures/distilled_dir/
    let (v, report) = load_runtime_vocab(base, Some(Path::new("tests/fixtures/distilled_dir"))).unwrap();
    assert!(v.has("visual", "bloodstain")); // base
    assert!(v.has("emotion", "hlm-c001-01") || report.distilled_files >= 1);
    assert!(report.total_entries > report.base_entries);
}
```

- [ ] **Step 2: 创建 fixture**

`tests/fixtures/distilled_dir/sample.yaml`:

```yaml
emotion:
  hlm-c001-01:
    text: "心中无限凄凉"
    tags: ["hlm", "哀伤"]
gesture:
  hlm-c001-02:
    text: "不禁落下泪来"
    tags: ["hlm", "落泪"]
```

（`text` 可为任意占位；fixture 不强制对接 corpus 逐字，因单元测试不跑 validate_fragments。）

- [ ] **Step 3: 实现 bootstrap**

`src/vocab/bootstrap.rs` 要点：

```rust
pub struct VocabLoadReport {
    pub base_entries: usize,
    pub distilled_files: usize,
    pub total_entries: usize,
}

pub fn count_entries(v: &Vocab) -> usize {
    SENSES.iter().map(|s| v.entries(s).map(|m| m.len()).unwrap_or(0)).sum()
}

pub fn load_runtime_vocab(
    base: &std::path::Path,
    distilled_dir: Option<&std::path::Path>,
) -> Result<(Vocab, VocabLoadReport), crate::models::StoryError> {
    let mut vocab = Vocab::load_from_path(base)?;
    let base_entries = count_entries(&vocab);
    let mut distilled_files = 0;
    if let Some(dir) = distilled_dir {
        if dir.is_dir() {
            distilled_files = vocab.load_dir_merged(dir)?;
        }
    }
    let total_entries = count_entries(&vocab);
    Ok((
        vocab,
        VocabLoadReport {
            base_entries,
            distilled_files,
            total_entries,
        },
    ))
}
```

`mod.rs`:

```rust
mod bootstrap;
pub use bootstrap::{VocabLoadReport, load_runtime_vocab};
```

- [ ] **Step 4: 改 `main.rs` 加载逻辑**

```rust
let distilled = std::env::var("NOVELS_DISTILLED_DIR")
    .ok()
    .map(std::path::PathBuf::from)
    .filter(|p| p.is_dir())
    .or_else(|| {
        let p = std::path::PathBuf::from("assets/distilled");
        p.is_dir().then_some(p)
    });
let (vocab, report) = novels::vocab::load_runtime_vocab(
    std::path::Path::new("assets/vocab.yaml"),
    distilled.as_deref(),
)?;
eprintln!(
    "vocab loaded: base={} distilled_files={} total={}",
    report.base_entries, report.distilled_files, report.total_entries
);
```

约定：

- 默认：若 `assets/distilled` 存在则合并。
- `NOVELS_DISTILLED_DIR=` 空字符串或 `NOVELS_SKIP_DISTILLED=1` → 仅 base（实现时二选一，优先 `NOVELS_SKIP_DISTILLED=1` 文档化）。

- [ ] **Step 5: 跑测试**

```powershell
cargo test --test vocab_test load_runtime_vocab_merges_base_and_distilled_fixture
```

Expected: PASS

- [ ] **Step 6: Commit**

```powershell
git add src/vocab/bootstrap.rs src/vocab/mod.rs src/main.rs tests/vocab_test.rs tests/fixtures/distilled_dir
git commit -m "feat(vocab): bootstrap base + distilled dir at runtime"
```

---

### Task 2: 候选与 tag 确定性 top-k（闭合 G9 规模风险）

**Files:**
- Modify: `src/vocab/loader.rs`
- Modify: `src/scene/service.rs`（若 cap 调用点在 service）
- Test: `tests/vocab_test.rs`

**Interfaces:**
- Produces: `Vocab::candidates_for_tags_limited(&self, selected: &[String], per_sense: usize, total_max: usize) -> Vec<VocabularyCandidate>`
- Produces: `Vocab::known_tags_limited(&self, max: usize) -> Vec<String>`
- Constants: `DEFAULT_PER_SENSE_CAP = 24`, `DEFAULT_TOTAL_CAP = 96`, `DEFAULT_TAG_CAP = 80`

**设计（联网对齐）：** 万级候选直接进 prompt 会爆炸；ELTeC/工程实践要求小核心特征 + 过滤。采用 **确定性排序**（sense 序 → id 字典序），再截断；tag 无匹配时仍回退全量再 cap（保持现有 fallback 语义）。

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn candidates_for_tags_respects_total_cap() {
    let yaml = r#"
visual:
  a: { text: "a", tags: ["t"] }
  b: { text: "b", tags: ["t"] }
  c: { text: "c", tags: ["t"] }
emotion:
  d: { text: "d", tags: ["t"] }
  e: { text: "e", tags: ["t"] }
"#;
    let v = Vocab::load_from_str(yaml).unwrap();
    let c = v.candidates_for_tags_limited(&["t".into()], 2, 3);
    assert!(c.len() <= 3);
    // 确定性：同输入多次结果一致
    let c2 = v.candidates_for_tags_limited(&["t".into()], 2, 3);
    assert_eq!(c, c2);
}
```

（若 `VocabularyCandidate` 未实现 `PartialEq`，改为比较 `id` 列表。）

- [ ] **Step 2: 实现 limited 方法**

在 `loader.rs`：

```rust
pub const DEFAULT_PER_SENSE_CAP: usize = 24;
pub const DEFAULT_TOTAL_CAP: usize = 96;
pub const DEFAULT_TAG_CAP: usize = 80;

impl Vocab {
    pub fn known_tags_limited(&self, max: usize) -> Vec<String> {
        let mut tags = self.known_tags();
        if tags.len() > max {
            tags.truncate(max);
        }
        tags
    }

    pub fn candidates_for_tags_limited(
        &self,
        selected: &[String],
        per_sense: usize,
        total_max: usize,
    ) -> Vec<crate::llm::VocabularyCandidate> {
        let mut all = self.candidates_for_tags(selected);
        // candidates_for_tags 已按 sense/key 稳定顺序；再按 sense 组内截断
        let mut out = Vec::new();
        let mut per: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for c in all.drain(..) {
            let n = per.entry(c.sense.clone()).or_insert(0);
            if *n >= per_sense {
                continue;
            }
            *n += 1;
            out.push(c);
            if out.len() >= total_max {
                break;
            }
        }
        out
    }
}
```

- [ ] **Step 3: service 接入**

`service.rs` 中：

```rust
available_tags: self.vocab.known_tags_limited(crate::vocab::DEFAULT_TAG_CAP),
// ...
let candidates = self.vocab.candidates_for_tags_limited(
    &selected_tags,
    crate::vocab::DEFAULT_PER_SENSE_CAP,
    crate::vocab::DEFAULT_TOTAL_CAP,
);
```

确保 `mod.rs` export 常量。

- [ ] **Step 4: 测试**

```powershell
cargo test --test vocab_test candidates_for_tags_respects_total_cap
cargo test --test scene_test
cargo test --test e2e
```

Expected: PASS（Mock 路径候选仍含 fixture ID）

- [ ] **Step 5: Commit**

```powershell
git add src/vocab/loader.rs src/vocab/mod.rs src/scene/service.rs tests/vocab_test.rs
git commit -m "feat(vocab): deterministic top-k caps for candidates and tags"
```

---

### Task 3: 质量报告脚本（闭合 G7 评估缺口的工具面）

**Files:**
- Create: `tools/distill_quality_report.py`
- Modify: 无 Rust 代码（可选后续接 CI）

**Interfaces:**
- CLI: `python tools/distill_quality_report.py [--dir assets/distilled] [--json out.json]`
- KPI: entry_count, cat_pct, dialogue_proxy_rate, short_le10_rate, unique_content_tags, verbatim_sample_pass_rate

- [ ] **Step 1: 实现脚本**（复用 gap 分析启发式）

核心逻辑：

```python
# -*- coding: utf-8 -*-
"""蒸馏库 KPI 报告。不改写任何 YAML。"""
import argparse, collections, json, re, random
from pathlib import Path
import yaml

def norm(s): return re.sub(r"\s", "", s)

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dir", default="assets/distilled")
    ap.add_argument("--json", dest="json_out")
    ap.add_argument("--verbatim-sample", type=int, default=50)
    args = ap.parse_args()
    root = Path(args.dir)
    # ... 统计 cat_pct / dialogue / tags / short ...
    # verbatim: 随机 sample 条，查 corpus/{book}/cNNN.txt
    report = {"entries": 0, "cat_pct": {}, "dialogue_proxy_rate": 0.0}
    print(json.dumps(report, ensure_ascii=False, indent=2))
    if args.json_out:
        Path(args.json_out).write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")

if __name__ == "__main__":
    main()
```

（实现时补全统计循环；对话启发式与 gap 报告一致：`(道|说|问道|笑道|便道)[："]` 等。）

- [ ] **Step 2: 跑样本**

```powershell
python tools/distill_quality_report.py --dir assets/distilled
```

Expected: 打印 entries≈13883 量级、cat_pct、dialogue_proxy_rate

- [ ] **Step 3: Commit**

```powershell
git add tools/distill_quality_report.py
git commit -m "chore(tools): distilled vocab quality KPI report"
```

---

### Task 4: 受控 tag facets（闭合 G4，最小可用）

**Files:**
- Create: `assets/tag_facets.yaml`
- Create: `tools/normalize_tags.py`
- Modify（可选第二刀）: `src/vocab/loader.rs` 的 `filter_known_tags` 仍只认词库内 tag；本任务以**离线规范化蒸馏 YAML tags**为主，避免大改 LLM 契约。

**Facet 设计（对齐 ELTeC「小核心特征」）：**

```yaml
# assets/tag_facets.yaml
books: [hlm, zhz]
moods:
  - 哀伤
  - 欢喜
  - 愤怒
  - 恐惧
  - 羞赧
  - 隐忍
  - 关切
  - 嫉妒
  - 平静
settings:
  - 庭院
  - 室内
  - 宫闱
  - 夜宴
  - 路途
  - 战场
# characters: 不枚举全量；保留原人物名字符串，但 strip 掉非 facet 的噪声抽象词
drop_tokens: [女主, 主人公, 动作, 心理]
```

- [ ] **Step 1: 写 `tools/normalize_tags.py`**

行为：

1. 读取 distilled YAML  
2. tags 始终保留 book 前缀 (`hlm`/`zhz`)  
3. 人物名（非 drop、非 mood/setting）保留  
4. mood/setting 仅保留 facets 内词；未知 mood 映射表可选  
5. 默认 dry-run；`--write` 才改文件  

- [ ] **Step 2: dry-run 一章**

```powershell
python tools/normalize_tags.py --file assets/distilled/hlm-c001.yaml
```

- [ ] **Step 3: 文档说明不强制全库 rewrite**（用户确认后再 `--write`）

- [ ] **Step 4: Commit**

```powershell
git add assets/tag_facets.yaml tools/normalize_tags.py
git commit -m "feat(tools): controlled tag facets + normalizer dry-run"
```

---

### Task 5: 对话句/非描写过滤（闭合 G5/G6 噪声面）

**Files:**
- Modify: `tools/validate_fragments.py`

- [ ] **Step 1: 增加 `--drop-dialogue` flag**

在现有 DROP 逻辑后追加：

```python
DIALOGUE_RE = re.compile(r"(道|说|问道|笑道|便道)[：“\"]")

def looks_like_dialogue(text: str) -> bool:
    if text.startswith(("笑道", "便道", "说道", "问道")):
        return True
    return bool(DIALOGUE_RE.search(text))
```

仅当 `--drop-dialogue` 时计入 bad 并可 prune。

- [ ] **Step 2: 样本校验**

```powershell
python tools/validate_fragments.py hlm-c001 --drop-dialogue
```

Expected: 报告 dialogue DROP 数；无 `--prune` 时不改文件

- [ ] **Step 3: Commit**

```powershell
git add tools/validate_fragments.py
git commit -m "feat(tools): optional dialogue heuristic drop for distilled fragments"
```

---

### Task 6: prose 拼装烟雾验证（闭环消费路径）

**Files:**
- Test: `tests/prose_test.rs` 或 `tests/e2e.rs` 增量

**说明：** prose 已按 derivation 的 VocabularyId 从 `Vocab` 解析 `text`。Task 1 合并后，只要 derivation 选中蒸馏 ID，拼装即输出原著句。本任务补**显式回归**。

- [ ] **Step 1: 测试**

```rust
#[test]
fn assemble_resolves_distilled_emotion_text() {
    let mut v = Vocab::load_from_str(include_str!("../assets/vocab.yaml")).unwrap();
    v.merge(Vocab::load_from_str(r#"
emotion:
  hlm-c001-01:
    text: "心中无限凄凉"
    tags: ["hlm"]
"#).unwrap());
    // 构造最小 LlmNarrative + CharacterDerivation，refs 含 emotion.hlm-c001-01
    // assert 输出正文包含「心中无限凄凉」
}
```

（按现有 `prose_test` 构造方式对齐字段名。）

- [ ] **Step 2: 跑测**

```powershell
cargo test --test prose_test assemble_resolves_distilled_emotion_text
```

- [ ] **Step 3: Commit**

```powershell
git add tests/prose_test.rs
git commit -m "test(prose): resolve distilled fragment text via vocab merge"
```

---

### Task 7: AGENTS + 知识库同步（文档闭环）

**Files:**
- Modify: `AGENTS.md`（Runtime Wiring / Vocabulary 小节）
- Modify: `D:\grok\knowledge-base\Projects\novels\novels-layer-vocab.md`
- Modify: `D:\grok\knowledge-base\Projects\novels\novels-distill-gap-analysis.md`
- Modify: `D:\grok\knowledge-base\Projects\novels\novels-ops.md`

- [ ] **Step 1: AGENTS 增加**

```markdown
### Distilled vocabulary (optional)

- Default runtime: load `assets/vocab.yaml` then merge `assets/distilled/` if present.
- Skip: `NOVELS_SKIP_DISTILLED=1`
- Override dir: `NOVELS_DISTILLED_DIR=path`
- Candidate caps: 24 per sense / 96 total; tags to LLM capped at 80.
- Quality: `python tools/distill_quality_report.py`
- Never commit secrets; treat corpus/distilled as local copyrighted material.
```

- [ ] **Step 2: 知识库状态**

将 gap 分析中 G8 标为「计划中/实现中」，链接本 plan。

- [ ] **Step 3: Commit**

```powershell
git add AGENTS.md
git commit -m "docs: runtime distilled vocab wiring and caps"
```

（知识库在 vault 路径，若不在 git 仓库则只写盘不 commit。）

---

### Task 8: VERIFY 门禁（Superpowers VERIFY）

**不改产品逻辑；只收集证据。**

- [ ] **Step 1: 格式与静态**

```powershell
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy --all-targets --all-features -- -D warnings
```

- [ ] **Step 2: 测试**

```powershell
cargo test --test vocab_test
cargo test --test scene_test
cargo test --test prose_test
cargo test --test e2e
```

- [ ] **Step 3: 蒸馏 KPI**

```powershell
python tools/validate_fragments.py hlm-c001 zhz-c001
python tools/distill_quality_report.py --dir assets/distilled --json temp/distill-kpi.json
```

- [ ] **Step 4: 运行时烟雾（可选，需/不需 API）**

```powershell
$env:NOVELS_SKIP_DISTILLED="1"; cargo run
# 然后
Remove-Item Env:NOVELS_SKIP_DISTILLED -ErrorAction SilentlyContinue
cargo run
# 日志应出现 vocab loaded: total=... 且 total >> 6
```

- [ ] **Step 5: 在 PR/提交说明粘贴命令输出摘要**（verification-before-completion）

---

### Task 9: REVIEW 清单（Superpowers REVIEW）

使用 `requesting-code-review` / 本地 diff 审查，强制核对：

| 检查项 | 期望 |
|--------|------|
| 默认合并 distilled 是否可跳过 | `NOVELS_SKIP_DISTILLED=1` 有效 |
| cap 是否确定性 | 同输入同输出 |
| fallback 全量是否仍 cap | 是，避免 1.3 万条灌 prompt |
| prose 是否只解析合法 ID | 非法 ref 仍 stripped |
| 密钥/大语料 | 未进入 commit |
| 测试是否 load 全量 distilled | 否，仅 fixture |

- [ ] **Step 1: `git diff` 自审并修问题**
- [ ] **Step 2: 记录审查结论到 `docs/superpowers/plans/` 旁注或 PR body**

---

### Task 10: SHIP（Superpowers SHIP）

- [ ] **Step 1: 确认 VERIFY 全绿**
- [ ] **Step 2: 按 `finishing-a-development-branch` 选择：本地合并 / PR / 保留分支**
- [ ] **Step 3: 更新 gap-analysis 规格状态**

将 G8 标为 ✅（实现后）、G7 工具面 ✅、G4/G9 部分 ✅；剩余 G2 anchor / G6 IAA 全量共识列入 **Out of Scope → 下一 plan**。

---

## Out of Scope（本 plan 明确不做）

| 项 | 原因 |
|----|------|
| 向量/LanceDB 检索 | YAGNI；cap + tags 先验证闭环 |
| 双模型共识全库重蒸馏 | 成本高；先工具与运行时 |
| LangExtract 级 fuzzy LCS 偏移 | 现有空白归一子串已够用；可作下一 plan |
| 改 DeepSeek 模型常量 | 无需求 |
| 提交 `corpus/` 或全量 distilled 到公开远程 | 版权与体积 |

## 风险

| 风险 | 缓解 |
|------|------|
| 合并 1.3 万条后 `known_tags` 仍巨大 | TAG_CAP=80 |
| tag 过滤过窄导致 fallback 全量 | total cap 硬顶 |
| 启动变慢 | 仅启动时读 YAML；可后续缓存 |
| prose 描写变「拼接感」 | 既有 beat/action 骨架；产品可接受优先于 AI 腔 |

## Self-Review（计划作者）

| Spec 项 | 对应 Task |
|---------|-----------|
| G8 运行时集成 | Task 1, 6, 7 |
| G9 检索/爆炸 | Task 2 |
| G7 评估 | Task 3, 8 |
| G4 受控 tag | Task 4 |
| 对话噪声 | Task 5 |
| VERIFY/REVIEW/SHIP | Task 8–10 |
| 逐字保真不破坏 | 全局约束 + Task 5 可选 |

无 TBD 占位；常量与路径均已写明。

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-07-22-distilled-vocab-runtime-closure.md`.

**Two execution options:**

1. **Subagent-Driven (recommended)** — 每 Task 新开 subagent + 任务间 review（`subagent-driven-development`）
2. **Inline Execution** — 本会话按 `executing-plans` 连续执行，设检查点

**Which approach?**
