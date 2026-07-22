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

