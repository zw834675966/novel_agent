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
use sqlx::query;
use uuid::Uuid;

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
