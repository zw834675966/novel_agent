# 小说素材蒸馏计划(红楼梦 + 甄嬛传)

## 目标

把 `temp/` 两本小说蒸馏成**带出处的细粒度描写素材库**。之后生成小说时:LLM 只负责故事主线和叙事骨架,情感、感官、神态等"灵魂描写"一律通过 ID 从素材库拉取原文片段——LLM 无法自造描写,从源头杜绝 AI 腔。

核心机制(沿用现有引擎的词库护栏思路):素材片段 = 词库条目。每个片段有稳定 ID(如 `emotion.hlm-c023-07`)、**逐字摘自原著的文本**、标签。LLM 推导时只能返回候选 ID,`validate_selection` 剥离编造 ID——这套护栏已存在,只需把词库从 6 个手写词扩成数千个蒸馏片段。

## 类别(8 类,已确认全套)

现有五感:`visual` `auditory` `olfactory` `tactile` `gustatory`
新增三类:`emotion`(情绪心理) `gesture`(动作神态) `atmosphere`(氛围环境)

## 防 AI 化的关键设计:逐字校验

蒸馏 LLM 返回的每个片段,程序校验 `excerpt` 是源章节文本的**连续子串**(允许去空白比对)。不是子串 = 幻觉 = 丢弃。这样素材库里 100% 是曹雪芹/流潋紫的原文,LLM 只做了"分类和打标签"这一件不会引入 AI 腔的事。

## 任务

### Task 1: 语料抽取脚本 `tools/extract_corpus.py`

- 红楼梦:按 `^# 第.+回` 切分 md,跳过序言/凡例/注释尾注,输出 `corpus/hlm/c001.txt` … `c120.txt`
- 甄嬛传:解压 epub,按 `index_split_*.html` 顺序去 HTML 标签,按"第X章"聚合,输出 `corpus/zhz/cNNN.txt`
- 每章输出前 200 字打印抽查,确认无残留 HTML/注释编号

### Task 2: 蒸馏器 `src/bin/distill.rs`(复用现有 rig DeepSeek 基建)

- 新 schema `LlmFragmentBatch { fragments: Vec<Fragment> }`,`Fragment { category, excerpt, tags, note }`,派生 JsonSchema 走现有 Extractor 模式
- 每章切 ~2500 字块,逐块调用,prompt 要求"只摘录原文,不改写"
- 程序端逐字子串校验;失败片段重试一次仍失败则丢弃并计数
- 片段去重(归一化文本哈希),ID 格式 `<category>.<book>-c<chap>-<seq>`
- 输出 `assets/distilled/hlm.yaml`、`assets/distilled/zhz.yaml`,结构与现有 vocab.yaml 一致(text = 原文摘录,tags 含书名/回目/人物)

### Task 3: 引擎扩展支持 8 类

- `VocabFile` 增加 emotion/gesture/atmosphere 三个 map;`Vocab` 支持加载多个 YAML 合并
- `SensorySelection` 增加 `emotion_ids/gesture_ids/atmosphere_ids`(schema 同步)
- `validate_selection` 增加三类前缀过滤
- `character_sensations` 表增加三列 JSON(novels.db 是演示库,直接改 schema.rs 重建,不写迁移)
- 全部测试补齐,四项质量门禁保持全绿

### Task 4: 试点运行(已确认先试点)

- 每本书前 5 回跑通蒸馏,产出样本 YAML
- 输出统计:片段数/类别分布/校验丢弃率
- **停下来交给用户抽检**片段质量和标签准确性,确认后才进 Task 5

### Task 5: 全量蒸馏(用户确认试点质量后)

- 全量 ~240 章,断点续跑(已完成章节跳过)
- 预估 ~1000-1500 次 DeepSeek 调用
- 最终合并、统计报告、质量门禁全绿

## 不做

- 不做正文生成器(那是下一阶段,本计划只建素材库)
- 不做向量检索;候选筛选沿用现有 tags 过滤
- 不改 LLM Provider

## 成本与风险

- 试点 ~50 次调用(约 ¥1);全量预估 ¥20-40、2-3 小时
- epub 章节边界若不规整,Task 1 用目录 spine 顺序兜底
- 红楼梦校注本可能有注释残留,抽查阶段处理
