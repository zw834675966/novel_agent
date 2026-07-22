# vocab 本地蒸馏：功能与质量 vs 专业做法 — 差异分析报告

> Superpowers 规格文档 · 2026-07-22  
> 范围：`novels-layer-vocab` 本地蒸馏链路（corpus → distill → validate → 词库合并/消费）  
> 证据：代码审读 + 产出统计脚本 + 联网检索（AEVS/CV/DH 标注/RAG grounding）  
> 非目标：不在本报告中改代码；结论供后续计划/实现选型

---

## 1. 执行摘要

| 维度 | 结论（一句话） |
|------|----------------|
| **设计意图** | 强：LLM 只做「定位与分类」，`text` 必须是源章连续子串 → 从源头压 AI 腔 |
| **产出规模** | 强：`assets/distilled` ≈ **13,883** 条 / **187** 章文件（hlm 119 + zhz 68） |
| **逐字保真** | 强：抽样校验 `hlm-c001`/`zhz-c001` → **293/293 verbatim** |
| **标签/类别质量** | 中弱：类别严重失衡；标签近 1.1 万种自由文本；对话句混入约 **10%** |
| **跨模型稳定** | 弱：同章 Jaccard 仅 **0.07–0.20**（不同 distill 目录互相重叠极低） |
| **运行时闭环** | **关键缺口**：`Vocab::load_dir_merged` 已实现，**`main` 只加载 6 条 `vocab.yaml`** |
| **相对专业实践** | 已踩中「span grounding / provenance」核心；缺 anchor 闭集、IAA、层级词表、覆盖率补充与系统评估 |

**总评**：蒸馏作为**素材生产线**已具备可工作的护栏与体量；作为**生产词库供给**尚未闭环；相对 2024–2026 业界「grounded extraction + controlled vocabulary + human audit」流水线，差在**标注规范、评估、检索与集成**，不差在「是否逐字」这一主轴。

---

## 2. 现状功能地图（As-Is）

### 2.1 流水线

```
temp/ 原著
  → tools/extract_corpus.py     # 切章 → corpus/<book>/cNNN.txt
  → src/bin/distill.rs          # DeepSeek Extractor：定位+分类+tags
  → assets/distilled/<book>-cNNN.yaml
  → tools/validate_fragments.py # 全量逐字/长度/类别/去重（可 --prune）
  → tools/verify_distilled.py   # 随机抽样二次复核
```

并行产出目录（实验对照，非统一入口）：

| 目录 | 规模（约） | 备注 |
|------|------------|------|
| `assets/distilled` | 187 files / 13883 条 | 主库 |
| `assets/distilled-deepseek` | 31 / 3701 | 早期/对照 |
| `assets/distilled-claude-agent` | 1 / 80 | 单章对照 |

### 2.2 蒸馏器硬约束（`distill.rs`）

| 参数 | 值 | 意图 |
|------|-----|------|
| 块大小 | 2500 字（尽量在换行切） | 召回 vs 上下文 |
| excerpt 长度 | 6–120 字 | 过短无神韵 / 过长不成「词条」 |
| 类别 | 8：五感 + emotion/gesture/atmosphere | 与 `VocabFile`/`validate_selection` 对齐 |
| 保真 | 去空白后 `source_norm.contains(excerpt_norm)` | 防改写 |
| 去重 | 章内归一化文本 `HashSet` | 防重复入库 |
| 断点 | 输出文件已存在则 skip | 可续跑 |
| ID | `{book}-c{chap}-{seq}`（落在类别 map 的 key 下） | 章级可追溯 |

Prompt 铁律：**只摘录、不改写**；tags 2–4 个（人物/情绪/场合）。

### 2.3 词库层消费能力（`src/vocab`）

| 能力 | 状态 |
|------|------|
| 8 类 YAML 结构 | ✅ 与蒸馏输出同形 |
| `validate_selection` 候选集护栏 | ✅ 防编造 ID |
| `merge` / `load_dir_merged` | ✅ API 存在 |
| `candidates` / `candidates_for_tags` | ✅ tag 过滤 + 无匹配回退全量 |
| **运行时加载 distilled** | ❌ `main.rs` 仅 `assets/vocab.yaml`（手写 **6** 条） |
| 向量/混合检索 | ❌ 计划明示不做；靠 tags |

### 2.4 与引擎契约的关系

蒸馏产物的 `text` 字段语义 = **可复用的原著片段**（长描写），而手写 `vocab.yaml` 的 `text` = **短显示词**（「血迹」「烛光」）。  
二者共享 YAML 外形，但**粒度与用途不同**：

