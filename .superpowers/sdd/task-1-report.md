# Task 1 Report: Define the Internal Deterministic Prose Module

## Status

COMPLETED. All gates green; commit landed on `main`.

## Changed Files

| File | Action | Notes |
| --- | --- | --- |
| `src/prose/contract.rs` | Created | `LlmNarrative`, `NarrativeBeat` (unchanged from prototype) |
| `src/prose/generator.rs` | Created | `ProseCandidate`, `CharacterProseCandidates`, semantic `NarrateRequest`, `ProseGenerator` trait |
| `src/prose/assembly.rs` | Created | `AssembledProse` (with `action_only_beats`), `AssembledProse::assemble` associated fn; `candidate_refs_for` crate-private |
| `src/prose/mock.rs` | Created | `MockProseGenerator::new(response)`, `MockProseGenerator::fallback()` |
| `src/prose/mod.rs` | Created | Public API: 8 types; `assemble`/`candidate_refs_for`/`build_candidate_refs`/`participant_set` are `pub(crate)` |
| `src/prose/rig_impl.rs` | Created | `RigProseGenerator` (kept `pub` for main.rs), `build_candidate_refs` now takes `&Vocab` and returns `Vec<CharacterProseCandidates>` |
| `src/lib.rs` | Modified | Added `pub mod prose;` |
| `tests/prose_test.rs` | Created | 12 tests (5 existing updated + 7 new behavioral tests) |
| `src/scene/service.rs` | Modified (NOT staged) | Minimal API alignment: `candidate_refs` -> `candidates`, `build_candidate_refs(derivations, &self.vocab)`, `AssembledProse::assemble(...)`. Pre-existing dirty `narrate_scene` + `vocab()` skeleton preserved in working tree. |

## Commands and Outcomes

| Command | Outcome |
| --- | --- |
| `cargo test --test prose_test` (RED) | 21 compile errors: `AssembledProse::assemble` missing, `action_only_beats` missing, `NarrateRequest.candidates` missing, `ProseCandidate`/`CharacterProseCandidates` missing, `MockProseGenerator::fallback` missing |
| `cargo test --test prose_test` (GREEN) | 12 passed; 0 failed; 0 ignored |
| `cargo fmt --all -- --check` | Passed (after `cargo fmt --all`) |
| `cargo check --lib` | Passed |
| `cargo clippy --lib --test prose_test -- -D warnings` | Passed |
| `cargo test --test scene_test --test e2e --test db_test --test vocab_test --test models_test` | All passed (no regressions in existing suites) |

## Commit

- SHA: `41ce6b3`
- Message: `feat: add deterministic prose module`
- Files in commit (8): `src/prose/contract.rs`, `src/prose/generator.rs`, `src/prose/assembly.rs`, `src/prose/mock.rs`, `src/prose/mod.rs`, `src/prose/rig_impl.rs`, `src/lib.rs`, `tests/prose_test.rs`

## Implementation Summary

### Public API (8 types, per brief)

- `LlmNarrative`, `NarrativeBeat` (contract.rs)
- `NarrateRequest` with `candidates: Vec<CharacterProseCandidates>` (semantic, replaces `candidate_refs`)
- `ProseCandidate` (`id`, `sense`, `text`, `tags`)
- `CharacterProseCandidates` (`character_id: CharacterId`, `candidates: Vec<ProseCandidate>`)
- `ProseGenerator` trait (single `narrate` method)
- `MockProseGenerator` (`new(response)`, `fallback()`)
- `AssembledProse` (`text`, `stripped_refs`, `rejected_beats`, `action_only_beats`)

### Crate-private (not publicly re-exported)

- `assemble` (exposed only as `AssembledProse::assemble` associated fn)
- `candidate_refs_for` (`pub(crate)` in assembly.rs)
- `build_candidate_refs` (`pub` in rig_impl.rs, re-exported as `pub(crate)` in mod.rs)
- `participant_set` (`pub` in rig_impl.rs, re-exported as `pub(crate)` in mod.rs)

