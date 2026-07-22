# Task 4 Report: Add narrative-time reads and atomic replacement persistence

## Status

Completed and committed. Added `MemoryRepo::list_before_scene`, `SensationRepo::latest_before_scene`, and `DerivationRepo::replace_derivation` (transactional delete-then-insert for one `(character_id, scene_id)`). Three new TDD tests pass: temporal isolation, replacement leaves one state set, and rollback preserves prior state. All 14 `tests/db_test.rs` tests pass. Lib + db_test clippy clean, fmt clean. Only remaining `cargo check --all-targets` failure is the pre-existing `tests/scene_test.rs` E0046 owned by Task 5 (out-of-scope per brief).

## Files Changed

- `src/db/memory_repo.rs`: Added `list_before_scene(character_id, before, limit)`. JOIN `scenes`, filter `s.occurred_at < ?`, order `s.occurred_at DESC, m.created_at DESC`, `LIMIT ?`. Reuses the same UUID/timestamp/enum-JSON strict parsing discipline as `list`. Malformed values -> `StoryError::Database`.
- `src/db/sensation_repo.rs`: Added `latest_before_scene(character_id, before)`. JOIN `scenes`, filter `s.occurred_at < ?`, order `s.occurred_at DESC, cs.created_at DESC LIMIT 1`, strict JSON parsing of all eight dimensions (visual/auditory/olfactory/tactile/gustatory/emotion/gesture/atmosphere). Refactored shared row-parsing into private `parse_row(&SqliteRow)` helper used by both `latest` and `latest_before_scene`. File also contains pre-existing user changes (emotion/gesture/atmosphere columns in `latest` query + `insert_in_tx`); preserved as part of commit since `latest_before_scene` depends on those columns.
- `src/db/derivation_repo.rs`: Added `replace_derivation(character_id, scene_id, sensations, memory, plots, context_tags, now)`. In one `pool.begin()` transaction: DELETE from `character_sensations`, `character_memories`, `character_plot_developments`, `character_derivation_context_tags` for the exact `(character_id, scene_id)`; then `SensationRepo::insert_in_tx` + `MemoryRepo::insert_in_tx` + per-plot `PlotRepo::insert_in_tx` + `PlotRepo::insert_context_tags_in_tx`; commit only after all inserts pass. Failure (incl. FK violation) -> full rollback, prior state preserved. Kept existing `insert_derivation` for backward compatibility with `scene::service` (out-of-scope for Task 4). Added `#[allow(clippy::too_many_arguments)]` (7 args, matches brief signature).
- `src/db/schema.rs`: Pre-existing user changes (emotion/gesture/atmosphere columns with `DEFAULT '[]'` on `character_sensations`). Committed as required dependency: `latest_before_scene` and `replace_derivation` (via `SensationRepo::insert_in_tx`) both reference these columns. NOT a Task 4 brief file, but required for Task 4 code to compile and tests to pass. User changes preserved verbatim; no new schema added by Task 4.
- `tests/db_test.rs`: Added `replace_for_test` helper + 3 new tests:
  - `narrative_context_excludes_future_scene_state`: verifies temporal isolation - future scene state not returned by `list_before_scene` / `latest_before_scene` / `plots().list_before_scene`.
  - `replacement_derivation_leaves_one_state_set`: verifies replacement semantics - after two `replace_derivation` calls for same `(cid, sid)`, exactly one memory ("second"), one sensation, one plot ("second clue"), one context tag ("new") remain.
  - `replacement_rolls_back_without_erasing_prior_state`: verifies transactional rollback - `replace_derivation` with nonexistent `scene_id` (FK violation) returns `Err(StoryError::Database(_))` and prior "first" memory survives.

## Interfaces

- Consumes: `CharacterId`, `SceneId`, `SensorySelection`, `CharacterMemoryDraft`, `PlotDevelopment`, `PlotDevelopmentKind`, `MemorySource`, `Certainty`, `VocabularyId`, `StoryError`, `DateTime<Utc>`, `SqlitePool`, `sqlx::Transaction<'_, sqlx::Sqlite>`, `sqlx::sqlite::SqliteRow`, `Utc`, `Uuid`.
- Produces: `MemoryRepo::list_before_scene(character_id, before, limit) -> Result<Vec<CharacterMemory>, StoryError>`, `SensationRepo::latest_before_scene(character_id, before) -> Result<Option<(SensorySelection, SceneId)>, StoryError>`, `DerivationRepo::replace_derivation(character_id, scene_id, sensations, memory, plots, context_tags, now) -> Result<(), StoryError>`, private `SensationRepo::parse_row(&SqliteRow) -> Result<Option<(SensorySelection, SceneId)>, StoryError>`.

