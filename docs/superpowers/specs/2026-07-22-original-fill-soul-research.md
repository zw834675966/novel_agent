# 原素材填充写作：论文与 GitHub 主流做法调研

> 目标：通过**原素材填充**让 AI 写作有灵魂，杜绝 **AI 味**与 **AI 因果逻辑**。  
> 日期：2026-07-22 · 方法：Exa + GitHub 检索 · 证据：`goal/research-sources.md`（会话 scratch，未入库；URL 见附录 A/B）

---

## 0. 核心结论（对应本调研目的）

**正确主路径不是「学风格再写」，而是：**

```
原著抽取并校验  ->  检索 / 闭集选 ID  ->  程序拼装注入原文  ->  再校验
```

| 目标 | 做法 | 对应机制 |
|------|------|----------|
| **有灵魂** | 高信息句 = 可溯源原著 quote；看 **quote density + provenance** 反 AI 味 | 灵魂句由检索选择，不由模型生成 |
| **灵魂句** | **禁止 paraphrase**；程序注入 `vocab.text` | ID 闭集 + 装配期粘贴 |
| **反 AI 因果** | 因果只在**短骨架 / action**；描写层不写通顺泛因果 | 骨架与描写分层；自由文本套话剥离 |

**论文线（摘要）：** RELiC / 长上下文 literary evidence retrieval（灵魂句 = 检索选择）· LangExtract（程序对齐 span，不信任模型 offset）· AEVS（锚点 -> 抽取 -> 校验 -> 补抽）· VeriCite / CiteFix（生成后核对「句 -> 源」）· CAT-LLM / 纯风格迁移（**不要当主路径**，会重写 = 重新 AI 味）。

**GitHub 主流：** `google/langextract` · `relic-retrieval` · `Constrained-Text-Generation-Studio` · `AEVS`；生产 RAG「cite / refuse if no evidence」（映射为：无合法 ID 则 strip/retry）。

**与 novels 的差距（关键）：**

| 已对齐 | 仍是缺口 |
|--------|----------|
| 逐字蒸馏 + 子串校验 | 候选：**BM25 + 声口加权**（见 §5a）；embedding 仍缺 |
| ID 闭集 + 程序注入 text | action 仍是 LLM 自由文本（**P0 已加护栏**；未模板化） |
| 运行时已 merge distilled | 装配后回源重扫 + `ProseQualityReport` **已落地**；低 density 拒收仍缺 |

> **一句话：** 机制方向对（填充原素材）；要「更有灵魂、更少 AI 因果」，下一刀应是**语义检索候选 + 压短/模板化 action**，而不是再堆风格迁移。
>
> **落地更新（2026-07-22）：** P0（词面排序 + action 护栏 + density）已由 `anti-ai-retrieve-action-p0` 落地；**下一阶段**已落地 **BM25 候选排序** + **装配后 quote 回源重扫** + `ProseQualityReport`。仍未做：embedding dense、低 density 拒收闭环、偏移 provenance / IAA。详见 §5a。

---

## 1. 问题界定（可观测）

| 用户说法 | 可操作定义（本调研采用） |
|----------|--------------------------|
| **有灵魂** | 高密度、可溯源的**原著级描写**进入正文；感官/神态/氛围不以模型“通顺改写”为主 |
| **AI 味** | 平滑空洞修辞、万能形容词、无出处的心理/环境描写 |
| **AI 因果逻辑** | 模型自造情节动机/因果链，而非用素材+骨架约束叙事 |

**核心技术命题（与“风格模仿”对立）：**

> **LLM 负责因果骨架与衔接；“有灵魂”的句子必须来自可验证的原素材 span，程序注入，禁止对 span 改写。**

这对应业界的 **grounded / constrained / evidence-first generation**，而不是纯 **style transfer**。

---

## 2. 论文线（≥5，附 takeaway）

