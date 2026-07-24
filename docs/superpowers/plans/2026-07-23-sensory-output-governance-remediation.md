# Sensory Output Governance & Remediation Plan (感知引擎输出质量整改方案)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 全面治理小说角色感知引擎中“感知输出降级为纯动作(gesture)+情绪(emotion)描写、弱势感官（嗅/味/触/听）被挤压、Prompt包含 Debug `{:?}` 语法噪声”等 6 大结构性问题。

**Architecture:**
1. **数据与语料**: 补全 `assets/vocab.yaml` 标准五感条目；蒸馏工具引入感官桶配额（Sensory Bucket Sampling）。
2. **检索与候选**: 在 `src/vocab/loader.rs` 引入弱势感官强保底配额（Sense Quota）+ 主导感官上限 (gesture/emotion ≤30%) + 场景感知焦点动态提权 (Sense Focus Weighting)。
3. **Prompt 渲染**: 为 `MemoryContentSlot` 提供 `display_narrative()`，完全清除 Prompt 中的 Rust Debug `{:?}` 语法（如 `Observation("...")` 解包为 `【亲历/确定】目击：...`）。
4. **LLM 推理 CoT**: 在 `LlmCharacterDerivation` Schema 增加 `sensory_analysis` 字段，引导 LLM 在选择 ID 前先进行中文感知思考。
5. **装配质量门禁**: 在 `ProseQualityReport` 新增 `sensory_diversity_score` 与 `missing_senses` 检查，拦截/预警 `DegradedSensoryDensity` 现象并优先融入基础五感引用。

**Tech Stack:** Rust 2024 · `src/vocab/` · `src/llm/` · `src/models/` · `src/scene/` · Python `tools/`

---

## 6 大根因与对策映射 (Root Causes & Remediation Matrix)

| 编号 | 根因描述 | 影响环节 | 修复方案与落地点 |
| :--- | :--- | :--- | :--- |
| **根因一** | 蒸馏语料 Gesture/Emotion 占 61%，嗅觉/味觉不足 2% | 语料分布 / 基座词库 | 补全 `vocab.yaml`；蒸馏工具对 gesture 降采样，对嗅味触强保护 |
| **根因二** | BM25 排序因候选池失衡加剧偏斜 | 候选检索 `Vocab` | `candidates_ranked_limited_with_quotas` 强制弱势感官配额保底 + 场景焦点提权 |
| **根因三** | DerivationRequest 包含 Rust Debug `{:?}` 格式 | Prompt 渲染 `rig_impl` | 实现 `MemoryContentSlot::display_narrative()`，转自然语言叙事模版 |
| **根因四** | rig Extractor 单步多任务缺少 CoT 思考 | LLM 推理 `contract` | Schema 增加 `sensory_analysis` 字段，引导 LLM“先思考场景焦点再选 ID” |
| **根因五** | AssembledProse 门禁过低（单条 gesture 即可静默通过） | 装配门禁 `service` | `ProseQualityReport` 增加 `sensory_diversity_score`，打上降级预警并回退基础引用 |
| **根因六** | 记忆文本经 `text_guard` 双重压缩导致失真 | 记忆存储/渲染 | 优化 Prompt 渲染层的记忆上下文承载度与锚点展示 |

---

## Global Constraints

- 保持 provider 为 `rig::providers::deepseek` 且模型为 `deepseek::DEEPSEEK_V4_FLASH`。
- 不破坏 `validate_selection` 原有白名单过滤逻辑及数据库事务原子性。
- 候选词汇硬顶维持全局上限 96 条不变。
- 保持“LLM 只选 ID，描写从语料拉取原文”的原汁原味拼装范式（不让 LLM 自由 Rewrite 描述）。

---

## Task-by-Task Implementation Plan

### Task 1: 扩充基座词库与蒸馏配额保护 (Vocab & Distillation Balance)

**Files:**
- Modify: `assets/vocab.yaml`
- Modify: `tools/distill_quality_report.py` / `tools/validate_fragments.py`
- Test: `tests/vocab_test.rs`

**Steps:**
- [ ] **Step 1:** 在 `assets/vocab.yaml` 中扩充 50~100 条覆盖视觉(visual)、听觉(auditory)、嗅觉(olfactory)、味觉(gustatory)、触觉(tactile)的标准感官条目。
- [ ] **Step 2:** 在 Python 蒸馏清洗工具中添加感官桶采样比例校验（Sensory Bucket Sampling），警示并截断超出比例的 gesture/emotion 碎片。
- [ ] **Step 3:** 运行 `cargo test --test vocab_test` 确认 YAML 解析无误。

---

### Task 2: 检索与候选强配额算法 (Sensory Quota Retrieval)

