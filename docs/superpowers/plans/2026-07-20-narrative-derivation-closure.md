# Narrative Derivation Closure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every character derivation use only earlier narrative state, select semantic vocabulary candidates through controlled tags, and atomically replace its persisted sensations, memory, plot developments, and context tags.

**Architecture:** Keep `StoryService` as orchestration boundary and `SenseGenerator` as LLM abstraction. Extend the existing request/response contracts for a tag-selection pass, move vocabulary metadata lookup into `Vocab`, and add repository-owned narrative-time queries plus one transactional replacement writer. The existing uncommitted extended sensory dimensions and merged distilled vocabulary remain intact and are treated as pre-existing work.

**Tech Stack:** Rust 2024, Tokio, SQLx SQLite, Serde/serde_json/serde_yaml, Schemars, Rig 0.40 DeepSeek Extractor, futures, cargo fmt, cargo test, cargo clippy.

## Global Constraints

- Preserve `rig::providers::deepseek` and `deepseek::DEEPSEEK_V4_FLASH`; do not add dependencies or use an OpenAI compatibility layer.
- Keep `MockSenseGenerator` deterministic and preserve its no-key runtime fallback.
- Keep all existing vocabulary ID and category-aware sensory validation.
- Keep `derive_scene()` partial-failure semantics and concurrency cap of four.
- Treat current uncommitted vocabulary, sensory-field, schema, and distilled-assets changes as user work; never revert or stage them incidentally.
- Use `occurred_at` for narrative causality and `created_at` only for auditing and tie-breaking within the same scene time.
- Use replacement, not accumulation, for repeated `(character_id, scene_id)` derivation.

---

### Task 1: Define controlled-tag and persisted-plot contracts

**Files:**
- Modify: `src/models/plot.rs`
- Modify: `src/models/mod.rs`
- Modify: `src/llm/contract.rs`
- Modify: `src/llm/generator.rs`
- Test: `tests/models_test.rs`

**Interfaces:**
- Consumes: existing `Character`, `Scene`, `PlotDevelopment`, and `VocabularyId` domain types.
- Produces: `LlmContextTagSelection`, `ContextTagRequest`, `VocabularyCandidate`, `StoredPlotDevelopment`, and an extended `DerivationRequest`.
- Produces: `SenseGenerator::select_context_tags(&ContextTagRequest)` in addition to existing `derive(&DerivationRequest)`.

- [ ] **Step 1: Write failing model-contract tests**

Add to `tests/models_test.rs`:

```rust
use chrono::Utc;
use novels::llm::LlmContextTagSelection;
use novels::models::{CharacterId, PlotDevelopment, PlotDevelopmentKind, SceneId, StoredPlotDevelopment};

#[test]
fn stored_plot_development_keeps_narrative_identity() {
    let stored = StoredPlotDevelopment {
        character_id: CharacterId(uuid::Uuid::new_v4()),
        scene_id: SceneId(uuid::Uuid::new_v4()),
        development: PlotDevelopment {
            kind: PlotDevelopmentKind::SuspicionRaised,
            reason: "the witness changed their story".into(),
        },
        created_at: Utc::now(),
    };

    assert_eq!(stored.development.reason, "the witness changed their story");
}

#[test]
fn context_tag_selection_deserializes_empty_tags() {
    let selected: LlmContextTagSelection = serde_json::from_str(r#"{"tags":[]}"#).unwrap();
    assert!(selected.tags.is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```text
cargo test --test models_test stored_plot_development_keeps_narrative_identity
```

Expected: FAIL because `StoredPlotDevelopment` and `LlmContextTagSelection` do not exist.

- [ ] **Step 3: Add minimal domain and LLM types**

Add to `src/models/plot.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPlotDevelopment {
    pub character_id: CharacterId,
    pub scene_id: SceneId,
    pub development: PlotDevelopment,
    pub created_at: DateTime<Utc>,
}
```

Add `StoredPlotDevelopment` re-export in `src/models/mod.rs`. In `src/llm/contract.rs`, define the structured extractor type and re-export it from `src/llm/mod.rs`:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct LlmContextTagSelection {
    #[serde(default)]
    pub tags: Vec<String>,
}
```

In `src/llm/generator.rs`, add:

