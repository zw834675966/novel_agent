# AI Maintenance Playbook Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `AGENTS.md` sufficient for an AI agent to safely maintain the Rust novel-character perception engine.

**Architecture:** Keep existing repository guidance authoritative and append one task-oriented `AI Maintenance Playbook` section. The playbook maps work categories to ownership files, invariants, validation commands, runtime operations, and data-safety boundaries; it introduces no runtime behavior or dependencies.

**Tech Stack:** Markdown, Cargo, Rust 2024, SQLite via SQLx, Rig DeepSeek integration, Python corpus utilities.

## Global Constraints

- Modify only `AGENTS.md`; do not change runtime code, dependencies, schema, assets, tests, or corpus files.
- Preserve existing guidance and terminology; append the new section after `Environment Setup`.
- Treat all unrelated dirty-worktree files as user work and do not stage, edit, delete, or normalize them.
- Never include credential values; name only `DEEPSEEK_API_KEY` as the environment-variable contract.
- Keep documented commands executable from repository root on Windows PowerShell.
- Do not describe `rig-lancedb` as an active runtime dependency or vector-store architecture.

---

## File Structure

- Modify: `AGENTS.md` — authoritative repository guide; append AI task playbooks and operational guardrails.
- Create: `docs/superpowers/plans/2026-07-20-ai-maintenance-playbook.md` — this execution plan only.

### Task 1: Append AI Maintenance Playbook

**Files:**
- Modify: `AGENTS.md` — append after the final `Environment Setup（Windows 环境）` section.
- Test: `AGENTS.md` — inspect section headings and command literals after editing.

**Interfaces:**
- Consumes: Existing module layout, runtime wiring, data flow, quality commands, baseline issue, `src/bin/distill.rs`, and `tools/*.py` behavior.
- Produces: `## AI Maintenance Playbook（AI 维护手册）`, a task-to-file ownership map, runtime workflows, data-safety rules, validation selection rules, and troubleshooting guidance.

- [ ] **Step 1: Confirm the documentation baseline before editing**

Run:

```powershell
rtk rg -n '^## |^### |cargo (fmt|check|test|clippy)|DEEPSEEK_API_KEY|rig-lancedb' AGENTS.md
rtk git status --short
```

Expected: Existing sections include `Scope`, `Commands`, `Runtime Wiring`, `Architecture`, `Data Flow`, `Known Baseline Issues`, and `Environment Setup`; unrelated modified and untracked user files may exist.

- [ ] **Step 2: Append the complete AI maintenance playbook**

Append this Markdown after the existing final line of `AGENTS.md`:

