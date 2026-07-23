# Task 1 Report: Plan contracts (JsonSchema types)

## Status: DONE

## What I implemented

Created a new module `src/prose/plan_contract.rs` containing five JSON-schema-compatible contract types that form the first piece of the Phase 1 Outline + Camera-Beat Assemble feature. These will later be consumed by a `ScenePlanner` trait (Task 2) and threaded through `NarrateRequest` (Task 4).

The structs follow the established pattern in `src/prose/contract.rs` and `src/llm/contract.rs` (`schemars::JsonSchema` + `serde::{Serialize, Deserialize}` derives), with `PartialEq, Eq` added so the round-trip test can use `assert_eq`.

### Types (all `#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]`)

- `OutlineAct { act_id, summary, emotional_beat, stakes }` — one act of a story outline.
- `StoryOutline { premise_one_liner, acts: Vec<OutlineAct> }` — top-level outline shape.
- `CameraBeat { beat_id, pov_name, intent, must_show: Vec<String> (serde default), location_hint: String (serde default) }` — a single camera/pov beat. `pov_name` is a display name the planner emits; the service later maps it to a UUID.
- `SceneCard { when, where_place, on_stage: Vec<String> (serde default), camera_beats: Vec<CameraBeat> }` — scene-level card. Uses `where_place` (not the reserved keyword `where`), per the brief's preference to avoid rename bugs.
- `LlmScenePlan { outline: StoryOutline, scene_card: SceneCard }` — the top-level rig `Extractor` target that the planner will produce.

Exported the five types from `src/prose/mod.rs` via `mod plan_contract;` + `pub use plan_contract::{CameraBeat, LlmScenePlan, OutlineAct, SceneCard, StoryOutline};`.

## What I tested and test results

Test: `scene_plan_roundtrip_json` (inline `#[cfg(test)]` module in `plan_contract.rs`).
- Builds an `LlmScenePlan` with one act and one camera beat.
- Serializes to JSON via `serde_json::to_string`, deserializes back, asserts field equality.

Command: `cargo test scene_plan_roundtrip_json -- --nocapture`
Result: **1 passed, 179 filtered out** (exit 0).

Quality gates:
- `cargo fmt --all -- --check` — exit 0 (passes).

## TDD Evidence (RED/GREEN)

The task brief specifies a compile-free-shape test that the structs must satisfy. Because the test and the structs were introduced together in this foundational contract task (the test cannot compile until the types exist), the RED/GREEN cycle here is:

- **RED (conceptual):** Before this change, `LlmScenePlan` / `StoryOutline` / `SceneCard` / `CameraBeat` / `OutlineAct` do not exist, so `cargo test scene_plan_roundtrip_json` would fail to compile ("cannot find type").
- **GREEN:** After adding the module + exports, `cargo test scene_plan_roundtrip_json -- --nocapture` → 1 passed.

Verified GREEN directly: `exit: 0 — 1 passed, 179 filtered out`.

## Files changed

- `src/prose/plan_contract.rs` (created, 79 lines) — the 5 structs + test.
- `src/prose/mod.rs` (modified, +2 lines) — `mod plan_contract;` declaration and `pub use` re-export.

Commit: `efd0394` — `feat(prose): add LlmScenePlan outline and camera-beat contracts`
Staged scope: exactly these two files (no `git add -A`; worktree has many unrelated dirty/untracked files left untouched).

## Self-review findings

- ✅ All five structs match the brief field-for-field, including types and `#[serde(default)]` placement (`CameraBeat.must_show`, `CameraBeat.location_hint`, `SceneCard.on_stage`). `SceneCard.camera_beats` intentionally has no `#[serde(default)]`, matching the brief.
- ✅ Used `where_place` everywhere (no `#[serde(rename)]`), matching the brief's preferred "avoid rename bugs" option.
- ✅ Derive list matches the brief (`Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq`) and is consistent with existing `prose/contract.rs` pattern (`JsonSchema` + `Serialize` + `Deserialize`), with `PartialEq, Eq` added for the `assert_eq` test.
- ✅ `pov_name` doc comment preserved (planner emits display name; service maps to UUID).
- ✅ Module declaration and `pub use` placed consistently alongside the existing sibling modules in `mod.rs`; alphabetical-ish ordering preserved within each group.
- ✅ Formatting gate passes.
- ✅ The crate compiles and the test links against `schemars`/`serde_json` already available as dependencies — no `Cargo.toml` change needed.
- ✅ Scope respected: only the two intended files were staged/committed; no unrelated worktree files touched.

## Issues or concerns

- None blocking. The types are pure data contracts with no behavior, exactly as specified for Task 1. Downstream tasks (Task 2 `ScenePlanner` trait, Task 4 `NarrateRequest` threading) will wire these into the generation pipeline; nothing here constrains that.
- Note (informational): I did not run the full `cargo clippy --all-targets --all-features` or `cargo test --all-targets` gates because the AGENTS.md documents a known Windows baseline failure in Lance 7.0.0 build scripts that occurs before this crate compiles. The targeted test and the fmt gate both pass, which is the relevant evidence for this isolated, dependency-free contract addition.
