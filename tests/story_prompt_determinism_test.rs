// 提示词工程模版与确定性测试 (Story Prompt Determinism Test)
// ===================================================================

use novels::llm::VocabularyCandidate;
use novels::prompt::{
    DerivationPromptInput, NarrationPromptInput, build_derivation_prompt, build_narration_prompt,
    build_system_prompt,
};
use std::collections::BTreeMap;

#[test]
fn test_system_prompt_is_constant_and_strict() {
    let sys1 = build_system_prompt();
    let sys2 = build_system_prompt();
    assert_eq!(sys1, sys2);
    assert!(sys1.contains("禁止使用“由于”、“因此”"));
    assert!(sys1.contains("PlotReasonSlot"));
}

#[test]
fn test_derivation_prompt_is_deterministic_and_sorted() {
    let mut candidates = BTreeMap::new();
    candidates.insert(
        "emotion".to_string(),
        vec![
            VocabularyCandidate {
                id: "emotion.hlm-c001-01".into(),
                sense: "emotion".into(),
                text: "心中无限凄凉".into(),
                tags: vec!["hlm".into(), "哀伤".into()],
            },
            VocabularyCandidate {
                id: "emotion.hlm-c001-02".into(),
                sense: "emotion".into(),
                text: "满腔悲愤".into(),
                tags: vec!["hlm".into(), "愤怒".into()],
            },
        ],
    );
    candidates.insert(
        "visual".to_string(),
        vec![VocabularyCandidate {
            id: "visual.bloodstain".into(),
            sense: "visual".into(),
            text: "血迹".into(),
            tags: vec!["hlm".into()],
        }],
    );

    let input = DerivationPromptInput {
        character_name: "姜宁",
        character_tags: &["坚韧".to_string(), "投资人".to_string()],
        character_skills: &["反向收购".to_string()],
        scene_objective_event: "拍卖会上与陆沉重逢",
        memories: &["十年前姜家破产".to_string()],
        candidates_by_sense: &candidates,
    };

    let prompt1 = build_derivation_prompt(&input);
    let prompt2 = build_derivation_prompt(&input);

    // 确定性验证：两次生成的 Prompt 必须 100% 逐字相同
    assert_eq!(prompt1, prompt2);
    assert!(prompt1.contains("姜宁"));
    assert!(prompt1.contains("visual.bloodstain"));
    assert!(prompt1.contains("emotion.hlm-c001-01"));
}

#[test]
fn test_narration_prompt_building() {
    let actions = vec![
        ("姜宁".to_string(), "她看向举牌者".to_string()),
        ("陆沉".to_string(), "他停步，问：\"你是谁？\"".to_string()),
    ];
    let reasons = vec![("姜宁".to_string(), "见对方眼中愧意".to_string())];
    let quotes = vec!["龙涎香的气味兜头转脸席卷而来".to_string()];

    let input = NarrationPromptInput {
        scene_objective_event: "拍卖会上重逢",
        character_actions: &actions,
        plot_reasons: &reasons,
        selected_quotes: &quotes,
    };

    let prompt = build_narration_prompt(&input);
    assert!(prompt.contains("拍卖会上重逢"));
    assert!(prompt.contains("姜宁: 她看向举牌者"));
    assert!(prompt.contains("龙涎香的气味兜头转脸席卷而来"));
}
