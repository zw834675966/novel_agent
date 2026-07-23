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
- Vocabulary bootstrap: load `assets/vocab.yaml`, then merge `assets/distilled/` when that directory exists.
  （词库引导：先加载 `assets/vocab.yaml`，若存在 `assets/distilled/` 则合并。）

### Distilled vocabulary (optional)（可选蒸馏词库）

- Default runtime: load `assets/vocab.yaml` then merge `assets/distilled/` if present.
  （默认运行时：加载 base 词库，若存在则合并 `assets/distilled/`。）
- Skip distilled merge: `NOVELS_SKIP_DISTILLED=1` (or `true`).
  （跳过蒸馏合并：`NOVELS_SKIP_DISTILLED=1`。）
- Override distilled dir: `NOVELS_DISTILLED_DIR=path` (must be an existing directory).
  （覆盖蒸馏目录：`NOVELS_DISTILLED_DIR=path`，须为已存在目录。）
- Candidate caps: 24 per sense / 96 total; tags sent to LLM capped at 80.
  （候选硬顶：每感官类最多 24、总计最多 96；进入 LLM 的 tags 最多 80。）
- Ranking (anti-AI retrieve): `candidates_ranked_limited` uses **BM25** over candidate `text`+tags (whole-term substring TF, IDF, k1=1.2/b=0.75) plus selected-tag and exact name-tag voice boosts; deterministic score DESC → SENSES → id. Plan: P0 `docs/superpowers/plans/2026-07-22-anti-ai-retrieve-action-p0.md`; BM25+provenance follow-up in research §5a.
  （排序：候选池 BM25 + 标签/声口加权，非纯字典序或布尔包含。）
- Free-text guard (`text_guard`): causal-filler strip + length caps for **action** (80), **memory** (120), **plot reason** (60).
  （自由文本护栏：action/记忆/情节 reason 剥套话并限长。）
- Assemble verify: `AssembledProse` post-checks each injected quote appears in final `text` (`unverified_quotes`); `ProseQualityReport` aggregates quote_density / action_only_rate / stripped_ref_rate / low_quote_density (`MIN_QUOTE_DENSITY=0.30`, flag only — no hard fail). `ProseQualityReport` also tracks `sensory_diversity_score` / `missing_senses` / `degraded_sensory_density` (flag when zero five-sense coverage). When degraded, `narrate_scene` builds the fallback inject pool from full ranked vocab candidates (five primary senses) — NOT from LLM-selected derivation IDs, or the inject path is dead code.
- **Sensory quota retrieval**: `candidates_ranked_limited_with_quotas` enforces weak-sense floors (auditory/olfactory/tactile/gustatory >=5, `WEAK_SENSE_FLOOR=5`), gesture/emotion caps (`GESTURE_EMOTION_CAP=15`), scene focus weighting. Total <=96.
- **Prompt rendering**: `MemoryContentSlot::display_narrative()` renders memories with Chinese narrative labels; `build_derivation_prompt` has no Rust Debug `{:?}` syntax.
- **CoT**: `LlmCharacterDerivation.sensory_analysis` field guides LLM to analyze sensory focus before selecting IDs
  （装配后回源重扫 + 质量报告；低 density 仅标志不报错。）
- Assemble rhythm + book-source isolation (platform reverse N1/N2): join injected quotes with `，`/`。` (no bare short-lemma paste); within a beat, conflicting `hlm`/`zhz` sources keep majority (base lemmas without source always kept). Desc↔action joined with `。` when needed.
  （拼装节奏 + 书源隔离：防清单感与跨书串味。）
- Tag shortlist: `known_tags_ranked_limited` ranks by scene/character query before cap 80; candidate score boosts exact name tags (voice isolation).
  （tag 短名单按场景/角色相关排序；候选对角色名 tag 强加权以减轻声口串味。）
- Quality report: `python tools/distill_quality_report.py` (includes sensory bucket sampling warnings for gesture/emotion overweight and weak-sense underrepresentation)
  （质量报告：`python tools/distill_quality_report.py`，含感官桶采样比例校验。）
- Never commit secrets; treat `corpus/` and `assets/distilled/` as local copyrighted material (do not commit unless explicitly approved).
  （勿提交密钥；`corpus/` 与 `assets/distilled/` 视为本地版权素材，未经明确批准勿提交。）

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
    ├── mod.rs      # pub use Vocab + validate + load_runtime_vocab
    ├── bootstrap.rs # 运行时：base + 可选 distilled 目录合并
    ├── loader.rs   # YAML 加载 + 候选集生成（含 top-k caps）
    └── validate.rs # LLM 输出校验（过滤非法词汇）

