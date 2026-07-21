# Deterministic Prose Generator Design

## Status

Approved for implementation planning.

## Goal

Generate scene prose through a deterministic, source-backed pipeline. The LLM chooses narrative beats and vocabulary references; application code validates those references and assembles final text from committed or distilled vocabulary entries.

The design prioritizes provenance, deterministic tests, partial-success behavior, and a small public interface.

## Decision Summary

- Keep Rig as the only LLM framework.
- Use Rig structured extraction for `LlmNarrative`.
- Keep reference validation and prose assembly as project-owned domain logic.
- Do not add MiniJinja, Askama, Tera, Swiftide, Kalosm, LangChain Rust, or a workflow engine.
- Make `StoryService` the only public orchestration entry.
- Inject `ProseGenerator` into `StoryService`; do not pass it per call.
- Keep `Vocab` ownership in `StoryService`; the Rig adapter must not own or reload vocabulary.
- Invalid references and invalid POV beats produce observable partial success rather than aborting the whole scene.

## Architecture

```text
StoryService
├── Db
├── Vocab
├── SenseGenerator
└── ProseGenerator

narrate_scene(scene_id, derivations)
  -> load and validate scene
  -> validate derivation ownership
  -> build NarrateRequest with source-backed candidate metadata
  -> ProseGenerator::narrate(request)
  -> validate beats and vocabulary references
  -> deterministically assemble source text plus action skeleton
  -> return AssembledProse with quality counters
```

### Public Interface

`StoryService` owns both generator adapters:

```rust
pub struct StoryService {
    db: Db,
    vocab: Vocab,
    sense_generator: Arc<dyn SenseGenerator>,
    prose_generator: Arc<dyn ProseGenerator>,
}
```

The narration interface is:

```rust
pub async fn narrate_scene(
    &self,
    scene_id: SceneId,
    derivations: &[CharacterDerivation],
) -> Result<AssembledProse, StoryError>;
```

Callers must not supply `Vocab`, candidate sets, an assembler, or a generator. These are implementation details hidden behind the `StoryService` interface.

`StoryService::new` accepts both generator adapters. Production wiring uses Rig adapters. Tests and no-key runtime wiring use deterministic Mock adapters.

### Prose Generator Seam

`ProseGenerator` has one responsibility: convert a complete `NarrateRequest` into an `LlmNarrative`.

```rust
#[async_trait]
pub trait ProseGenerator: Send + Sync {
    async fn narrate(&self, request: &NarrateRequest)
        -> Result<LlmNarrative, StoryError>;
}
```

Two adapters make this a real seam:

- `RigProseGenerator`: DeepSeek structured extraction.
- `MockProseGenerator`: deterministic test and local fallback output.

The Rig adapter receives candidate metadata through `NarrateRequest`. It does not own `Vocab`, query the database, validate references, or assemble prose.

## Contracts

