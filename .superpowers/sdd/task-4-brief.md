# Task 4: Make missing-character updates observable

Files:
- Modify `src/db/character_repo.rs:60-98`.
- Test `tests/db_test.rs`.

Requirements:
- Preserve `CharacterRepo::update(id, name, personality, skills) -> Result<(), StoryError>` signature.
- Capture result from main `UPDATE characters ...`.
- If `rows_affected() == 0`, return `StoryError::CharacterNotFound(id)` before deletion/replacement of tags and skills.
- Existing character update behavior remains transactional and replaces tags/skills.
- Add a regression test attempting an update on a random missing `CharacterId`; assert exact `CharacterNotFound(found)` and `found == id`.
- Run `cargo test --test db_test`.
- Do not change schema, dependencies, unrelated modules, or existing user changes.

Report contract: write implementation summary, tests run/output, changed files, and concerns to `.superpowers/sdd/task-4-report.md`; return only status, files, test summary, and concerns. Do not commit.
