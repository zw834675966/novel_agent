# Deterministic Prose Generator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the existing prose prototype into a deep `StoryService` module that generates a structured narrative skeleton through Rig and deterministically assembles source-backed prose with observable partial success.

**Architecture:** `StoryService` owns `Db`, `Vocab`, `SenseGenerator`, and `ProseGenerator`. `ProseGenerator` returns only typed beats; internal Rust assembly enforces POV ownership, vocabulary provenance, fixed category ordering, and quality counters. Rig remains the only LLM framework, and no template or workflow dependency is added.

**Tech Stack:** Rust 2024, Tokio, async-trait, Rig 0.40 DeepSeek Extractor, Serde, Schemars, SQLx SQLite.

## Global Constraints

- Preserve `rig::providers::deepseek` and `deepseek::DEEPSEEK_V4_FLASH`.
- Do not add MiniJinja, Askama, Tera, Swiftide, Kalosm, LangChain Rust, or a workflow engine.
- LLM output contains only POV, action skeleton, and offered vocabulary IDs.
- Accepted descriptive text must resolve from service-owned `Vocab`.
- Invalid references and invalid POV beats produce observable partial success; they do not discard valid output.
- Empty LLM beats are a hard `StoryError::Llm` failure.
- Identical inputs produce identical candidate ordering and assembly output.
- Preserve existing uncommitted prose, distillation, vocabulary, and tooling work; stage only task-owned hunks.
- Keep all existing character-derivation behavior, narrative-time semantics, validation retry, and scene concurrency cap of four.

---

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

### Task 2: Implement the Rig Prose Adapter Without Vocabulary Ownership

**Files:**
- Create or reconcile: `src/prose/rig_impl.rs`
- Modify: `src/prose/mod.rs`
- Test: `src/prose/rig_impl.rs`

**Interfaces:**
- Consumes `NarrateRequest` semantic candidate metadata from Task 1.
- Produces `RigProseGenerator::new(client: deepseek::Client) -> Self`.
- Implements `ProseGenerator` through one `Extractor<deepseek::CompletionModel, LlmNarrative>`.

- [ ] **Step 1: Write failing adapter tests**

Add module tests beside `build_prompt`:

```rust
#[test]
fn prompt_contains_stable_semantic_candidates() {
    let request = request_with_unsorted_candidates();
    let prompt = build_prompt(&request);

    assert!(prompt.contains(
        "id: visual.bloodstain | sense: visual | text: 血迹 | tags: crime, injury"
    ));
    assert!(prompt.find("visual.bloodstain").unwrap()
        < prompt.find("gesture.weep").unwrap());
}

#[test]
fn prompt_restricts_action_and_reference_output() {
    let prompt = build_prompt(&minimal_request());
    assert!(prompt.contains("action 只写客观动作和对话"));
    assert!(prompt.contains("sensation_refs 只能从提供的候选"));
}
```

- [ ] **Step 2: Run tests and confirm red state**

Run:

```text
cargo test prose::rig_impl::tests
```

Expected: FAIL because the current adapter owns `Arc<Vocab>`, prints only IDs, and does not sort semantic metadata.

- [ ] **Step 3: Implement the minimal adapter**

Use this shape:

```rust
pub struct RigProseGenerator {
    extractor: Extractor<deepseek::CompletionModel, LlmNarrative>,
}

impl RigProseGenerator {
    pub fn new(client: deepseek::Client) -> Self {
        let extractor = client
            .extractor::<LlmNarrative>(deepseek::DEEPSEEK_V4_FLASH)
            .retries(1)
            .build();
        Self { extractor }
    }
}
```

Prompt construction must clone and sort characters by typed ID, character candidate groups by typed ID, candidates by ID, and tags lexicographically. Render candidates as:

```text
id: visual.bloodstain | sense: visual | text: 血迹 | tags: crime, injury
```

The adapter must not import `Vocab`, query `Db`, validate references, or assemble prose. Extraction errors map to `StoryError::Llm`.

- [ ] **Step 4: Run adapter and prose tests**

Run:

```text
cargo test prose::rig_impl::tests
cargo test --test prose_test
cargo fmt --all -- --check
cargo clippy --lib --test prose_test -- -D warnings
```

Expected: all tests and checks pass.

- [ ] **Step 5: Commit**

```text
git add src/prose/rig_impl.rs src/prose/mod.rs
git commit -m "feat: add Rig prose adapter"
```

