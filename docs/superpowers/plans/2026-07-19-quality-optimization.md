# Rust Quality Optimization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复 Rust 项目中的推导结果不一致、词汇字段越界、数据库错误静默降级和更新不存在角色仍成功问题，并建立全绿质量门禁。

**Architecture:** 保持现有 models、Repository、vocab、LLM 和 StoryService 分层。只在现有边界内增加严格错误传播和完整推导结果校验，不引入新依赖、迁移或大规模重构。每个人物推导仍由一个 SQLite 事务写入感官和记忆。

**Tech Stack:** Rust 2024, Tokio, SQLx SQLite, Serde/serde_json, thiserror, futures, cargo fmt, cargo test, cargo clippy。

## Global Constraints

- 数据库持久化数据解析失败时严格返回 `StoryError::Database`。
- LLM 重试使用第二次完整结果，感官、记忆和剧情必须来自同一次响应。
- 两次感官结果都无效时不落库，返回 `InvalidVocabularySelection`。
- 部分非法感官 ID 被剥离，合法 ID 继续保留。
- 感官字段必须匹配自身类别。
- 不新增依赖，不修改数据库表结构，不引入迁移。
- 不修改用户已有的无关工作树变更。

---

### Task 1: Enforce sensory-field vocabulary ownership

**Files:**
- Modify: `src/vocab/validate.rs:27-65`
- Test: `tests/vocab_test.rs`

**Interfaces:**
- Consumes: existing `validate_selection(sel: &SensorySelection, candidates: &HashSet<String>) -> ValidationResult`.
- Produces: same public function and result type; each sensory vector now accepts only IDs with matching `<sense>.` prefix and membership in `candidates`.

- [ ] **Step 1: Write failing cross-field validation test**

Add to `tests/vocab_test.rs`:

```rust
#[test]
fn validate_rejects_candidate_from_wrong_sense_field() {
    let mut candidates = HashSet::new();
    candidates.insert("visual.bloodstain".to_string());
    candidates.insert("auditory.footsteps".to_string());

    let mut selection = SensorySelection::default();
    selection
        .visual_ids
        .push(VocabularyId::new("auditory.footsteps").unwrap());

    let result = validate(&selection, &candidates);

    assert!(result.cleaned.visual_ids.is_empty());
    assert_eq!(result.stripped, vec!["auditory.footsteps"]);
    assert!(result.all_empty);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```text
cargo test validate_rejects_candidate_from_wrong_sense_field
```

Expected: FAIL because current validation checks only global candidate membership.

- [ ] **Step 3: Implement field-aware filtering**

Replace the local cleaner in `src/vocab/validate.rs` with a sense-aware closure:

```rust
let mut clean = |sense: &str, ids: &[VocabularyId]| -> Vec<VocabularyId> {
    ids.iter()
        .filter_map(|id| {
            if id.sense() == sense && candidates.contains(id.as_str()) {
                Some(id.clone())
            } else {
                stripped.push(id.as_str().to_string());
                None
            }
        })
        .collect()
};

cleaned.visual_ids = clean("visual", &sel.visual_ids);
cleaned.auditory_ids = clean("auditory", &sel.auditory_ids);
cleaned.olfactory_ids = clean("olfactory", &sel.olfactory_ids);
cleaned.tactile_ids = clean("tactile", &sel.tactile_ids);
cleaned.gustatory_ids = clean("gustatory", &sel.gustatory_ids);
```

- [ ] **Step 4: Run vocabulary tests**

Run:

```text
cargo test --test vocab_test
```

Expected: all vocabulary tests pass.

- [ ] **Step 5: Commit**

```text
git add src/vocab/validate.rs tests/vocab_test.rs
git commit -m "fix: enforce sensory vocabulary ownership"
```

### Task 2: Make retry results atomic and complete

**Files:**
- Modify: `src/scene/service.rs:127-175`
- Test: `tests/scene_test.rs`

**Interfaces:**
- Consumes: `SenseGenerator::derive()` and existing `ValidationResult`.
- Produces: `validate_with_retry()` returns `(LlmCharacterDerivation, SensorySelection)` so persisted and returned memory, plot, and sensations come from one response.

- [ ] **Step 1: Write failing retry consistency test**

Add a sequence generator and test to `tests/scene_test.rs`:

```rust
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Mutex;