| # | 文献 / 线 | 要点 | 对本目标：抄 / 不抄 |
|---|-----------|------|---------------------|
| 1 | **RELiC** (Thai et al., ACL 2022) — *Retrieving Evidence for Literary Claims* | 文学分析上下文 → 从全书候选中**检索应引用的原句**；难点是少词面重叠、需叙事理解 | **抄**：把“写得好”定义成**引用检索**而非生成。**不抄**：不要只优化 embedding 相似度，文学证据常不靠字面相似 |
| 2 | **Literary Evidence Retrieval via Long-Context LMs** (Thai & Iyyer, 2025) | 全书上下文 + 缺失引文任务；强模型仍 overgenerate、细节文学信号弱 | **抄**：评估“能否找回正确原句”。**不抄**：把全书塞进上下文当唯一方案（成本高、仍会幻觉） |
| 3 | **RAG + grounded citation / refuse**（工程与综述共识，2024–2026） | Context highlighting、强制引用、检索失败则拒绝、Groundedness 评分 | **抄**：**无证据不写美句**。**不抄**：允许“根据常识补全”的开放生成 |
| 4 | **CRAG / Self-RAG / multi-stage guardrails** | 检索质量评估、弱证据换源或拒答、自检是否受支持 | **抄**：检索不足时**降级骨架-only 或拒写描写**。**不抄**：单遍生成后无校验 |
| 5 | **LangExtract 类 span grounding**（Google，2025 起） | 抽取后 **post-hoc alignment** 回源字符偏移；`char_interval=None` 丢弃 | **抄**：素材库条目必须程序可回源。**不抄**：相信模型自称“原文摘录” |
| 5a | **AEVS** Anchor–Extraction–Verification–Supplement（[MDPI Computers 2026](https://www.mdpi.com/2073-431X/15/3/178) · [repo](https://github.com/yyz-nbt/AEVS)） | 先**闭集锚点** -> 再抽取 -> 还原校验 -> 覆盖率**补抽**；幻觉可检测 | **抄**：provenance 流程（锚点 -> 校验 -> 补抽）。**不抄**：把 KG 三元组格式硬套进五感词库 |
| 5b | **VeriCite / CiteFix**（RAG 引用可靠性，[VeriCite](https://arxiv.org/html/2510.11394v1) · [CiteFix](https://arxiv.org/html/2504.15629v1)） | 生成后**核对引用是否真支撑句子**；错误引用毁信任 | **抄**：post-gen「句 ↔ 源」核对。**不抄**：小说场景只贴脚注 URL，应对**注入片段 ⊆ 词库 text** |
| 6 | **Constrained text generation for LLMs** (e.g. arXiv 2310.16343) | 词汇/结构/关系约束；开源模型常“说遵守但未遵守” | **抄**：**程序侧强制**约束（ID 白名单、模板槽位）。**不抄**：只靠 prompt “必须用这些词” |
| 7 | **AutoTemplate**（lexical constraints via template then fill） | 先出带槽模板，再填约束词 | **抄**：**骨架 → 填槽**与“原素材填充”同构。**不抄**：槽内允许同义改写灵魂句 |
| 8 | **ZeroStylus / CAT-LLM**（中文长文风格模板） | 层次模板/风格定义引导改写 | **抄**：段落结构模板可借鉴。**不抄**作为主路径——**改写**会制造新 AI 味，与“原句填充”冲突 |

**反模式汇总：** “像曹雪芹一样写”式 style transfer、无引用开放生成、仅靠 temperature/prompt 压幻觉。

---

## 3. GitHub / 开源主流做法（≥4）

| # | 项目 | 做法 | 抄 / 不抄 |
|---|------|------|-----------|
| 1 | [google/langextract](https://github.com/google/langextract) | LLM 抽取 + **程序对齐**字符 span；无 span 丢弃 | **抄**进蒸馏/校验；**不抄**用它直接写小说 |
| 2 | [martiansideofthemoon/relic-retrieval](https://github.com/martiansideofthemoon/relic-retrieval) | 文学证据 **dense retrieval** 基准与代码 | **抄**“按主张检索原句”评估；**不抄**默认 BM25-only |
| 3 | [Hellisotherpeople/Constrained-Text-Generation-Studio](https://github.com/Hellisotherpeople/Constrained-Text-Generation-Studio) | 约束解码/约束写作实验台 | **抄**硬约束思想；**不抄**诗歌 lipogram 玩法当主叙事 |
| 3a | [yyz-nbt/AEVS](https://github.com/yyz-nbt/AEVS) | anchor 闭集 + verification 流水线 | **抄**进蒸馏/校验流程；**不抄**完整 KG 栈 |
| 4 | [taozhen1110/cat-llm](https://github.com/taozhen1110/cat-llm) | 中文长文风格定义 + LLM 迁移 | **抄**风格特征分析作**检索 facet**；**不抄**全文风格改写主路径 |
| 5 | 生产 RAG 实践（LangChain/社区 CRAG·Self-RAG 文档流） | cite-only、拒答、分阶段护栏 | **抄**拒答与引用强制；**不抄**把知识库 FAQ 当文学素材 |

**主流收敛（2024–2026）：**

```
Retrieve (evidence) → Constrain (whitelist / slots) → Generate skeleton → Assemble quotes programmatically → Verify groundedness
```

**不是：**

```
Generate full prose in style-of-X → hope it sounds human
```

---

## 4. 推荐技术栈排序（retrieve → constrain → assemble → verify）

| 优先级 | 层 | 做什么 | 对“灵魂” | 对“反 AI 逻辑” |
|--------|----|--------|----------|----------------|
| **P0** | **Assemble 程序注入** | 正文描写槽只粘贴 `VocabularyId → 原文 text`，禁止改写 | 灵魂句=原著句 | 模型不能用“通顺描写”偷因果 |
| **P0** | **Constrain ID 白名单** | 仅候选集 ID 可写入；非法剥离；全空重试/失败 | 防编造 ID | 防编造描写入口 |
| **P1** | **Retrieve 语义/叙事相关** | 场景事件+角色+tag → 相关片段（dense/BM25/hybrid），**禁止纯字典序 top-k** | 贴戏才像“有灵魂” | 减少胡乱引用导致的伪因果 |
| **P1** | **Skeleton 因果外置** | 情节/动作 beat 结构化（plot/action），与描写分离 | 骨架可平淡 | **因果由结构+记忆表驱动**，不由修辞驱动 |
| **P2** | **Verify** | 输出描写必须 ⊆ 源章子串；可加 groundedness / 引用密度 KPI | 可审计“灵魂含量” | 拒写无证据美句 |
| **P3** | **素材生产** | 蒸馏 = 定位+分类+逐字校验（LangExtract 式）；清理对话句/自由 tag | 库更纯 | 少噪声干扰检索 |
| **Avoid as primary** | Style transfer | CAT-LLM / ZeroStylus 式全改写 | 易出新 AI 味 | 模型重写因果与修辞 |

**“灵魂”操作定义（建议 KPI）：**

- **Quote density**：描写字符中，来自原素材的比例（目标高，如 ≥70% 感官/emotion/gesture 行）  
- **Verbatim rate**：注入串 100% 回源  
- **Ungrounded lyric rate**：无 ID 的“文学描写”句 → 0（允许的只有 action/plot 骨架）

**“反 AI 因果”操作定义：**

- 情节推进只来自 `plot_development` / 场景 objective / 记忆，不来自模型即兴抒情  
- action 可 LLM；**解释动机的美句若无 ID 则剥离**

---

## 5. 与 novels 现状的差距表

| 能力 | novels 现状（2026-07 合并 runtime-closure 后） | 与推荐栈 | 标签 |
|------|-----------------------------------------------|----------|------|
| 原著 span 入库 | `distill.rs` 空白归一子串校验 + validate 工具 | 对齐 LangExtract **程序回源**思想 | **已对齐** |
| 只选 ID 不写描写 | `SenseGenerator` + `validate_selection` | 对齐 constrain whitelist | **已对齐** |
| 程序拼装正文 | `prose::assemble` 按 ref 取 `vocab.text` | 对齐 assemble fill | **已对齐** |
| 运行时加载蒸馏库 | `load_runtime_vocab` 默认 merge `assets/distilled` | 素材可进主路径 | **已对齐** |
| 候选截断 | `candidates_ranked_limited`：**BM25** + selected/声口 boost；tags `known_tags_ranked_limited` | 已脱字典序与纯布尔打分；embedding 仍缺 | **BM25 已落地** |
| 检索 | 候选池 BM25（整词子串 TF+IDF）+ 角色名 tag 强加权 + cap | 零依赖 IR；dense 仍缺 | **BM25 已落地** |
| 拒写无证据描写 | 全空感官可报错；action 经 `sanitize_action` 剥套话 + 限长 80；memory/plot `text_guard` 限长 | action AI 因果粘合已压；**未 ID 化/模板化** | **P0 已加护栏** |
| 输出 groundedness 指标 | `quote_density()` + `unverified_quotes` 回源重扫 + `ProseQualityReport`（含 low_quote_density） | KPI + 回扫已有；低 density 拒收仍缺 | **已落地** |
| 素材清洗 | 工具有 `--drop-dialogue` / facet；默认未全库 prune | 生产可选 | **可选** |
| 风格迁移 | 未做（正确） | 不应作主路径 | **刻意不做** |
| 向量库 | 未做 | P1 可用 hybrid，非必须先上 | **可选** |

**一句话：** novels **已经走在正确的“原素材填充”主航道上**；当前短板不是“要不要像 AI 学文风”，而是 **检索贴戏 + 骨架/描写分离更严 + 可量化的灵魂/接地指标**。

---

### 5a. 研究结论 -> 代码落地状态（2026-07-22 校准）

> 本节把上方「缺口」与 §6 的「下一实现」对照**当前代码实况**，避免研究结论停留在机制层。
> 计划：`docs/superpowers/plans/2026-07-22-anti-ai-retrieve-action-p0.md`（Task 1–3 全部 `[x]`）。

| 研究建议（§6 / 批判 P0–P1） | 落地实现 | 状态 | 仍未做 |
|------------------------------|----------|------|--------|
| 语义相关候选（替字典序 cap） | `candidates_ranked_limited`：候选池上 **BM25**（整词子串 TF + IDF）+ selected/声口 boost；确定性排序 | ✅ **P0+BM25** | embedding dense 检索（显式 defer） |
| tag 短名单按场景/角色排序 | `known_tags_ranked_limited`：query_terms 排序后再 cap 80 | ✅ **P0 落地** | 受控词表（仍是 folksonomy） |
| action 套话禁词 + 长度预算 | `AssembledProse::sanitize_action` -> `text_guard::sanitize_free_text`（剥 `因此/于是/不禁…` + `MAX_ACTION_CHARS=80`） | ✅ **P0 落地** | action 模板化 / ID 化（仍是自由文本） |
| memory / plot 自由文本护栏 | `text_guard`：`MAX_MEMORY_CHARS=120`、`MAX_PLOT_REASON_CHARS=60` + 同款套话剥离 | 🟡 **P1 部分** | 未结构化/ID 化；仅限长 + 剥套话 |
| quote density KPI | `quote_density()` + `ProseQualityReport` + main 日志（含 low_quote_density 标志） | ✅ **P0 落地** | 阈值拒收 / 重选 ID 闭环 |
| 声口隔离 | 候选打分对**角色名 tag 精确命中**强加权（BM25 之上） | 🟡 **P1 部分** | 跨 beat 声口一致性校验 |
| post-assemble 全文回源扫描 | assemble 后对每条注入 quote 做 `text.contains` 重扫 → `unverified_quotes` | ✅ **已落地** | 低 density 自动 strip/retry |
| 偏移 provenance / IAA / 稀疏感官补抽 | 仅空白归一子串；无 char offset / 无 gold / 无共识 | ❌ **未做** | 见 `novels-distill-gap-analysis` G1/G6/G5 |
| 风格迁移（主路径） | 未做 | ✅ **刻意不做**（正确） | - |

**净结论：** BM25 检索 + 装配后回源重扫 + `ProseQualityReport` 已落地；**embedding dense 检索**与 **低 density 拒收闭环**仍待。批判 D1–D9 中 D2/D6/D7 部分关闭，其余（D1 灵魂定义 / D3 query 自研 / D4 硬拼节奏 / D5 中文语域 / D8–D9）仍开放，见 [[novels-conclusion-critique]]。

---

## 6. 给 novels 的落地优先级（研究结论 → 下一实现）

1. **语义相关候选**（替换纯字典序 cap）：query = 场景 objective + 角色名 + selected tags；检索 distilled `text`/tags；再 cap。  
2. **描写禁改写契约**：prose prompt 明确 action-only 可生成；`sensation_refs` 文本仅程序插入。  
3. **无证据则空描写**：不允许模型在 refs 外写环境/心理句。  
4. **KPI**：quote density、ungrounded lyric rate、verbatim 抽检纳入 `distill_quality_report` / 运行日志。  
5. **库治理**：对主库 dry-run `normalize_tags` + 抽样 `--drop-dialogue`，确认后再 prune。  
6. **勿优先**：全文 style transfer、开放“文青模式”生成。

---

## 7. 结论

| 问题 | 答案 |
|------|------|
| 论文/GitHub 主流如何“有灵魂”？ | **引用并填充原作证据**，不是模仿文风生成 |
| 如何杜绝 AI 味？ | 灵魂句 **不可由模型撰写**；只可选择与粘贴 |
| 如何杜绝 AI 因果？ | 因果放在 **plot/action 结构**；描写不得承担解释职责 |
| novels 方向对吗？ | **对**；补检索与更严组装即可逼近最佳实践 |
| 「下一刀」落地了吗？ | **P0 + BM25 + 装配后回源**已落地（见 §5a）；embedding dense 与低 density 拒收闭环仍待 |

---

## 附录 A — 检索证据

会话 scratch `goal/research-sources.md`（查询列表与 URL）为临时文件，**未入库**；可回溯的 URL 已沉淀在本节附录 B 与 §2/§3 表格内。

## 附录 B — 关键链接速查

- RELiC paper: https://aclanthology.org/2022.acl-long.517/ · arXiv: https://arxiv.org/abs/2203.10053  
- RELiC site: https://relic.cs.umass.edu/  
- Literary evidence retrieval (long-context, ACL 2025): https://arxiv.org/html/2506.03090v1  
- LangExtract: https://github.com/google/langextract  
- AEVS: https://www.mdpi.com/2073-431X/15/3/178 · https://github.com/yyz-nbt/AEVS  
- VeriCite: https://arxiv.org/html/2510.11394v1 · CiteFix: https://arxiv.org/html/2504.15629v1  
- relic-retrieval: https://github.com/martiansideofthemoon/relic-retrieval  
- CAT-LLM: https://github.com/taozhen1110/cat-llm · arXiv: https://arxiv.org/html/2401.05707v1  
- CTGS: https://github.com/Hellisotherpeople/Constrained-Text-Generation-Studio  
- 落地计划：`docs/superpowers/plans/2026-07-22-anti-ai-retrieve-action-p0.md`
