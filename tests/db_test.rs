// 数据库层集成测试
// ===================
// 使用 Db::open_in_memory() 在每个测试函数中创建独立的内存数据库。
// 测试覆盖：
//   - 角色 CRUD（创建、更新、查询）
//   - 最新感官查询（按时间倒序取最新）
//   - 记忆上限（最多返回 50 条）
//   - 推导事务回滚（外键约束失败时感官数据不持久化）

use chrono::Utc;
use novels::db::Db;
use novels::models::*;
use sqlx::{query, sqlite::SqlitePoolOptions};
use uuid::Uuid;

#[tokio::test]
async fn migrate_existing_sensations_table_adds_extended_dimension_columns() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    query(
        "CREATE TABLE characters (id TEXT PRIMARY KEY, name TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
         CREATE TABLE scenes (id TEXT PRIMARY KEY, objective_event TEXT NOT NULL, occurred_at TEXT NOT NULL);
         CREATE TABLE character_sensations (
             id TEXT PRIMARY KEY,
             character_id TEXT NOT NULL,
             scene_id TEXT NOT NULL,
             visual_ids_json TEXT NOT NULL,
             auditory_ids_json TEXT NOT NULL,
             olfactory_ids_json TEXT NOT NULL,
             tactile_ids_json TEXT NOT NULL,
             gustatory_ids_json TEXT NOT NULL,
             created_at TEXT NOT NULL,
             FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE,
             FOREIGN KEY (scene_id) REFERENCES scenes(id) ON DELETE CASCADE
         );",
    )
    .execute(&pool)
    .await
    .unwrap();

    novels::db::migrate(&pool).await.unwrap();
    novels::db::migrate(&pool).await.unwrap();

    for column in [
        "emotion_ids_json",
        "gesture_ids_json",
        "atmosphere_ids_json",
    ] {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('character_sensations') WHERE name = ?",
        )
        .bind(column)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "missing {column}");
    }
}

#[tokio::test]
async fn create_and_get_character() {
    let db = Db::open_in_memory().await.unwrap();
    let id = CharacterId(Uuid::new_v4());
    db.characters()
        .create(
            id,
            "Alice",
            &["cautious".to_string()],
            &["sword".to_string()],
        )
        .await
        .unwrap();
    let c = db.characters().get(id).await.unwrap().unwrap();
    assert_eq!(c.name, "Alice");
    assert_eq!(c.personality, vec!["cautious".to_string()]);
    assert_eq!(c.skills, vec!["sword".to_string()]);
}

#[tokio::test]
async fn update_character_replaces_tags() {
    // 验证 update 操作会"替换"而非"追加"标签
    let db = Db::open_in_memory().await.unwrap();
    let id = CharacterId(Uuid::new_v4());
    db.characters()
        .create(id, "Bob", &["brave".to_string()], &[])
        .await
        .unwrap();
    db.characters()
        .update(
            id,
            "Bob",
            &["brave".to_string(), "loyal".to_string()],
            &["archery".to_string()],
        )
        .await
        .unwrap();
    let c = db.characters().get(id).await.unwrap().unwrap();
    assert_eq!(
        c.personality,
        vec!["brave".to_string(), "loyal".to_string()]
    );
    assert_eq!(c.skills, vec!["archery".to_string()]);
}

#[tokio::test]
async fn update_missing_character_returns_not_found() {
    let db = Db::open_in_memory().await.unwrap();
    let id = CharacterId(Uuid::new_v4());

    let result = db.characters().update(id, "Missing", &[], &[]).await;

    assert!(matches!(result, Err(StoryError::CharacterNotFound(found)) if found == id));
}

