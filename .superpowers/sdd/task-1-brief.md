# Task 1: Enforce sensory-field vocabulary ownership

Files:
- Modify `src/vocab/validate.rs:27-65`.
- Test `tests/vocab_test.rs`.

Requirements:
- Keep `validate_selection(sel: &SensorySelection, candidates: &HashSet<String>) -> ValidationResult` public signature unchanged.
- Each sensory vector must accept only IDs with matching prefix and membership in candidates: visual, auditory, olfactory, tactile, gustatory.
- Preserve partial-valid behavior: keep valid IDs, put invalid IDs in `stripped`, compute `all_empty` from cleaned fields.
- Add regression test for `auditory.footsteps` in `visual_ids` when both IDs are globally present.
- Run `cargo test --test vocab_test`.
- Do not modify unrelated files or add dependencies.

Report contract: write implementation summary, tests run/output, commit hash, and concerns to `.superpowers/sdd/task-1-report.md`; return only status, commit, test summary, and concerns.
