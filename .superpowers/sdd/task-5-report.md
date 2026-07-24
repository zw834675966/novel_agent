# Task 5 Report: Wire planner into StoryService::narrate_scene + bootstrap

## Summary

Replaced the `LlmScenePlan::minimal()` placeholder in `narrate_scene` with a real
`ScenePlanner::plan_scene()` call. Created `RigScenePlanner`, added the `scene_planner`
field to `StoryService`, updated `new()`, bootstrap, the scenario binary, and all 18
`StoryService::new()` call sites. All quality gates pass.

## Changes

### 1. Created `src/prose/planner_rig.rs` (new file)

`RigScenePlanner` mirrors the `RigProseGenerator` / `RigSenseGenerator` pattern exactly:
- Constructor takes `deepseek::Client` **by value** (consuming), matching `RigProseGenerator::new`.
- Extractor built via `client.extractor::<LlmScenePlan>(deepseek::DEEPSEEK_V4_FLASH).retries(1).build()`.
- Uses `use rig::client::CompletionClient;` for the `.extractor()` method.
- Errors mapped with `StoryError::Llm(format!("{e:?}"))` — identical to both existing rig impls.
- `build_planner_prompt` sorts characters by `CharacterId` (UUID) for prompt stability,
  matching the canonical-ordering invariant used in `RigProseGenerator::build_prompt`.
- Includes 2 unit tests: prompt content/constraints assertion + character-id sort stability.

### 2. Exported in `src/prose/mod.rs`

Added `mod planner_rig;` and `pub use planner_rig::RigScenePlanner;`.

### 3. Updated `src/scene/service.rs`

- **Imports**: Replaced `LlmScenePlan` with `PlanRequest` + `ScenePlanner` in the
  `crate::prose` use block (`LlmScenePlan` no longer referenced after removing the
  placeholder; kept in `plan_contract.rs` for test/`NarrateRequest` use).
- **Struct**: Added `scene_planner: Arc<dyn ScenePlanner>` field.
- **`new()`**: Added `scene_planner: Arc<dyn ScenePlanner>` as 5th parameter.
- **`narrate_scene`**: Replaced the `LlmScenePlan::minimal(...)` placeholder with:
  - `self.scene_planner.plan_scene(&PlanRequest { scene, characters })` call
    (clones `scene` + `characters` since `characters` is later moved into `NarrateRequest`).
  - Validation: rejects empty `camera_beats`; rejects any `pov_name` not among the
    scene participants' display names. Both return `StoryError::Llm(...)`.
- Updated the doc comment flow list (now 8 steps, planner is step 6).

### 4. Updated `src/bootstrap.rs`

- Added `MockScenePlanner`, `RigScenePlanner`, `ScenePlanner` to the `crate::prose` import.
- Restructured the generator construction to avoid clippy `type_complexity` on a 4-tuple:
  `using_mock` is now derived from `deepseek_client.is_err()` separately, and the match
  produces a 3-tuple `(Arc<dyn SenseGenerator>, Arc<dyn ProseGenerator>, Arc<dyn ScenePlanner>)`.
  - `Ok` arm: real triple (`RigSenseGenerator`, `RigProseGenerator`, `RigScenePlanner`),
    client cloned for sense+prose, consumed by planner last.
  - `Err` arm: mock triple (`MockSenseGenerator`, `MockProseGenerator`, `MockScenePlanner::fallback()`).
- `StoryService::new` call updated with 5th arg.
- Doc comments updated (triple, not pair).

### 5. Updated `src/bin/run_childhood_rivalry.rs`

- Added `MockScenePlanner`, `RigScenePlanner`, `ScenePlanner` to the `novels::prose` import.
- Match now produces a 3-tuple with planner; `StoryService::new` call updated.
- `cargo fmt` applied (fixed pre-existing formatting in the touched block).

### 6. Updated all 18 `StoryService::new()` call sites

