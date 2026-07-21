# Task 3 Report: Deepen StoryService Around Narration

## Status
COMPLETE.

## Commit
- SHA: `fcd3baa348aae3e76191ab64106217af611d5ac1`
- Message: `feat: integrate deterministic scene narration`
- Files changed: 8 (530 insertions, 58 deletions)

## Files (task-owned, staged exclusively)
- `src/scene/service.rs` — deepened: owns `ProseGenerator`, removed `vocab()` accessor + per-call generator arg, added full validation pipeline + sorting + internal assembly call
- `src/models/error.rs` — added `InvalidNarrationContext(String)` variant
- `src/main.rs` — single `deepseek::Client::from_env()` constructs both Rig adapters; no-key branch constructs both Mock adapters; prints `stripped_refs`/`rejected_beats`/`action_only_beats`
- `tests/scene_test.rs` — 9 `StoryService::new` call sites updated to pass `Arc::new(MockProseGenerator::fallback())`
- `tests/e2e.rs` — 1 call site updated
- `tests/prose_test.rs` — added `RecordingProseGenerator` + 4 service-level tests (missing scene, wrong scene derivation, duplicate character, partial-success counters)
- `src/prose/mod.rs` — `pub use rig_impl::RigProseGenerator` (was `pub(crate)`)
- `src/prose/rig_impl.rs` — `pub struct RigProseGenerator` + `pub fn new` (was `pub(crate)` + `#[allow(dead_code)]`)

## Rationale for extra files (src/prose/mod.rs, src/prose/rig_impl.rs)
Brief listed only 6 files but Step 5 requires `main.rs` to construct `RigProseGenerator::new(client)`. Previously the type was `pub(crate)` — impossible to reference from the binary crate. Visibility bump is the minimal change to satisfy Step 5's production wiring. No behavior change to the generator itself.

## Commands & Outcomes

| Command | Outcome |
|---|---|
| `cargo test --test prose_test` (red, post-Step 1) | FAIL — `InvalidNarrationContext` not found, `narrate_scene` arity mismatch (7 errors). Red state confirmed. |
| `cargo test --test prose_test` (green, post-Step 6) | ok. 16 passed; 0 failed. |
| `cargo test --test scene_test` | ok. 9 passed; 0 failed. |
| `cargo test --test e2e` | ok. 1 passed; 0 failed. |
| `cargo fmt --all -- --check` | clean (after `cargo fmt --all` applied) |
| `cargo check --all-targets` | Finished, no errors. |
| `cargo test --all-targets` | all pass: db_test 16, e2e 1, models_test 6, prose_test 16, scene_test 9, sensory_dimensions_test 1, vocab_test 10. |
| `cargo clippy --all-targets --all-features -- -D warnings` | Finished, no warnings. |

## Self-Review

### Brief compliance
- [x] Step 1: 4 failing tests added with `RecordingProseGenerator` (AtomicUsize counter + Mutex<Vec<NarrateRequest>>).
- [x] Step 2: red state confirmed (compile errors on missing variant + arity).
- [x] Step 3: `InvalidNarrationContext(String)` variant added with `#[error("invalid narration context: {0}")]`.
- [x] Step 4: `StoryService` owns `prose_generator: Arc<dyn ProseGenerator>`; constructor takes 4 args; `vocab()` removed; per-call generator arg removed; validation pipeline (scene load -> scene_id match -> participant check -> duplicate check -> character load -> candidate build -> sort -> narrate -> assemble) implemented exactly per spec.
- [x] Step 5: all 9 `StoryService::new` call sites updated; main.rs uses single `from_env()` with atomic branch constructing both adapters; demo prints all 3 counters.
- [x] Step 6: all gates green.
- [x] Step 7: committed with exact message `feat: integrate deterministic scene narration`.

### Behavioral preservation
- Derivation pipeline untouched (validation-and-retry, MEMORY_LIMIT=50, CONCURRENCY=4, atomic `replace_derivation`).
- `AssembledProse::assemble` contract unchanged — service calls it with the same args.
- `candidate_refs_for` still crate-public; `build_candidate_refs` moved into `service.rs` as a private fn (was `pub(crate)` in `rig_impl.rs`). Old `pub(crate) use rig_impl::{build_candidate_refs, participant_set}` re-export kept to avoid breaking other internal callers; `participant_set` no longer used by service (inlined `HashSet` construction) but re-export retained for non-breaking surface.

