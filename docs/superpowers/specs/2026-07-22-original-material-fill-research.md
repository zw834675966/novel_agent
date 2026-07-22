# 原素材填充写作：论文线 + GitHub 主流做法研究（反 AI 味 / 反 AI 因果）

> ⚠️ **已并入权威全文** [`2026-07-22-original-fill-soul-research.md`](./2026-07-22-original-fill-soul-research.md)（含核心结论 §0、AEVS/VeriCite 补全、研究->代码落地状态 §5a）。本文件保留作历史快照，不再更新；以权威全文 + `novels-conclusion-critique` + 代码为准。

> Superpowers research · 2026-07-22  
> 目标：用**原著连续片段填充**（verbatim fill），让生成有「灵魂」，并压制 **AI 味** 与 **AI 因果逻辑**。  
> 证据日志：scratch `research-sources.md`（会话 goal 目录）  
> 非目标：本文件不重架构引擎、不重跑全量蒸馏。

---

## 1. Goal framing（验收标准 1）

### 1.1 用户目标的可操作定义

| 口语目标 | 本报告的可观测定义 | 反模式（不要做） |
|----------|-------------------|------------------|
| **有灵魂** | 感官/情绪/神态等高信息句**密度高**，且来自**可溯源原著片段**（quote density + provenance） | 用 LLM「学风格」后自由改写高信息句 |
| **杜绝 AI 味** | 高信息描写**禁止 paraphrase**；程序只注入词库 `text`；校验非原文则丢弃 | 风格迁移 / 「像曹雪芹一样写」 |
| **杜绝 AI 因果逻辑** | 情节因果由**有界骨架**（objective event / plot draft / action 模板）承载；LLM **不得**用通顺的泛因果句填满描写层 | 让模型端到端写完整段落因果链 |

### 1.2 「原素材填充」流水线（概念）

```
primary corpus
  → extract grounded spans (locate, don't rewrite)
  → controlled index (IDs + tags / retrieval keys)
  → retrieve / select IDs under constraints
  → assemble: skeleton + inject verbatim texts
  → verify: every soul-bearing span ⊆ source
```

核心原则（论文与工程收敛点）：

1. **Soul-bearing content is retrieved and quoted, not generated.**  
2. **LLM 只做：分类、检索排序辅助、骨架与衔接**（且衔接应可被压短或模板化）。  
3. **程序验证 provenance**（子串 / 字符偏移），不信任模型自称「引自原文」。

---

## 2. Peer-level paper lines（≥5）

