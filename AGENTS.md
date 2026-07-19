# Repository Guide（仓库指南）

## Scope（项目范围）

- This is one Rust 2024 binary crate, not a workspace. The only application entrypoint is `src/main.rs`.
  （这是一个独立的 Rust 2024 二进制 crate，不是工作空间。唯一的应用入口是 `src/main.rs`。）
- `rig-lancedb` is declared but not used yet. Do not infer a vector-store architecture from the dependency alone.
  （`rig-lancedb` 已声明但尚未使用。不要仅从依赖推断出向量存储架构。）

## Commands（命令）

- Fast compile check（快速编译检查）: `cargo check --all-targets`
- Full test command（完整测试）: `cargo test --all-targets`
- Formatting check（格式检查）: `cargo fmt --all -- --check`
- Lint（代码检查）: `cargo clippy --all-targets --all-features -- -D warnings`
- Run the current agent（运行当前 agent）: Set `DEEPSEEK_API_KEY` in `.env` or shell, then run `cargo run`.
- No test targets exist yet; focused tests use Cargo's normal filter form: `cargo test <test-name>`.
  （暂无测试目标；针对性的测试使用 Cargo 的标准过滤方式。）

## Runtime Wiring（运行时依赖）

- Uses `rig::providers::deepseek` (not OpenAI compat layer).
  （使用 rig 内置的 DeepSeek provider，而非 OpenAI 兼容层。）
- `src/main.rs` loads `.env` file via `dotenv::dotenv().ok()`.
  （代码会通过 dotenv 自动加载 .env 文件。）
- Reads `DEEPSEEK_API_KEY` from environment.
  （从环境读取 DEEPSEEK_API_KEY。）
- Configured model: `deepseek::DEEPSEEK_V4_FLASH` (constant in rig crate).
  （配置模型：deepseek::DEEPSEEK_V4_FLASH。）
- Falls back to `MockSenseGenerator` when API key is absent (prints warning, no crash).
  （无 API Key 时静默降级为 Mock 生成器，不崩溃。）
- DB path: `novels.db` (SQLite, auto-created on first run).
  （数据库路径：novels.db，SQLite，首次运行自动创建。）

## Architecture（架构）

```
Project: novels — AI-driven novel character perception engine
（AI 驱动的小说角色感知引擎）

src/
├── lib.rs          # 库根，导出所有模块
├── main.rs         # 二进制入口：初始化 → 演示流程
├── llm/            # LLM 推理抽象层
│   ├── mod.rs      # 公开 trait SenseGenerator + 两套实现
│   ├── contract.rs # LLM 输出契约 LlmCharacterDerivation (JsonSchema)
│   ├── generator.rs # DerivationRequest 上下文 + SenseGenerator trait
│   ├── rig_impl.rs  # 生产实现：rig + DeepSeek
│   └── mock.rs      # 测试实现：固定返回值
├── models/         # 数据模型（纯结构体，无业务逻辑）
│   ├── mod.rs      # 重新导出所有公共类型
│   ├── ids.rs      # 类型安全 ID newtype（CharacterId/SceneId/MemoryId/VocabularyId）
│   ├── error.rs    # StoryError 枚举（thiserror）
│   ├── character.rs # Character
│   ├── derivation.rs # CharacterDerivation
│   ├── memory.rs   # CharacterMemory / CharacterMemoryDraft
│   ├── memory_source.rs # MemorySource / Certainty 枚举
│   ├── plot.rs     # PlotDevelopment / PlotDevelopmentKind
│   ├── scene.rs    # Scene / CreateScene
│   └── sensation.rs # SensorySelection（五感）
├── db/             # 数据库层（SQLite + Repository 模式）
│   ├── mod.rs      # Db 句柄 + 工厂方法
│   ├── schema.rs   # DDL + migrate()
│   ├── character_repo.rs  # 角色 CRUD（含标签子表）
│   ├── scene_repo.rs      # 场景 CRUD（含参与者关联）
│   ├── memory_repo.rs     # 记忆查询/插入
│   ├── sensation_repo.rs  # 五感查询/插入
│   └── derivation_repo.rs # 跨表事务写入（感官+记忆原子操作）
├── scene/          # 业务服务层
│   ├── mod.rs      # pub use StoryService
│   └── service.rs  # 场景创建/角色推导/批量推导
└── vocab/          # 感官词库层
    ├── mod.rs      # pub use Vocab + validate
    ├── loader.rs   # YAML 加载 + 候选集生成
    └── validate.rs # LLM 输出校验（过滤非法词汇）

tests/
├── db_test.rs      # DB 层集成测试（内存 SQLite）
├── e2e.rs          # 端到端测试（Mock LLM）
├── models_test.rs  # VocabularyId 单元测试
├── scene_test.rs   # StoryService 单元测试
└── vocab_test.rs   # 词库加载/校验测试

assets/
└── vocab.yaml      # 五感词汇定义（visual/auditory/olfactory/tactile/gustatory）
```