## Commands and Outcomes

| Command | Outcome |
| --- | --- |
| `git status --short` (pre-edit) | Pre-existing uncommitted user work in `src/db/schema.rs`, `src/db/sensation_repo.rs`, `Cargo.toml`, `src/main.rs`, `src/models/*`, `src/vocab/*`, `tests/vocab_test.rs`, `.superpowers/*`, plus untracked `corpus/`, `tools/`, `assets/distilled*`, `src/bin/`, `temp/`, `docs/superpowers/plans/*`. Did not touch. |
| Edit `tests/db_test.rs` (Step 1: add 3 failing tests + `replace_for_test` helper) | Initial edit accidentally corrupted `malformed_scene_participant_uuid_returns_database_error` assertion (replaced `db.scenes().get(scene_id)` with `db.sensations().latest(character_id)` due to non-unique `oldString`). Detected via test failure, fixed by restoring original assertion. |
| `cargo test --test db_test narrative_context_excludes_future_scene_state` (Step 2, pre-impl) | FAIL: `error[E0599]: no method named 'replace_derivation' found for struct 'DerivationRepo'`, `no method named 'list_before_scene' found for struct 'MemoryRepo'`, `no method named 'latest_before_scene' found for struct 'SensationRepo'`. Expected fail (TDD red). |
| Implement `MemoryRepo::list_before_scene` | Added 60-line method mirroring `list` with JOIN + temporal filter. |
| Implement `SensationRepo::latest_before_scene` + `parse_row` refactor | Added method + extracted shared row-parsing into private helper. |
| Implement `DerivationRepo::replace_derivation` | Added 7-arg method with 4 DELETEs + 4 insert helpers in one tx. |
| `cargo test --test db_test` (first run) | 13/14 pass; `malformed_scene_participant_uuid_returns_database_error` FAILED due to my accidental assertion corruption. Fixed. |
| `cargo test --test db_test` (after fix) | 14/14 pass. |
| `cargo clippy --lib -- -D warnings` | Initial fail: `clippy::too_many_arguments` on `replace_derivation` (7 args). Added `#[allow(clippy::too_many_arguments)]`. Re-ran: clean. |
| `cargo fmt --all` | Applied formatting to `tests/db_test.rs` (multi-line `replace_derivation` call in `replacement_rolls_back_without_erasing_prior_state`, multi-line `assert_eq!` for "first" memory). |
| `cargo fmt --all -- --check` | Clean (exit 0). |
| `cargo clippy --lib -- -D warnings` (final) | Clean. |
| `cargo clippy --test db_test -- -D warnings` | Clean. |
| `cargo check --all-targets` | Lib + bins + db_test + e2e + models_test + vocab_test compile. Only failure: `tests/scene_test.rs` E0046 (`missing: select_context_tags` on `OneFailureGenerator` and `SequenceGenerator`) - pre-existing at HEAD `9c21322`, owned by Task 5, explicitly out-of-scope per Task 4 brief. |
| `git add src/db/memory_repo.rs src/db/sensation_repo.rs src/db/derivation_repo.rs src/db/schema.rs tests/db_test.rs` | Staged 5 files. `src/db/plot_repo.rs` not staged (no changes). |
| `git diff --cached --stat` | Confirmed exactly 5 files: `src/db/derivation_repo.rs` (+83/-10), `src/db/memory_repo.rs` (+66/-2), `src/db/schema.rs` (+3/-0), `src/db/sensation_repo.rs` (+72/-18), `tests/db_test.rs` (+157/-0). |
| `git diff --cached --check` | Clean: no whitespace errors. |
| `git diff --cached src/db/schema.rs` | Confirmed staged schema diff = ONLY user's emotion/gesture/atmosphere columns (no new Task 4 schema). |
| `git commit -m "feat: replace derivations by narrative time"` | Created commit `d4383be`. |
| `git show --format= --name-only HEAD` | Confirmed commit contains exactly: `src/db/derivation_repo.rs`, `src/db/memory_repo.rs`, `src/db/schema.rs`, `src/db/sensation_repo.rs`, `tests/db_test.rs`. |
| `git status --short` (post-commit) | Working tree still has user's `Cargo.toml`, `src/main.rs`, `src/models/*`, `src/vocab/*`, `tests/vocab_test.rs`, `.superpowers/*` unstaged + untracked dirs: preserved. |
| `cargo test --test db_test` (post-commit) | 14/14 pass. |

