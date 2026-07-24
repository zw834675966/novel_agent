// N5 长线物理状态机与逻辑断言 TDD 测试套件
// ===============================================

use chrono::Utc;
use novels::{
    db::Db,
    llm::{LlmCharacterDerivation, LlmContextTagSelection, MockSenseGenerator},
    models::{
        Certainty, CharacterId, CharacterMemoryDraft, CharacterState, CreateScene, MemorySource,
        SensorySelection,
    },
    prose::{MockProseGenerator, MockScenePlanner},
    scene::StoryService,
    vocab::Vocab,
};
use std::sync::Arc;

#[tokio::test]
async fn test_character_state_serialization_and_db_roundtrip() {
    let db = Db::open_in_memory().await.unwrap();
    let cid = CharacterId(uuid::Uuid::new_v4());

    db.characters()
        .create(cid, "测试者", &[], &[])
        .await
        .unwrap();

    let state = CharacterState::Holding("佩剑".to_string());
    db.states().set_state(cid, &state).await.unwrap();

    let fetched = db.states().get_state(cid).await.unwrap();
    assert_eq!(fetched, Some(state));
}

#[tokio::test]
async fn test_derive_character_rejects_dead_or_absent_participant() {
    let db = Db::open_in_memory().await.unwrap();
    let vocab = Vocab::load_from_str(
        r#"
visual:
  bloodstain:
    text: "血迹"
    tags: ["hlm"]
"#,
    )
    .unwrap();

    let sense_gen = Arc::new(MockSenseGenerator::new(
        LlmContextTagSelection::default(),
        LlmCharacterDerivation {
            sensory_analysis: String::new(),
            sensations: SensorySelection::default(),
            new_memory: CharacterMemoryDraft {
                content: "mock".into(),
                source: MemorySource::Witnessed,
                certainty: Certainty::Certain,
            },
            plot_development: vec![],
            relationship_candidates: vec![],
        },
    ));
    let prose_gen = Arc::new(MockProseGenerator::fallback());
    let service = StoryService::new(
        db.clone(),
        vocab,
        sense_gen,
        prose_gen,
        Arc::new(MockScenePlanner::fallback()),
    );

    let cid = CharacterId(uuid::Uuid::new_v4());
    db.characters()
        .create(cid, "受害者", &[], &[])
        .await
        .unwrap();

    // 显式将角色状态设为 Dead (死亡)
    db.states()
        .set_state(cid, &CharacterState::Dead)
        .await
        .unwrap();

    let scene_id = service
        .create_scene(CreateScene {
            objective_event: "案发研讨".into(),
            participant_ids: vec![cid],
            occurred_at: Utc::now(),
        })
        .await
        .unwrap();

    // 逻辑断言拦截：已死亡的角色不能参与场景推导，必须返回 StoryError
    let result = service.derive_character(scene_id, cid).await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("死亡") || err_msg.contains("不在场") || err_msg.contains("Dead"));
}