### Task 3: Deepen StoryService Around Narration

**Files:**
- Modify: `src/scene/service.rs`
- Modify: `src/models/error.rs`
- Modify: `src/main.rs`
- Modify: `tests/scene_test.rs`
- Modify: `tests/e2e.rs`
- Modify: `tests/prose_test.rs`

**Interfaces:**
- Consumes Task 1 `ProseGenerator`, `NarrateRequest`, internal assembly, and Task 2 Rig adapter.
- Produces `StoryService::new(db, vocab, sense_generator, prose_generator)` and `StoryService::narrate_scene(scene_id, derivations)`.
- Removes the public `StoryService::vocab()` escape hatch and per-call generator argument.

- [ ] **Step 1: Write failing StoryService narration tests**

Add service-level tests in `tests/prose_test.rs`:

```rust
#[tokio::test]
async fn missing_scene_fails_before_prose_generator() {
    let recorder = Arc::new(RecordingProseGenerator::default());
    let service = service_fixture(recorder.clone()).await;

    let result = service
        .narrate_scene(SceneId(Uuid::new_v4()), &[])
        .await;

    assert!(matches!(result, Err(StoryError::SceneNotFound(_))));
    assert_eq!(recorder.calls(), 0);
}

#[tokio::test]
async fn derivation_from_another_scene_fails_before_generator() {
    let fixture = service_with_scene().await;
    let wrong = derivation_for(SceneId(Uuid::new_v4()), fixture.character_id);
    let result = fixture.service.narrate_scene(fixture.scene_id, &[wrong]).await;

    assert!(matches!(result, Err(StoryError::InvalidNarrationContext(_))));
    assert_eq!(fixture.generator.calls(), 0);
}

#[tokio::test]
async fn duplicate_character_derivations_fail_before_generator() {
    let fixture = service_with_scene().await;
    let derivation = derivation_for(fixture.scene_id, fixture.character_id);
    let result = fixture
        .service
        .narrate_scene(fixture.scene_id, &[derivation.clone(), derivation])
        .await;

    assert!(matches!(result, Err(StoryError::InvalidNarrationContext(_))));
    assert_eq!(fixture.generator.calls(), 0);
}

#[tokio::test]
async fn narration_returns_source_text_and_quality_counters() {
    let fixture = service_with_partial_success_response().await;
    let prose = fixture
        .service
        .narrate_scene(fixture.scene_id, &fixture.derivations)
        .await
        .unwrap();

    assert!(prose.text.contains("血迹"));
    assert!(prose.text.contains("她俯身查看。"));
    assert_eq!(prose.stripped_refs, 1);
    assert_eq!(prose.action_only_beats, 1);
}
```

`RecordingProseGenerator` increments an `AtomicUsize`, records cloned requests behind a `Mutex`, and returns a configured `LlmNarrative`.

- [ ] **Step 2: Run focused tests and confirm red state**

Run:

```text
cargo test --test prose_test
```

Expected: FAIL because `StoryService` does not own `ProseGenerator`, accepts it per call, and lacks `InvalidNarrationContext`.

- [ ] **Step 3: Add explicit narration-context errors**

Add one error variant:

```rust
#[error("invalid narration context: {0}")]
InvalidNarrationContext(String),
```

Use it for wrong-scene derivations, duplicate character derivations, missing participant characters, and derivations for non-participants. Keep generator/extractor failures as `StoryError::Llm`.

- [ ] **Step 4: Make StoryService own the prose seam**

Rename the existing sense field for clarity and add the prose field:

```rust
pub struct StoryService {
    db: Db,
    vocab: Vocab,
    sense_generator: Arc<dyn SenseGenerator>,
    prose_generator: Arc<dyn ProseGenerator>,
}
```

Constructor:

```rust
pub fn new(
    db: Db,
    vocab: Vocab,
    sense_generator: Arc<dyn SenseGenerator>,
    prose_generator: Arc<dyn ProseGenerator>,
) -> Self;
```

Before invoking the prose generator, `narrate_scene` must:

1. Load the scene or return `SceneNotFound`.
2. Require every derivation `scene_id` to equal the requested scene.
3. Require every derivation character to be a participant.
4. Reject duplicate character derivations.
5. Load every participant character or return `CharacterNotFound`.
6. Build candidate metadata from service-owned `Vocab`; include only IDs present in each derivation and currently resolvable in `Vocab`.
7. Sort characters, derivations, candidate groups, candidates, and tags before creating `NarrateRequest`.
8. Call `self.prose_generator.narrate(&request)`.
9. Invoke internal assembly and return `AssembledProse`.