#[tokio::test]
async fn latest_sensation_returns_most_recent() {
    // 验证 latest() 返回的是最新插入的感官，而不是最旧的
    let db = Db::open_in_memory().await.unwrap();
    let cid = CharacterId(Uuid::new_v4());
    let sid1 = SceneId(Uuid::new_v4());
    let sid2 = SceneId(Uuid::new_v4());
    db.characters().create(cid, "C", &[], &[]).await.unwrap();
    db.scenes()
        .create(sid1, "event1", &[cid], Utc::now())
        .await
        .unwrap();
    db.scenes()
        .create(sid2, "event2", &[cid], Utc::now())
        .await
        .unwrap();
    db.derivations()
        .insert_derivation(
            cid,
            sid1,
            &SensorySelection::default(),
            &CharacterMemoryDraft {
                content: "m1".into(),
                source: MemorySource::Witnessed,
                certainty: Certainty::Certain,
            },
            Utc::now(),
        )
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    db.derivations()
        .insert_derivation(
            cid,
            sid2,
            &SensorySelection::default(),
            &CharacterMemoryDraft {
                content: "m2".into(),
                source: MemorySource::Witnessed,
                certainty: Certainty::Certain,
            },
            Utc::now(),
        )
        .await
        .unwrap();
    let latest = db.sensations().latest(cid).await.unwrap();
    assert!(latest.is_some());
    assert_eq!(latest.unwrap().1, sid2);
}

#[tokio::test]
async fn sensation_round_trip_preserves_all_eight_dimensions() {
    let db = Db::open_in_memory().await.unwrap();
    let cid = CharacterId(Uuid::new_v4());
    let sid = SceneId(Uuid::new_v4());
    db.characters().create(cid, "C", &[], &[]).await.unwrap();
    db.scenes()
        .create(sid, "event", &[cid], Utc::now())
        .await
        .unwrap();
    let sensations = SensorySelection {
        visual_ids: vec![VocabularyId::new("visual.x").unwrap()],
        auditory_ids: vec![VocabularyId::new("auditory.x").unwrap()],
        olfactory_ids: vec![VocabularyId::new("olfactory.x").unwrap()],
        tactile_ids: vec![VocabularyId::new("tactile.x").unwrap()],
        gustatory_ids: vec![VocabularyId::new("gustatory.x").unwrap()],
        emotion_ids: vec![VocabularyId::new("emotion.x").unwrap()],
        gesture_ids: vec![VocabularyId::new("gesture.x").unwrap()],
        atmosphere_ids: vec![VocabularyId::new("atmosphere.x").unwrap()],
    };
    db.derivations()
        .insert_derivation(
            cid,
            sid,
            &sensations,
            &CharacterMemoryDraft {
                content: "m".into(),
                source: MemorySource::Witnessed,
                certainty: Certainty::Certain,
            },
            Utc::now(),
        )
        .await
        .unwrap();

    let (stored, stored_scene) = db.sensations().latest(cid).await.unwrap().unwrap();
    assert_eq!(stored_scene, sid);
    assert_eq!(stored.visual_ids, sensations.visual_ids);
    assert_eq!(stored.auditory_ids, sensations.auditory_ids);
    assert_eq!(stored.olfactory_ids, sensations.olfactory_ids);
    assert_eq!(stored.tactile_ids, sensations.tactile_ids);
    assert_eq!(stored.gustatory_ids, sensations.gustatory_ids);
    assert_eq!(stored.emotion_ids, sensations.emotion_ids);
    assert_eq!(stored.gesture_ids, sensations.gesture_ids);
    assert_eq!(stored.atmosphere_ids, sensations.atmosphere_ids);
}

#[tokio::test]
async fn list_memories_caps_at_50() {
    // 验证记忆查询上限为 50 条
    let db = Db::open_in_memory().await.unwrap();
    let cid = CharacterId(Uuid::new_v4());
    let sid = SceneId(Uuid::new_v4());
    db.characters().create(cid, "C", &[], &[]).await.unwrap();
    db.scenes()
        .create(sid, "event", &[cid], Utc::now())
        .await
        .unwrap();
    for _ in 0..60 {
        db.derivations()
            .insert_derivation(
                cid,
                sid,
                &SensorySelection::default(),
                &CharacterMemoryDraft {
                    content: "m".into(),
                    source: MemorySource::Witnessed,
                    certainty: Certainty::Certain,
                },
                Utc::now(),
            )
            .await
            .unwrap();
    }
    let ms = db.memories().list(cid, 50).await.unwrap();
    assert_eq!(ms.len(), 50);
}