## Design Patterns（设计模式）

### 1. Trait 抽象 + 两套实现（LLM 层）

```
SenseGenerator (trait)
├── RigSenseGenerator  → 生产：rig + deepseek::Client + Extractor
└── MockSenseGenerator → 测试：返回固定 LlmCharacterDerivation
```

通过 `Arc<dyn SenseGenerator>` 注入到 StoryService，松耦合。

### 2. Repository 模式（DB 层）

每个实体有独立 Repo struct，通过 `Db` 的工厂方法获取：
- `db.characters()` → `&CharacterRepo`
- `db.scenes()` → `&SceneRepo`
- `db.memories()` → `&MemoryRepo`
- `db.sensations()` → `&SensationRepo`
- `db.derivations()` → `&DerivationRepo`

### 3. 事务原子性（DerivationRepo）

`insert_derivation()` 在一次事务中写入感官（sensations）+ 记忆（memories）两张表。
任一失败自动回滚，不会出现"有感官没记忆"的不一致状态。

### 4. LLM 输出安全护栏

```
LLM 输出 → validate_selection() → 过滤非法词汇
                                 → 全空时自动重试一次
                                 → 重试仍全空时返回错误
```

### 5. Newtype ID

```rust
pub struct CharacterId(pub Uuid);  // 类型安全，防止 ID 混淆
pub struct VocabularyId(String);   // 校验格式 "sense.key"
```

## Data Flow（核心数据流）

```
create_scene(CreateScene)
  → 生成 SceneId → 写入 scenes 表 + scene_participants 表
  → 返回 SceneId

derive_character(scene_id, character_id)
  → 校验场景/角色存在性、参与关系
  → 加载角色最近的 50 条记忆
  → 加载角色最近一次感官（用于连续性）
  → 从 vocab 生成候选集
  → 构造 DerivationRequest
  → SenseGenerator::derive() 调用 LLM
  → validate_selection() 校验 + 重试
  → insert_derivation() 原子写入感官 + 记忆
  → 返回 CharacterDerivation

derive_scene(scene_id)
  → 加载场景的所有参与者
  → 并发（上限 4 路）调用 derive_character
  → 返回 Vec<Result<CharacterDerivation, StoryError>>
```

## Database Schema（数据库表结构）

```
characters (id, name, created_at, updated_at)
character_personality_tags (character_id, tag)
character_skills (character_id, skill)
scenes (id, objective_event, occurred_at)
scene_participants (scene_id, character_id)
character_memories (id, character_id, scene_id, content, source, certainty, created_at)
  INDEX: (character_id, created_at DESC)
character_sensations (id, character_id, scene_id, visual_ids_json, ..., created_at)
  INDEX: (character_id, created_at DESC)
```

All FKs use `ON DELETE CASCADE`. IDs/timestamps stored as TEXT (UUID / ISO 8601).
所有外键级联删除。ID 和时间戳以 TEXT 存储。

## Key Vocabulary YAML Format（词库格式）

```yaml
visual:                    # 感官类别
  bloodstain:              # 词汇键（VocabularyId = "visual.bloodstain"）
    text: "血迹"           # 显示文本
    tags: ["injury"]       # 标签（用于过滤）
```

Five categories (五类): visual, auditory, olfactory, tactile, gustatory.

## Test Strategy（测试策略）

- All DB tests use `Db::open_in_memory()` — independent, fast, no cleanup needed.
- LLM tests use `MockSenseGenerator` — no API key or network required.
- Run all tests: `cargo test --all-targets`
- Tests directory corresponds to modules: db_test ↔ db/, scene_test ↔ scene/, etc.

## Known Baseline Issues（已知基线问题）

- `cargo check --all-targets` / `cargo test --all-targets` fail in Lance 7.0.0 dependency build scripts on Windows before compiling this crate. Do not attribute that failure to a new change without comparing the error. If `rig-lancedb` is removed, builds may work.
  （Windows 上编译 Lance 7.0.0 依赖时构建脚本会失败。去除 rig-lancedb 依赖后可解决。）
- Repository has no commits yet; all current project files are untracked. Treat existing files as user work and avoid cleanup/reversion.
  （仓库还没有任何提交；所有当前项目文件都是未跟踪状态。）
- `.env` is gitignored but still risky. Never commit secrets.
  （.env 已在 gitignore 中，但仍需注意不要提交密钥。）

## Environment Setup（Windows 环境）

- `protoc` required for lance-* crates. Install at `C:\Tools\protoc\bin\protoc.exe`, set `$env:PROTOC = "C:\Tools\protoc\bin\protoc.exe"`.
  （lance 相关 crate 需要 protoc。安装到指定路径并设置环境变量。）
- If not using lancedb features, removing `rig-lancedb` from Cargo.toml eliminates the protoc requirement.
  （如不使用 lance 功能，移除 rig-lancedb 依赖可避免 protoc 要求。）