- 短词：适合「感官选择 ID」的离散符号表  
- 长片段：适合 prose 装配时「按 ID 拉原文」防 AI 腔  

当前引擎推导路径仍按 **ID 选择 + 短候选** 设计；长片段进候选集会显著增大 prompt 与选择空间，需单独产品策略。

---

## 3. 产出质量实测（Evidence）

统计对象：`assets/distilled/**/*.yaml`（脚本 `temp/_analyze_distill_quality.py`）。

### 3.1 规模与长度

| 指标 | 值 |
|------|-----|
| 条目总数 | 13,883 |
| 长度 p50 / p90 / max | 21 / 46 / 119 字 |
| ≤10 字占比 | **13.8%** |
| 平均长度 | ~25 字 |

### 3.2 类别分布（严重不均）

| 类别 | 占比 | 条目约数 |
|------|------|----------|
| gesture | **35.2%** | 4891 |
| emotion | **27.3%** | 3786 |
| visual | 15.5% | 2153 |
| auditory | 8.5% | 1185 |
| atmosphere | 8.4% | 1167 |
| tactile | 3.1% | 433 |
| olfactory | 1.4% | 197 |
| gustatory | **0.5%** | 71 |

**解读**：古白话/宫廷叙事中「动作神态 + 心理」天然密集，五感（尤其味嗅）稀疏——与原著文体一致，但对「五感引擎」产品目标而言，**候选库有效多样性不足**。

### 3.3 噪声启发式（非金标，仅代理）

| 代理指标 | 结果 | 含义 |
|----------|------|------|
| 对话/说白句模式 | **~10.1%** | 含「笑道/便道/道：」等，偏叙事对话而非纯描写 |
| visual∩心理词 | ~3% of visual | 轻度跨类（视觉条混入心绪） |
| emotion∩「只见」等 | 23 条 | 情绪类混环境描写（少量） |
| 除书名外无 content tag | **0%** | 标签覆盖完整（但未必规范） |
| 唯一 content tag 数 | **10,899** | 近「一标签一世界」——**无受控词表** |

### 3.4 逐字保真

```
python tools/validate_fragments.py hlm-c001 zhz-c001
→ 293 verbatim, 0 dropped
```

主库抽样路径与 `distill.rs` 内联规则一致：**归一化空白后的连续子串**。  
局限：不记录字符偏移；跨空白归一化可能掩盖标点/排版歧义，但整体对齐「防改写」目标。

### 3.5 跨模型/跨目录一致性（可重复性）

同章文本集合 Jaccard：

| 对比 | Jaccard |
|------|---------|
| hlm-c001 distilled vs deepseek | **0.067** |
| hlm-c001 distilled vs claude | **0.094** |
| hlm-c001 deepseek vs claude | **0.203** |
| zhz-c001 distilled vs deepseek | **0.095** |

**解读**：同一源章，不同模型/跑次抽到的「值得复用」span 集合高度发散 → 无 gold set / 无 ensemble 共识时，**素材库是「某次 LLM 审美」而非「稳定文学标注」**。

### 3.6 人工观感（样例）

高质量（神韵 + 可复用）：

- `生得仪容不俗，眉目清明，虽无十分姿色，却亦有动人之处`
- `将一条街烧得如火焰山一般`

风险型：

- 过短：6–10 字碎片，作「词条」信息量低  
- 对话混入：削弱「描写素材」纯度  
- 标签自由：`关切`/`女主`/`主人公` 与人物名混层，难做稳定过滤

---

## 4. 专业做法对照（To-Be 参考，联网）

以下为 2024–2026 相关实践的收敛要点（非完整学术综述）。

### 4.1 Span grounding / 防幻觉抽取

| 做法 | 要点 | 来源类型 |
|------|------|----------|
| **AEVS**（Anchor–Extraction–Verification–Supplement） | 先发现文本锚点 → 闭集抽取 → 还原校验 → 覆盖率补抽；元素须可回链到源 span | 学术框架（KG 抽取） |
| 程序化校验 | 输出与源文对比；无引用则 flag；分层验证门 | 工程实践（grounded RAG/agent） |
| 分层护栏 | retrieve → constrain → verify → abstain | 综述性工程文 |

与 novels 对照：你们已有 **Extraction + Verification（子串）**；缺 **Anchor 闭集生成**、**字符级 provenance**、**coverage-aware supplement**。

### 4.2 受控词表（Controlled Vocabulary）

专业信息组织（图书馆学 / 生物医学本体实践的可迁移原则）：

