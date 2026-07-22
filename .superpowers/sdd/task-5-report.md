# Task 5 Report

## Status

Implemented and committed Task 5 LLM-layer changes. Focused semantic request test remains blocked by Task 6-owned `StoryService` orchestration, which does not invoke `select_context_tags` yet. No service code changed.

## Implementation

- `RigSenseGenerator` now owns independent DeepSeek extractors for `LlmContextTagSelection` and `LlmCharacterDerivation`, both built with `retries(1)`.
- Tag selection prompt includes character details, objective event, previous plot kind/reason, and sorted available tags. It explicitly restricts output to listed tags.
- Derivation prompt includes prior plots, memories, previous sensation, and ID-sorted semantic vocabulary candidates rendered with ID, sense, text, and sorted tags.
- Both production LLM calls convert extraction errors to `StoryError::Llm`.
- `MockSenseGenerator` accepts deterministic tag and derivation responses.
- Mock fallback and e2e/scene fixtures construct `LlmContextTagSelection::default()` where tags are not needed.
- `tests/scene_test.rs` contains recording generator coverage for semantic candidate metadata and available tags.

## TDD Evidence

1. RED: changed scene mock fixtures to require a tag response before changing production mock code.
2. Ran `cargo test --test scene_test derivation_passes_semantic_vocabulary_candidates`.
3. Observed expected compile failure: `MockSenseGenerator::new` accepted one argument, while fixtures supplied controlled `LlmContextTagSelection` plus derivation.
4. GREEN: implemented deterministic mock tag response and production two-extractor behavior.

## Commands And Results

- `cargo test --test scene_test derivation_passes_semantic_vocabulary_candidates` before implementation: failed with two `E0061` errors, proving missing controlled tag-response constructor.
- `cargo test --test scene_test derivation_passes_semantic_vocabulary_candidates` after implementation: compiled and ran, then failed at `select_context_tags was not invoked`. This is expected until Task 6 changes `StoryService`; Task 5 was explicitly prohibited from changing that orchestration.
- `rustfmt --edition 2024 src/llm/rig_impl.rs src/llm/mock.rs src/main.rs tests/scene_test.rs tests/e2e.rs`: passed.
- `cargo test --test e2e`: passed, 1 test passed.
- `cargo check --bin novels`: passed.
- `git diff --check`: passed before commit with no whitespace errors.
- Staged diff review: passed; no secrets or unrelated staged paths. The staged `main.rs` diff excluded pre-existing distilled-vocabulary changes.

## Commit

- SHA: `0f5c4bb`
- Message: `feat: select tags before character derivation`
- Committed files:
  - `src/llm/rig_impl.rs`
  - `src/llm/mock.rs`
  - `src/main.rs` (Task 5 mock fallback hunk only)
  - `tests/e2e.rs`
  - `tests/scene_test.rs`

## Self-Review And Concerns

- `StoryService` selection invocation remains Task 6 work. Its absence prevents the semantic recording test from passing; this task intentionally leaves service orchestration untouched.
- Existing dirty-worktree changes, including vocabulary distillation support in `main.rs`, were preserved and excluded from this commit.
- No live DeepSeek request was run; extractor construction and error mapping compile through `cargo check --bin novels`.
