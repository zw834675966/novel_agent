// 真实故事线端到端集成测试：青梅竹马 + 商战破产 + 搬迁乡下 + 多年后崛起重逢
// =================================================================================

use chrono::{Duration, Utc};
use novels::{
    db::Db,
    llm::{DerivationRequest, LlmCharacterDerivation, LlmContextTagSelection, SenseGenerator},
    models::{
        Certainty, CharacterId, CharacterMemoryDraft, CreateScene, MemorySource, PlotDevelopment,
        PlotDevelopmentKind, PlotReasonSlot, SensorySelection, VocabularyId,
    },
    prose::{MockProseGenerator, MockScenePlanner},
    scene::StoryService,
    vocab::Vocab,
};
use std::sync::Arc;

struct StoryLineGenerator;

#[async_trait::async_trait]
impl SenseGenerator for StoryLineGenerator {
    async fn select_context_tags(
        &self,
        _req: &novels::llm::ContextTagRequest,
    ) -> Result<LlmContextTagSelection, novels::models::StoryError> {
        Ok(LlmContextTagSelection {
            tags: vec!["hlm".into(), "zhz".into(), "哀伤".into(), "冷漠".into()],
        })
    }

    async fn derive(
        &self,
        req: &DerivationRequest,
    ) -> Result<LlmCharacterDerivation, novels::models::StoryError> {
        let is_lu_chen = req.character.name.contains("陆沉");

        if is_lu_chen {
            Ok(LlmCharacterDerivation {
                sensory_analysis: String::new(),
                sensations: SensorySelection {
                    visual_ids: vec![VocabularyId::new("visual.bloodstain").unwrap()],
                    auditory_ids: vec![],
                    olfactory_ids: vec![],
                    tactile_ids: vec![],
                    gustatory_ids: vec![],
                    emotion_ids: vec![VocabularyId::new("emotion.hlm-c001-01").unwrap()],
                    gesture_ids: vec![VocabularyId::new("gesture.hlm-c001-02").unwrap()],
                    atmosphere_ids: vec![VocabularyId::new("atmosphere.hlm-c001-06").unwrap()],
                },
                new_memory: CharacterMemoryDraft {
                    content: "多年后重逢，她已非当年破产离去的少女".into(),
                    source: MemorySource::Witnessed,
                    certainty: Certainty::Certain,
                },
                plot_development: vec![PlotDevelopment {
                    kind: PlotDevelopmentKind::RelationshipShifted,
                    reason: PlotReasonSlot::BehaviorOdd("清冷自若".into()),
                }],
                relationship_candidates: vec![],
            })
        } else {
            Ok(LlmCharacterDerivation {
                sensory_analysis: String::new(),
                sensations: SensorySelection {
                    visual_ids: vec![],
                    auditory_ids: vec![],
                    olfactory_ids: vec![],
                    tactile_ids: vec![],
                    gustatory_ids: vec![],
                    emotion_ids: vec![VocabularyId::new("emotion.hlm-c001-01").unwrap()],
                    gesture_ids: vec![VocabularyId::new("gesture.hlm-c001-02").unwrap()],
                    atmosphere_ids: vec![],
                },
                new_memory: CharacterMemoryDraft {
                    content: "拍卖会上与陆沉对视，家族破产往事历历在目".into(),
                    source: MemorySource::Witnessed,
                    certainty: Certainty::Certain,
                },
                plot_development: vec![PlotDevelopment {
                    kind: PlotDevelopmentKind::ConflictEscalated,
                    reason: PlotReasonSlot::Observed("对方眼中愧意".into()),
                }],
                relationship_candidates: vec![],
            })
        }
    }
}

#[tokio::test]
async fn test_childhood_rivalry_reunion_storyline() {
    let db = Db::open_in_memory().await.unwrap();
    let vocab = Vocab::load_from_str(
        r#"
visual:
  bloodstain:
    text: "血迹"
    tags: ["hlm"]
emotion:
  hlm-c001-01:
    text: "心中无限凄凉"
    tags: ["hlm", "哀伤"]
gesture:
  hlm-c001-02:
    text: "不禁落下泪来"
    tags: ["hlm", "落泪"]
atmosphere:
  hlm-c001-06:
    text: "满室肃然"
    tags: ["hlm"]
"#,
    )
    .unwrap();

    let sense_gen = Arc::new(StoryLineGenerator);
    let prose_gen = Arc::new(MockProseGenerator::fallback());
    let service = StoryService::new(
        db.clone(),
        vocab,
        sense_gen,
        prose_gen,
        Arc::new(MockScenePlanner::fallback()),
    );

    // 1. 创建男主（陆沉）与女主（姜宁）
    let lu_chen_id = CharacterId(uuid::Uuid::new_v4());
    let jiang_ning_id = CharacterId(uuid::Uuid::new_v4());

    db.characters()
        .create(
            lu_chen_id,
            "陆沉",
            &["冷峻".into(), "商界巨头".into()],
            &["资本运作".into()],
        )
        .await
        .unwrap();

    db.characters()
        .create(
            jiang_ning_id,
            "姜宁",
            &["坚韧".into(), "新锐投资人".into()],
            &["并购重组".into()],
        )
        .await
        .unwrap();

    // 2. 场景1（十年前破产）：姜家受陆氏恶意竞争破产清算
    let past_time = Utc::now() - Duration::days(3650);
    let past_scene_id = service
        .create_scene(CreateScene {
            objective_event: "陆氏恶意竞争致姜家破产清算，姜宁随父搬迁乡下".into(),
            participant_ids: vec![lu_chen_id, jiang_ning_id],
            occurred_at: past_time,
        })
        .await
        .unwrap();

    let deriv_past = service.derive_scene(past_scene_id).await;
    assert_eq!(deriv_past.len(), 2);
    assert!(deriv_past.iter().all(|r| r.is_ok()));

    // 3. 场景2（十年后重逢）：多年后姜宁崛起为顶级投资人，拍卖会与陆沉重逢
    let present_time = Utc::now();
    let present_scene_id = service
        .create_scene(CreateScene {
            objective_event: "慈善拍卖会上，姜宁以资本新贵身份高价举牌，与陆沉正面重逢".into(),
            participant_ids: vec![lu_chen_id, jiang_ning_id],
            occurred_at: present_time,
        })
        .await
        .unwrap();

    let deriv_present = service.derive_scene(present_scene_id).await;
    assert_eq!(deriv_present.len(), 2);
    let present_derivations: Vec<_> = deriv_present.into_iter().map(|r| r.unwrap()).collect();

    // 4. 验证时序隔离：当前推导只引用较早场景记忆与剧情，无因果倒置
    let lu_chen_memories = db.memories().list(lu_chen_id, 50).await.unwrap();
    assert_eq!(lu_chen_memories.len(), 2);

    // 5. 验证 N4 PlotReasonSlot 渲染与断言
    let plot_developments = db
        .plots()
        .list_before_scene(lu_chen_id, present_time + Duration::seconds(10))
        .await
        .unwrap();
    assert!(!plot_developments.is_empty());
    let slot_render = plot_developments[0].development.reason.render();
    assert!(!slot_render.is_empty());
    assert!(!slot_render.contains("由于"));
    assert!(!slot_render.contains("因此"));

    // 6. 执行正文拼装叙事
    let assembled = service
        .narrate_scene(present_scene_id, &present_derivations)
        .await
        .unwrap();

    assert!(!assembled.text.is_empty());
    assert_eq!(assembled.unverified_quotes, 0);
}
