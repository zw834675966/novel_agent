# 人物感官推导强类型接口设计

## 目标

将当前自由文本 LLM 对话改为结构化人物感官推导：输入已持久化场景与单个人物上下文，输出可反序列化、可校验、可持久化的人物感官、私有记忆与剧情发展。

系统必须满足：

- 一次调用只推导一个人物。
- 人物只能读取自己的私有记忆，不能获得客观事件的全知信息。
- 五感词汇只能引用知识库中的稳定词汇 ID，不能由 LLM 自造。
- 性格与技能只允许通过显式更新接口修改，LLM 不可改写。
- 感官和私有记忆按场景保存；人物再次出场时加载其历史上下文。

## 非目标

- 不实现向量检索或使用 `rig-lancedb`。
- 不实现 HTTP 服务、CLI 交互层或用户认证。
- 不让 LLM 自动修改人物性格、技能或客观事件。
- 不生成最终小说正文。

## 核心术语

- `Scene`：客观发生的场景事件，只由场景创建者写入。
- `CharacterMemory`：某人物实际目击、听说或推测到的内容，不等于 Scene 的完整事实。
- `VocabularyId`：词库词汇的稳定标识，例如 `visual.bloodstain`。
- `Sensation`：人物在某场景中选择出的五感词汇 ID 集合。
- `CharacterDerivation`：一次单人物推导的结构化结果。

## 架构

```text
调用方创建 Scene + 参与人物
             |
             v
derive_scene(scene_id)
             |
             +-- 对每个参与人物（最多 4 个并发）
                   |
                   +-- 加载 Character、CharacterMemory、最近 Sensation
                   +-- 从 YAML 词库筛选候选 VocabularyId
                   +-- 以强类型 schema 调用 LLM
                   +-- 校验 schema 与候选词汇范围
                   +-- SQLite 事务写入 Sensation + CharacterMemory
                   +-- 返回 CharacterDerivation 或单人物错误
```

`Scene` 是客观事实层。`CharacterMemory` 是认知层。人物上下文只使用认知层，防止全知视角。

## 类型与公开接口

所有 ID 使用 newtype，避免把不同实体 ID 混用。

```rust
pub trait StoryService {
    async fn create_scene(&self, input: CreateScene) -> Result<SceneId, StoryError>;

    async fn derive_character(
        &self,
        scene_id: SceneId,
        character_id: CharacterId,
    ) -> Result<CharacterDerivation, StoryError>;

    async fn derive_scene(
        &self,
        scene_id: SceneId,
    ) -> Vec<Result<CharacterDerivation, StoryError>>;
}

pub struct Character {
    pub id: CharacterId,
    pub name: String,
    pub personality: Vec<PersonalityTag>,
    pub skills: Vec<SkillName>,
}

pub struct Scene {
    pub id: SceneId,
    pub objective_event: String,
    pub participant_ids: Vec<CharacterId>,
    pub occurred_at: DateTime<Utc>,
}

pub struct SensorySelection {
    pub visual_ids: Vec<VocabularyId>,
    pub auditory_ids: Vec<VocabularyId>,
    pub olfactory_ids: Vec<VocabularyId>,
    pub tactile_ids: Vec<VocabularyId>,
    pub gustatory_ids: Vec<VocabularyId>,
}

pub struct CharacterDerivation {
    pub character_id: CharacterId,
    pub scene_id: SceneId,
    pub sensations: SensorySelection,
    pub new_memory: CharacterMemoryDraft,
    pub plot_development: Vec<PlotDevelopment>,
}
```

`Sensation` 不存 `current_senses` 字段。人物当前感官状态由该人物最近一条持久化感官记录派生，避免双写与状态漂移。

## LLM 输出契约

LLM 输出结构必须派生 `Serialize`、`Deserialize` 与 `JsonSchema`：

```rust
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct LlmCharacterDerivation {
    sensations: SensorySelection,
    new_memory: CharacterMemoryDraft,
    plot_development: Vec<PlotDevelopment>,
}
```

优先使用 rig 0.40 的 `Extractor<LlmCharacterDerivation>`。它以 `ToolChoice::Required` 强制 LLM 调用 `submit`，并使用类型生成参数 schema。

实现前必须完成最小可行性验证：确认 `deepseek::DEEPSEEK_V4_FLASH` 在当前 rig DeepSeek provider 下支持 Extractor 的 tool-calling 往返。若验证失败，使用 JSON schema 提示词、`serde_json` 反序列化与一次格式修复重试作为替代方案。

强类型约束保证结构、枚举和字段边界；它不保证叙事事实正确。因此系统仍需对词汇候选范围、人物归属和数据库状态进行应用层校验。

## 剧情发展契约

`PlotDevelopment` 以受限类型表示剧情变化：

```rust
pub enum PlotDevelopmentKind {
    SuspicionRaised,
    ConflictEscalated,
    GoalChanged,
    RelationshipShifted,
    NewClue,
}

pub struct PlotDevelopment {
    pub kind: PlotDevelopmentKind,
    pub reason: String,
}
```

`reason` 允许自然语言解释。感官词汇不得自由生成；剧情理由与人物记忆内容可以自然语言表达，但都必须符合强类型外形。

## 词库

词库存于本地 YAML，由人工维护，运行时加载为只读目录。