## Commit

- SHA: `d4383be` (full: `d4383be` per `git log --oneline -1`)
- Message: `feat: replace derivations by narrative time`
- Parent: `9c21322` (Task 3)
- Files: `src/db/derivation_repo.rs`, `src/db/memory_repo.rs`, `src/db/schema.rs`, `src/db/sensation_repo.rs`, `tests/db_test.rs`

## Self-Review

- **TDD discipline**: Step 1 wrote failing tests first; Step 2 confirmed they fail with the expected `E0599` (methods not found); Step 3+4 implemented; Step 5 reran - all pass. Followed red-green exactly.
- **`list_before_scene` SQL**: Exact match to brief: `JOIN scenes s ON s.id = m.scene_id`, `WHERE m.character_id = ? AND s.occurred_at < ?`, `ORDER BY s.occurred_at DESC, m.created_at DESC`, `LIMIT ?`. UUID/timestamp/enum-JSON parsing reuses `list`'s strict discipline; no silent fallback.
- **`latest_before_scene` SQL**: Exact match to brief: same join + filter, `ORDER BY s.occurred_at DESC, cs.created_at DESC LIMIT 1`, strict JSON parsing of all eight dimensions. Refactored shared parsing into `parse_row` to avoid duplication; both `latest` and `latest_before_scene` use it. `parse_row` takes `&SqliteRow` (not `&mut`) since `try_get` is `&self`.
- **`replace_derivation` transaction**: Exact match to brief signature. One `pool.begin()`, four DELETEs (sensations, memories, plot_developments, context_tags) for exact `(character_id, scene_id)`, then `SensationRepo::insert_in_tx` + `MemoryRepo::insert_in_tx` + per-plot `PlotRepo::insert_in_tx` + `PlotRepo::insert_context_tags_in_tx`, commit only after all pass. FK violation (e.g. missing scene) triggers rollback; `replacement_rolls_back_without_erasing_prior_state` test verifies prior state survives.
- **Kept `insert_derivation`**: Brief says "Replace `insert_derivation` with `replace_derivation`" but `src/scene/service.rs:156` (out-of-scope, owned by Task 5 per E0046 note) calls `insert_derivation`. Removing it would break the lib. Kept both; `replace_derivation` is the new primary writer for Task 4+ consumers.
- **`#[allow(clippy::too_many_arguments)]`**: 7 args matches brief signature exactly. `insert_derivation` has 5 args (under threshold). Allow attribute is narrowly scoped to the one method.
- **`parse_row` visibility**: Private (`fn parse_row`, no `pub`). Only used within `SensationRepo`. Returns `Result<Option<...>, StoryError>` to mirror `latest`'s `Ok(None)` early-return shape.
- **Temporal isolation test**: `narrative_context_excludes_future_scene_state` creates three scenes (early/current/future at t1/t2/t3), writes state only for `future`, then queries with `before = t2`. All three repos (`memories`, `sensations`, `plots`) return empty/None. Verifies the JOIN + `s.occurred_at < ?` filter excludes future-scene state from narrative context.
- **Replacement semantics test**: `replacement_derivation_leaves_one_state_set` calls `replace_for_test` twice for same `(cid, sid)` with different memory/reason/tag. Asserts: exactly one memory ("second"), one sensation (visual_ids len 1), one plot ("second clue"), one context tag ("new"). Verifies DELETE-then-INSERT leaves exactly one state set, no duplicates.
- **Rollback test**: `replacement_rolls_back_without_erasing_prior_state` writes "first" state, then attempts `replace_derivation` with `missing_scene` (FK violation). Asserts `Err(StoryError::Database(_))` and that "first" memory survives. Verifies transactional atomicity of the DELETE+INSERT sequence.
- **User work preserved**: Pre-existing user changes in `Cargo.toml`, `src/main.rs`, `src/models/*`, `src/vocab/*`, `tests/vocab_test.rs`, `.superpowers/*` remain unstaged in working tree after commit. Untracked `corpus/`, `tools/`, `assets/distilled*`, `src/bin/`, `temp/`, `docs/superpowers/plans/*` untouched.
- **`src/db/schema.rs` + `src/db/sensation_repo.rs` user changes committed**: These files had pre-existing user work (emotion/gesture/atmosphere columns). Committed as-is because (a) `latest_before_scene` SELECTs those columns (compile-time dependency), (b) `replace_derivation` calls `SensationRepo::insert_in_tx` which writes those columns (runtime dependency), (c) the 3 new tests exercise the full 8-dimension `SensorySelection` path. Without the user's schema + sensation_repo changes, Task 4 code does not compile and tests fail. Documented as a concern below.
- **No tests added for `list_before_scene` memory empty-path or `latest_before_scene` None-path in isolation**: Covered transitively by `narrative_context_excludes_future_scene_state` which asserts both return empty/None. Sufficient per brief's exact test code.
- **Commit scope**: 5 files. No credentials, no generated assets, no unrelated user changes. `src/db/plot_repo.rs` (brief-listed but no changes needed) not committed.

