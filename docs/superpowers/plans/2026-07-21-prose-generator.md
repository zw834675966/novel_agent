# 正文生成器:LLM 编主线,描写从原著拉取

## 目标

把 `derive_scene` 产出的结构化 `CharacterDerivation`(每角色的感官/情绪/动作/氛围 ID + 记忆 + 剧情)编排成小说正文。

**核心防 AI 化契约**:
- LLM 只写**叙事骨架**(动作、对话、视角切换、情节推进)。
- "灵魂描写"(感官/情绪/神态/氛围)**一律通过 VocabularyId 从素材库拉取原著原文片段**,LLM 不得自造描写性文字。
- 程序校验 LLM 引用的片段 ID 必须选自该角色 derivation 的 sensations 候选集,造词一律剥离。

这是蒸馏阶段"逐字护栏"思路的延续:LLM 只做"编排",描写 100% 是曹雪芹/流潋紫原文。

## 架构

```text
derive_scene(scene_id) -> Vec<CharacterDerivation>
             |
             v
narrate_scene(scene_id)
  +-- 聚合 scene + characters + derivations
  +-- ProseGenerator.narrate(req) -> LlmNarrative(叙事节拍序列)
  +-- 校验每个 beat 的 sensation_refs ⊆ pov 角色的 derivation.sensations
  +-- assemble:按 ID 从 Vocab 拉取原文片段,按类别拼装成正文
  +-- 返回正文 String
```

## LLM 输出契约

LLM 输出一个**叙事节拍序列**,每节拍标注视角角色、纯叙事动作、引用哪些描写片段:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmNarrative {
    pub beats: Vec<NarrativeBeat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NarrativeBeat {
    /// 视角角色 UUID(必须是场景参与者)
    pub pov: String,
    /// 纯叙事:动作 + 对话。严禁感官/情绪/环境/神态描写。
    /// 错:"她悲伤地哭了" 悲伤是情绪描写
    /// 对:"她转身走向窗前,低声道:'我没事。'"
    pub action: String,
    /// 本节拍要呈现的描写片段 ID,只能从该 pov 角色的候选片段中选
    #[serde(default)]
    pub sensation_refs: Vec<String>,
}
```

走现有 rig `Extractor<LlmNarrative>` 模式,与 `LlmCharacterDerivation` 同构。

## 模块边界

```text
src/prose.rs            - ProseGenerator trait、NarrateRequest、拼装入口
src/prose/contract.rs   - LlmNarrative / NarrativeBeat(JsonSchema)
src/prose/generator.rs  - ProseGenerator trait + NarrateRequest
src/prose/assembly.rs   - 按 ID 拉取片段 + 按类别拼装正文 + ref 校验
src/prose/rig_impl.rs   - RigProseGenerator(rig Extractor,复用 DeepSeek)
src/prose/mock.rs       - MockProseGenerator(测试,返回固定 beats)
```

`ProseGenerator` 是可替换边界,与 `SenseGenerator` 对称:生产用 rig,测试用 mock,正文编排测试不依赖网络。

## 拼装规则(assembly.rs)

对每个 beat:
1. **校验 ref**:`sensation_refs` 中不在 pov 角色 `derivation.sensations` 候选集内的 ID 剥离并记 warning(复用 `validate_selection` 思路)。
2. **按 ID 拉取原文**:用 `Vocab::entries(sense).get(key).text` 取片段。
3. **按类别有序拼装**(模拟小说段落节奏):
   - `atmosphere` 片段开头铺环境
   - `action` 叙事骨架
   - `visual` / `auditory` / `olfactory` / `tactile` / `gustatory` 感官穿插
   - `emotion` 心理描写
   - `gesture` 动作神态
4. 多 beat 之间用换行分隔,形成段落。

## StoryService 集成

新增 `narrate_scene(scene_id) -> Result<String, StoryError>`:
1. 取场景 + 参与者 + 已持久化的 derivations(从 `derivation_repo` 读)。
2. 构造 `NarrateRequest{scene, characters, derivations, candidate_fragments}`。
3. 调 `ProseGenerator::narrate` 得 `LlmNarrative`。
4. 校验 + 拼装正文。
5. 返回正文。

`main.rs` 演示:`derive_scene` 后调 `narrate_scene`,打印正文。

## LLM prompt 约束(关键)

```
你是小说叙事编排器。只写叙事骨架,描写一律通过引用片段 ID 实现。

铁律:
1. action 只写客观动作和对话,严禁感官/情绪/环境/神态修饰词。
   禁止:悲伤地、愤怒地、冰冷地、香喷喷、泪流满面(这些是描写,用 sensation_refs 引用)
   允许:她起身、他推开门、黛玉说"……"
2. sensation_refs 只能从提供的候选片段 ID 中选,不得造词。
3. pov 必须是场景参与者 UUID。
4. 用多个 beat 呈现场景,每个 beat 一个视角,引用该视角角色的描写片段。

候选片段(按角色分组):
[角色A UUID]: emotion.hlm-c003-39="心中未免悔恨" | gesture.zhz-c022-326="她伸手把帕子绞了又绞" | ...
```

## 验证策略

- **单元测试**:`assembly` 按类别拼装顺序正确;非法 ref 剥离;空 refs 只输出 action。
- **单元测试**:`MockProseGenerator` 返回固定 beats,端到端 `narrate_scene` 产出含原著片段的正文。
- **单元测试**:pov 非参与者时该 beat 被拒。
- **集成验证**:`main.rs` 演示端到端,正文里可见红楼梦/甄嬛传原文片段。
- 四项质量门禁保持全绿。

## 不做

- 不做多场景连续叙事(MVP 单场景)。
- 不做正文风格微调(语气、节奏参数化),后续按需加。
- 不换 LLM provider,复用现有 rig DeepSeek 基建;`ProseGenerator` trait 留好扩展点,质量不够再加 StepFun/Claude 实现。

## 实施顺序

1. `prose/contract.rs` + `prose/generator.rs`(trait + 请求结构)。
2. `prose/assembly.rs`(拉取 + 拼装 + ref 校验)+ 单元测试。
3. `prose/mock.rs` + `prose/rig_impl.rs`。
4. `StoryService::narrate_scene` + 持久化读取。
5. `main.rs` 演示 + 端到端测试 + 门禁。