```rust
#[derive(Debug, Clone)]
pub struct VocabularyCandidate {
    pub id: String,
    pub sense: String,
    pub text: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ContextTagRequest {
    pub character: Character,
    pub scene: Scene,
    pub prior_plot_developments: Vec<StoredPlotDevelopment>,
    pub available_tags: Vec<String>,
}
```

Replace `candidate_ids` and `candidate_tags` in `DerivationRequest` with `candidates: Vec<VocabularyCandidate>` and add `prior_plot_developments: Vec<StoredPlotDevelopment>`. Extend the trait:

```rust
async fn select_context_tags(
    &self,
    req: &ContextTagRequest,
) -> Result<LlmContextTagSelection, StoryError>;
```

- [ ] **Step 4: Run model tests**

Run:

```text
cargo test --test models_test
```

Expected: PASS.

- [ ] **Step 5: Commit**

```text
git add src/models/plot.rs src/models/mod.rs src/llm/contract.rs src/llm/generator.rs src/llm/mod.rs tests/models_test.rs
git commit -m "feat: define narrative derivation contracts"
```

### Task 2: Expose semantic vocabulary candidates and controlled tag fallback

**Files:**
- Modify: `src/vocab/loader.rs`
- Modify: `src/vocab/mod.rs`
- Test: `tests/vocab_test.rs`

**Interfaces:**
- Consumes: `VocabularyCandidate` from `llm::generator` and YAML-backed `VocabEntry`.
- Produces: `Vocab::known_tags() -> Vec<String>`, `Vocab::candidates_for_tags(&[String]) -> Vec<VocabularyCandidate>`, and `Vocab::filter_known_tags(&[String]) -> Vec<String>`.
- `candidates_for_tags` returns every entry when no supplied, valid tag matches an entry.

- [ ] **Step 1: Write failing semantic-candidate tests**

Add to `tests/vocab_test.rs`:

```rust
#[test]
fn candidates_for_tags_include_semantic_metadata() {
    let v = Vocab::load_from_str(sample_yaml()).unwrap();
    let candidates = v.candidates_for_tags(&["injury".to_string()]);

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].id, "visual.bloodstain");
    assert_eq!(candidates[0].sense, "visual");
    assert_eq!(candidates[0].text, "血迹");
    assert_eq!(candidates[0].tags, vec!["injury"]);
}

#[test]
fn candidates_for_tags_fall_back_when_tags_have_no_entry_match() {
    let v = Vocab::load_from_str(sample_yaml()).unwrap();
    let candidates = v.candidates_for_tags(&["known-but-unmatched".to_string()]);

    assert_eq!(candidates.len(), 2);
}
```

Use a fixture entry tagged `known-but-unmatched` in this test so `filter_known_tags` retains it but no current candidate has it.

- [ ] **Step 2: Run tests to verify they fail**

Run:

```text
cargo test --test vocab_test candidates_for_tags_include_semantic_metadata
```

Expected: FAIL because `Vocab::candidates_for_tags` does not exist.

- [ ] **Step 3: Implement vocabulary metadata projection and fallback**

Add helper methods to `src/vocab/loader.rs` that iterate categories and entries in stable sorted order. Construct `VocabularyCandidate` with `id: format!("{sense}.{key}")`, cloned entry text, and tags. Implement matching as `entry.tags.iter().any(|tag| selected.contains(tag))`.

Implement fallback exactly as:

```rust
pub fn candidates_for_tags(&self, selected: &[String]) -> Vec<VocabularyCandidate> {
    let matched = self.collect_candidates(Some(selected));
    if matched.is_empty() {
        self.collect_candidates(None)
    } else {
        matched
    }
}
```

`known_tags()` must deduplicate and sort all entry tags. `filter_known_tags()` must retain only tags in this set, deduplicate, and sort them.

- [ ] **Step 4: Run vocabulary tests**

Run:

```text
cargo test --test vocab_test
```

Expected: PASS, including existing extended-category and distilled-vocabulary tests.

- [ ] **Step 5: Commit**

```text
git add src/vocab/loader.rs src/vocab/mod.rs tests/vocab_test.rs
git commit -m "feat: expose semantic vocabulary candidates"
```

### Task 3: Persist plot developments and derivation context tags