| File | Count | Planner arg |
|------|-------|-------------|
| `src/bootstrap.rs` | 1 | `scene_planner` (from match) |
| `src/bin/run_childhood_rivalry.rs` | 1 | `scene_planner` (from match) |
| `tests/prose_test.rs` | 2 | `Arc::new(MockScenePlanner::fallback())` |
| `tests/scene_test.rs` | 7 | `Arc::new(MockScenePlanner::fallback())` |
| `tests/cli_test.rs` | 1 | `Arc::new(MockScenePlanner::fallback())` |
| `tests/bm25_provenance_real_test.rs` | 2 | `Arc::new(MockScenePlanner::fallback())` |
| `tests/api_test.rs` | 1 | `Arc::new(novels::prose::MockScenePlanner::fallback())` |
| `tests/e2e.rs` | 1 | `Arc::new(MockScenePlanner::fallback())` |
| `tests/state_machine_test.rs` | 1 | `Arc::new(MockScenePlanner::fallback())` |
| `tests/story_test_childhood_rivalry.rs` | 1 | `Arc::new(MockScenePlanner::fallback())` |
| **Total** | **18** | |

Import updates per file:
- `tests/prose_test.rs`: glob import (`use novels::prose::*;`) already covers `MockScenePlanner`.
- `tests/scene_test.rs`, `tests/cli_test.rs`, `tests/e2e.rs`: `MockProseGenerator` → `{MockProseGenerator, MockScenePlanner}`.
- `tests/state_machine_test.rs`, `tests/story_test_childhood_rivalry.rs`: `prose::MockProseGenerator` → `prose::{MockProseGenerator, MockScenePlanner}` (inside `novels::{}` glob).
- `tests/bm25_provenance_real_test.rs`: added `MockScenePlanner` to the per-test `use novels::prose::{...}` blocks.
- `tests/api_test.rs`: fully-qualified `novels::prose::MockScenePlanner::fallback()` (no import change).

## Design decisions

1. **Client consumption order**: In bootstrap, `client.clone()` for `RigSenseGenerator` and
   `RigProseGenerator`, then `client` consumed by `RigScenePlanner` last — preserves the
   "single `from_env()` call" invariant and avoids an unnecessary clone.

2. **`type_complexity` fix**: Separated `using_mock: bool` from the generator tuple rather
   than suppressing the clippy lint. The 3-tuple of `Arc<dyn Trait>` matches the threshold
   that the original 3-tuple passed under; the 4-tuple (3 arcs + bool) did not.

3. **Validation placement**: Plan validation (non-empty beats, valid pov_name) runs in
   `narrate_scene` **before** constructing `NarrateRequest`, so an invalid plan fails fast
   with a clear `StoryError::Llm` message before the prose generator is invoked.

4. **`characters` borrow flow**: `characters.clone()` is passed to `PlanRequest` because
   `characters` is moved into `NarrateRequest` afterward. The `valid_names` `HashSet<&str>`
   borrows `characters` after the clone, then `characters` is moved into `req`.

5. **Prompt stability**: `build_planner_prompt` sorts characters by `CharacterId` (UUID),
   consistent with `RigProseGenerator::build_prompt` and the project's deterministic-decoding
   invariants.

## Constraints honored

- ✅ Did NOT change `MockProseGenerator` or `RigProseGenerator` behavior (Task 6).
- ✅ Did NOT remove `LlmScenePlan::minimal()` — still in `plan_contract.rs`, used by tests
  and `NarrateRequest` construction in `rig_impl.rs` tests.
- ✅ Did NOT revert any pre-existing uncommitted changes (N5/MemoryContentSlot refactor).
- ✅ Did NOT modify BM25/tag/candidate retrieval logic.
- ✅ Did NOT touch assemble logic (Task 3).

## Quality gates

| Gate | Result |
|------|--------|
| `cargo check --all-targets` | ✅ clean |
| `cargo test --all-targets` | ✅ 184 passed, 0 failed |
| `cargo fmt --all -- --check` | ✅ clean |
| `cargo clippy --all-targets --all-features -- -D warnings` | ✅ clean |

New tests added (in `planner_rig.rs`):
- `prompt_lists_event_characters_and_constraints`
- `prompt_sorts_characters_by_id`

## Files modified

- `src/prose/planner_rig.rs` (new)
- `src/prose/mod.rs`
- `src/scene/service.rs`
- `src/bootstrap.rs`
- `src/bin/run_childhood_rivalry.rs`
- `tests/prose_test.rs`
- `tests/scene_test.rs`
- `tests/cli_test.rs`
- `tests/bm25_provenance_real_test.rs`
- `tests/api_test.rs`
- `tests/e2e.rs`
- `tests/state_machine_test.rs`
- `tests/story_test_childhood_rivalry.rs`