tests/
├── db_test.rs      # DB 层集成测试（内存 SQLite）
├── e2e.rs          # 端到端测试（Mock LLM）
├── models_test.rs  # VocabularyId 单元测试
├── scene_test.rs   # StoryService 单元测试
└── vocab_test.rs   # 词库加载/校验/bootstrap 测试

assets/
├── vocab.yaml      # 五感词汇定义（visual/auditory/olfactory/tactile/gustatory）
└── distilled/      # 可选本地蒸馏词库目录（默认运行时合并，若存在）
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

## AI Maintenance Playbook（AI 维护手册）

### Start Here（开始前）

Before changing code:

1. Read this file, then inspect the owning module and its matching integration test.
2. Run `git status --short`; treat every unrelated modified or untracked path as user work. Do not revert, stage, delete, or reformat it.
3. Confirm whether the requested behavior is already specified by a model type, repository method, service flow, LLM contract, vocabulary fixture, or test.
4. Keep changes within the narrowest owning layer. Cross a layer only when the public contract requires it.

Authoritative sources, in priority order:

- `AGENTS.md` defines repository constraints, architecture, runtime wiring, and known baseline failures.
- `src/` defines current runtime behavior.
- `tests/` defines observable regression contracts.
- `Cargo.toml` defines supported dependencies and binary targets.
- `assets/vocab.yaml` is the committed base vocabulary. Optional corpus or distillation material, if present, is local user work rather than baseline repository behavior.

### Task Playbooks（任务手册）

#### Domain Models and IDs（领域模型与 ID）

Owns: `src/models/` and `src/models/mod.rs`.

- Put pure domain structures, enums, and typed IDs in `src/models/`; keep persistence and orchestration out of model files.
- Use `CharacterId`, `SceneId`, `MemoryId`, and `VocabularyId` instead of raw IDs at public boundaries.
- Preserve `VocabularyId` format `sense.key`; validate vocabulary identity before persistence or selection.
- When a model changes, update affected repository serialization, service construction, LLM contract conversion, and focused tests in `tests/models_test.rs`, `tests/db_test.rs`, `tests/scene_test.rs`, or `tests/vocab_test.rs`.
- Checks: Run `cargo test --test models_test` for typed-ID changes; run affected repository, scene, or vocabulary tests when a model change crosses those existing contracts.
- Boundary: Do not expand model-only work into repository, service, LLM, or vocabulary changes unless an explicit user request requires the affected contract; preserve typed IDs and `VocabularyId` validation.

#### Database and Repositories（数据库与仓储）

Owns: `src/db/schema.rs`, `src/db/*_repo.rs`, and `src/db/mod.rs`.

- Add schema changes through `migrate()`; preserve foreign keys and existing `ON DELETE CASCADE` behavior.
- Keep entity-specific queries in their repository factory returned by `Db`.
- Use `DerivationRepo::insert_derivation()` for sensation plus memory writes that must remain atomic. Do not split its transaction into independent writes.
- Keep timestamps and IDs stored as documented text values.
- Verify with the relevant in-memory SQLite test in `tests/db_test.rs`; use `Db::open_in_memory()` for new database tests.
- Checks: Run `cargo test --test db_test`, including `derivation_tx_atomic_on_memory_failure` when changing derivation persistence.
- Boundary: Do not change schema, repository ownership, foreign keys, cascade behavior, or transaction boundaries outside an explicit user request; preserve atomic sensation-plus-memory writes.

#### Scene Orchestration（场景推导）

Owns: `src/scene/service.rs` and `src/scene/mod.rs`.

- `StoryService` validates scene existence, character existence, and scene participation before derivation.
- Derivation context includes recent memories, prior sensation continuity, vocabulary candidates, and current scene details.
- Preserve validation-and-retry behavior: invalid LLM selections are filtered, an all-empty valid selection is retried once, and a second all-empty result returns `StoryError`.
- `derive_scene()` limits concurrent character derivation to four; do not replace bounded concurrency with unbounded fan-out.
- Extend `tests/scene_test.rs` for service behavior and `tests/e2e.rs` for end-to-end mock-generator flows.
- Checks: Run `cargo test --test scene_test`; run `cargo test --test e2e` for flows spanning service, generator, validation, and persistence.
- Boundary: Do not broaden scene orchestration into generator, repository, or vocabulary redesign without an explicit user request; preserve prerequisite validation, retry semantics, and concurrency limit of four.

