# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`novels` is a Rust narrative engine that drives AI-assisted Chinese novel generation, built specifically to suppress AI-flavored prose ("AI 腔"). It uses DeepSeek (via `rig`) for LLM inference, SQLite for persistence, and a distilled corpus of original-text fragments from 红楼梦 (`hlm`) and 甄嬛传 (`zhz`) to anchor sensory description in verbatim source quotations. The binary also serves an Axum HTTP API + a React/Vite workbench (`web/`) that renders the relationship graph and derivation inspector.

## Commands

Rust (primary):
```powershell
cargo run                                  # default binary: opens DB, runs demo derive+narrate, serves API on 127.0.0.1:3000
cargo run --bin run_childhood_rivalry      # prebuilt storyline scenario
cargo run --bin distill -- hlm 1 5         # corpus distiller: extract fragments from corpus/<book>/cNNN.txt -> assets/distilled/<book>-cNNN.yaml (resumable)
cargo test --all-targets                   # full test suite
cargo test --test e2e                      # single integration test file
cargo test --test e2e end_to_end_with_mock # single test fn
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

Web workbench (`web/`, separate npm project, served by Axum via `ServeDir::new("web/dist")`):
```powershell
cd web; npm run dev      # Vite dev server
cd web; npm run build    # tsc -b && vite build -> web/dist (consumed by Rust fallback_service)
cd web; npm run test     # vitest
```

Python quality tooling (`tools/`, for corpus/vocab sanity, not part of build):
```powershell
python tools/distill_quality_report.py
python tools/verify_distilled.py
```

## Environment

- `DEEPSEEK_API_KEY` - required for real LLM inference. If unset, `main.rs` degrades BOTH `SenseGenerator` and `ProseGenerator` to mock implementations (no half-configured state). Tests always use mocks.
- `NOVELS_SKIP_DISTILLED=1` - load only `assets/vocab.yaml`, skip the distilled-fragment merge.
- `NOVELS_DISTILLED_DIR=<path>` - override the default `assets/distilled` directory (must be a directory).
- `.env` is loaded via `dotenv` and is gitignored.

## Architecture

Layered library (`src/lib.rs` declares the module order bottom-to-top). Understanding the pipeline requires reading across layers:

```
models   ->  db       ->  vocab   ->  llm       ->  prose    ->  scene          ->  api
domain    SQLite       sense       trait+rig     assembly     StoryService       Axum
types     repos        YAML+BM25   +mock         +ref check   orchestration      routes
```

- **`models/`** - pure domain types, newtype IDs (`CharacterId`, `SceneId`, `VocabularyId`, ...), `StoryError` (thiserror). IDs are UUIDs stored as TEXT in SQLite. `StructuredAction` / `PlotReasonSlot` / `MemoryContentSlot` are algebraic-slot enums (N3/N4) that force the LLM into stage-direction or short-fact shapes instead of free prose - central to the anti-AI-taste design.
- **`db/`** - `Db` wraps a `SqlitePool` (cheap `Clone`). Each entity has a Repository obtained via `db.characters()`, `db.scenes()`, `db.memories()`, `db.sensations()`, `db.derivations()`, `db.plots()`, `db.relationships()`, `db.states()`. `DerivationRepo` writes sensory+memory atomically across tables. Schema DDL lives in `src/db/schema.rs` (all FKs `ON DELETE CASCADE`; times/IDs as TEXT). `Db::open` auto-migrates.
- **`vocab/`** - loads `assets/vocab.yaml` plus optional distilled YAMLs into `Vocab`. The candidate pipeline is BM25-shaped: `known_tags_ranked_limited` (context tag shortlist) -> `candidates_ranked_limited` (per-sense candidates). `validate` filters LLM output against the candidate set and reports `all_empty` (triggers one retry). Caps live here: `DEFAULT_TAG_CAP`, `DEFAULT_PER_SENSE_CAP`, `DEFAULT_TOTAL_CAP`.
- **`llm/`** - `SenseGenerator` trait + `RigSenseGenerator` (DeepSeek via `rig::extractor`, forced JSON schema) + `MockSenseGenerator`. Output contract `LlmCharacterDerivation` is the strongly-typed rig extraction target. Rig impls use `deepseek::DEEPSEEK_V4_FLASH`.
- **`prose/`** - mirrors `llm/`: `ProseGenerator` trait + rig/mock. `assembly.rs` is the anti-AI-taste core: LLM writes only the action skeleton; all description is pulled verbatim via `VocabularyId` refs from the distilled corpus. Refs outside the character's candidate set are stripped; non-pov beats are rejected. `AssembledProse` exposes KPIs (`quote_density`, `stripped_refs`, `unverified_quotes`, `low_quote_density`); `MIN_QUOTE_DENSITY = 0.30`.
- **`prompt/`** - prompt engineering decoupled from LLM code. `build_system_prompt` / `build_derivation_prompt` / `build_narration_prompt` produce deterministic prompts (BTreeMap canonical ordering, capped candidates). See `docs/prompt_template_guide.md`.
- **`text_guard.rs`** - shared free-text sanitizer: strips causal/AI glue fillers (`因此`/`于是`/`不禁`/`意味着`/...) and hard-clamps length (`MAX_ACTION_CHARS=80`, `MAX_MEMORY_CHARS=120`, `MAX_PLOT_REASON_CHARS=60`). Called on memory content, plot reasons, and prose actions.
- **`scene/StoryService`** - main business surface. `derive_character` is the key flow: validate scene+participant -> block `Dead`/`Absent` states -> load memories/sensations/plot developments **strictly before** `scene.occurred_at` (no future leakage) -> tag shortlist -> candidate set -> LLM derive -> `validate_with_retry` (one retry if all-empty) -> `sanitize_derivation_free_text` -> atomic `replace_derivation` -> persist relationship candidates filtered to scene participants. `derive_scene` fans out with `buffer_unordered(CONCURRENCY=4)`. `narrate_scene` sorts characters/derivations/candidates by UUID/SENSES order for prompt stability.
- **`api/`** - Axum 0.8. Routes in `routes.rs`; `AppState { service: StoryService }`. Fallback service serves `web/dist` (SPA). Security headers (`nosniff`, `DENY` frame, `no-referrer`) and 1MB body limit applied at the router layer.

## Anti-AI-taste invariants (do not break)

When modifying generation or assembly code, preserve these load-bearing rules - they are the project's reason for existing:

1. LLM writes action skeleton only; descriptive text must come from `VocabularyId` refs resolved against the distilled corpus, never freely generated.
2. `VocabularyId` refs outside the character's candidate set are stripped (not silently kept).
3. Memory/plot-reason/action free text passes through `text_guard::sanitize_free_text` before persistence or rendering.
4. `Dead` / `Absent` characters are rejected before derivation (`StoryService::derive_character` state check).
5. Context loading uses `*_before_scene` queries - never read events after `scene.occurred_at` into a derivation request.
6. Deterministic decoding (`temperature=0.0`, `top_p=1.0`, `seed=42`) and BTreeMap canonical ordering in `prompt/` must remain - they guarantee reproducible prompts.

## Testing

Tests live in `tests/` (integration, use `Db::open_in_memory` + `MockSenseGenerator` / `MockProseGenerator`) and inline `#[cfg(test)]` modules. Integration tests never hit the real DeepSeek API. Notable suites: `e2e.rs` (full create->derive->narrate flow), `story_test_childhood_rivalry.rs` (scenario), `bm25_provenance_real_test.rs` (real corpus), `story_prompt_determinism_test.rs` (prompt stability).

## Branch / planning context

Active branch `story-workbench`. Implementation plans and design specs live under `docs/superpowers/plans/` and `docs/superpowers/specs/` (dated `YYYY-MM-DD-*.md`). Consult these before large changes - they capture ongoing initiatives (corpus distillation, prose generator, BM25 provenance, anti-AI retrieve-action, N5 state machine, relationship graph workbench).