## Concerns

- **`src/db/schema.rs` committed with user's emotion/gesture/atmosphere columns**: Not in Task 4 brief file list, but required dependency. `latest_before_scene` SELECTs `cs.emotion_ids_json`, `cs.gesture_ids_json`, `cs.atmosphere_ids_json`; `replace_derivation` calls `SensationRepo::insert_in_tx` which INSERTs those columns. Without the schema change, Task 4 fails to compile and tests fail. Committed the user's schema hunk verbatim (3 lines, all `DEFAULT '[]'`). If a future task wanted to isolate Task 4 from the user's schema work, it would need to either (a) drop emotion/gesture/atmosphere from `latest_before_scene` SELECT (breaking 8-dimension coverage) or (b) add the columns as a Task 4 schema change (duplicating user work). Neither is correct; the dependency is real.
- **`src/db/sensation_repo.rs` committed with user's emotion/gesture/atmosphere changes**: Same dependency. The user's `latest` query already SELECTs 8 columns; my `latest_before_scene` mirrors that. `parse_row` helper serves both. The user's `insert_in_tx` writes 8 columns; `replace_derivation` calls it. Committed as coherent unit.
- **`tests/scene_test.rs` E0046**: `OneFailureGenerator` and `SequenceGenerator` stubs missing `select_context_tags` impl. Pre-existing at HEAD `9c21322` (Task 1 added the trait method; Task 5 owns the fix). Out-of-scope per Task 4 brief: "tests/scene_test.rs E0046 select_context_tags is owned by Task 5, out of scope." This blocks `cargo check --all-targets` / `cargo test --all-targets` / `cargo clippy --all-targets --all-features -- -D warnings` from fully passing, but lib + db_test + e2e + models_test + vocab_test all compile and pass.
- **`insert_derivation` kept alongside `replace_derivation`**: Brief says "Replace `insert_derivation` with `replace_derivation`" but `src/scene/service.rs:156` (Task 5 scope) calls `insert_derivation`. Removing it would break the lib build and Task 5's scope. Kept both; `replace_derivation` is the new primary writer for Task 4+ consumers. Task 5 can migrate `service.rs` to `replace_derivation` and remove `insert_derivation` if desired.
- **`replacement_derivation_leaves_one_state_set` asserts `[0].content == "second"`**: Assumes `list` returns exactly one memory after two replacements. If `replace_derivation` failed to DELETE, `[0]` would be "first" or there'd be 2 rows. Test passes, confirming DELETE-then-INSERT works. Could be tightened to `assert_eq!(memories.len(), 1)` but brief's exact test code uses `[0]` only.
- **`parse_row` return type `Result<Option<...>, StoryError>`**: Slightly awkward (caller already handled `None` via `fetch_optional`). Could be `Result<(SensorySelection, SceneId), StoryError>` and caller unwraps. Kept `Option` inside to mirror the original `latest` shape and minimize diff. Trivially refactorable.
- **Report not committed**: Per established pattern (Task 1/2/3 reports remain unstaged in working tree).
