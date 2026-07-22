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