#[tokio::test]
async fn derivation_tx_atomic_on_memory_failure() {
    // 验证衍生写入的事务原子性：
    // 当 memory 插入因外键失败时，sensation 插入也应回滚
    let db = Db::open_in_memory().await.unwrap();
    let cid = CharacterId(Uuid::new_v4());
    let real_sid = SceneId(Uuid::new_v4());
    let fake_sid = SceneId(Uuid::new_v4());
    db.characters().create(cid, "C", &[], &[]).await.unwrap();
    db.scenes()
        .create(real_sid, "event", &[cid], Utc::now())
        .await
        .unwrap();
    let result = db
        .derivations()
        .insert_derivation(
            cid,
            fake_sid,
            &SensorySelection::default(),
            &CharacterMemoryDraft {
                content: "x".into(),
                source: MemorySource::Witnessed,
                certainty: Certainty::Certain,
            },
            Utc::now(),
        )
        .await;
    assert!(result.is_err());
    let senses = db.sensations().latest(cid).await.unwrap();
    assert!(
        senses.is_none(),
        "sensation must not persist after rollback"
    );
}

#[tokio::test]
async fn malformed_scene_participant_uuid_returns_database_error() {
    let db = Db::open_in_memory().await.unwrap();
    let scene_id = SceneId(Uuid::new_v4());
    let character_id = CharacterId(Uuid::new_v4());
    db.characters()
        .create(character_id, "C", &[], &[])
        .await
        .unwrap();
    db.scenes()
        .create(scene_id, "event", &[], Utc::now())
        .await
        .unwrap();

    let mut conn = db.pool().acquire().await.unwrap();
    query("PRAGMA foreign_keys = OFF")
        .execute(&mut *conn)
        .await
        .unwrap();
    query("INSERT INTO scene_participants (scene_id, character_id) VALUES (?, ?)")
        .bind(scene_id.0.to_string())
        .bind("not-a-uuid")
        .execute(&mut *conn)
        .await
        .unwrap();

    assert!(matches!(
        db.scenes().get(scene_id).await,
        Err(StoryError::Database(_))
    ));
}

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
    db.scenes()
        .create(early, "early", &[cid], t1)
        .await
        .unwrap();
    db.scenes()
        .create(current, "current", &[cid], t2)
        .await
        .unwrap();
    db.scenes()
        .create(future, "future", &[cid], t3)
        .await
        .unwrap();
    replace_for_test(&db, cid, future, "future memory", "future clue", &[]).await;

    assert!(
        db.memories()
            .list_before_scene(cid, t2, 50)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        db.sensations()
            .latest_before_scene(cid, t2)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        db.plots()
            .list_before_scene(cid, t2)
            .await
            .unwrap()
            .is_empty()
    );
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

    assert_eq!(
        db.memories().list(cid, 50).await.unwrap()[0].content,
        "second"
    );
    assert_eq!(
        db.sensations()
            .latest(cid)
            .await
            .unwrap()
            .unwrap()
            .0
            .visual_ids
            .len(),
        1
    );
    assert_eq!(
        db.plots()
            .list_before_scene(cid, at + chrono::Duration::seconds(1))
            .await
            .unwrap()[0]
            .development
            .reason,
        "second clue"
    );
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
    db.scenes()
        .create(sid, "event", &[cid], Utc::now())
        .await
        .unwrap();
    replace_for_test(&db, cid, sid, "first", "first clue", &[]).await;

    let missing_scene = SceneId(Uuid::new_v4());
    let result = db
        .derivations()
        .replace_derivation(
            cid,
            missing_scene,
            &SensorySelection::default(),
            &CharacterMemoryDraft {
                content: "bad".into(),
                source: MemorySource::Witnessed,
                certainty: Certainty::Certain,
            },
            &[],
            &[],
            Utc::now(),
        )
        .await;
    assert!(matches!(result, Err(StoryError::Database(_))));
    assert_eq!(
        db.memories().list(cid, 50).await.unwrap()[0].content,
        "first"
    );
}