| # | Paper / line | Venue / ID | Copy | Don't copy |
|---|--------------|------------|------|------------|
| P1 | **RELiC: Retrieving Evidence for Literary Claims** (Thai et al.) | ACL 2022 · [arXiv:2203.10053](https://arxiv.org/abs/2203.10053) | 把「文学性」建成 **claim ↔ quote 检索任务**；灵魂句是 **候选集合上的选择**，不是开放生成 | 不要把批评文本任务直接当小说写作目标；BM25/字面重合在文学域会失败——需要更好的 query |
| P2 | **Literary Evidence Retrieval via Long-Context LMs** | ACL 2025 short · [arXiv:2506.03090](https://arxiv.org/html/2506.03090v1) · [PDF](https://aclanthology.org/2025.acl-short.29.pdf) | 强调 **可验证**：缺引语必须从 primary source **找回**；长上下文仍会 overgenerate、丢 nuance | 不要依赖「整书塞进上下文再生成引语」当生产方案（成本与过生成） |
| P3 | **LangExtract (Google)** design | [Google blog](https://developers.googleblog.com/introducing-langextract-a-gemini-powered-information-extraction-library/) + [resolver](https://github.com/google/langextract/issues/259) | **Post-hoc span alignment**：模型抽文本，程序对齐字符偏移；`char_interval=None` 即丢弃 | 不要让 LLM 自己报 offset 当真理 |
| P4 | **AEVS** Anchor–Extraction–Verification–Supplement | [MDPI Computers 2026](https://www.mdpi.com/2073-431X/15/3/178) | **先闭集锚点 → 再抽取 → 还原校验 → 覆盖补抽**；幻觉可检测 | 不要把 KG 三元组格式硬套进五感词库，只借 **provenance 流程** |
| P5 | **VeriCite** / **CiteFix** (RAG citation reliability) | [VeriCite](https://arxiv.org/html/2510.11394v1) · [CiteFix](https://arxiv.org/html/2504.15629v1) | 生成后 **核对引用是否真支撑句子**；错误引用会毁掉信任 | 小说场景不要只贴「脚注 URL」，要对 **注入片段 ⊆ 词库 text** |
| P6 | **CAT-LLM** Chinese article-style transfer | [arXiv:2401.05707](https://arxiv.org/html/2401.05707v1) | 理解「风格特征可抽取」 | **不要作为主路径**：风格迁移 = 改写 = 重新引入 AI 味；仅可作骨架语气的次要参考 |
| P7 | **CTG / DATG surveys** controllable generation | [CTG survey arXiv:2408.12599](https://arxiv.org/abs/2408.12599) · Findings ACL 2024 DATG | **解码期约束 / 属性控制**可限制连接词、长度、禁用 AI 套话 | 控制风格 ≠ 填充原著灵魂；约束应服务 **ID 闭集与长度**，不是「更像古文」 |

---

## 3. GitHub / open-source practice lines（≥4）

| # | Project | URL | Copy | Don't copy |
|---|---------|-----|------|------------|
| G1 | **google/langextract** | https://github.com/google/langextract | 抽取后 **程序对齐**；过滤未 grounded 结果；示例要求 **verbatim extraction_text** | 不把临床/报告域 schema 原样搬进小说 |
| G2 | **martiansideofthemoon/relic-retrieval** | https://github.com/martiansideofthemoon/relic-retrieval | dense claim→quote 检索；评估用 recall@k | 不要无 query 设计就硬训 dense 模型；先把 novels 的 scene/character query 定义好 |
| G3 | **Hellisotherpeople/Constrained-Text-Generation-Studio (CTGS)** | https://github.com/Hellisotherpeople/Constrained-Text-Generation-Studio | **解码约束**思想：禁词、强制结构——可映射为「描写位只能是 vocabulary ID」 | 不要用 CTGS 去「诗化」生成灵魂句 |
| G4 | **yyz-nbt/AEVS** | https://github.com/yyz-nbt/AEVS | anchor 闭集 + verification 流水线可借到蒸馏 | 不必上完整 KG 栈 |
| G5 | **taozhen1110/cat-llm** | https://github.com/taozhen1110/cat-llm | 风格定义模块可参考「特征清单」 | **主路径禁用**全文 style transfer |
| G6 | **Production RAG citation patterns** (llmware evidence posts / VeriCite line) | e.g. [llmware evidence verification writeups](https://medium.com/@darrenoberst/using-llmware-for-rag-evidence-verification-8611abf2dbeb) | **post-gen evidence check**；无证据则拒答/重抽 | 不要把网页 RAG 当小说灵魂来源 |

---

## 4. Recommended stack（排序）— retrieve → constrain → assemble → verify

与「灵魂 / 反 AI 逻辑」的绑定：

| Rank | Stage | 推荐技术选择 | 服务「灵魂」 | 服务「反 AI 味」 | 服务「反 AI 因果」 |
|------|-------|--------------|--------------|------------------|--------------------|
| **1** | **Verify-first index** | 蒸馏/抽取时：substring 或 LangExtract-class offset；非法丢弃 | 库内只剩真原文 | 阻断 AI 改写入库 | — |
| **2** | **Retrieve (semantic)** | claim/scene/character → top-k **相关** quotes（RELiC-style dense 或 hybrid BM25+embed）；**禁止**字典序当语义 | 贴戏的灵魂句 | 减少乱塞套话描写 | 检索条件绑定场景，不编因果 |
| **3** | **Constrain selection** | LLM **只输出 VocabularyId / span-id 闭集**；解码可借鉴 CTGS/CTG 思路限制输出形态 | ID 指向真句 | 模型不能发明描写 | 模型不能用自由描写冒充情节 |
| **4** | **Assemble (programmatic inject)** | 程序：`skeleton + join(vocab.text)`；**禁止**模型重写 `text` | 原著腔密度 | 消灭 paraphrase 层 | 骨架与描写分层 |
| **5** | **Verify output** | 拼装后：每条灵魂句仍 ∈ 词库/源文；CiteFix/VeriCite 式「句 ↔ 源」核对；失败则 strip/retry | 可审计灵魂 | 漏网幻觉描写被剥 | action 过长可另设规则 |
| **6 (optional)** | **Skeleton-only generation** | LLM 只写短 action / 舞台提示；或模板因果 | — | 减少 AI 腔衔接 | **主战场**：因果只在骨架 |

**反模式总表（明确不推荐作主路径）**

- 「用 CAT-LLM / 风格迁移学红楼腔再生成」  
- 「整书塞进上下文让模型自由写一章」  
- 「RAG 摘要改写后再当描写用」  
- 「仅 prompt：请减少 AI 味」而无闭集与校验  

---

## 5. Gap vs current **novels** path

当前路径（读码 + 既有 gap-analysis / runtime-closure）：

```
corpus → distill (locate+classify) → substring validate
  → assets/distilled YAML (ID, text, tags)
  → load_runtime_vocab merge
  → select_context_tags → candidates_for_tags_limited (24/96)
  → SenseGenerator selects IDs → validate_selection
  → prose: assemble injects vocab.text for refs + free action string
```

| Area | novels 现状 | 相对推荐栈 | 标记 |
|------|-------------|------------|------|
| 原著逐字入库 | `distill` + 空白归一子串校验；非原文丢弃 | ≈ Verify-first index | **已对齐** |
| 程序注入描写 | `AssembledProse::assemble` 用 `vocab` 解析 ref → `text` | ≈ Assemble inject | **已对齐** |
| ID 闭集选择 | `validate_selection` 剥非法 ID | ≈ Constrain selection | **已对齐** |
| 运行时合并蒸馏库 | `load_runtime_vocab` + env skip | 供给侧闭环 | **已对齐**（runtime-closure 后） |
| 候选截断 | **字典序 / sense 序 top-k**，非语义检索 | ≠ Retrieve semantic | **缺口** |
| 标签体系 | 自由 folksonomy ~1 万 tag；facet 工具可选未强制 | 弱 query | **缺口** |
| 情节/action 层 | beat 的 **action 仍为 LLM 自由文本**，可空描写只留 action | AI 因果与 AI 腔主要泄漏点 | **缺口** |
| 输出二次校验 | 拼装后不强制「全文灵魂句 ⊆ 源」再扫（依赖入库时校验） | ≠ VeriCite 式 post-check | **缺口（轻）** |
| 字符偏移 provenance | 仅归一化子串，无 char_start/end | LangExtract 完整 provenance | **可选后期** |
| 双模型共识 / IAA | 无 | 标注信度 | **可选后期** |
| 向量库 | 无（YAGNI 阶段正确） | 语义检索可选实现 | **可选后期** |

### 对用户问题的直接含义

- **调用链路**已能「填原素材」→ 灵魂**来源机制**正确。  
- **选材仍偏字典序 + 自由 action** → **贴戏灵魂不足**，**AI 因果仍可从 action 漏出**。  
- 下一步应优先：**语义检索候选** + **压缩/模板化 action**，而不是再蒸馏更多自由 tag。

---

## 6. Practical next steps（研究结论 → 工程顺序，非本 goal 实现）

1. **Query 设计**：用 `objective_event + character.name + selected facets` 作检索 query（RELiC claim 类比）。  
2. **替换 top-k**：`candidates_for_tags_limited` 之后或之前改为 **相关度排序**（字符重合 / BM25 / 小 embedding）。  
3. **Action 预算**：限制 action 字数；禁止出现「因此/于是/不禁想到」类因果套话表（CTG 禁词思想）。  
4. **Quote density KPI**：拼装结果中 `vocab.text` 字符占比；低于阈值则拒收或重选 ID。  
5. **Post-assemble verify**：对输出中每个 ref 的 text 再 assert ⊆ vocab（防装配回归）。

---

## 7. References (compact)

- Thai et al., RELiC, ACL 2022: https://arxiv.org/abs/2203.10053  
- Literary evidence retrieval long-context, ACL 2025: https://arxiv.org/html/2506.03090v1  
- google/langextract: https://github.com/google/langextract  
- AEVS: https://www.mdpi.com/2073-431X/15/3/178 · https://github.com/yyz-nbt/AEVS  
- relic-retrieval: https://github.com/martiansideofthemoon/relic-retrieval  
- CTGS: https://github.com/Hellisotherpeople/Constrained-Text-Generation-Studio  
- CAT-LLM: https://arxiv.org/html/2401.05707v1 · https://github.com/taozhen1110/cat-llm  
- VeriCite: https://arxiv.org/html/2510.11394v1  
- CiteFix: https://arxiv.org/html/2504.15629v1  