```markdown

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
- `assets/vocab.yaml` is the base vocabulary; `assets/distilled/*.yaml` is optional merged vocabulary material.

### Task Playbooks（任务手册）

#### Domain Models and IDs（领域模型与 ID）

Owns: `src/models/` and `src/models/mod.rs`.

- Put pure domain structures, enums, and typed IDs in `src/models/`; keep persistence and orchestration out of model files.
- Use `CharacterId`, `SceneId`, `MemoryId`, and `VocabularyId` instead of raw IDs at public boundaries.
- Preserve `VocabularyId` format `sense.key`; validate vocabulary identity before persistence or selection.
- When a model changes, update affected repository serialization, service construction, LLM contract conversion, and focused tests in `tests/models_test.rs`, `tests/db_test.rs`, `tests/scene_test.rs`, or `tests/vocab_test.rs`.

#### Database and Repositories（数据库与仓储）

Owns: `src/db/schema.rs`, `src/db/*_repo.rs`, and `src/db/mod.rs`.

- Add schema changes through `migrate()`; preserve foreign keys and existing `ON DELETE CASCADE` behavior.
- Keep entity-specific queries in their repository factory returned by `Db`.
- Use `DerivationRepo::insert_derivation()` for sensation plus memory writes that must remain atomic. Do not split its transaction into independent writes.
- Keep timestamps and IDs stored as documented text values.
- Verify with the relevant in-memory SQLite test in `tests/db_test.rs`; use `Db::open_in_memory()` for new database tests.

#### Scene Orchestration（场景推导）

Owns: `src/scene/service.rs` and `src/scene/mod.rs`.

- `StoryService` validates scene existence, character existence, and scene participation before derivation.
- Derivation context includes recent memories, prior sensation continuity, vocabulary candidates, and current scene details.
- Preserve validation-and-retry behavior: invalid LLM selections are filtered, an all-empty valid selection is retried once, and a second all-empty result returns `StoryError`.
- `derive_scene()` limits concurrent character derivation to four; do not replace bounded concurrency with unbounded fan-out.
- Extend `tests/scene_test.rs` for service behavior and `tests/e2e.rs` for end-to-end mock-generator flows.

#### LLM Contracts and Generators（LLM 契约与生成器）

Owns: `src/llm/contract.rs`, `src/llm/generator.rs`, `src/llm/rig_impl.rs`, `src/llm/mock.rs`, and `src/llm/mod.rs`.

- `SenseGenerator` is the abstraction boundary. Production behavior uses `RigSenseGenerator`; deterministic tests use `MockSenseGenerator` injected as `Arc<dyn SenseGenerator>`.
- Keep structured output types in `contract.rs` compatible with `schemars::JsonSchema` and serde derivation required by Rig extraction.
- Do not use an OpenAI compatibility layer; production provider is `rig::providers::deepseek` and model constant is `deepseek::DEEPSEEK_V4_FLASH`.
- When an LLM output field changes, update the contract, `DerivationRequest` context if needed, both generator implementations, validation, persistence mapping, and every fixture constructing `LlmCharacterDerivation`.

#### Vocabulary and Validation（词库与校验）

Owns: `assets/vocab.yaml`, `src/vocab/loader.rs`, `src/vocab/validate.rs`, and `src/vocab/mod.rs`.

- Base vocabulary contains five categories: `visual`, `auditory`, `olfactory`, `tactile`, and `gustatory`.
- A vocabulary entry is keyed as `<sense>.<key>` and contains display `text` plus `tags`.
- Keep YAML loading and candidate generation in `loader.rs`; keep LLM output filtering in `validate.rs`.
- Preserve the guardrail that only vocabulary-backed selections survive validation.
- Test parser, candidate, and invalid-selection behavior in `tests/vocab_test.rs`.

#### Corpus Distillation（语料蒸馏）

Owns: `src/bin/distill.rs`, `tools/extract_corpus.py`, `tools/distill_langextract.py`, `tools/validate_fragments.py`, `tools/verify_distilled.py`, `corpus/`, and `assets/distilled/`.

- Treat `corpus/<book>/cNNN.txt` as source material. Never rewrite it unless the task explicitly changes corpus extraction.
- Rust distillation requires `DEEPSEEK_API_KEY` and runs as `cargo run --bin distill -- <book> <start_chap> <end_chap>`.
- Rust distillation writes `assets/distilled/<book>-cNNN.yaml` and skips a chapter output that already exists; do not overwrite generated material without explicit instruction.
- Output fragments must be continuous source-text substrings after whitespace normalization. The generator locates and classifies; it must not invent or rewrite prose.
- Validate generated entries before treating them as usable vocabulary:

```powershell
python tools/validate_fragments.py
python tools/verify_distilled.py
```

- `python tools/validate_fragments.py --prune` rewrites generated files. Run it only with explicit approval after inspecting reported invalid entries.

### Runtime Operations（运行操作）

Run from repository root:

```powershell
cargo run
```

- `src/main.rs` loads `.env` through `dotenv::dotenv().ok()`.
- With `DEEPSEEK_API_KEY`, it constructs `RigSenseGenerator` and calls DeepSeek.
- Without the key, it prints a warning and uses `MockSenseGenerator`; this path must remain runnable for local development and tests.
- `novels.db` is created or reused in the repository root.
- Base vocabulary loads from `assets/vocab.yaml`; `assets/distilled/` is merged only when that directory exists.

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
- Do not delete, move, reformat, or regenerate `corpus/`, `assets/distilled/`, `temp/`, or unrelated worktree files without explicit user instruction.
- Keep generated vocabulary traceable to its corpus chapter. Validate text provenance before merging or committing generated YAML.
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

- `DEEPSEEK_API_KEY` missing during `cargo run`: expected fallback to the mock generator. During `cargo run --bin distill`, this is an error because distillation requires a real provider client.
- Missing `assets/distilled/`: expected; main binary loads only `assets/vocab.yaml` and continues.
- Invalid or empty LLM sensory output: inspect vocabulary IDs and `validate_selection()` behavior before changing retry or persistence code.
- Database consistency concern: inspect `src/db/derivation_repo.rs`; do not add independent sensation and memory writes around the existing transaction.
- Windows build fails in Lance tooling before crate compilation: verify `$env:PROTOC` points to `C:\Tools\protoc\bin\protoc.exe`, then compare the failure with the known baseline before editing application code.
```

- [ ] **Step 3: Verify documented structure and required operational facts**

Run:

```powershell
rtk rg -n '^## AI Maintenance Playbook|^### (Start Here|Task Playbooks|Runtime Operations|Quality Gates|Data and Safety Rules|Change Checklist|Troubleshooting)|cargo run --bin distill|validate_fragments.py --prune|DEEPSEEK_API_KEY|git reset --hard' AGENTS.md
```

Expected: All seven playbook headings occur once, distillation command and destructive `--prune` warning are present, and credentials plus destructive Git safeguards are named without secret values.

- [ ] **Step 4: Run documentation-level validation**

Run:

```powershell
rtk cargo fmt --all -- --check
rtk git diff --check -- AGENTS.md
```

Expected: Both commands exit zero. No Rust code formatting or whitespace regression is introduced.

- [ ] **Step 5: Review staged scope and commit documentation change**

Run:

```powershell
rtk git add -- AGENTS.md
rtk git diff --staged --check
rtk git diff --staged -- AGENTS.md
rtk git status --short
rtk git commit -m "docs: add AI maintenance playbook"
```

Expected: Only `AGENTS.md` is staged for this commit. Existing user modifications, corpus files, distilled assets, tools, and temporary files remain unstaged.

## Plan Self-Review

### Spec Coverage

- Start-here workflow: Task 1, Step 2, `Start Here`.
- Module ownership and task playbooks: Task 1, Step 2, `Task Playbooks`.
- Main and distillation runtime operations: Task 1, Step 2, `Runtime Operations` and `Corpus Distillation`.
- Quality gates and Windows baseline: Task 1, Step 2, `Quality Gates`.
- Credentials, corpus, generated artifacts, and dirty worktree handling: Task 1, Step 2, `Data and Safety Rules`.
- Cross-layer regression prevention: Task 1, Step 2, `Change Checklist`.
- Required operational failures: Task 1, Step 2, `Troubleshooting`.

### Placeholder Scan

No unresolved marker, deferred implementation, ambiguous test instruction, or unspecified file path remains.

### Type Consistency

All references use existing names: `StoryService`, `SenseGenerator`, `RigSenseGenerator`, `MockSenseGenerator`, `LlmCharacterDerivation`, `DerivationRepo::insert_derivation()`, and `validate_selection()`.
