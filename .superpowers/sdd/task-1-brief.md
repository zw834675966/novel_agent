### Task 1: Define the Internal Deterministic Prose Module

**Files:**
- Create or reconcile: `src/prose/contract.rs`
- Create or reconcile: `src/prose/generator.rs`
- Create or reconcile: `src/prose/assembly.rs`
- Create or reconcile: `src/prose/mock.rs`
- Create or reconcile: `src/prose/mod.rs`
- Modify: `src/lib.rs`
- Test: `tests/prose_test.rs`

**Interfaces:**
- Produces public `LlmNarrative`, `NarrativeBeat`, `NarrateRequest`, `ProseCandidate`, `CharacterProseCandidates`, `ProseGenerator`, `MockProseGenerator`, and `AssembledProse`.
- Keeps `assemble`, candidate collection, participant-set construction, and fixed category order crate-private.
- `AssembledProse` contains `text`, `stripped_refs`, `rejected_beats`, and `action_only_beats`.

- [ ] **Step 1: Write failing assembly and contract tests**

Extend `tests/prose_test.rs` with these behavioral tests, using existing scene/character/derivation fixtures:

```rust
#[test]
fn all_invalid_refs_keep_action_and_count_degradation() {
    let prose = assemble_fixture(
        &["emotion.fabricated"],
        &["emotion.sorrow"],
        "她落座。",
    )
    .unwrap();

    assert_eq!(prose.text, "她落座。");
    assert_eq!(prose.stripped_refs, 1);
    assert_eq!(prose.action_only_beats, 1);
}

#[test]
fn empty_refs_keep_action_and_count_degradation() {
    let prose = assemble_fixture(&[], &[], "她转身离开。").unwrap();
    assert_eq!(prose.text, "她转身离开。");
    assert_eq!(prose.action_only_beats, 1);
}

#[test]
fn empty_narrative_is_an_llm_error() {
    let err = assemble_empty_narrative_fixture().unwrap_err();
    assert!(matches!(err, StoryError::Llm(message) if message.contains("no beats")));
}

#[test]
fn cross_character_ref_is_stripped() {
    let prose = assemble_cross_character_fixture().unwrap();
    assert_eq!(prose.stripped_refs, 1);
    assert_eq!(prose.action_only_beats, 1);
}

#[test]
fn missing_derivation_pov_is_rejected() {
    let prose = assemble_missing_derivation_fixture().unwrap();
    assert_eq!(prose.rejected_beats, 1);
    assert!(prose.text.is_empty());
}
```

Complete the existing category-order fixture so all eight categories are present and assert their text offsets increase in this order:

```text
atmosphere < visual < auditory < olfactory < tactile < gustatory < emotion < gesture
```

- [ ] **Step 2: Run focused tests and confirm red state**

Run:

```text
cargo test --test prose_test
```

Expected: FAIL because `action_only_beats`, hard empty-beat validation, and several fixture helpers do not exist.

- [ ] **Step 3: Implement typed requests and deterministic assembly**

Use these request-side contracts in `src/prose/generator.rs`:

```rust
#[derive(Debug, Clone)]
pub struct ProseCandidate {
    pub id: String,
    pub sense: String,
    pub text: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CharacterProseCandidates {
    pub character_id: CharacterId,
    pub candidates: Vec<ProseCandidate>,
}

#[derive(Debug, Clone)]
pub struct NarrateRequest {
    pub scene: Scene,
    pub characters: Vec<Character>,
    pub derivations: Vec<CharacterDerivation>,
    pub candidates: Vec<CharacterProseCandidates>,
}
```

Keep `ProseGenerator` at one method:

```rust
#[async_trait::async_trait]
pub trait ProseGenerator: Send + Sync {
    async fn narrate(&self, request: &NarrateRequest)
        -> Result<LlmNarrative, StoryError>;
}
```

Implement `AssembledProse` exactly as:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProse {
    pub text: String,
    pub stripped_refs: usize,
    pub rejected_beats: usize,
    pub action_only_beats: usize,
}
```

Assembly must:

1. Return `StoryError::Llm("prose generator returned no beats".into())` when `beats.is_empty()`.
2. Reject a beat when POV is not a participant or has no matching derivation.
3. Strip a reference unless it belongs to that POV derivation and resolves through `Vocab`.
4. Group accepted text by the fixed eight-category order.
5. Preserve reference order within each category and beat order across the narrative.
6. Increment `action_only_beats` whenever an accepted beat resolves no descriptive text.
7. Emit action-only beats when action is non-empty.
8. Join accepted non-empty beats with one newline.

Do not publicly re-export `assemble`, `candidate_refs_for`, `participant_set`, or candidate-building helpers from `src/prose/mod.rs`.

- [ ] **Step 4: Make Mock behavior deterministic**

Implement two constructors:

```rust
pub fn new(response: LlmNarrative) -> Self;
pub fn fallback() -> Self;
```

`new` returns the fixed response. `fallback` creates one beat for the first request character, uses the scene objective event as action, and selects the first candidate ID for that character when available. If no character exists, it returns an empty beat list so service validation reports the hard error.

- [ ] **Step 5: Run module tests and quality checks**

Run:

```text
cargo test --test prose_test
cargo fmt --all -- --check
cargo check --lib
cargo clippy --lib --test prose_test -- -D warnings
```

Expected: all prose tests pass and checks exit 0.

- [ ] **Step 6: Commit**

```text
git add src/prose/contract.rs src/prose/generator.rs src/prose/assembly.rs src/prose/mock.rs src/prose/mod.rs src/lib.rs tests/prose_test.rs
git commit -m "feat: add deterministic prose module"
```

