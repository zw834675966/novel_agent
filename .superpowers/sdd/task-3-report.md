# Task 3 Report: Strict Database Parsing Errors

## Implementation Summary

- Replaced silent SQL row extraction defaults in `CharacterRepo` with fallible collection.
- Made malformed scene participant UUIDs return `StoryError::Database`.
- Made malformed memory `MemorySource` and `Certainty` JSON return `StoryError::Database`.
- Propagated memory serialization failures as `StoryError::Database` before binding values.
- Made malformed sensation JSON and invalid stored `VocabularyId` values return `StoryError::Database`.
- Added raw-SQL regression tests with valid parent character and scene rows for malformed participant UUID, memory enum JSON, sensation JSON, and sensation vocabulary IDs.

## Tests Run

1. `cargo test --test db_test`
   - Red: 5 passed, 4 failed. The new malformed-data tests exposed silent UUID, enum JSON, sensation JSON, and vocabulary-ID fallback behavior.
   - Green: 9 passed, 0 failed.

## Changed Files

- `src/db/character_repo.rs`
- `src/db/scene_repo.rs`
- `src/db/memory_repo.rs`
- `src/db/sensation_repo.rs`
- `tests/db_test.rs`
- `.superpowers/sdd/task-3-report.md`

## Concerns

- No schema, dependency, public API, or unrelated module changes made.
- `ids_to_json` still uses `unwrap_or_default`, but it serializes `Vec<String>` in a write-only sensation path; Task 3 strict requirements target malformed stored data read paths and memory insert serialization.
- Repository worktree contains pre-existing modifications outside Task 3; they were preserved.