**Files:**
- Modify: `src/vocab/loader.rs`
- Modify: `src/vocab/mod.rs`
- Modify: `src/scene/service.rs`
- Test: `tests/vocab_test.rs`

**Interfaces:**
- Produces: `Vocab::candidates_ranked_limited_with_quotas(&self, selected_tags: &[String], query_terms: &[String], per_sense_cap: usize, total_max: usize) -> Vec<VocabularyCandidate>`

**Steps:**
- [ ] **Step 1:** 在 `loader.rs` 中增加强配额分组逻辑：嗅/味/触/听每类独立保底 4~6 条候选；`gesture` 和 `emotion` 单类各自上限卡死在 12~15 条。
- [ ] **Step 2:** 基于场景 `objective_event` 匹配关键词感官倾向（如“血/暗/冷”自动提升 `visual` / `olfactory` / `tactile` 得分）。
- [ ] **Step 3:** 在 `service.rs` 中替换原 `candidates_ranked_limited` 调用为带有强配额的检索方法。
- [ ] **Step 4:** 增加单元测试 `tests/vocab_test.rs` 验证在倾斜语料下弱势感官仍能获得保底候选。

---

### Task 3: Memory 解包与 Prompt 自然语言渲染 (Prompt Refactoring)

**Files:**
- Modify: `src/models/memory.rs`
- Modify: `src/llm/rig_impl.rs`
- Test: `tests/scene_test.rs`

**Steps:**
- [ ] **Step 1:** 为 `MemoryContentSlot` 实现 `display_narrative(&self) -> String`：
  - `Observation(s)` → `"【亲历/目击】" + s`
  - `Dialogue(s)` → `"【耳闻/对话】" + s`
  - `Action(s)` → `"【行为/反应】" + s`
- [ ] **Step 2:** 重构 `rig_impl.rs` 中的 `build_derivation_prompt`：
  - 替换 `{:?}` 语法为中文描述。
  - 将记忆列表渲染为 `"- [来源/确定度] 叙事类型: 内容"`。
  - 清晰分段：`【角色特质】`、`【场景情况】`、`【记忆线索】`、`【候选感官库】`。
- [ ] **Step 3:** 运行 `cargo test --test scene_test` 确认 Prompt 拼接无异常。

---

### Task 4: Extractor 思维链 (CoT) 架构改造 (Schema CoT)

**Files:**
- Modify: `src/llm/contract.rs`
- Modify: `src/llm/rig_impl.rs`
- Modify: `src/llm/mock.rs`
- Test: `tests/e2e.rs`

**Steps:**
- [ ] **Step 1:** 在 `LlmCharacterDerivation` 顶部新增 `pub sensory_analysis: String` 字段（配 doc 注释引导 JSON Schema 顺序）。
- [ ] **Step 2:** 更新 `build_derivation_prompt`，显式要求 LLM 必须先在 `sensory_analysis` 填写“对此场景的感官焦点分析”，再输出 ID。
- [ ] **Step 3:** 同步更新 `MockSenseGenerator` 的固定返回值，确保测试能反序列化 `sensory_analysis` 字段。
- [ ] **Step 4:** 运行 `cargo test --test e2e` 验证 Mock 和 Schema 解析完整通过。

---

### Task 5: Prose 装配质量门禁与感官多样性校验 (Assembly Quality Gate)

**Files:**
- Modify: `src/scene/service.rs` (及装配相关模块)
- Test: `tests/scene_test.rs`

**Steps:**
- [ ] **Step 1:** 在 `ProseQualityReport` 中新增 `sensory_diversity_score: f64` 与 `missing_senses: Vec<String>`。
- [ ] **Step 2:** 门禁规则：若推导结果中五感（visual/auditory/olfactory/tactile/gustatory）覆盖维度为 0（即全为 gesture/emotion），标记 `DegradedSensoryDensity` 标志并打印 warning 日志。
- [ ] **Step 3:** 在拼装时自动优先嵌入与场景匹配的底层 `vocab.yaml` 基础感官描述。
- [ ] **Step 4:** 编写单元测试验证低感官密度警示逻辑。

---

### Task 6: 部署校验与文档同步

**Files:**
- Modify: `AGENTS.md`
- Run: 质量门禁全套测试

**Steps:**
- [ ] **Step 1:** 更新 `AGENTS.md` 中的“数据流”与“候选池机制”章节。
- [ ] **Step 2:** 执行全面质量检查：
```powershell
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
```

---

## Deviations & Notes

- 数据层与运行时代码层双管齐下，避免单点掩盖问题。
- `LlmCharacterDerivation` 字段变动为向下兼容扩展，保留原有 Extractor 单次 API 调用的高效优势。
