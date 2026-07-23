# Task 2 Report: ScenePlanner trait + mock

## What I implemented

Created the Phase 1 scene-planner abstraction layer, mirroring the existing
`ProseGenerator` (trait + mock) pattern in `src/prose/generator.rs` /
`src/prose/mock.rs`. Three files touched:

### `src/prose/planner.rs` (new)

- `pub struct PlanRequest { pub scene: Scene, pub characters: Vec<Character> }`
  — derives `Debug, Clone`. Carries only the minimal planning context
  (objective event + participants); no derivations / candidate fragments.
- `#[async_trait::async_trait] pub trait ScenePlanner: Send + Sync`
  with `async fn plan_scene(&self, req: &PlanRequest) -> Result<LlmScenePlan, StoryError>`.
  Matches the `ProseGenerator` / `SenseGenerator` shape exactly.

### `src/prose/planner_mock.rs` (new)

- `pub struct MockScenePlanner { pub response: Option<LlmScenePlan> }`
  (field is `pub`, per the brief — diverges from `MockProseGenerator`'s private
  fields, but the brief explicitly specifies `pub response`).
- `MockScenePlanner::new(plan)` → `response: Some(plan)` (fixed response).
- `MockScenePlanner::fallback()` → `response: None` (dynamic construction).
- `impl ScenePlanner`: when `response` is `Some`, returns the clone; when
  `None`, builds a deterministic fallback plan:
  - `outline.premise_one_liner` = first 40 chars of `objective_event`
    (char-safe `truncate_chars` helper).
  - one `OutlineAct { act_id: "a1", summary: premise, emotional_beat: "推进", stakes: "未定" }`.
  - `SceneCard { when: "场景当下", where_place: "未标注", on_stage: <names>, camera_beats }`.
  - `camera_beats`: characters **sorted ascending by `name`**, then for
    `(i, c)`: `beat_id = format!("b{}", i+1)`, `pov_name = c.name`,
    `intent = format!("回应：{}", premise)`, `must_show = []`, `location_hint = ""`.
  - `on_stage` follows the same sorted order for coherence with the beats.

### `src/prose/mod.rs` (modified)

Added `mod planner;` / `mod planner_mock;` declarations and
`pub use planner::{PlanRequest, ScenePlanner};` /
`pub use planner_mock::MockScenePlanner;` re-exports alongside the existing
`plan_contract` exports.

## TDD evidence

### Test

`fallback_one_beat_per_character_sorted_by_name` in `planner_mock.rs`. Builds
two characters (苏念卿 `a`, 顾承烨 `b`), passes them to the fallback planner in
non-sorted order `vec![b, a]`, and asserts:

- `camera_beats.len() == 2`
- `camera_beats[0].pov_name == "苏念卿"`
- `camera_beats[1].pov_name == "顾承烨"`
- `outline.premise_one_liner` is non-empty

The input order `[顾, 苏]` is deliberately the **reverse** of the
ascending-sorted order, so the test only passes if a real sort happens (a
no-op iteration would leave 顾 first).

### ⚠ Deviation from the brief's literal test (Unicode ordering)

The brief's test asserted `beats[0] == "顾承烨"` and `beats[1] == "苏念卿"`
with the comment *"sorted by name: 顾 before 苏 (Unicode/lexicographic)"*.
That comment is factually wrong: **苏 = U+82CF (33487) < 顾 = U+987E (39038)**,
so an ascending lexicographic `sort_by(|a,b| a.name.cmp(&b.name))` puts
苏念卿 first. Verified via `python -c "print('苏' < '顾')"` → `True`.

The task name (`..._sorted_by_name`), the implementation spec
("sort characters by name"), and the test name all unambiguously call for an
ascending name sort. The only error was the two assertion lines, which carried
a Unicode-ordering misconception. Rather than implement a wrong/reverse sort
to satisfy a buggy assertion, I:

1. Implemented the **correct ascending lexicographic sort** (honors the task
   name and spec), and
2. **Corrected the two assertion lines** to the true sorted order
   (苏念卿 first, 顾承烨 second).

The input vector and all other test lines are unchanged from the brief. This
is the reason for the DONE_WITH_CONCERNS status.

### Result

```
cargo test fallback_one_beat_per_character_sorted_by_name -- --nocapture
→ 1 passed, 180 filtered out (19 suites, 0.01s)
```

## Quality gates

- `cargo fmt --all -- --check` → exit 0 (clean).
- `cargo clippy --all-targets --all-features -- -D warnings` → exit 0, no issues.
- Targeted test → 1 passed.
- No Lance/lancedb build-script errors (rig-lancedb is not in Cargo.toml, so the
  known Windows baseline blocker does not apply).

## Files changed

- Created: `src/prose/planner.rs` (27 lines)
- Created: `src/prose/planner_mock.rs` (130 lines incl. test)
- Modified: `src/prose/mod.rs` (+2 mod declarations, +2 pub use re-exports)

## Commit

```
git add src/prose/planner.rs src/prose/planner_mock.rs src/prose/mod.rs
git commit -m "feat(prose): add ScenePlanner trait and mock fallback"
```

Only the three intended files were staged and committed. No other modified or
untracked worktree files were touched (there were many unrelated `M`/`??`
entries in `git status`; all left untouched per worktree rules).

## Self-review findings

- ✅ `PlanRequest` + `ScenePlanner` signatures match the brief exactly
  (`#[async_trait::async_trait]`, `Send + Sync`, `Result<LlmScenePlan, StoryError>`).
- ✅ `MockScenePlanner` exposes `pub response: Option<LlmScenePlan>` (brief-specified)
  with `new(plan)` / `fallback()` constructors.
- ✅ Fallback builds one beat per character, sorted by name, with `beat_id = b{i}`,
  `intent` derived from the truncated `objective_event`, empty `must_show`/`location_hint`.
- ✅ Pattern matches `ProseGenerator`/`MockProseGenerator` (async trait, mock dual-mode).
- ✅ `Character` / `Scene` field usage verified against `src/models/character.rs` and
  `src/models/scene.rs` (Character has no timestamps; Scene has `occurred_at`/`participant_ids`).
- ✅ Re-exports added to `mod.rs`; no public-contract changes to other layers.
- ✅ Worktree discipline: only 3 files staged/committed.
- ✅ fmt + clippy clean.

## Concerns

- **(Blocking-for-fidelity, resolved by judgment call)** The brief's test
  asserted the wrong name order due to a Unicode misconception (assumed 顾 < 苏;
  actually 苏 < 顾). I implemented the correct ascending sort and corrected the
  two assertion lines rather than implementing a reverse/no-op sort. Flagged as
  DONE_WITH_CONCERNS so the reviewer can confirm the deviation is acceptable.
  If the reviewer actually wants 顾-first, the fix is a one-line
  `sort_by(|a,b| b.name.cmp(&a.name))` (descending) — but that would contradict
  the "sorted by name" intent.
- Minor: `on_stage` is emitted in sorted (beat) order rather than input
  participant order. The brief only says "on_stage = character names" without
  specifying order; sorted order keeps it coherent with `camera_beats`.
