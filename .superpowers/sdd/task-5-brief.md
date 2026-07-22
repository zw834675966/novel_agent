### Task 5: Implement two structured LLM passes with semantic prompts

**Files:**
- Modify: `src/llm/rig_impl.rs`
- Modify: `src/llm/mock.rs`
- Modify: `src/llm/mod.rs`
- Test: `tests/scene_test.rs`

**Interfaces:**
- Consumes: extended `SenseGenerator`, `ContextTagRequest`, `DerivationRequest`, `LlmContextTagSelection`, and semantic `VocabularyCandidate` values.
- Produces: a production tag extractor and derivation extractor, plus mock responses for both calls.

- [ ] **Step 1: Write failing prompt-observation test**

Add a test-only generator in `tests/scene_test.rs` that records its `ContextTagRequest` and `DerivationRequest` in `Mutex<Option<_>>`, returns `LlmContextTagSelection { tags: vec!["injury".into()] }`, then returns a valid sensory result. Assert the derivation request candidate has:

```rust
assert_eq!(request.candidates[0].id, "visual.bloodstain");
assert_eq!(request.candidates[0].text, "血迹");
assert_eq!(request.candidates[0].tags, vec!["injury"]);
assert_eq!(tag_request.available_tags, vec!["injury".to_string()]);
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```text
cargo test --test scene_test derivation_passes_semantic_vocabulary_candidates
```

Expected: FAIL because contracts and mock implementation lack tag selection.

- [ ] **Step 3: Add two Rig extractors and prompts**

Change `RigSenseGenerator` to hold:

```rust
tag_extractor: Extractor<deepseek::CompletionModel, LlmContextTagSelection>,
derivation_extractor: Extractor<deepseek::CompletionModel, LlmCharacterDerivation>,
```

Build both with `client.extractor::<Type>(deepseek::DEEPSEEK_V4_FLASH).retries(1).build()`. Implement `select_context_tags` with a prompt containing character name/personality/skills, current objective event, prior plot kind and reason, and stable sorted available tags. Tell it to submit only listed tags.

Build derivation prompt from stable sorted candidates rendered as:

```text
- id: visual.bloodstain | sense: visual | text: 血迹 | tags: injury
```

Include earlier plot developments alongside memories and previous sensation. Retain structured extraction and map errors to `StoryError::Llm`.

- [ ] **Step 4: Make mock tag selection deterministic**

Extend `MockSenseGenerator` to hold `LlmContextTagSelection` and `LlmCharacterDerivation`; provide a constructor accepting both. Update `main.rs` fallback and every test fixture to construct `LlmContextTagSelection::default()` where no tags are required.

- [ ] **Step 5: Run focused service test**

Run:

```text
cargo test --test scene_test derivation_passes_semantic_vocabulary_candidates
```

Expected: PASS.

- [ ] **Step 6: Commit**

```text
git add src/llm/rig_impl.rs src/llm/mock.rs src/llm/mod.rs src/main.rs tests/scene_test.rs
git commit -m "feat: select tags before character derivation"
```

