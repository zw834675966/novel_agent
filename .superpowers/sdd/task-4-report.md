### Task 4 Report: NarrateRequest carries plan; NarrativeBeat optional camera_beat_id

**Base commit:** `fb8c86b`
**Status:** ✅ Complete — all quality gates pass

---

## Summary

Plumbing task that threads `LlmScenePlan` through `NarrateRequest` and adds an
optional `camera_beat_id` field to `NarrativeBeat`. No behavioral changes —
Task 5/6 will wire the real flow. All 182 existing tests still pass.

---

## Changes by file

### 1. `src/prose/contract.rs` — add `camera_beat_id` to `NarrativeBeat`

Added after `sensation_refs`:

```rust
/// 对应 `LlmScenePlan` 中 `CameraBeat::beat_id`（空字符串 = 未关联镜头）。
#[serde(default)]
pub camera_beat_id: String,
```

`#[serde(default)]` preserves JSON backward compatibility — existing serialized
narratives without this field still deserialize.

### 2. `src/prose/generator.rs` — add `plan` to `NarrateRequest`

- Added import: `use super::plan_contract::LlmScenePlan;`
- Added field: `pub plan: LlmScenePlan,` to `NarrateRequest`

### 3. `src/prose/plan_contract.rs` — add `LlmScenePlan::minimal()`

Added `impl LlmScenePlan` block with a `minimal(event, character_names)`
constructor that builds a one-act plan with one camera beat per character name.
Used as a placeholder in `service.rs` (Task 5 replaces with real
`ScenePlanner::plan_scene()`) and in test helpers.

### 4. `src/prose/mock.rs` — update `NarrativeBeat` literal

Added `camera_beat_id: String::new(),` to the `MockProseGenerator::narrate`
fallback beat (line ~49). Mock behavior unchanged — still returns one beat per
character.

### 5. `src/prose/assembly.rs` — update test helper

Added `camera_beat_id: String::new(),` to the `narrative()` test helper closure
(line ~473).

### 6. `src/prose/rig_impl.rs` — update 2 test helpers

- `minimal_request()`: added `plan: crate::prose::LlmScenePlan::minimal("事件", &["甲"])`
- `request_with_unsorted_candidates()`: added `plan: crate::prose::LlmScenePlan::minimal("深夜来访", &["甲", "乙"])`

`build_prompt` was NOT modified (Task 6 wires plan content into the prompt).

### 7. `src/scene/service.rs` — add placeholder `plan` to production path

- Added `LlmScenePlan` to the `use crate::prose::{...}` import
- Constructed `plan` via `LlmScenePlan::minimal(...)` before the `NarrateRequest`
  literal (using sorted `characters` names), then passed it as the `plan` field

The plan is a **placeholder** — Task 5 replaces this with a real
`ScenePlanner::plan_scene()` call. `ScenePlanner` was NOT added to `StoryService`.

### 8. `tests/prose_test.rs` — update 3 `NarrativeBeat` fixtures

Added `camera_beat_id: String::new(),` to all 3 `NarrativeBeat` literals
(lines ~139, ~148, ~281).

### 9. `tests/bm25_provenance_real_test.rs` — update 2 `NarrativeBeat` fixtures

Added `camera_beat_id: String::new(),` to both `NarrativeBeat` literals
(lines ~160, ~249).

---

## Construction sites verified

**NarrateRequest (3 sites):**
- ✅ `src/scene/service.rs` — production path (placeholder plan)
- ✅ `src/prose/rig_impl.rs` — `minimal_request()` test helper
- ✅ `src/prose/rig_impl.rs` — `request_with_unsorted_candidates()` test helper

**NarrativeBeat (7 sites):**
- ✅ `src/prose/mock.rs` — MockProseGenerator fallback
- ✅ `src/prose/assembly.rs` — test helper `narrative()`
- ✅ `tests/prose_test.rs` — 3 test fixtures
- ✅ `tests/bm25_provenance_real_test.rs` — 2 test fixtures

---

## Quality gates

| Gate | Command | Result |
|------|---------|--------|
| Compile | `cargo check --all-targets` | ✅ clean |
| Tests | `cargo test --all-targets` | ✅ 182 passed (19 suites) |
| Format | `cargo fmt --all -- --check` | ✅ clean |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings` | ✅ clean |

---

## What was NOT changed (per constraints)

- `NarrateRequest` consumers do NOT use the `plan` field yet (Task 5/6)
- `MockProseGenerator::narrate` behavior unchanged (Task 6 changes this)
- `RigProseGenerator::build_prompt` does NOT include plan content (Task 6)
- `ScenePlanner` NOT added to `StoryService` (Task 5)
- `#[serde(default)]` retained on `camera_beat_id`
- No unrelated worktree files touched