```yaml
visual:
  bloodstain:
    text: "血迹"
    tags: ["injury", "crime"]
auditory:
  footsteps:
    text: "脚步声"
    tags: ["movement", "tension"]
```

完整 ID 由感官类别与 YAML 键组成，例如 `visual.bloodstain`。每次推导由程序按场景标签和感官类别筛选候选集，并把候选 ID 注入 LLM 上下文。

LLM 只能返回候选 ID。由于候选集合按请求动态变化，静态 Rust schema 无法直接表达动态 `enum` 范围；应用层必须逐项校验输出 ID 同时满足：

- 存在于已加载词库。
- 属于本次请求的候选集。
- 所属感官类别与输出字段一致。

## SQLite 数据模型

```text
characters
  id, name, created_at, updated_at

character_personality_tags
  character_id, tag

character_skills
  character_id, skill

scenes
  id, objective_event, occurred_at

scene_participants
  scene_id, character_id

character_memories
  id, character_id, scene_id, content, source, certainty, created_at

character_sensations
  id, character_id, scene_id,
  visual_ids_json, auditory_ids_json, olfactory_ids_json,
  tactile_ids_json, gustatory_ids_json, created_at
```

`character_memories` 取代 `known_scene_ids`。一条记忆明确表达人物知道的内容、来源和确定性：

```rust
pub enum MemorySource {
    Witnessed,
    Heard,
    Inferred,
}

pub enum Certainty {
    Certain,
    Suspected,
    Uncertain,
}
```

## 单人物推导流程

1. 校验场景存在且人物是该场景参与者。
2. 加载人物性格与技能。
3. 加载该人物私有记忆，按时间倒序最多 50 条。
4. 加载该人物最近一条 `character_sensations` 作为连续感官状态。
5. 从 YAML 词库筛选候选 ID。
6. 调用 LLM，取得 `LlmCharacterDerivation`。
7. 校验所有感官 ID 属于对应候选集。
8. 单 SQLite 事务写入本场景感官记录与新人物记忆。
9. 返回 `CharacterDerivation`。

首次出场时，历史记忆和上次感官为空；五个感官数组允许为空。

## 错误与恢复

公开接口返回 `StoryError`，至少区分：

- `SceneNotFound`
- `CharacterNotFound`
- `NotSceneParticipant`
- `VocabularyLoad`
- `InvalidVocabularySelection`
- `Llm`
- `Database`

词汇校验失败时：

1. 剥离非法 ID 并记录 warning。
2. 若至少一个感官仍有有效词，继续持久化。
3. 若五感全空，带更明确的候选约束重试一次。
4. 第二次仍全空，持久化空感官并返回可观测的 `InvalidVocabularySelection` 错误结果。

每人物的感官与记忆写入必须处于同一个 SQLite 事务。任一步写入失败，整个人物结果回滚。`derive_scene` 不回滚其他人物已成功的结果。

## 并发与上下文限制

`derive_scene` 以最大 4 的有界并发执行人物推导，避免无限并行触发 DeepSeek 限流。

每人物失败隔离，调用方可获得部分成功结果。人物记忆最多注入 50 条，避免随着故事增长无限占用 token；超过限制的旧记忆不进入本次上下文。未来若需要长期检索，再引入摘要或向量检索。

## 模块边界

```text
src/models.rs   - ID、领域类型、LLM schema
src/db.rs       - SQLite schema、事务与 CRUD
src/vocab.rs    - YAML 加载、候选筛选、ID 校验
src/llm.rs      - SenseGenerator trait、rig Extractor 实现
src/scene.rs    - StoryService 编排与有界并发
src/main.rs     - 配置加载与演示入口
```

`SenseGenerator` 是可替换边界：生产实现包装 rig，测试实现返回固定的 `LlmCharacterDerivation`，使场景编排测试不依赖网络或 API key。

## 依赖变化

- 新增：`serde`、`serde_json`、`schemars`、`serde_yaml`、SQLite 驱动、`thiserror`、`chrono`、`futures`。
- 删除：未使用的 `rig-lancedb`。
- 保留：`rig`、`tokio`、`dotenv`、`anyhow`（仅二进制入口可选）。

具体 SQLite crate 在实现计划阶段选定；优先选择能支持 Tokio 场景的方案，避免在异步 LLM 调用路径阻塞运行时。

## 验证策略

- 单元测试：YAML 词库加载、候选筛选、跨感官 ID 拒绝、非法 ID 处理。
- 单元测试：人物只能加载自己的记忆；最近感官状态读取正确。
- 单元测试：感官和新记忆任一写入失败时事务回滚。
- 单元测试：并发任务中单人物失败不影响其他人物结果。
- 集成验证：DeepSeek Extractor 最小 tool-calling 往返。
- 集成测试：真实或临时 SQLite 数据库完成创建场景到持久化推导的完整流程。

## 实施顺序

1. 验证 rig + DeepSeek Extractor 可行性。
2. 建立领域类型、词库模块与词库校验测试。
3. 建立 SQLite schema、仓储层与事务测试。
4. 实现可替换 `SenseGenerator` 与 LLM 输出解析。
5. 实现场景编排、并发控制、记忆装配。
6. 完成端到端测试与 main 演示。