**Files:**
- Create: `src/db/plot_repo.rs`
- Modify: `src/db/schema.rs`
- Modify: `src/db/mod.rs`

**Interfaces:**
- Consumes: `PlotDevelopment`, `StoredPlotDevelopment`, `CharacterId`, `SceneId`, and a `Sqlite` transaction.
- Produces: `Db::plots() -> &PlotRepo`, `PlotRepo::list_before_scene(...)`, and transaction-local plot insertion.
- Produces: schema tables `character_plot_developments` and `character_derivation_context_tags` with cascade foreign keys.

- [ ] **Step 1: Add schema and repository**

Append tables to `SCHEMA_SQL`:

```sql
CREATE TABLE IF NOT EXISTS character_plot_developments (
    id TEXT PRIMARY KEY,
    character_id TEXT NOT NULL,
    scene_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE,
    FOREIGN KEY (scene_id) REFERENCES scenes(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_plot_developments_character_scene
    ON character_plot_developments(character_id, scene_id);
CREATE TABLE IF NOT EXISTS character_derivation_context_tags (
    character_id TEXT NOT NULL,
    scene_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    PRIMARY KEY (character_id, scene_id, tag),
    FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE,
    FOREIGN KEY (scene_id) REFERENCES scenes(id) ON DELETE CASCADE
);
```

Implement `PlotRepo::list_before_scene(character_id, before)` with a join to `scenes`, filter `s.occurred_at < ?`, and order `s.occurred_at DESC, p.created_at DESC`. Parse enum JSON exactly as existing memory enums do; return malformed values as `StoryError::Database`.

- [ ] **Step 2: Run compile check**

Run:

```text
cargo check --all-targets
```

Expected: the crate compiles, or the documented Windows Lance baseline fails before this crate compiles. Plot behavior is verified through `replace_derivation` in Task 4.

- [ ] **Step 3: Commit**

```text
git add src/db/plot_repo.rs src/db/schema.rs src/db/mod.rs
git commit -m "feat: persist narrative plot developments"
```

### Task 4: Add narrative-time reads and atomic replacement persistence

**Files:**
- Modify: `src/db/memory_repo.rs`
- Modify: `src/db/sensation_repo.rs`
- Modify: `src/db/derivation_repo.rs`
- Modify: `src/db/plot_repo.rs`
- Test: `tests/db_test.rs`

**Interfaces:**
- Consumes: current scene `occurred_at`, generated sensations, memory draft, plot developments, and selected context tags.
- Produces: `MemoryRepo::list_before_scene`, `SensationRepo::latest_before_scene`, and `DerivationRepo::replace_derivation`.
- `replace_derivation` deletes old state for exactly one `(character_id, scene_id)` and inserts all replacement state in one transaction.

- [ ] **Step 1: Write failing temporal-isolation and replacement tests**

Add to `tests/db_test.rs`:

```rust
async fn replace_for_test(
    db: &Db,
    character_id: CharacterId,
    scene_id: SceneId,
    memory: &str,
    reason: &str,
    tags: &[String],
) {
    db.derivations()
        .replace_derivation(
            character_id,
            scene_id,
            &SensorySelection {
                visual_ids: vec![VocabularyId::new("visual.x").unwrap()],
                ..Default::default()
            },
            &CharacterMemoryDraft {
                content: memory.into(),
                source: MemorySource::Witnessed,
                certainty: Certainty::Certain,
            },
            &[PlotDevelopment {
                kind: PlotDevelopmentKind::NewClue,
                reason: reason.into(),
            }],
            tags,
            Utc::now(),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn narrative_context_excludes_future_scene_state() {
    let db = Db::open_in_memory().await.unwrap();
    let cid = CharacterId(Uuid::new_v4());
    let early = SceneId(Uuid::new_v4());
    let current = SceneId(Uuid::new_v4());
    let future = SceneId(Uuid::new_v4());
    let t1 = Utc::now();
    let t2 = t1 + chrono::Duration::minutes(1);
    let t3 = t2 + chrono::Duration::minutes(1);
    db.characters().create(cid, "A", &[], &[]).await.unwrap();
    db.scenes().create(early, "early", &[cid], t1).await.unwrap();
    db.scenes().create(current, "current", &[cid], t2).await.unwrap();
    db.scenes().create(future, "future", &[cid], t3).await.unwrap();
    replace_for_test(&db, cid, future, "future memory", "future clue", &[]).await;

    assert!(db.memories().list_before_scene(cid, t2, 50).await.unwrap().is_empty());
    assert!(db.sensations().latest_before_scene(cid, t2).await.unwrap().is_none());
    assert!(db.plots().list_before_scene(cid, t2).await.unwrap().is_empty());
}

#[tokio::test]
async fn replacement_derivation_leaves_one_state_set() {
    let db = Db::open_in_memory().await.unwrap();
    let cid = CharacterId(Uuid::new_v4());
    let sid = SceneId(Uuid::new_v4());
    let at = Utc::now();
    db.characters().create(cid, "A", &[], &[]).await.unwrap();
    db.scenes().create(sid, "event", &[cid], at).await.unwrap();
    replace_for_test(&db, cid, sid, "first", "first clue", &["old".into()]).await;
    replace_for_test(&db, cid, sid, "second", "second clue", &["new".into()]).await;

    assert_eq!(db.memories().list(cid, 50).await.unwrap()[0].content, "second");
    assert_eq!(db.sensations().latest(cid).await.unwrap().unwrap().0.visual_ids.len(), 1);
    assert_eq!(db.plots().list_before_scene(cid, at + chrono::Duration::seconds(1)).await.unwrap()[0].development.reason, "second clue");
    let tag_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM character_derivation_context_tags WHERE character_id = ? AND scene_id = ? AND tag = 'new'",
    ).bind(cid.0.to_string()).bind(sid.0.to_string()).fetch_one(db.pool()).await.unwrap();
    assert_eq!(tag_count, 1);
}

#[tokio::test]
async fn replacement_rolls_back_without_erasing_prior_state() {
    let db = Db::open_in_memory().await.unwrap();
    let cid = CharacterId(Uuid::new_v4());
    let sid = SceneId(Uuid::new_v4());
    db.characters().create(cid, "A", &[], &[]).await.unwrap();
    db.scenes().create(sid, "event", &[cid], Utc::now()).await.unwrap();
    replace_for_test(&db, cid, sid, "first", "first clue", &[]).await;

    let missing_scene = SceneId(Uuid::new_v4());
    let result = db.derivations().replace_derivation(
        cid, missing_scene, &SensorySelection::default(),
        &CharacterMemoryDraft { content: "bad".into(), source: MemorySource::Witnessed, certainty: Certainty::Certain },
        &[], &[], Utc::now(),
    ).await;
    assert!(matches!(result, Err(StoryError::Database(_))));
    assert_eq!(db.memories().list(cid, 50).await.unwrap()[0].content, "first");
}
```

Build all fixtures through existing `Db::open_in_memory()`, repositories, `Utc` timestamps, and valid `VocabularyId::new("visual.x")` values.

- [ ] **Step 2: Run tests to verify they fail**

Run:

```text
cargo test --test db_test narrative_context_excludes_future_scene_state
cargo test --test db_test replacement_derivation_leaves_one_state_set
cargo test --test db_test replacement_rolls_back_without_erasing_prior_state
```

Expected: FAIL because temporal reads and replacement writer do not exist.

- [ ] **Step 3: Implement narrative-time repository queries**

Add `MemoryRepo::list_before_scene(character_id, before, limit)` using:

```sql
SELECT m.id, m.scene_id, m.content, m.source, m.certainty, m.created_at
FROM character_memories m
JOIN scenes s ON s.id = m.scene_id
WHERE m.character_id = ? AND s.occurred_at < ?
ORDER BY s.occurred_at DESC, m.created_at DESC
LIMIT ?
```

Add `SensationRepo::latest_before_scene(character_id, before)` with the same join and filter, `ORDER BY s.occurred_at DESC, cs.created_at DESC LIMIT 1`, and current strict JSON parsing of all eight dimensions.

- [ ] **Step 4: Implement transactional replacement**

Replace `insert_derivation` with:

```rust
pub async fn replace_derivation(
    &self,
    character_id: CharacterId,
    scene_id: SceneId,
    sensations: &SensorySelection,
    memory: &CharacterMemoryDraft,
    plots: &[PlotDevelopment],
    context_tags: &[String],
    now: DateTime<Utc>,
) -> Result<(), StoryError>
```

