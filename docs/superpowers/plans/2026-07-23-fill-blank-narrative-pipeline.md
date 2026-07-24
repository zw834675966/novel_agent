# 填空式叙事管线改造方案（Research → Design）

> **状态：** 方案阶段 · **禁止直接改运行时代码**（需用户确认后再开实现 plan）  
> **背景：** 现网 harness 在「出处合法 / 防 AI 腔」上过关，在「可读连贯 / 现代语域」上失败。  
> **用户意图：** LLM 做**填空题**，不是做「点菜单 + 硬贴原句」。

---

## 1. 问题再表述（当前 vs 目标）

### 1.1 当前逻辑（已验证）

```text
场景事件 + 角色
  → tag 短名单 + BM25 截断候选 ≤96
  → LLM#1 勾 VocabularyId + 短记忆/剧情槽
  → LLM#2 写 beat.action + 再勾 refs
  → assemble：原著 text 按 sense 序粘贴 + 舞台动作
```

**优化目标：** ID 合法、quote_density、回源。  
**未优化：** 大纲、因果、场记、语域、镜头衔接。

### 1.2 用户目标逻辑（填空题）

```text
设定 + 参数
  → ① 编导大纲（故事发展 / 必要编排）
  → ② 人物 × 场地规划
  → ③ 调用蒸馏切片「填写」细节槽（描写/对白骨架）
  → ④ 语义对齐：古文料 → 符合本设定的现代小说语
  → ⑤ loop 工程：一点一点拉平对齐（连贯 / 语域 / 设定一致）
```

一句话：**大纲定骨架 → 规划定槽位 → 蒸馏填料 → 改写对齐 → 迭代验收。**

---

## 2. 技术依据（联网检索映射）

| 用户意图切片 | 对应技术路线 | 代表工作 / 共识 | 对本项目的含义 |
|--------------|--------------|-----------------|----------------|
| 先大纲再正文 | **Plan-and-Write** | Yao et al. 2019；后续 ASP/outline 变体 arXiv:2406.00554 | 禁止「一跳到 beat 列表」；大纲是一等公民产物 |
| 长程连贯 + 递归起草 | **Re³**（Plan → Draft → Rewrite → Edit） | Yang et al. EMNLP 2022, arXiv:2210.06774 | 与「loop 拉平」同构；适合多幕商战故事 |
| 大纲控制角色动作 | **StoryVerse / Act Director** | Autodesk StoryVerse：抽象 act → 具体动作序列 | 「编导大纲 → 人物规划」可做成结构化 act/beat sheet |
| 迭代自评改写 | **Self-Refine** | Madaan et al. NeurIPS 2023, arXiv:2303.17651 | 固定维度反馈：连贯/指代/语域/出处/AI 腔 |
| 检索作证据再生成 | **RAG + rewrite / query rewriting** | 工业 RAG 共识：先检索再生成；query 改写抬召回 | 蒸馏作 **evidence**，不是 **final surface form** |
| 古文→现代对齐 | **文风迁移 / 文言现代化** | CAT-LLM（中文文章风格迁移）；多 agent 文言→白话（Nature Sci Rep 2025）；古典↔现代风格转移研究 | 第 ④ 步有独立学术地位，不可省 |
| 层次大纲 + 记忆 | **Hierarchical outline + MEM** | 2025 LLM story survey 中 outline-based 一脉 | 跨场景记忆应用「大纲节点 ID」而不仅是 memory 字符串 |
| 可控槽位生成 | **Constrained / slot / template filling** | PlotMachines 类「概念短语条件」；工业上 JSON Schema 槽位 | 与现有 `StructuredAction` / JsonSchema 抽取一致，可扩展 |

### 2.1 关键范式转变（文献共识）

1. **Structure-first > text-first**  
   直接端到端生成长文，长程连贯差；先 plan 再 expand 是主流补救。

2. **Retrieved text ≠ published text**  
   RAG 正确用法是 *grounded generation*（基于证据生成），不是 *quote collage*（引文拼贴）。  
   现网 assemble 属于后者；用户第 ④ 步要求前者。

