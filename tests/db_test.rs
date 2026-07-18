use chrono::Utc;
use novels::db::Db;
use novels::models::*;
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
async fn latest_sensation_returns_most_recent() {
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