Inside one `pool.begin()` transaction, delete rows for the exact character and scene from `character_sensations`, `character_memories`, `character_plot_developments`, and `character_derivation_context_tags`; call existing static sensation and memory inserts; call `PlotRepo::insert_in_tx`; insert every selected tag; commit only after all inserts pass.

- [ ] **Step 5: Run focused database tests**

Run:

```text
cargo test --test db_test
```

Expected: PASS, including existing `derivation_tx_atomic_on_memory_failure` updated to use replacement semantics.

- [ ] **Step 6: Commit**

```text
git add src/db/memory_repo.rs src/db/sensation_repo.rs src/db/derivation_repo.rs src/db/plot_repo.rs tests/db_test.rs
git commit -m "feat: replace derivations by narrative time"
```

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

### Task 6: Wire StoryService to the complete causal loop

**Files:**
- Modify: `src/scene/service.rs`
- Test: `tests/scene_test.rs`
- Test: `tests/e2e.rs`

**Interfaces:**
- Consumes: narrative-time repository APIs, `Vocab` metadata APIs, `SenseGenerator::select_context_tags`, and `DerivationRepo::replace_derivation`.
- Produces: `derive_character` with temporal context, controlled candidate filtering, retained retry behavior, and replacement persistence.

- [ ] **Step 1: Write failing end-to-end behavior tests**

Add to `tests/scene_test.rs`:

```rust
#[tokio::test]
async fn derive_character_uses_earlier_plot_and_replaces_same_scene() {
    let (db, service, generator, cid, early, later) = narrative_service_fixture(
        LlmContextTagSelection { tags: vec!["injury".into()] },
        vec![derivation("early memory", "early plot"), derivation("first later", "first plot"), derivation("second later", "second plot")],
    ).await;
    service.derive_character(early, cid).await.unwrap();
    service.derive_character(later, cid).await.unwrap();
    service.derive_character(later, cid).await.unwrap();

    let requests = generator.derivation_requests();
    assert_eq!(requests[1].prior_plot_developments[0].development.reason, "early plot");
    assert_eq!(db.memories().list(cid, 50).await.unwrap()[0].content, "second later");
}

#[tokio::test]
async fn empty_selected_tags_use_all_vocabulary_candidates() {
    let (_db, service, generator, cid, early, _later) = narrative_service_fixture(
        LlmContextTagSelection::default(),
        vec![derivation("memory", "plot")],
    ).await;
    service.derive_character(early, cid).await.unwrap();

    let request = generator.derivation_requests().pop().unwrap();
    assert_eq!(request.candidates.len(), 2);
    assert!(request.candidates.iter().any(|candidate| candidate.id == "visual.bloodstain"));
    assert!(request.candidates.iter().any(|candidate| candidate.id == "auditory.footsteps"));
}

#[tokio::test]
async fn later_derivation_never_receives_future_scene_context() {
    let (db, service, generator, cid, early, later) = narrative_service_fixture(
        LlmContextTagSelection::default(),
        vec![derivation("future memory", "future plot"), derivation("early memory", "early plot")],
    ).await;
    service.derive_character(later, cid).await.unwrap();
    service.derive_character(early, cid).await.unwrap();

    let request = generator.derivation_requests().pop().unwrap();
    assert!(request.recent_memories.is_empty());
    assert!(request.last_sensation.is_none());
    assert!(request.prior_plot_developments.is_empty());
    assert_eq!(db.memories().list(cid, 50).await.unwrap()[0].content, "early memory");
}
```

Define `narrative_service_fixture`, `derivation`, and `RecordingGenerator` directly above these tests. `RecordingGenerator` stores `Vec<DerivationRequest>` and `Vec<ContextTagRequest>` behind `Mutex`, pops deterministic tag and derivation responses from `Mutex<VecDeque<_>>`, and implements both `SenseGenerator` methods by recording a clone before returning the next queued response. The fixture loads visual `bloodstain` tagged `injury` plus auditory `footsteps` tagged `movement`, creates one character, and creates `early` then `later` scenes one minute apart.