Remove `StoryService::vocab()` and the generator parameter from `narrate_scene`.

- [ ] **Step 5: Update all constructor call sites**

Update all nine current `StoryService::new` calls in `src/main.rs`, `tests/scene_test.rs`, and `tests/e2e.rs`. Existing derivation-only tests inject `Arc::new(MockProseGenerator::fallback())`.

Production startup calls `deepseek::Client::from_env()` once:

```rust
let (sense_generator, prose_generator): (
    Arc<dyn SenseGenerator>,
    Arc<dyn ProseGenerator>,
) = match deepseek::Client::from_env() {
    Ok(client) => (
        Arc::new(RigSenseGenerator::new(client.clone(), vocab.clone())),
        Arc::new(RigProseGenerator::new(client)),
    ),
    Err(_) => (
        Arc::new(MockSenseGenerator::new(/* existing deterministic output */)),
        Arc::new(MockProseGenerator::fallback()),
    ),
};
```

If DeepSeek client is not `Clone`, create both clients inside the same success branch with `deepseek::Client::from_env()` and keep the branch atomic; never configure only one production adapter.

Call narration as:

```rust
let prose = service.narrate_scene(scene_id, &derivations).await?;
```

Print `stripped_refs`, `rejected_beats`, and `action_only_beats` in the demo.

- [ ] **Step 6: Run focused and full tests**

Run:

```text
cargo test --test prose_test
cargo test --test scene_test
cargo test --test e2e
cargo fmt --all -- --check
cargo test --all-targets
cargo check --all-targets
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: all tests and quality gates pass.

- [ ] **Step 7: Commit**

```text
git add src/scene/service.rs src/models/error.rs src/main.rs tests/scene_test.rs tests/e2e.rs tests/prose_test.rs
git commit -m "feat: integrate deterministic scene narration"
```

### Task 4: Final Scope and Architecture Review

**Files:**
- Modify only files required by review findings.

**Interfaces:**
- Consumes the complete implementation.
- Produces verification evidence and no new public capability.

- [ ] **Step 1: Verify public surface**

Run code search and confirm:

```text
StoryService::narrate_scene has exactly two explicit inputs: scene_id and derivations.
StoryService has no public vocab accessor.
RigProseGenerator::new accepts only DeepSeek client.
assemble and candidate construction are not publicly re-exported.
No added template, RAG, alternate LLM, or workflow dependency exists.
```

- [ ] **Step 2: Run complete quality gates**

Run:

```text
cargo fmt --all -- --check
cargo test --all-targets
cargo check --all-targets
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
```

Expected: all commands exit 0.

- [ ] **Step 3: Review every acceptance criterion**

Verify against `docs/superpowers/specs/2026-07-21-deterministic-prose-generator-design.md`:

```text
1. No generator argument on narrate_scene.
2. StoryService owns ProseGenerator.
3. RigProseGenerator does not own Vocab.
4. Accepted descriptions resolve through service-owned Vocab.
5. Invalid refs and POV beats preserve valid output and increment counters.
6. Action-only beats increment action_only_beats.
7. Empty beats fail.
8. All eight categories have deterministic-order coverage.
9. Production and no-key paths construct both adapters.
10. All quality gates pass.
11. No retry, template engine, or free-form rewrite was added.
```

- [ ] **Step 4: Commit review fixes only when needed**

Use one focused commit containing only fixes required by final review. Skip this step when review is clean.

## Plan Self-Review

- Spec coverage: Task 1 covers contracts, deterministic assembly, quality counters, partial success, and empty-beat failure. Task 2 covers the isolated Rig adapter and semantic stable prompts. Task 3 covers the deep StoryService interface, validation, runtime fallback, and integration tests. Task 4 verifies every acceptance criterion.
- Placeholder scan: no TBD, TODO, unspecified validation, or deferred code step remains.
- Type consistency: `ProseCandidate`, `CharacterProseCandidates`, `NarrateRequest`, `LlmNarrative`, `ProseGenerator`, and `AssembledProse` retain the same names and field shapes across tasks.
- Existing worktree safety: current untracked `src/prose/` and `tests/prose_test.rs` are reconciled rather than deleted; unrelated distillation and tooling files remain unstaged.
