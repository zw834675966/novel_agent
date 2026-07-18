use novels::db::Db;
use novels::llm::{LlmCharacterDerivation, MockSenseGenerator};
use novels::models::*;
use novels::scene::StoryService;
use novels::vocab::Vocab;
use std::sync::Arc;

fn canned_derivation() -> LlmCharacterDerivation {
    LlmCharacterDerivation {
        sensations: SensorySelection {
            visual_ids: vec![VocabularyId::new("visual.x").unwrap()],
            ..Default::default()
        },
        new_memory: CharacterMemoryDraft {
            content: "witnessed".into(),
            source: MemorySource::Witnessed,
            certainty: Certainty::Certain,
        },
        plot_development: vec![],
    }
}

#[tokio::test]
async fn derive_character_persists_and_returns() {
    let db = Db::open_in_memory().await.unwrap();
    let yaml = "visual:\n  x:\n    text: x\n    tags: []\n";
    let vocab = Vocab::load_from_str(yaml).unwrap();
    let generator = Arc::new(MockSenseGenerator::new(canned_derivation()));
    let svc = StoryService::new(db.clone(), vocab, generator);

    let cid = CharacterId(uuid::Uuid::new_v4());
    let sid = SceneId(uuid::Uuid::new_v4());
    db.characters().create(cid, "A", &[], &[]).await.unwrap();
    db.scenes()
        .create(sid, "事件X", &[cid], chrono::Utc::now())
        .await
        .unwrap();

    let r = svc.derive_character(sid, cid).await;
    assert!(r.is_ok(), "{:?}", r.err());
    let d = r.unwrap();
    assert_eq!(d.character_id, cid);
    assert_eq!(d.scene_id, sid);
    let mems = db.memories().list(cid, 50).await.unwrap();
    assert_eq!(mems.len(), 1);
    assert_eq!(mems[0].content, "witnessed");
    let latest = db.sensations().latest(cid).await.unwrap();
    assert!(latest.is_some());
}

#[tokio::test]
async fn derive_character_rejects_non_participant() {
    let db = Db::open_in_memory().await.unwrap();
    let vocab = Vocab::load_from_str("visual:\n  x:\n    text: x\n    tags: []\n").unwrap();
    let generator = Arc::new(MockSenseGenerator::new(canned_derivation()));
    let svc = StoryService::new(db.clone(), vocab, generator);
    let cid = CharacterId(uuid::Uuid::new_v4());
    let sid = SceneId(uuid::Uuid::new_v4());
    db.characters().create(cid, "A", &[], &[]).await.unwrap();
    db.scenes()
        .create(sid, "e", &[cid], chrono::Utc::now())
        .await
        .unwrap();
    let other = CharacterId(uuid::Uuid::new_v4());
    db.characters().create(other, "B", &[], &[]).await.unwrap();
    let err = svc.derive_character(sid, other).await.unwrap_err();
    assert!(matches!(err, StoryError::NotSceneParticipant(_, _)));
}

#[tokio::test]
async fn derive_scene_returns_partial_on_one_failure() {
    let db = Db::open_in_memory().await.unwrap();
    let vocab = Vocab::load_from_str("visual:\n  x:\n    text: x\n    tags: []\n").unwrap();
    let generator = Arc::new(MockSenseGenerator::new(canned_derivation()));
    let svc = StoryService::new(db.clone(), vocab, generator);
    let c1 = CharacterId(uuid::Uuid::new_v4());
    let s = SceneId(uuid::Uuid::new_v4());
    db.characters().create(c1, "A", &[], &[]).await.unwrap();
    db.scenes()
        .create(s, "e", &[c1], chrono::Utc::now())
        .await
        .unwrap();
    let results = svc.derive_scene(s).await;
    assert_eq!(results.len(), 1);
    assert!(results[0].is_ok());
}