3. **Refinement loop 需要可观测维度**  
   Self-Refine / Re³ 都强调：**反馈维度显式化** + **有界迭代**（防无限烧 token）。

4. **古文料现代化是独立子问题**  
   「信达雅」式文言→白话、风格一致性、文化保真，需要 **对齐模块**，不能指望 BM25 直接吐出现代商战段落。

---

## 3. 目标管线（Fill-Blank Narrative Pipeline）

### 3.1 阶段总览

```text
┌─────────────────────────────────────────────────────────────┐
│ Stage 0  INPUT                                               │
│  设定包: 题材/时代/语域/禁忌；角色卡；场景客观事件；全局参数   │
└───────────────────────────┬─────────────────────────────────┘
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 1  DIRECTOR  编导大纲（LLM，结构化 JSON）               │
│  StoryOutline: 幕/节拍、因果箭头、情绪弧、信息揭示、禁区      │
└───────────────────────────┬─────────────────────────────────┘
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 2  BLOCKING  人物×场地规划（LLM，结构化 JSON）         │
│  SceneCard: 地点/时间/在场/道具/镜头序列/POV 切换规则        │
│  CharBeatPlan: 每角色目标/障碍/动作意图/对白意图/感知焦点    │
└───────────────────────────┬─────────────────────────────────┘
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 3  RETRIEVE  蒸馏切片检索（程序 + 可选 query rewrite） │
│  用「现代意图句」作 query，不是用古文作最终句                 │
│  输出: SlotEvidence[] = {slot_id, vocab_ids, raw_texts, tags}│
└───────────────────────────┬─────────────────────────────────┘
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 4  FILL  槽位填写（LLM，填空）                         │
│  输入: 空槽模板 + evidence；输出: 填好的 SlotDraft             │
│  仍禁止自由开写整章；只能填声明过的槽                         │
└───────────────────────────┬─────────────────────────────────┘
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 5  ALIGN  语义/语域对齐（LLM）                         │
│  古文 evidence → 现代设定语；保留意象/情绪/感官功能           │
│  产出 AlignRecord{source_id, modern_span, fidelity_notes}    │
└───────────────────────────┬─────────────────────────────────┘
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 6  ASSEMBLE  程序拼装                                  │
│  按 SceneCard 镜头序拼接 modern_span + 受限动作/对白          │
│  禁止按 sense 字典序重排（根治「跳跃感」）                    │
└───────────────────────────┬─────────────────────────────────┘
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 7  LOOP  拉平对齐（有界 Self-Refine）                   │
│  Critic 多维打分 → PatchPlan → 局部重填/重对齐 → 再验收       │
│  max_rounds=2~3；只改不达标维度                               │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 与用户表述的一一对应

| 用户原话 | 方案阶段 |
|----------|----------|
| 输入设定场景和提示词、参数 | Stage 0 |
| LLM 推理必要编排和故事发展编导大纲 | Stage 1 |
| 对人物和场地做规划 | Stage 2 |
| 人物细节描写和说话叙事调用蒸馏切片做填写 | Stage 3–4 |
| 简化语义，原素材对齐故事设定（古文不适配现代） | Stage 5 |
| loop 工程一点点拉平 | Stage 7（Stage 6 是确定性组装） |

---

## 4. 核心数据契约（设计级，非实现）

### 4.1 Stage 0 — `StoryBrief`

```text
era: modern_business | period | mixed
register: 当代都市商战白话（强制）
genre_tags: [青梅竹马, 商战, 复仇, 重逢]
forbidden: [无故穿越古装意象, 无来源抒情因果套话, ...]
characters: [{id, name, tags, skills, goals}]
scene: {objective_event, time_hint?, location_hint?}
params: {max_beats, max_loop_rounds, density_target, modernize: true}
```

### 4.2 Stage 1 — `StoryOutline`（编导大纲）

```text
premise_one_liner
acts[]:
  act_id, summary, causal_from[], emotional_beat,
  reveal_info[], stakes, exit_condition
global_constraints[]   # 例如：商战真相对念卿不可知直到 Act3
```

### 4.3 Stage 2 — `SceneCard` + `CharBeatPlan`

```text
SceneCard:
  when, where, props[], on_stage[], camera_beats[]:
    beat_id, pov, intent, must_show[], must_not[],
    slots: [atmosphere|visual|...|dialogue|action]