struct SequenceGenerator {
    responses: Mutex<VecDeque<LlmCharacterDerivation>>,
}

impl SequenceGenerator {
    fn new(responses: Vec<LlmCharacterDerivation>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
        }
    }
}

#[async_trait]
impl SenseGenerator for SequenceGenerator {
    async fn derive(
        &self,
        _req: &novels::llm::DerivationRequest,
    ) -> Result<LlmCharacterDerivation, StoryError> {
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| StoryError::Llm("sequence exhausted".into()))
    }
}

#[tokio::test]
async fn retry_uses_second_complete_derivation() {
    let db = Db::open_in_memory().await.unwrap();
    let vocab = Vocab::load_from_str(
        "visual:\n  valid:\n    text: valid\n    tags: []\n",
    )
    .unwrap();
    let first = LlmCharacterDerivation {
        sensations: SensorySelection {
            visual_ids: vec![VocabularyId::new("visual.invalid").unwrap()],
            ..Default::default()
        },
        new_memory: CharacterMemoryDraft {
            content: "first-memory".into(),
            source: MemorySource::Witnessed,
            certainty: Certainty::Certain,
        },
        plot_development: vec![],
    };
    let second = LlmCharacterDerivation {
        sensations: SensorySelection {
            visual_ids: vec![VocabularyId::new("visual.valid").unwrap()],
            ..Default::default()
        },
        new_memory: CharacterMemoryDraft {
            content: "second-memory".into(),
            source: MemorySource::Inferred,
            certainty: Certainty::Suspected,
        },
        plot_development: vec![PlotDevelopment {
            kind: PlotDevelopmentKind::NewClue,
            reason: "second-plot".into(),
        }],
    };
    let generator = Arc::new(SequenceGenerator::new(vec![first, second]));
    let svc = StoryService::new(db.clone(), vocab, generator);
    let cid = CharacterId(uuid::Uuid::new_v4());
    let sid = SceneId(uuid::Uuid::new_v4());
    db.characters().create(cid, "A", &[], &[]).await.unwrap();
    db.scenes().create(sid, "event", &[cid], chrono::Utc::now()).await.unwrap();

    let result = svc.derive_character(sid, cid).await.unwrap();

    assert_eq!(result.new_memory.content, "second-memory");
    assert_eq!(result.plot_development[0].reason, "second-plot");
    assert_eq!(result.sensations.visual_ids[0].as_str(), "visual.valid");
    assert_eq!(db.memories().list(cid, 50).await.unwrap()[0].content, "second-memory");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```text
cargo test --test scene_test retry_uses_second_complete_derivation
```

Expected: FAIL because current code returns and persists first response memory/plot.

- [ ] **Step 3: Implement complete-result retry**

Change the helper to return a complete derivation and cleaned sensations. The second attempt must replace the entire result:

```rust
async fn validate_with_retry(
    &self,
    req: &DerivationRequest,
    raw: LlmCharacterDerivation,
    candidate_set: &HashSet<String>,
) -> Result<(LlmCharacterDerivation, SensorySelection), StoryError> {
    let v1 = crate::vocab::validate(&raw.sensations, candidate_set);
    if !v1.all_empty {
        return Ok((raw, v1.cleaned));
    }

    let raw2 = self.generator.derive(req).await?;
    let v2 = crate::vocab::validate(&raw2.sensations, candidate_set);
    if v2.all_empty {
        return Err(StoryError::InvalidVocabularySelection(
            "all senses empty after retry".into(),
        ));
    }
    Ok((raw2, v2.cleaned))
}
```

Update `derive_character()` to call it with owned `raw`, then use returned `derivation` for memory and plot:

```rust
let (derivation, sensations) =
    self.validate_with_retry(&req, raw, &candidate_set).await?;
self.db
    .derivations()
    .insert_derivation(character_id, scene_id, &sensations, &derivation.new_memory, now)
    .await?;
Ok(CharacterDerivation {
    character_id,
    scene_id,
    sensations,
    new_memory: derivation.new_memory,
    plot_development: derivation.plot_development,
})
```

- [ ] **Step 4: Add no-write retry failure test**

Use two `LlmCharacterDerivation` values with invalid `visual_ids`, invoke `derive_character()`, assert `InvalidVocabularySelection`, then assert `memories().list(cid, 50)` is empty and `sensations().latest(cid)` is `None`.

- [ ] **Step 5: Run scene tests**

Run:

```text
cargo test --test scene_test
```

Expected: all scene tests pass, including retry and no-write cases.

- [ ] **Step 6: Commit**

```text
git add src/scene/service.rs tests/scene_test.rs
git commit -m "fix: keep retry derivation results consistent"
```

### Task 3: Propagate strict database parsing errors

**Files:**
- Modify: `src/db/character_repo.rs:101-148`
- Modify: `src/db/scene_repo.rs:45-76`
- Modify: `src/db/memory_repo.rs:43-68, 85-97`
- Modify: `src/db/sensation_repo.rs:20-37, 50-81`
- Test: `tests/db_test.rs`

**Interfaces:**
- Consumes: existing Repository methods and `StoryError::Database`.
- Produces: same Repository method signatures, but malformed stored values return errors instead of defaults or dropped values.

- [ ] **Step 1: Write failing malformed-data tests**

Add tests that insert malformed rows through `db.pool()` and assert `matches!(result, Err(StoryError::Database(_)))` for:

```rust
#[tokio::test]
async fn malformed_scene_participant_uuid_is_database_error() {
    let db = Db::open_in_memory().await.unwrap();
    let sid = SceneId(uuid::Uuid::new_v4());
    sqlx::query("INSERT INTO scenes (id, objective_event, occurred_at) VALUES (?, ?, ?)")
        .bind(sid.0.to_string())
        .bind("event")
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(db.pool())
        .await
        .unwrap();
    sqlx::query("INSERT INTO scene_participants (scene_id, character_id) VALUES (?, ?)")
        .bind(sid.0.to_string())
        .bind("not-a-uuid")
        .execute(db.pool())
        .await
        .unwrap();

    let result = db.scenes().get(sid).await;

    assert!(matches!(result, Err(StoryError::Database(_))));
}
```

Add equivalent rows for invalid memory enum JSON and invalid sensation JSON. Use valid parent character/scene rows so only parsing fails.

- [ ] **Step 2: Run tests to verify they fail**

Run:

```text
cargo test --test db_test malformed_
```

Expected: FAIL because current code uses `unwrap_or_default`, `unwrap_or`, and silent filtering.

- [ ] **Step 3: Remove silent fallbacks from character and scene reads**

Convert iterator closures to fallible collection:

```rust
let participant_ids = sqlx::query("SELECT character_id FROM scene_participants WHERE scene_id = ?")
    .bind(id.0.to_string())
    .fetch_all(&self.pool)
    .await?
    .iter()
    .map(|row| {
        let value: String = sqlx::Row::try_get(row, "character_id")?;
        let uuid = Uuid::parse_str(&value)
            .map_err(|error| StoryError::Database(error.to_string()))?;
        Ok(CharacterId(uuid))
    })
    .collect::<Result<Vec<_>, StoryError>>()?;
```

Apply the same `Result<Vec<_>, StoryError>` pattern to personality and skill tags; remove unused `parse_uuid()` if no callers remain.

- [ ] **Step 4: Make memory parsing strict**

Replace enum fallbacks with explicit errors:

```rust
let source = serde_json::from_str(&source_str)
    .map_err(|error| StoryError::Database(error.to_string()))?;
let certainty = serde_json::from_str(&certainty_str)
    .map_err(|error| StoryError::Database(error.to_string()))?;
```

For inserts, serialize with `map_err` and propagate the resulting `StoryError::Database` instead of binding an empty string.

- [ ] **Step 5: Make sensation JSON parsing strict**

Change `parse_ids` to:

```rust
fn parse_ids(s: &str) -> Result<Vec<VocabularyId>, StoryError> {
    serde_json::from_str::<Vec<String>>(s)
        .map_err(|error| StoryError::Database(error.to_string()))?
        .into_iter()
        .map(|id| VocabularyId::new(&id).map_err(|error| StoryError::Database(error.to_string())))
        .collect()
}
```

Use `parse_ids(&visual)?` and equivalent calls in `latest()`.

- [ ] **Step 6: Run database tests**

Run:

```text
cargo test --test db_test
```

Expected: all database tests pass, including malformed-data tests and transaction rollback.

- [ ] **Step 7: Commit**

```text
git add src/db tests/db_test.rs
git commit -m "fix: fail loudly on malformed database values"
```

### Task 4: Make missing-character updates observable

**Files:**
- Modify: `src/db/character_repo.rs:60-98`
- Test: `tests/db_test.rs`

**Interfaces:**
- Consumes: `CharacterRepo::update(id, name, personality, skills) -> Result<(), StoryError>`.
- Produces: same signature; returns `StoryError::CharacterNotFound(id)` when the main update affects zero rows.

- [ ] **Step 1: Write failing test**

```rust
#[tokio::test]
async fn updating_missing_character_returns_not_found() {
    let db = Db::open_in_memory().await.unwrap();
    let id = CharacterId(uuid::Uuid::new_v4());

    let result = db.characters().update(id, "Missing", &[], &[]).await;

    assert!(matches!(result, Err(StoryError::CharacterNotFound(found)) if found == id));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```text
cargo test --test db_test updating_missing_character_returns_not_found
```

Expected: FAIL because current update continues after zero affected rows.

- [ ] **Step 3: Check `rows_affected()` before child-table replacement**

Store the update result and return before deleting tags:

```rust
let result = sqlx::query("UPDATE characters SET name = ?, updated_at = ? WHERE id = ?")
    .bind(name)
    .bind(&now)
    .bind(id.0.to_string())
    .execute(&mut *tx)
    .await?;
if result.rows_affected() == 0 {
    return Err(StoryError::CharacterNotFound(id));
}
```

- [ ] **Step 4: Run database tests**

Run:

```text
cargo test --test db_test
```

Expected: all database tests pass.

- [ ] **Step 5: Commit**

```text
git add src/db/character_repo.rs tests/db_test.rs
git commit -m "fix: report missing character updates"
```

### Task 5: Apply formatting, Clippy fixes, and full verification

**Files:**
- Modify: Rust files reported by `cargo fmt --all -- --check`.
- Modify: `src/db/derivation_repo.rs` and `src/models/ids.rs` for doc-comment list indentation.
- Test: all existing tests.

**Interfaces:**
- Consumes: completed Tasks 1-4.
- Produces: clean formatting, strict Clippy, compile and test gates.

- [ ] **Step 1: Run formatter**

Run:

```text
cargo fmt --all
```

Expected: formatting changes only; do not manually alter unrelated behavior.

- [ ] **Step 2: Fix strict Clippy documentation errors**

Indent continuation lines in `src/db/derivation_repo.rs` and `src/models/ids.rs` so list continuation text is part of the preceding list item, or insert a blank doc paragraph where appropriate.

- [ ] **Step 3: Run all quality gates**

Run each command independently:

```text
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: all commands exit with status 0; test output reports all tests passing.

- [ ] **Step 4: Inspect final diff and status**

Run:

```text
git status --short
```

Confirm only planned source, test, and formatting changes are present. Do not revert unrelated user changes.

- [ ] **Step 5: Commit**

```text
git add src tests docs/superpowers/plans/2026-07-19-quality-optimization.md
```

## Plan Self-Review

- Spec coverage: retry consistency is Task 2; no-write failure is Task 2; field ownership is Task 1; strict UUID/JSON/enum parsing is Task 3; missing update is Task 4; quality gates are Task 5.
- Placeholder scan: no `TODO`, `TBD`, vague edge-case instructions, or undefined helper names remain.
- Type consistency: `validate_with_retry` accepts owned `LlmCharacterDerivation` and returns `(LlmCharacterDerivation, SensorySelection)`; caller uses returned derivation for memory and plot.
- Scope: no schema migration, dependency, provider, prompt, observability, or architecture rewrite is included.