#### LLM Contracts and Generators（LLM 契约与生成器）

Owns: `src/llm/contract.rs`, `src/llm/generator.rs`, `src/llm/rig_impl.rs`, `src/llm/mock.rs`, and `src/llm/mod.rs`.

- `SenseGenerator` is the abstraction boundary. Production behavior uses `RigSenseGenerator`; deterministic tests use `MockSenseGenerator` injected as `Arc<dyn SenseGenerator>`.
- Keep structured output types in `contract.rs` compatible with `schemars::JsonSchema` and serde derivation required by Rig extraction.
- Do not use an OpenAI compatibility layer; production provider is `rig::providers::deepseek` and model constant is `deepseek::DEEPSEEK_V4_FLASH`.
- When an LLM output field changes, update the contract, `DerivationRequest` context if needed, both generator implementations, validation, persistence mapping, and every fixture constructing `LlmCharacterDerivation`.
- Checks: Run `cargo test --test scene_test` and `cargo test --test e2e` after contract or generator changes to exercise mock-backed derivation flows.
- Boundary: Do not replace `SenseGenerator`, change provider or model selection, or alter unrelated scene, persistence, or vocabulary behavior without an explicit user request; preserve production DeepSeek wiring and deterministic mock injection.

#### Vocabulary and Validation（词库与校验）

Owns: `assets/vocab.yaml`, `src/vocab/bootstrap.rs`, `src/vocab/loader.rs`, `src/vocab/validate.rs`, and `src/vocab/mod.rs`.

- Base vocabulary contains five categories: `visual`, `auditory`, `olfactory`, `tactile`, and `gustatory`.
- A vocabulary entry is keyed as `<sense>.<key>` and contains display `text` plus `tags`.
- Runtime load path is `load_runtime_vocab(base, distilled_dir?)` in `bootstrap.rs`: base YAML, optional merge of a distilled directory.
- Keep YAML loading and candidate generation in `loader.rs`; keep LLM output filtering in `validate.rs`.
- Preserve deterministic candidate/tag caps: `DEFAULT_PER_SENSE_CAP=24`, `DEFAULT_TOTAL_CAP=96`, `DEFAULT_TAG_CAP=80`.
- Preserve the guardrail that only vocabulary-backed selections survive validation.
- Test parser, candidate, bootstrap merge, and invalid-selection behavior in `tests/vocab_test.rs`.
- Checks: Run `cargo test --test vocab_test` after vocabulary loader, validation, bootstrap, or committed base vocabulary changes.
- Boundary: Do not expand base-vocabulary work into optional corpus or distillation assets, generator behavior, or persistence changes without an explicit user request; preserve five senses, caps, and vocabulary-backed validation.

#### Corpus Distillation（语料蒸馏）

Optional local assets only, possibly uncommitted: `src/bin/distill.rs`, `tools/extract_corpus.py`, `tools/distill_langextract.py`, `tools/validate_fragments.py`, `tools/verify_distilled.py`, `tools/distill_quality_report.py`, `corpus/`, and `assets/distilled/`. Distilled YAML is local material; runtime may merge it when present (see Distilled vocabulary above).

- Before editing these paths or running their commands, confirm each required path exists and get explicit user instruction.
- If present, treat `corpus/<book>/cNNN.txt` as source material. Never rewrite it unless the task explicitly changes corpus extraction.
- If `src/bin/distill.rs` is present and the user explicitly requests local distillation, it requires `DEEPSEEK_API_KEY` and runs as `cargo run --bin distill -- <book> <start_chap> <end_chap>`.
- If that local tool is present, it writes `assets/distilled/<book>-cNNN.yaml` and skips a chapter output that already exists; do not overwrite generated material without explicit instruction.
- Local output fragments must be continuous source-text substrings after whitespace normalization. The generator locates and classifies; it must not invent or rewrite prose.
- For explicitly approved local workflows, validate generated entries and report KPIs before treating them as usable vocabulary:

```powershell
python tools/validate_fragments.py
python tools/verify_distilled.py
python tools/distill_quality_report.py
```