CharBeatPlan:
  character_id, goal, obstacle, knowledge_state,
  intended_actions[], intended_lines[], sensory_focus[]
```

**镜头序 `camera_beats[]` 成为 assemble 的唯一顺序源**（取代 sense 固定序）。

### 4.4 Stage 3–4 — 填空槽

```text
SlotSpec:
  slot_id, beat_id, kind, query_modern  # 用现代汉语意图检索
  constraints: max_chars, must_evidence

SlotEvidence:
  slot_id, candidates[{vocab_id, text, tags, score}]

SlotDraft:
  slot_id, chosen_vocab_ids[], fill_intent  # 仍是「填什么」，不是最终现代句
```

### 4.5 Stage 5 — 对齐记录（可审计）

```text
AlignRecord:
  slot_id
  sources: [{vocab_id, original_text}]
  modern_text                 # 进入正文的唯一描写表面
  keep: [意象/情绪/感官功能]
  drop: [专名/朝代/宫廷器物...]  # 如 华妃、太监、龙涎香→可映射为红酒/冷光
  fidelity: high|medium|low   # 低则 loop 优先重做
```

**政策选择（需拍板）：**

| 模式 | 含义 | 防 AI 腔 | 可读性 |
|------|------|----------|--------|
| A. **Hard quote**（现状） | 正文必须含 original 子串 | 最强出处 | 现代题差 |
| B. **Soft ground**（推荐） | 正文用 modern_text；保留 AlignRecord 溯源 | 出处在元数据 | 可读强 |
| C. **Hybrid** | 短 lemma 可硬贴；长句必须对齐 | 折中 | 折中 |

**推荐默认 B**，并用 `fidelity` + `provenance` 报告替代「字面 quote_density」作为主 KPI。

### 4.6 Stage 7 — Critic 维度（loop 打分卡）

| 维度 | 检测问题 | 失败时动作 |
|------|----------|------------|
| premise_alignment | 是否贴题（破产/重逢等） | 回 Stage1/2 局部 |
| causal_chain | 镜头间是否缺因果/时间 | 插 transition 槽或重排 |
| referent_clarity | 谁/何处/何时是否可解 | 重填 action/dialogue |
| register_modern | 是否仍有未映射古装专名 | 重跑 Stage5 |
| evidence_use | 描写是否仍锚定 evidence | 重检索或重填 |
| anti_ai_glue | 套话/因果水词 | text_guard + 重写 |
| continuity | 与既有 memory/大纲冲突 | 回 Stage1 或 memory 修正 |

停止条件：全维度 ≥ 阈值 **或** `round >= max_rounds`。

---

## 5. 蒸馏语料在新管线中的正确角色

### 5.1 现在（错误用法）

```text
BM25(场景字面) → 勾 ID → 原句硬贴 → 高 density 假繁荣
```

### 5.2 目标（正确用法）

```text
CharBeatPlan.sensory_focus / slot.query_modern
  → (可选) LLM query rewrite 成「检索友好」意图
  → BM25/向量 取 evidence
  → FILL 决定用哪几条 evidence 填哪一槽
  → ALIGN 把 evidence 功能迁移到现代语
  → 正文只出现 modern_text；AlignRecord 可审计
