# Narrative Derivation Closure Design

## Status

Approved design. Implementation planning pending this document review.

## Goal

Make character derivation a narrative-time-consistent, persistent causal loop:

```text
Scene at occurred_at
  -> earlier memories, sensations, plot developments
  -> controlled context-tag selection
  -> vocabulary candidates with ID, text, and tags
  -> character derivation
  -> validated, atomic replacement of derived state
  -> later scenes consume that state
```

## Decisions

### One Current Derivation Per Character and Scene

Repeated derivation of the same `(character_id, scene_id)` replaces its prior generated state. The replacement covers:

- Sensory selection
- Character memory
- Plot developments
- Context tags selected for the derivation

The repository performs replacement in one transaction. A failed write leaves the previous derivation intact.

### Narrative-Time Context

`derive_character(scene_id, character_id)` only loads state attached to scenes whose `occurred_at` is strictly earlier than the current scene. Queries order this state from newest narrative time to oldest, then the prompt presents it oldest to newest.

This rule applies to memories, sensations, and plot developments. Creation time is audit metadata only and must not define narrative causality.

### Controlled Context Tags

Derivation is two-stage behind `SenseGenerator`:

1. Select context tags from the vocabulary's known tag universe using character traits, current objective event, and earlier plot developments.
2. Filter vocabulary entries by those selected tags, then derive sensations, memory, and plot developments.

If no entries match selected tags, the vocabulary service falls back to every known entry. Both model implementations use the same public trait; the mock remains deterministic.

### Semantic Vocabulary Prompting

The production prompt includes each candidate's vocabulary ID, display text, sense/category, and tags. It must not ask the model to choose opaque IDs without their source meaning.

The model's selected IDs still pass existing category-aware candidate validation before persistence.

### Persistent Plot State

Each plot development persists with its character, scene, kind, reason, and creation timestamp. Earlier plot developments are supplied to both context-tag selection and character derivation so story state can influence future perception.

## Module Responsibilities

| Module | Responsibility |
| --- | --- |
| `models` | Add typed context-tag and persisted plot-development data where needed. Preserve typed IDs and vocabulary ID validation. |
| `db` | Add schema and repositories for plot developments and derivation context tags. Add narrative-time query methods and transactional replacement. |
| `vocab` | Expose known tags and candidate entries with metadata. Own tag-based filtering and all-candidate fallback. |
| `llm` | Extend `SenseGenerator` with controlled tag selection and contracts. Build prompts from semantic candidate metadata. |
| `scene` | Assemble only earlier narrative state, invoke two-stage generation, validate output, and call the atomic replacement repository method. |
| `tests` | Cover time ordering, replacement semantics, plot continuity, candidate metadata, fallback behavior, and rollback. |

## Error Handling

- A missing scene, character, or scene membership remains an error before model invocation.
- An empty valid tag selection is allowed and triggers all-vocabulary fallback.
- An all-invalid sensory selection retries once; a second all-invalid selection returns `InvalidVocabularySelection` and writes nothing.
- Any database write failure rolls back replacement and preserves prior derivation state.
- Per-character failures in `derive_scene` remain isolated; concurrency remains capped at four.

## Non-Goals

- No vector database, retrieval system, or new external service.
- No free-form vocabulary creation by the LLM.
- No change to DeepSeek provider, model constant, or Mock fallback behavior.
- No user-facing API or UI work.

## Acceptance Criteria

1. A future scene's state is never used while deriving an earlier scene.
2. Re-deriving one character in one scene replaces all prior generated state atomically.
3. Persisted plot developments appear in later same-character derivation context.
4. Production prompt receives candidate metadata, including source text and tags.
5. Selected context tags constrain candidates; an empty match set falls back to all vocabulary.
6. Existing vocabulary validation and bounded scene concurrency remain intact.
7. Focused database, scene, vocabulary, LLM-contract, and end-to-end tests pass.