1. **Authority**：术语有规范形与别名  
2. **层级**：broader/narrower，而非扁平自由 tag  
3. **特异性优先**：避免过宽标签；废弃词不得再用  
4. **版本化**：词表变更可追溯  

novels 现状：类别 8 个固定；**tags 完全开放** → 接近「folksonomy」，不是 CV。

### 4.3 数字人文（DH）语料标注

1. 书面 **annotation guidelines** + 多标注者  
2. **IAA**（Cohen’s κ / Krippendorff’s α）  
3. 工具链：brat / CATMA / Annotation Studio 等  
4. 标注不确定度进入下游评估  

novels 现状：单模型自动标注；无 IAA；无 gold 章；`verify` 仅查逐字不查类别正确性。

### 4.4 生产级 LLM 抽取（如 ODKE+ 类）

- Ontology/schema 约束抽取  
- Grounding verification 显著降幻觉（文献称量级 30%+）  
- 人工/自动 precision 审计与 ranking  

### 4.5 风格保真生成（RAG 引用）

工程共识：**需要原著腔时，应 quote 检索片段而非让模型改写**。  
这与 corpus distillation 计划的产品叙事一致；但落地依赖 **检索 + 装配**，不仅是「抽进 YAML」。

---

## 5. 差异矩阵（Gap Matrix）

| # | 能力维度 | 当前 novels | 专业基准 | 差距 | 严重度 |
|---|----------|-------------|----------|------|--------|
| G1 | **文本保真** | 空白归一化子串校验 + prune 工具 | 字符偏移 provenance + 多级匹配（exact/fuzzy） | 主轴已对齐；缺偏移与 soft-match 分级 | 中 |
| G2 | **Anchor 闭集** | 开放生成 excerpt | 先发现可抽取锚点再约束 | 无闭集 → 发散抽取 | 高 |
| G3 | **类别体系** | 8 扁平类 | 指南定义边界 + 互斥/多标规则 | 边界含糊 → 失衡与串类 | 高 |
| G4 | **标签体系** | 自由 tag（~1.1 万） | Authority / 层级 / 有限 facets | 过滤不稳、候选爆炸 | 高 |
| G5 | **覆盖率** | 每块「尽量 8–20 条」无保证 | coverage-aware 补抽稀疏模态 | 味嗅触严重不足 | 高 |
| G6 | **标注信度** | 单 LLM | 多模型共识 + 人工金标 + IAA | 跨跑 Jaccard 极低 | 高 |
| G7 | **评估** | drop 计数 / 抽样逐字 | 类别 precision、tag 精度、人类 Likert、下游任务 | 无端到端质量 KPI | 高 |
| G8 | **运行时集成** | merge API 闲置；演示 6 词 | 闭环：加载 → 候选 → 选择 → 引用装配 | **蒸馏成果未进入主路径** | **关键** |
| G9 | **检索** | tags 精确匹配 | 混合：facet + embedding + 角色/场景过滤 | 万级条目时 tag 过滤失效风险 | 中（规模化后变高） |
| G10 | **粒度策略** | 短词与长片段同 schema | 分层：lemma 词表 vs quote 素材库 | 产品语义混淆 | 中 |
| G11 | **权限/版权** | 本地 corpus 处理 | 再分发与训练合规策略 | 知识库需注明使用边界 | 中（合规） |
| G12 | **可重复构建** | 断点 skip 已有文件 | 版本哈希、seed、模型版本元数据 | 难以复现同一库 | 中 |

---

## 6. 根因分析（Why）

1. **目标成功定义偏「防 AI 改写」**  
   逐字校验把「幻觉改写」压到接近零，但**不检验「该不该抽 / 类对不对 / 标稳不稳」**。

2. **Prompt 驱动开放抽取**  
   无 anchor 阶段 → 模型按偏好抽 gesture/emotion → 与分布统计一致的系统性偏置。

3. **Schema 共用掩盖产品分层**  
   `text` 既是短标签又是长 quote，导致蒸馏成功却难直接塞进当前 `SenseGenerator` 候选语义。

4. **集成滞后**  
   计划文档写明「词库从 6 扩到数千」；代码有 merge，入口未接 → 质量再高也是**离线资产**。

5. **无评估闭环**  
   有 validate/verify 工具，但 KPI 停在 verbatim；没有「类别金标章」或「下游 derive 可用性」指标。

---

## 7. 建议路线（差异关闭顺序）

按杠杆排序，便于拆成后续 superpowers plan。

### P0 — 闭环集成（否则蒸馏无业务价值）