```

### 5.3 检索改进（仍程序主导）

1. **Query 用现代意图句**（来自 Stage2），不是客观事件全文硬切。  
2. **Query rewriting**（文献/工业 RAG 标配）：把「谈判桌冷漠」扩成「冷笑、推文件、空气凝滞」等可命中 tag 的 query。  
3. **书源隔离保留在 evidence 层**；正文层只看 modern 一致性。  
4. **可选第二语料桶**（中期）：现代商战/都市描写蒸馏，与 hlm/zhz 分 index；古代桶主供「情绪/节奏/修辞功能」。

---

## 6. 与现有 novels 架构的映射（迁移，不推倒）

| 现有模块 | 新角色 |
|----------|--------|
| `vocab` + BM25 | 降级为 Stage3 Retriever；接口扩展 `query_modern` |
| `SenseGenerator` derive | **拆分**：Director / Blocking / Fill 可分 extractor；短期可串行多 schema |
| `ProseGenerator` narrate | 改为 Fill+（可选）Align；不再直接产出「可贴正文」 |
| `AssembledProse::assemble` | 改为按 `camera_beats` 拼 `modern_text`；去掉 sense 字典序主序 |
| `text_guard` | 保留，作用于 action/dialogue/modern 自由槽 |
| `validate_selection` | 扩展为：evidence ID 合法 + AlignRecord 必填 |
| CLI harness observation | 增加 outline / critic scores / fidelity / loop rounds |
| `quote_density` | 降为辅指标；主指标见 §7 |

**反 AI 腔不丢弃，改定义：**

- 旧：描写必须 verbatim 原著。  
- 新：描写必须 **evidence-grounded + 可追溯 + 禁套话**；表面允许现代化。

---

## 7. 验收指标（替代「读起来累」的主观唯一标准）

| 指标 | 定义 | 目标（初版） |
|------|------|----------------|
| **coherence_score** | Critic 或规则：指代可解、时间不跳、因果可复述 | ≥ 0.7 |
| **register_violation_count** | 未映射古装专名/朝代器物 | = 0（现代题） |
| **outline_coverage** | 大纲 must_show 出现在正文 | ≥ 0.9 |
| **evidence_ground_rate** | 描写槽有 AlignRecord 且 sources 非空 | ≥ 0.8 |
| **fidelity_low_rate** | fidelity=low 占比 | ≤ 0.2 |
| **loop_rounds** | 实际迭代 | 中位 ≤ 2 |
| **anti_ai_glue** | text_guard 命中率 | 下降趋势 |
| quote_density（旧） | 仅 Soft ground 下改测「evidence 使用率」 | 观测 |

人工验收剧本：沿用「青梅→商战→破产→蛰伏→重逢」，要求**不经二次翻译即可通读**。

---

## 8. 分阶段落地（仍不写代码，只排期）

### Phase 0 — 规格冻结（0.5–1 天）

- 拍板：Hard / Soft / Hybrid（§4.5）  
- 固定 JSON schema 草案：Outline / SceneCard / AlignRecord / CriticReport  
- 写 1 个黄金样例（人工作弊填满全链路），作为回归 fixture  

### Phase 1 — 大纲 + 场记（最小可读跃迁）

- 在 derive **之前**插入 Stage1–2  
- assemble **改为镜头序**（可仍用硬贴 quote，先治跳跃）  
- 验收：破产场不再「看封条→吐血→冰凉」无主语乱跳  

### Phase 2 — 填空 + evidence 元数据

- Stage3–4：槽位 + 检索 + 填空契约  
- 保留现有 VocabularyId 校验  

### Phase 3 — ALIGN 现代化（治语域错位）

- Stage5 Soft ground  
- register_violation 门禁  
- 对照 CAT-LLM / 文言现代化：prompt 约束 keep/drop 列表  

### Phase 4 — LOOP

- Critic 多维 + 有界 refine（Self-Refine 形态）  
- CLI 输出 critic 与 round  

### Phase 5 — 检索与语料升级（可选）

- query rewrite、现代语料桶、向量检索（若 BM25 不够）  

**建议第一实现切口：Phase 1**（性价比最高，直接打你反馈的「跳跃/不连贯」）。

---

## 9. 风险与护栏

| 风险 | 缓解 |
|------|------|
| 调用次数爆炸（每场景 5–10 次 LLM） | 有界 loop；Stage1 可跨场景缓存；mock 路径完整 |
| ALIGN 滑向自由胡写 | 强制 sources；fidelity 低重做；禁无 evidence 的纯描写 |
| 丢掉「反 AI 腔」品牌 | 明文 KPI：evidence_ground + anti_ai_glue；对外报告溯源 |
| 大纲与正文漂移 | Re³ 已知问题：需在 Critic 查 outline_coverage |
| schema 过重导致抽取失败 | 每阶段独立 extractor + 单次重试；槽位上限 |

---

## 10. 推荐默认决策（供确认）

1. **范式：** Soft-grounded fill-blank（B），非硬贴拼贴。  
2. **顺序：** Director → Blocking → Retrieve → Fill → Align → Assemble → Loop。  
3. **组合序：** 镜头表驱动，废除 sense 字典序作为主序。  
4. **蒸馏：** 功能素材库（情绪/感官/节奏），非最终句子库。  
5. **第一刀：** Phase 1 大纲+场记+镜头序拼装。  
6. **第二刀：** Align 现代化。  
7. **第三刀：** Critic loop。  

---

## 11. 参考文献（检索入口）

- Yao et al., Plan-and-Write, 2019  
- Yang et al., Re³ Recursive Reprompting and Revision, EMNLP 2022 (arXiv:2210.06774)  
- Madaan et al., Self-Refine, NeurIPS 2023 (arXiv:2303.17651)  
- Guiding LLM Story Generation via ASP outlines, arXiv:2406.00554  
- StoryVerse Act Director (Autodesk)  
- CAT-LLM Chinese article-style transfer (arXiv:2401.05707)  
- Multi-agent Classical→Modern Chinese translation, Sci Rep 2025  
- Industrial RAG: query rewriting + grounded generation (Azure AI Search 等)  
- Survey: LLMs for Story Generation (Findings EMNLP 2025 outline-based 节)  

---

## 12. 决策记录

| # | 决策项 | 状态 | 取值 |
|---|--------|------|------|
| 1 | 出处模式 Soft / Hard / Hybrid | **已确认（用户：`A`）** | **A Hard**：正文描写必须含原著子串；溯源=字面回源 |
| 2 | 第一实现切口 | **已确认（用户：`1`）** | **只做 Phase 1**（大纲 + 场记 + 镜头序拼装） |
| 3 | 多轮 LLM / loop 成本 | **Phase 1 不适用** | Phase 1 仅 +1～2 次结构化 LLM；loop 留 Phase 4 |

### 12.1 决策含义（A Hard + Phase 1）

**Hard 约束（全局政策，后续 Phase 不得静默改写）：**

- 注入正文的描写 span **必须是** `VocabEntry.text` 的连续子串（可整句，可短 lemma）。
- `unverified_quotes` / 字面 `contains` 回源 **继续作为硬门禁**（不得降为仅元数据）。
- **不做** Stage 5「古文→现代改写后进正文」作为默认路径；若未来要 Soft，必须另开决策，不可在 Hard 下偷渡 paraphrase。
- 语域错位（华妃/太监出现在商战）**不靠改写解决**，靠：更好的检索 query、镜头意图过滤、tag/书源策略、可选拒用高冲突 evidence。

**与早期方案文中「推荐 Soft」的关系：**  
§3–§5 中 Soft ground 描述保留为**备选路线**；**当前冻结政策 = Hard**。Phase 3 若启动，默认改为「检索/过滤增强」而非 Align 改写，除非用户改口 Soft。

### 12.2 Phase 1 冻结范围（已确认）

**做：**

1. Stage 1 `StoryOutline`（编导大纲，JSON schema + extractor）
2. Stage 2 `SceneCard` + `camera_beats`（人物×场地×镜头表）
3. `assemble` **主序改为 `camera_beats`**（废除 sense 字典序作为镜头主序）
4. 硬贴路径保留：`camera_beats` 上的 refs → 仍 `entry.text` 原样注入 + 回源校验
5. CLI / observation 暴露大纲与镜头摘要
6. 回归：同一「破产清算」类场景，镜头间主语/时序可通读；quote 回源仍 0 unverified

**不做（显式延后）：**

- Stage 5 Align 现代化进正文（与 **A Hard** 冲突）
- Stage 7 Critic loop
- 现代语料桶 / 大改 BM25（可作为 Phase 1.5 小优化，非本刀必须）
- 推翻 VocabularyId 校验与 text_guard

### 12.3 下一步

- [x] 用户确认第一刀 = Phase 1  
- [x] 用户确认出处 = **A Hard**  
- [x] Phase 1 实现 plan：`docs/superpowers/plans/2026-07-23-phase1-outline-camera-assemble.md`  
- [ ] 用户选择执行方式后按实现 plan 动代码  