- `python tools/validate_fragments.py --prune` rewrites local generated files when that tool is present. Run it only with explicit approval after inspecting reported invalid entries.
- Checks: Only when optional local tools/assets are present and the user requests distillation work, run `python tools/validate_fragments.py`, `python tools/verify_distilled.py`, and optionally `python tools/distill_quality_report.py`; exclude `--prune` because it rewrites files.
- Boundary: Do not create, edit, delete, move, or regenerate optional local `src/bin/distill.rs`, `tools/`, `corpus/`, or `assets/distilled/` paths unless the user explicitly requests that local workflow. Preserve source-text provenance and do not treat these paths as committed baseline.

### Runtime Operations（运行操作）

Run from repository root:

```powershell
cargo run                                  # bare: demo + API on :3000 (legacy mode)
cargo run -- repl                          # interactive story-operation REPL
cargo run -- character create 宝玉 --tags 痴情  # one-shot subcommand
cargo run -- scene create 事件 --with 宝玉
cargo run -- derive --character 宝玉
cargo run -- narrate
cargo run -- show derivation
cargo run -- show prose
```

- `src/main.rs` parses `Cli` via clap. No subcommand -> legacy demo + API; subcommand -> `cli::run_cli`.
- CLI commands route through `StoryService` exclusively; no free chat, no `--raw-llm` / `--skip-validate`.
- Without `DEEPSEEK_API_KEY`, both sense + prose generators degrade to Mock; CLI reports `status: warning` +「非生产质量」.
- SQLite busy/lock errors surface as a readable Chinese message; MVP does not provide multi-writer merge.
- REPL prompt goes to stderr so stdout stays clean for body pipelines (narrate/show prose).
- `novels.db` is created or reused in the repository root.
- Vocabulary: `load_runtime_vocab` loads `assets/vocab.yaml`, then merges `assets/distilled/` when present unless `NOVELS_SKIP_DISTILLED=1`; override dir with `NOVELS_DISTILLED_DIR`.

### Quality Gates（质量门禁）

Use the narrowest relevant test during development, then run applicable repository checks:

```powershell
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
```

- Model or vocabulary changes: run the matching `tests/models_test.rs` or `tests/vocab_test.rs` test filter, then full tests when practical.
- Repository or schema changes: run matching `tests/db_test.rs` filter, then full tests when practical.
- Service or LLM-flow changes: run matching `tests/scene_test.rs` or `tests/e2e.rs` filter, then full tests when practical.
- Documentation-only changes: run `cargo fmt --all -- --check` and inspect Markdown headings and commands; no behavior test is required.
- On Windows, Lance 7.0.0 build-script failure can occur before this crate compiles. Compare failure output with the known baseline before attributing it to a change. `protoc` is expected at `C:\Tools\protoc\bin\protoc.exe` when Lance dependencies are built.

### Data and Safety Rules（数据与安全规则）

- Never add `.env`, API keys, tokens, or source credentials to Git or documentation.
- Do not infer an active LanceDB or vector-store feature solely because `rig-lancedb` appears in dependency history.
- Do not delete, move, reformat, or regenerate local `corpus/`, `assets/distilled/`, `temp/`, or unrelated worktree files when present without explicit user instruction.
- Keep locally generated vocabulary traceable to its corpus chapter. Validate text provenance before merging or committing generated YAML.
- Do not run destructive Git commands such as `git reset --hard` or `git checkout --` unless explicitly approved.

### Change Checklist（变更检查表）

Before completing a task, verify:

1. The changed file belongs to the intended layer.
2. Public model, repository, service, LLM, vocabulary, and persistence contracts changed together where required.
3. A behavior change has a focused regression test using the established test module.
4. Mock LLM fixtures still construct every required contract field.
5. Database multi-write behavior remains transactional.
6. LLM-selected vocabulary still passes `validate_selection()` before storage.
7. Formatting, targeted tests, and applicable quality gates have evidence or a documented baseline blocker.
8. Git staging contains only intended files and no credentials or generated assets outside requested scope.

### Troubleshooting（故障处理）

- `DEEPSEEK_API_KEY` missing during `cargo run`: expected fallback to the mock generator.
- Optional local corpus or distillation paths absent: expected on a clean checkout. If present as untracked user assets, do not edit or run them without explicit instruction.
- Invalid or empty LLM sensory output: inspect vocabulary IDs and `validate_selection()` behavior before changing retry or persistence code.
- Database consistency concern: inspect `src/db/derivation_repo.rs`; do not add independent sensation and memory writes around the existing transaction.
- Windows build fails in Lance tooling before crate compilation: verify `$env:PROTOC` points to `C:\Tools\protoc\bin\protoc.exe`, then compare the failure with the known baseline before editing application code.
