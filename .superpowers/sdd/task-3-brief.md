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