Extend `tests/e2e.rs` canned mock construction with default tag selection and assert an E2E scene derivation still succeeds.

- [ ] **Step 2: Run tests to verify they fail**

Run:

```text
cargo test --test scene_test derive_character_uses_earlier_plot_and_replaces_same_scene
cargo test --test scene_test empty_selected_tags_use_all_vocabulary_candidates
cargo test --test scene_test later_derivation_never_receives_future_scene_context
```

Expected: FAIL because `StoryService` still reads by insertion time and supplies an all-vocabulary ID set directly.

- [ ] **Step 3: Implement ordered orchestration**

In `derive_character`, after validating scene, character, and participation:

1. Query `memories().list_before_scene(character_id, scene.occurred_at, MEMORY_LIMIT)`.
2. Query `sensations().latest_before_scene(character_id, scene.occurred_at)`.
3. Query `plots().list_before_scene(character_id, scene.occurred_at)`.
4. Build `ContextTagRequest` with `vocab.known_tags()` and call `select_context_tags`.
5. Sanitize the response with `vocab.filter_known_tags(&raw_tags.tags)`.
6. Build candidates with `vocab.candidates_for_tags(&selected_tags)`.
7. Build `DerivationRequest` from temporal context, prior plots, and semantic candidates.
8. Use the existing one-retry sensory validation against a `HashSet<String>` projected from candidates.
9. Call `replace_derivation` with cleaned sensations, same-response memory and plots, selected tags, and `Utc::now()`.

Do not modify `derive_scene` stream shape or `CONCURRENCY`.

- [ ] **Step 4: Run scene and end-to-end tests**

Run:

```text
cargo test --test scene_test
cargo test --test e2e
```

Expected: PASS.

- [ ] **Step 5: Commit**

```text
git add src/scene/service.rs tests/scene_test.rs tests/e2e.rs
git commit -m "feat: close narrative derivation loop"
```

### Task 7: Verify integration and review regression boundaries

**Files:**
- Modify: `docs/superpowers/specs/2026-07-20-narrative-derivation-closure-design.md` only if acceptance criteria need evidence notes.

**Interfaces:**
- Consumes: completed implementation and all existing integration tests.
- Produces: verification evidence; no production API changes.

- [ ] **Step 1: Format source**

Run:

```text
cargo fmt --all
cargo fmt --all -- --check
```

Expected: formatting check exits 0.

- [ ] **Step 2: Run complete test suite**

Run:

```text
cargo test --all-targets
```

Expected: all tests pass. If Windows Lance build tooling fails before this crate compiles, capture and compare it with documented baseline instead of attributing it to this work.

- [ ] **Step 3: Run static quality gates**

Run:

```text
cargo check --all-targets
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: exit 0, or documented baseline Lance failure before project compilation.

- [ ] **Step 4: Review changed behavior against acceptance criteria**

Verify each item in `docs/superpowers/specs/2026-07-20-narrative-derivation-closure-design.md`:

```text
1. Future state excluded by narrative-time tests.
2. Same-scene replacement covered by DB and service tests.
3. Prior persisted plot appears in later request test.
4. Candidate text and tags observed in request test.
5. Selected-tag filtering and fallback covered by vocabulary and service tests.
6. Existing invalid-selection retry and four-way bounded concurrency remain covered.
7. Focused and complete quality commands have recorded outcomes.
```

- [ ] **Step 5: Commit verification notes only when changed**

```text
git add docs/superpowers/specs/2026-07-20-narrative-derivation-closure-design.md
git commit -m "docs: record narrative derivation verification"
```

Skip this commit when no documentation changed.

## Plan Self-Review

- Spec coverage: Tasks 1-2 implement controlled tag contracts and semantic candidates; Tasks 3-4 implement durable state, narrative-time reads, and replacement transaction; Tasks 5-6 implement both LLM passes and orchestration; Task 7 verifies every acceptance criterion.
- Placeholder scan: no TBD, deferred implementation, test-only persistence APIs, or unspecified error handling remains.
- Type consistency: `LlmContextTagSelection` is the extractor output; `ContextTagRequest` is generator input; `VocabularyCandidate` is derivation input; `StoredPlotDevelopment` is persistence-query output; `replace_derivation` is the only multi-state writer.
