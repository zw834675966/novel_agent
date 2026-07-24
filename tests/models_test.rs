// 模型层单元测试
// =================
// 测试 VocabularyId 的序列化/反序列化和格式校验逻辑。

use chrono::Utc;
use novels::llm::LlmContextTagSelection;
use novels::models::*;
use novels::models::{
    CharacterId, PlotDevelopment, PlotDevelopmentKind, SceneId, StoredPlotDevelopment,
};

#[test]
fn vocabulary_id_roundtrip() {
    // 验证 VocabularyId 的 JSON 序列化/反序列化一致性
    let id = VocabularyId::new("visual.bloodstain").unwrap();
    let s = serde_json::to_string(&id).unwrap();
    let back: VocabularyId = serde_json::from_str(&s).unwrap();
    assert_eq!(id, back);
    assert_eq!(id.sense(), "visual");
    assert_eq!(id.key(), "bloodstain");
}

#[test]
fn vocabulary_id_rejects_missing_dot() {
    // 不包含 '.' 的字符串应被拒绝
    assert!(VocabularyId::new("noseparator").is_err());
}

#[test]
fn vocabulary_id_rejects_empty_segment() {
    // '.' 前后有空段的格式应被拒绝
    assert!(VocabularyId::new(".foo").is_err());
    assert!(VocabularyId::new("foo.").is_err());
}

#[test]
fn plot_reason_slot_serde_adjacent_tagging() {
    let slot = PlotReasonSlot::Observed("发现血迹".into());
    let json = serde_json::to_string(&slot).unwrap();
    assert_eq!(json, r#"{"kind":"observed","detail":"发现血迹"}"#);

    let back: PlotReasonSlot = serde_json::from_str(&json).unwrap();
    assert_eq!(back, slot);
    assert_eq!(back.render(), "见发现血迹");
}

#[test]
fn stored_plot_development_keeps_narrative_identity() {
    let slot = PlotReasonSlot::BehaviorOdd("目不转睛".into());
    let stored = StoredPlotDevelopment {
        character_id: CharacterId(uuid::Uuid::new_v4()),
        scene_id: SceneId(uuid::Uuid::new_v4()),
        development: PlotDevelopment {
            kind: PlotDevelopmentKind::SuspicionRaised,
            reason: slot.clone(),
        },
        created_at: Utc::now(),
    };

    assert_eq!(stored.development.reason, slot);
    assert_eq!(stored.development.reason.render(), "目不转睛举止反常");
}

#[test]
fn context_tag_selection_deserializes_empty_tags() {
    let selected: LlmContextTagSelection = serde_json::from_str(r#"{"tags":[]}"#).unwrap();
    assert!(selected.tags.is_empty());
}

#[test]
fn sensory_selection_deserializes_legacy_five_dimension_payloads() {
    let selection: SensorySelection = serde_json::from_str(
        r#"{
            "visual_ids": [],
            "auditory_ids": [],
            "olfactory_ids": [],
            "tactile_ids": [],
            "gustatory_ids": []
        }"#,
    )
    .unwrap();

    assert!(selection.emotion_ids.is_empty());
    assert!(selection.gesture_ids.is_empty());
    assert!(selection.atmosphere_ids.is_empty());
}
