// 端到端集成测试（Mock LLM）
// ============================
// 使用 MockSenseGenerator 替代真实 LLM 调用。
// 覆盖完整的"创建角色 → 创建场景 → 推导场景"流程。
// 验证：
//   - StoryService 能正确串联 DB 和 LLM
//   - 推导结果能正确持久化到数据库

use novels::db::Db;
use novels::llm::{LlmCharacterDerivation, LlmContextTagSelection, MockSenseGenerator};
use novels::models::*;
use novels::prose::{MockProseGenerator, MockScenePlanner};
use novels::scene::StoryService;
use novels::vocab::Vocab;
use std::sync::Arc;

#[tokio::test]
async fn end_to_end_with_mock() {
    let db = Db::open_in_memory().await.unwrap();
    let yaml = std::include_str!("../assets/vocab.yaml");
    let vocab = Vocab::load_from_str(yaml).unwrap();
    let canned = LlmCharacterDerivation {
        sensory_analysis: "分析场景中的视觉焦点".into(),
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
        relationship_candidates: vec![],
    };
    let generator = Arc::new(MockSenseGenerator::new(
        LlmContextTagSelection::default(),
        canned.clone(),
    ));
    let svc = StoryService::new(
        db.clone(),
        vocab,
        generator,
        Arc::new(MockProseGenerator::fallback()),
        Arc::new(MockScenePlanner::fallback()),
    );

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
    assert_eq!(d.new_memory.content.render(), "saw it");

    // Task 4: sensory_analysis CoT field deserializes from mock
    let serialized = serde_json::to_string(&canned).unwrap();
    let back: LlmCharacterDerivation = serde_json::from_str(&serialized).unwrap();
    assert_eq!(back.sensory_analysis, "分析场景中的视觉焦点");
}
