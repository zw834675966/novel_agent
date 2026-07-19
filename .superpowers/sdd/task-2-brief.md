# Task 2: Make retry results atomic and complete

Files:
- Modify `src/scene/service.rs:127-175`.
- Test `tests/scene_test.rs`.

Requirements:
- `validate_with_retry` must receive owned `LlmCharacterDerivation` and return both complete accepted derivation and cleaned `SensorySelection`.
- If first response has at least one valid sensory ID, return that complete response with cleaned sensations.
- If first response is all invalid, invoke generator once more.
- If second response has valid sensations, use its complete memory, plot development, and cleaned sensations. Never combine fields from two attempts.
- If second response is all invalid, return `StoryError::InvalidVocabularySelection` before database write. No sensation or memory row may be inserted.
- Add regression test with response sequence: invalid first result has distinct memory; valid second result has distinct sensation, memory, plot; returned and persisted data must all be second result.
- Add regression test with two invalid responses; assert error and no memories/sensations.
- Run `cargo test --test scene_test`.
- Do not modify unrelated files, schema, or dependencies.

Existing Task 1 validation makes wrong-sense IDs invalid; rely on current `crate::vocab::validate` public alias.

Report contract: write implementation summary, tests run/output, changed files, and concerns to `.superpowers/sdd/task-2-report.md`; return only status, files, test summary, and concerns. Do not commit because shared worktree has pre-existing uncommitted changes.