1. `main` / `StoryService` 启动：`vocab.yaml` + `load_dir_merged("assets/distilled")`（或可配置路径）  
2. 明确两阶段用途：  
   - **感知选择**：可对蒸馏库做「压缩视图」（lemma / 摘要 tag）或限制候选 top-K  
   - **正文装配**：按 ID 注入 `text` 原文 quote（prose 层）  
3. 候选爆炸防护：按 scene tags / 角色名 / 书 source 过滤；硬顶候选数

### P1 — 质量治理（让库「可依赖」）

1. **Annotation guideline** 一页纸：八类互斥规则 + 禁止对话句 + 最短有效描写定义  
2. **Gold set**：hlm/zhz 各 1–2 章人工修订类别与 tag facets  
3. **双模型共识**：仅保留 ≥2 源同意的 span（或人工仲裁）→ 抬升稳定性  
4. **Tag 收束**：人物 / 情绪 / 场合三 facet；映射到有限枚举 + 别名表  
5. **稀疏类补抽**：对 olfactory/gustatory/tactile 单独 prompt 或规则预召回（含感官关键词表）

### P2 — 专业级 provenance

1. 存储 `char_start/char_end` 或段落 ID  
2. 校验分级：exact → whitespace-norm → 拒绝；记录 drop 原因仪表盘  
3. 构建元数据：`model` / `prompt_ver` / `source_sha256` / `built_at`

### P3 — 评估与门禁

| KPI | 目标草案 |
|-----|----------|
| Verbatim rate | ≥ 99.9%（已接近） |
| 非描写句率（对话启发式或金标） | ≤ 3% |
| 类别 macro-F1（相对 gold） | ≥ 0.75 |
| Tag 命中受控表比例 | ≥ 95% |
| 跨模型 Jaccard（共识后） | ≥ 0.4 |
| 下游：derive 后 quote 可装配率 | ≥ 90% |

门禁：`validate_fragments` 失败阻塞；gold 章回归；可选 `cargo test` 加载合并词库冒烟。

### 明确不优先

- 全量向量库（与「ID 护栏」哲学可后接，非第一刀）  
- 自动改写润色（违背项目反 AI 腔宗旨）  
- 未授权公开分发原著片段

---

## 8. 与既有 Superpowers 文档的关系

| 文档 | 关系 |
|------|------|
| `plans/2026-07-19-corpus-distillation.md` | 本报告验证 Task1–5 产物规模；指出 Task「引擎吃词库」未完成 |
| `specs/2026-07-19-quality-optimization-design.md` | 护栏语义一致；蒸馏质量不在其范围 |
| prose 相关 plan | 长片段 quote 装配是闭环关键下游 |

已产出执行计划：  
**`docs/superpowers/plans/2026-07-22-distilled-vocab-runtime-closure.md`**  
（DEFINE→PLAN 完成；BUILD 起按 Task 1–10 推进。）

---

## 9. 风险与合规

- 蒸馏库含《红楼梦》《甄嬛传》连续原文：仅限本地引擎引用时需注意版权与再分发。  
- 高体量自由 tag 进入 LLM prompt 可能泄露角色名噪声、抬高 token 成本。  
- `merge` 键冲突「后者覆盖」：多目录混并可能静默丢片。

---

## 10. 结论

本地蒸馏在 **「LLM 定位 + 程序逐字校验」** 上与当代 grounded extraction 同向，且已积累可用规模的原著片段资产。  

主要差异不在「会不会幻觉改写」，而在：

1. **未接入运行时**（关键）  
2. **无受控标签与类别规范**  
3. **无信度/覆盖率/下游评估**  
4. **短词符号表 vs 长句素材库未分层**  

关闭 G8 + G4 + G6 后，蒸馏才能从「离线语料工程」变成「novels 感知/叙事质量护城河」。

---

## 附录 A — 复现命令

```powershell
# 逐字校验（样本）
python tools/validate_fragments.py hlm-c001 zhz-c001

# 质量代理统计
python temp/_analyze_distill_quality.py

# 蒸馏（需 API Key；已存在章跳过）
cargo run --bin distill -- hlm 1 1
```

## 附录 B — 关键代码锚点

| 路径 | 职责 |
|------|------|
| `src/bin/distill.rs` | 蒸馏主程序 |
| `tools/extract_corpus.py` | 切章 |
| `tools/validate_fragments.py` | 全量护栏 |
| `tools/verify_distilled.py` | 抽样复核 |
| `src/vocab/loader.rs` | 加载/合并/候选 |
| `src/vocab/validate.rs` | ID 护栏 |
| `src/main.rs` | **未合并 distilled** |
| `assets/vocab.yaml` | 6 条手写基线 |
| `assets/distilled/` | 主蒸馏产出 |