#[tokio::test]
async fn malformed_memory_enum_json_returns_database_error() {
    let db = Db::open_in_memory().await.unwrap();
    let character_id = CharacterId(Uuid::new_v4());
    let scene_id = SceneId(Uuid::new_v4());
    db.characters()
        .create(character_id, "C", &[], &[])
        .await
        .unwrap();
    db.scenes()
        .create(scene_id, "event", &[character_id], Utc::now())
        .await
        .unwrap();

    query(
        "INSERT INTO character_memories \
         (id, character_id, scene_id, content, source, certainty, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(character_id.0.to_string())
    .bind(scene_id.0.to_string())
    .bind("memory")
    .bind("not-json")
    .bind(serde_json::to_string(&Certainty::Certain).unwrap())
    .bind(Utc::now().to_rfc3339())
    .execute(db.pool())
    .await
    .unwrap();

    assert!(matches!(
        db.memories().list(character_id, 1).await,
        Err(StoryError::Database(_))
    ));
}

#[tokio::test]
async fn malformed_memory_certainty_json_returns_database_error() {
    let db = Db::open_in_memory().await.unwrap();
    let character_id = CharacterId(Uuid::new_v4());
    let scene_id = SceneId(Uuid::new_v4());
    db.characters()
        .create(character_id, "C", &[], &[])
        .await
        .unwrap();
    db.scenes()
        .create(scene_id, "event", &[character_id], Utc::now())
        .await
        .unwrap();

    query(
        "INSERT INTO character_memories \
         (id, character_id, scene_id, content, source, certainty, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(character_id.0.to_string())
    .bind(scene_id.0.to_string())
    .bind("memory")
    .bind(serde_json::to_string(&MemorySource::Witnessed).unwrap())
    .bind("not-json")
    .bind(Utc::now().to_rfc3339())
    .execute(db.pool())
    .await
    .unwrap();

    assert!(matches!(
        db.memories().list(character_id, 1).await,
        Err(StoryError::Database(_))
    ));
}

#[tokio::test]
async fn malformed_sensation_json_returns_database_error() {
    let db = Db::open_in_memory().await.unwrap();
    let character_id = CharacterId(Uuid::new_v4());
    let scene_id = SceneId(Uuid::new_v4());
    db.characters()
        .create(character_id, "C", &[], &[])
        .await
        .unwrap();
    db.scenes()
        .create(scene_id, "event", &[character_id], Utc::now())
        .await
        .unwrap();

    query(
        "INSERT INTO character_sensations \
         (id, character_id, scene_id, visual_ids_json, auditory_ids_json, \
          olfactory_ids_json, tactile_ids_json, gustatory_ids_json, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(character_id.0.to_string())
    .bind(scene_id.0.to_string())
    .bind("not-json")
    .bind("[]")
    .bind("[]")
    .bind("[]")
    .bind("[]")
    .bind(Utc::now().to_rfc3339())
    .execute(db.pool())
    .await
    .unwrap();

    assert!(matches!(
        db.sensations().latest(character_id).await,
        Err(StoryError::Database(_))
    ));
}

#[tokio::test]
async fn invalid_sensation_vocabulary_id_returns_database_error() {
    let db = Db::open_in_memory().await.unwrap();
    let character_id = CharacterId(Uuid::new_v4());
    let scene_id = SceneId(Uuid::new_v4());
    db.characters()
        .create(character_id, "C", &[], &[])
        .await
        .unwrap();
    db.scenes()
        .create(scene_id, "event", &[character_id], Utc::now())
        .await
        .unwrap();

    query(
        "INSERT INTO character_sensations \
         (id, character_id, scene_id, visual_ids_json, auditory_ids_json, \
          olfactory_ids_json, tactile_ids_json, gustatory_ids_json, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(character_id.0.to_string())
    .bind(scene_id.0.to_string())
    .bind(r#"["invalid"]"#)
    .bind("[]")
    .bind("[]")
    .bind("[]")
    .bind("[]")
    .bind(Utc::now().to_rfc3339())
    .execute(db.pool())
    .await
    .unwrap();

    assert!(matches!(
        db.sensations().latest(character_id).await,
        Err(StoryError::Database(_))
    ));
}