### LLM Output

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmNarrative {
    pub beats: Vec<NarrativeBeat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NarrativeBeat {
    pub pov: String,
    pub action: String,
    pub sensation_refs: Vec<String>,
}
```

The LLM output is intentionally narrow:

- `pov` identifies one scene participant.
- `action` contains narrative skeleton, action, and dialogue.
- `sensation_refs` contains only vocabulary IDs offered for that POV.

The LLM must not invent descriptive prose that should come from vocabulary entries.

### Narration Request

`NarrateRequest` contains everything the adapter needs to build its prompt:

- Current scene objective event and occurrence time.
- Scene characters and their traits.
- Character derivations.
- Candidate vocabulary metadata grouped by character.

Candidate metadata includes ID, sense, display/source text, and tags. Ordering is stable before prompt construction so identical input produces identical prompts.

### Result

```rust
pub struct AssembledProse {
    pub text: String,
    pub stripped_refs: usize,
    pub rejected_beats: usize,
}
```

- `text` is final deterministic output.
- `stripped_refs` counts unknown, malformed, wrong-owner, or missing-vocabulary references.
- `rejected_beats` counts beats whose POV is not a scene participant or has no matching derivation.

These counters are part of the interface because partial success must be observable.

## Domain Invariants

1. Every beat POV must be a participant in the requested scene.
2. Every accepted POV must have exactly one matching `CharacterDerivation` for the scene.
3. Every accepted vocabulary reference must belong to that POV's derivation sensations.
4. Every accepted reference must resolve through the service-owned `Vocab`.
5. Malformed or unauthorized references are stripped and counted; they do not abort valid beats.
6. Invalid POV beats are rejected and counted; other beats continue.
7. A beat with no valid references still emits its action.
8. An empty LLM beat list is an LLM error because no narrative skeleton exists.
9. Description order is fixed:
   `atmosphere`, `visual`, `auditory`, `olfactory`, `tactile`, `gustatory`, `emotion`, `gesture`.
10. Beats retain LLM sequence order and are separated by one newline.

## Deterministic Assembly

Assembly remains ordinary Rust rather than a template DSL:

1. Build a map from character ID to allowed vocabulary IDs.
2. Process beats in source order.
3. Reject invalid POV beats.
4. Resolve each allowed reference through `VocabularyId` and `Vocab`.
5. Group resolved text by the fixed category order.
6. Concatenate descriptions followed by the action skeleton.
7. Join accepted beats with newlines.

Assembly is internal to the prose module. Unit tests may live beside the implementation so the function does not need to become part of the crate's public interface. Integration tests exercise the same behavior through `StoryService::narrate_scene`.

## Error Handling

Hard errors return `StoryError` and no prose:

- Scene does not exist.
- Derivation belongs to a different scene.
- Duplicate derivations exist for the same character.
- Generator or structured extraction fails.
- Generator returns no beats.

Recoverable content defects return `AssembledProse`:

- Unknown reference.
- Reference from the wrong character.
- Vocabulary entry missing after generation.
- Non-participant POV beat.
- Beat with no valid references.

No retry is added for invalid references. Retrying would increase cost and make deterministic partial-success behavior dependent on another stochastic call.

## Runtime Wiring

Production startup constructs one DeepSeek client and uses it to create both Rig adapters. The service owns both behind `Arc<dyn Trait>`.

When `DEEPSEEK_API_KEY` is absent, startup constructs both Mock adapters. Runtime must not create a partially configured service where character derivation works but prose narration exits early.

## Testing Strategy

### Contract Tests

- `LlmNarrative` and `NarrativeBeat` serialize and deserialize through serde.
- Rig extractor types continue satisfying `JsonSchema` requirements.

### Assembly Unit Tests

- Fixed eight-category ordering.
- Beat sequence ordering.
- Unknown and malformed references stripped.
- Cross-character references stripped.
- Missing vocabulary entries stripped.
- Non-participant POV rejected.
- Missing derivation POV rejected.
- Empty refs emit action only.
- Empty beats return an error.

### Service Tests

- Missing scene fails before generator invocation.
- Derivation from another scene fails before generator invocation.
- Duplicate character derivations fail before generator invocation.
- Mock narration returns source-backed prose and counters.
- Partial success retains valid beats and actions.

### Quality Gates

```text
cargo fmt --all -- --check
cargo test --all-targets
cargo check --all-targets
cargo clippy --all-targets --all-features -- -D warnings
```

## Library Evaluation

### Rig: Reuse

Rig already provides DeepSeek integration, typed extraction through serde and schemars, required tool submission, and retries. Adding another LLM framework would duplicate provider, request, error, and testing abstractions.

Source: <https://github.com/0xPlaygrounds/rig>

### Swiftide: Reject for Current Scope

Swiftide focuses on streaming indexing, retrieval, querying, and agentic applications. The prose pipeline has no RAG or indexing requirement.

Source: <https://github.com/bosun-ai/swiftide>

### Kalosm: Reject for Current Scope

Kalosm focuses on local pretrained models. The selected runtime uses DeepSeek through Rig.

Source: <https://github.com/floneum/kalosm>

### LangChain Rust: Reject

LangChain Rust would overlap with Rig and create two competing LLM abstractions without removing project-specific validation.

Source: <https://github.com/Abraxas-365/langchain-rust>

### MiniJinja, Askama, and Tera: Defer

All three are mature template engines, but current assembly has one fixed ordering rule and no user-editable templates. A template engine would add a DSL, loading behavior, and another error model while moving domain invariants away from Rust types.

Reconsider MiniJinja when requirements include user-editable templates, multiple prose layouts, localization, conditional sections, or runtime template reload.

Sources:

- <https://github.com/mitsuhiko/minijinja>
- <https://github.com/askama-rs/askama>
- <https://github.com/Keats/tera>

## Non-Goals

- Multi-scene prose continuity.
- User-editable templates.
- Style parameterization.
- Free-form LLM descriptive prose.
- RAG or vector retrieval.
- A workflow/state-machine framework.
- Persisting generated prose.
- Switching provider or model.

## Acceptance Criteria

1. `StoryService::narrate_scene` no longer accepts a generator argument.
2. `StoryService` owns one `ProseGenerator` adapter.
3. `RigProseGenerator` does not own `Vocab`.
4. All accepted descriptive text resolves from service-owned vocabulary entries.
5. Invalid references and invalid POV beats are counted without discarding valid output.
6. Action-only beats remain in output.
7. Empty beat output is a hard error.
8. Assembly order is deterministic and covered across all eight categories.
9. Production and no-key runtime paths construct fully usable services.
10. All quality gates pass.
