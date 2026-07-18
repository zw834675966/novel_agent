use novels::db::Db;
use novels::llm::{LlmCharacterDerivation, MockSenseGenerator};
use novels::models::*;
use novels::scene::StoryService;
use novels::vocab::Vocab;
use std::sync::Arc;

#[tokio::test]
async fn end_to_end_with_mock() {
    let db = Db::open_in_memory().await.unwrap();
    let yaml = std::include_str!("../assets/vocab.yaml");
    let vocab = Vocab::load_from_str(yaml).unwrap();
    let canned = LlmCharacterDerivation {
        sensations: SensorySelection {
            visual_ids: vec![VocabularyId::new("visual.bloodstain").unwrap()],
            ..Default::default()
        },
        new_memory: CharacterMemoryDraft {
            content: "saw it".into(),
            source: MemorySource::Witnessed,
            certainty: Certainty::Certain,
        },
        plot_development: vec![],
    };
    let generator = Arc::new(MockSenseGenerator::new(canned));
    let svc = StoryService::new(db.clone(), vocab, generator);

    let cid = CharacterId(uuid::Uuid::new_v4());
    db.characters()
        .create(cid, "Hero", &["brave".to_string()], &["sword".to_string()])
        .await
        .unwrap();
    let sid = svc
        .create_scene(CreateScene {
            objective_event: "城里发生凶案".into(),
            participant_ids: vec![cid],
            occurred_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    let results = svc.derive_scene(sid).await;
    assert_eq!(results.len(), 1);
    assert!(results[0].is_ok());
    let d = results[0].as_ref().unwrap();
    assert_eq!(d.character_id, cid);
    assert_eq!(d.new_memory.content, "saw it");
}
