# Task 3: Propagate strict database parsing errors

Files:
- Modify `src/db/character_repo.rs`.
- Modify `src/db/scene_repo.rs`.
- Modify `src/db/memory_repo.rs`.
- Modify `src/db/sensation_repo.rs`.
- Test `tests/db_test.rs`.

Requirements:
- Keep existing repository method signatures and public API unchanged.
- Malformed stored UUIDs return `StoryError::Database`; never convert to nil UUID.
- SQL row field extraction failures return `StoryError::Database`; never use `unwrap_or_default` in read paths.
- Malformed `MemorySource` or `Certainty` JSON returns `StoryError::Database`; never fall back to enum defaults.
- Malformed sensation JSON returns `StoryError::Database`; never fall back to empty lists.
- Invalid `VocabularyId` strings stored in sensation JSON return `StoryError::Database`; never silently filter them.
- Serialization errors in memory inserts propagate as `StoryError::Database`; never bind an empty fallback string.
- Use fallible iterator collection (`collect::<Result<Vec<_>, StoryError>>()?`) where needed.
- Add focused DB tests using `db.pool()` raw SQL for malformed scene participant UUID, memory enum JSON, sensation JSON, and sensation VocabularyId. Parent rows must be valid so parsing is the only failure.
- Run `cargo test --test db_test`.
- Do not change schema, dependencies, unrelated modules, or existing user changes.

Report contract: write implementation summary, tests run/output, changed files, and concerns to `.superpowers/sdd/task-3-report.md`; return only status, files, test summary, and concerns. Do not commit.