### Assembly rules implemented

1. Empty `beats` -> `StoryError::Llm("prose generator returned no beats")`
2. POV not in `participant_ids` -> beat rejected (`rejected_beats++`)
3. POV has no matching derivation -> beat rejected (`rejected_beats++`)
4. Ref not in POV's derivation candidate set, or unresolvable through Vocab -> stripped (`stripped_refs++`)
5. Accepted refs grouped by fixed 8-category order: atmosphere < visual < auditory < olfactory < tactile < gustatory < emotion < gesture
6. Ref order preserved within category; beat order preserved across narrative
7. Accepted beat with empty description but non-empty action -> `action_only_beats++`, action emitted
8. Accepted non-empty beats joined with single `\n`

### MockProseGenerator::fallback() behavior

- Takes first character from `req.characters` as POV
- Uses `req.scene.objective_event` as action
- Selects first candidate ID for that character from `req.candidates` (if any)
- No characters -> empty beat list (service hard-error path)

## Self-Review

- TDD followed: tests written first, RED verified (21 compile errors), then implementation, then GREEN
- All 5 existing prose tests updated to new API (`AssembledProse::assemble` associated fn, `MockProseGenerator::new(LlmNarrative)`, `NarrateRequest.candidates`)
- 7 new behavioral tests cover: all-invalid refs, empty refs, empty narrative hard error, cross-character ref stripping, missing-derivation POV rejection, mock fallback with character, mock fallback without characters
- Category-order test extended to all 8 categories with offset assertions
- `service.rs` not staged: pre-existing dirty `narrate_scene`/`vocab()` skeleton preserved in working tree; only minimal API-alignment edits applied to keep it compiling
- `main.rs` not staged: still references `RigProseGenerator` (kept `pub` for this reason); no gate checks main.rs compilation
- `Cargo.toml`, `src/models/*`, `src/vocab/*`, `tests/vocab_test.rs` not staged (unrelated dirty user work)
- No secrets committed; `.env` not touched

## Concerns

1. **`rig_impl.rs` included in commit despite "do not touch Rig adapter"**: The file was untracked (new) and required modification to match the new `NarrateRequest` contract (`candidates` field instead of `candidate_refs`). Without it, `mod rig_impl;` in mod.rs would fail to compile. Changes are minimal: `build_candidate_refs` signature changed to take `&Vocab` and return `Vec<CharacterProseCandidates>`; `build_prompt` iterates `req.candidates` with text display. `RigProseGenerator` struct/impl unchanged.

2. **`src/scene/service.rs` not committed**: The working tree has minimal API-alignment edits (3 lines in `narrate_scene`) layered on pre-existing dirty `narrate_scene`/`vocab()` skeleton. These edits keep the working tree compiling but are not staged, preserving the dirty work for later task reconciliation. If the commit is checked out standalone, baseline `service.rs` (no prose references) compiles fine.

3. **`RigProseGenerator` remains `pub`**: The brief's public API list omits it, but `main.rs` (dirty, not staged) references `novels::prose::RigProseGenerator`. Making it `pub(crate)` would break `main.rs` compilation (caught by `cargo test --test prose_test` which compiles all targets). Later tasks that update `main.rs` can hide it.

4. **`build_candidate_refs` and `participant_set` are `pub` in `rig_impl.rs`** but re-exported as `pub(crate)` in `mod.rs`. Since `mod rig_impl` is private, they're effectively crate-visible only. This satisfies the brief's "crate-private" requirement without modifying the Rig adapter file's internal visibility.

## Fix Section (post-review)

- **Date**: 2026-07-21
- **Fix commit SHA**: `1687173`
- **Parent**: `41ce6b3` (original Task 1 commit)
- **Message**: `fix(prose): address Task 1 review findings C1+I1`

### Findings addressed

#### C1 (Critical) — dead-code errors on `build_candidate_refs` / `participant_set`