### Sorting stability
Service sorts characters/derivations/candidate groups by `CharacterId.0` (Uuid), candidates by `SENSES` index then id, tags lexicographically. Matches `build_prompt` ordering in `rig_impl.rs`.

### Validation order (before generator)
1. Scene load (SceneNotFound)
2. Per-derivation: scene_id match (InvalidNarrationContext)
3. Per-derivation: participant check (InvalidNarrationContext)
4. Per-derivation: duplicate check (InvalidNarrationContext)
5. Per-participant: Character load (CharacterNotFound)
6. Candidate build from service-owned Vocab
7. Sort + NarrateRequest construction
8. `prose_generator.narrate()` (Llm error)
9. `AssembledProse::assemble` (Llm error on empty beats)

Test `missing_scene_fails_before_prose_generator` asserts `recorder.calls() == 0` for nonexistent scene. `derivation_from_another_scene_fails_before_generator` and `duplicate_character_derivations_fail_before_generator` assert `calls() == 0` for their respective InvalidNarrationContext paths.

## Concerns
1. **`participant_set` now unused at call sites** — `service.rs` builds `HashSet<String>` inline for `assemble`. `rig_impl.rs::participant_set` is still `pub(crate)` re-exported but no longer consumed by service. Kept for non-breaking internal surface; clippy did not flag (the `#[allow(unused_imports)]` on the re-export suppresses it). Future task could prune.
2. **`build_candidate_refs` duplication** — moved into `service.rs` as private fn. `rig_impl.rs::build_candidate_refs` still exists (now only used by its own `#[cfg(test)]` tests if any, and `pub(crate)` re-export). Not a blocker but a future cleanup candidate.
3. **`nul` file in worktree root** — pre-existing Windows artifact (`?? nul` in git status), not touched. Left alone per "preserve unrelated dirty work" rule.
4. **Cargo.toml / src/models/{character,ids}.rs / src/vocab/loader.rs / tests/vocab_test.rs dirty** — pre-existing user work, not touched. Committed only task-owned files.

## Test Summary
All 59 tests pass across 7 test binaries; fmt/check/clippy clean.

---

## Fix (Task 3 review findings)

### Findings addressed
- **Important 1 (scope creep)**: `src/main.rs` vocab loading reverted to single `let vocab = Vocab::load_from_path(std::path::Path::new("assets/vocab.yaml"))?;`. Removed `mut`, dropped distilled-dir probe block (`distilled_dir`/`is_dir()`/`load_dir_merged()`/eprintln) - that belonged to corpus distillation work, not Task 3. Dual-adapter construction + counter printing kept.
- **Important 2 (dead code)**: Deleted `build_candidate_refs` and `participant_set` from `src/prose/rig_impl.rs`. Removed `pub(crate) use rig_impl::{build_candidate_refs, participant_set}` re-export from `src/prose/mod.rs` and refreshed module comment. Test module re-imports `ProseCandidate`/`CharacterProseCandidates` from `crate::prose` for the remaining `build_prompt` ordering tests (no test referenced the deleted fns). Service-owned private copies in `src/scene/service.rs` preserved.
- **Nit 3 (optional, applied)**: `AssembledProse::assemble(...)` now receives `&req.derivations` (the sorted slice) instead of the original unsorted `derivations` parameter - restores symmetry between prompt and assembly. `sorted_derivations` is moved into `NarrateRequest`; borrow from `req.derivations` avoids the clone.

### Quality gates
- `cargo fmt --all -- --check`: clean
- `cargo check --all-targets`: clean
- `cargo test --test prose_test`: 16/16 ok
- `cargo test --test scene_test`: 9/9 ok
- `cargo test --test e2e`: 1/1 ok
- `cargo test --all-targets`: 62 passed, 0 failed across 10 binaries (lib unit 3, integration 59)
- `cargo clippy --all-targets --all-features -- -D warnings`: clean

### Scope
Staged files (Task 3 fix only): `src/main.rs`, `src/prose/mod.rs`, `src/prose/rig_impl.rs`, `src/scene/service.rs`. Unrelated dirty/ untracked paths (Cargo.toml, src/models/*, src/vocab/loader.rs, tests/vocab_test.rs, corpus/, assets/distilled/, tools/, src/bin/, etc.) left untouched per preserve-user-work rule.