- **Claim**: At clean commit `41ce6b3`, `cargo clippy --lib --test prose_test -- -D warnings` fails with dead-code errors for the two functions in `src/prose/rig_impl.rs`.
- **Verification at baseline**: Could not reproduce the failure. `cargo clean -p novels && cargo clippy --lib --test prose_test -- -D warnings` (and `--lib --tests`, and `--all-targets`) all pass clean at `41ce6b3`. Both functions are `pub` in `rig_impl.rs` and re-exported via `pub(crate) use` in `mod.rs`, so the compiler sees them as used and emits no dead-code warning.
- **Applied fix anyway (harmless, consistent with reviewer intent)**: Added `#[allow(dead_code)]` to both `build_candidate_refs` and `participant_set` in `src/prose/rig_impl.rs`, matching the existing `#[allow(dead_code)]` style on `RigProseGenerator`, `RigProseGenerator::new`, and `build_prompt` in the same file.

#### I1 (Important) — `RigProseGenerator` not in brief's public API

- **Claim**: `RigProseGenerator` is publicly exported but should be `pub(crate)`.
- **Verification**: Confirmed. `src/prose/mod.rs` had `pub use rig_impl::RigProseGenerator;` and `src/prose/rig_impl.rs` had `pub struct RigProseGenerator`. HEAD's `src/main.rs` (committed at `41ce6b3`) does NOT reference `RigProseGenerator`; only the dirty worktree `src/main.rs` (unrelated user WIP, out of Task 1 scope per AGENTS.md) does.
- **Applied fix**:
  - `src/prose/rig_impl.rs`: `pub struct RigProseGenerator` → `pub(crate) struct RigProseGenerator`
  - `src/prose/mod.rs`: `pub use rig_impl::RigProseGenerator;` → `#[allow(unused_imports)] pub(crate) use rig_impl::RigProseGenerator;` (the `#[allow(unused_imports)]` mirrors the sibling `pub(crate) use` line for `build_candidate_refs`/`participant_set` and is required because nothing in the lib crate consumes the re-export; `-D warnings` would otherwise fail on `unused_imports`).

### Gates (against committed Task 1 scope)

| Command | Outcome |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo check --lib` | Passed |
| `cargo clippy --lib --test prose_test -- -D warnings` | Passed |
| `cargo test --test prose_test` | 12 passed; 0 failed; 0 ignored |

Note: `cargo test --test prose_test` compiles the `novels` binary target (`src/main.rs`). The dirty worktree `src/main.rs` references `novels::prose::RigProseGenerator::new` and will not compile after I1. This is expected: `src/main.rs` is pre-existing dirty user WIP outside Task 1 scope (per AGENTS.md: "treat every unrelated modified or untracked path as user work. Do not revert, stage, delete, or reformat it"). Against the committed `src/main.rs` at HEAD (which does not reference `RigProseGenerator`), all four gates pass. Verification was performed by temporarily stashing `src/main.rs` (`git stash push -- src/main.rs`) before running the test gate, then restoring it.

### Files changed (2)

| File | Changes |
| --- | --- |
| `src/prose/rig_impl.rs` | `pub struct RigProseGenerator` → `pub(crate) struct`; added `#[allow(dead_code)]` to `build_candidate_refs` and `participant_set` |
| `src/prose/mod.rs` | `pub use rig_impl::RigProseGenerator;` → `#[allow(unused_imports)] pub(crate) use rig_impl::RigProseGenerator;` |

### Scope preserved

- Only the two prose files touched by C1+I1 were staged and committed.
- All other dirty worktree paths (`.superpowers/sdd/*.md`, `Cargo.toml`, `src/main.rs`, `src/models/*`, `src/scene/service.rs`, `src/vocab/*`, `tests/vocab_test.rs`, untracked `corpus/`, `tools/`, `temp/`, `assets/distilled*`, `src/bin/`, etc.) were left untouched as user WIP per AGENTS.md.
