// 感官词库单元测试
// ===================
// 测试 Vocab 的 YAML 加载、候选集生成、validate_selection 校验逻辑。

use novels::models::*;
use novels::vocab::{Vocab, validate};
use std::collections::HashSet;

fn sample_yaml() -> &'static str {
    r#"
visual:
  bloodstain:
    text: "血迹"
    tags: ["injury"]
auditory:
  footsteps:
    text: "脚步声"
    tags: ["movement"]
"#
}

#[test]
fn loads_all_senses() {
    // 验证 YAML 能正确加载所有感官类别
    let v = Vocab::load_from_str(sample_yaml()).unwrap();
    assert!(v.has("visual", "bloodstain"));
    assert!(v.has("auditory", "footsteps"));
    assert!(!v.has("visual", "footsteps")); // 跨类别不匹配
}

#[test]
fn candidates_filter_by_tag() {
    // 验证按标签过滤候选集
    let v = Vocab::load_from_str(sample_yaml()).unwrap();
    let cands = v.candidates("visual", &["injury"]);
    assert_eq!(cands.len(), 1);
    assert_eq!(cands[0].as_str(), "visual.bloodstain");
}

#[test]
fn candidates_for_tags_include_semantic_metadata() {
    let v = Vocab::load_from_str(sample_yaml()).unwrap();
    let candidates = v.candidates_for_tags(&["injury".to_string()]);

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].id, "visual.bloodstain");
    assert_eq!(candidates[0].sense, "visual");
    assert_eq!(candidates[0].text, "血迹");
    assert_eq!(candidates[0].tags, vec!["injury"]);
}

#[test]
fn candidates_for_tags_fall_back_when_tags_have_no_entry_match() {
    let v = Vocab::load_from_str(sample_yaml()).unwrap();
    let candidates = v.candidates_for_tags(&["known-but-unmatched".to_string()]);

    assert_eq!(candidates.len(), 2);
}

#[test]
fn known_tags_and_filter_known_tags_are_sorted_and_deduplicated() {
    let v = Vocab::load_from_str(sample_yaml()).unwrap();
    let tags = [
        "movement".to_string(),
        "unknown".to_string(),
        "injury".to_string(),
        "movement".to_string(),
    ];

    assert_eq!(v.known_tags(), vec!["injury", "movement"]);
    assert_eq!(v.filter_known_tags(&tags), vec!["injury", "movement"]);
}

#[test]
fn candidates_for_tags_fall_back_when_filter_known_tags_drops_all() {
    // 回归测试：filter_known_tags 产出的空选择必须触发 candidates_for_tags 全量回退。
    let v = Vocab::load_from_str(sample_yaml()).unwrap();
    let absent = "absent-tag".to_string();
    let filtered = v.filter_known_tags(&[absent]);
    assert!(filtered.is_empty());

    let candidates = v.candidates_for_tags(&filtered);
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].id, "visual.bloodstain");
    assert_eq!(candidates[1].id, "auditory.footsteps");
}

#[test]
fn validate_strips_unknown_ids() {
    // 非法 ID 应被 validate 移除，合法的保留
    let _v = Vocab::load_from_str(sample_yaml()).unwrap();
    let mut set = HashSet::new();
    set.insert("visual.bloodstain".to_string());
    set.insert("auditory.footsteps".to_string());
    let mut sel = SensorySelection::default();
    sel.visual_ids
        .push(VocabularyId::new("visual.bloodstain").unwrap());
    sel.visual_ids
        .push(VocabularyId::new("visual.unknown").unwrap());
    let result = validate(&sel, &set);
    assert_eq!(result.cleaned.visual_ids.len(), 1);
    assert!(!result.stripped.is_empty());
}

#[test]
fn validate_rejects_wrong_sense() {
    // 全局候选集中存在、但属于其他感官类别的 ID 也必须被拒绝。
    let mut set = HashSet::new();
    set.insert("visual.bloodstain".to_string());
    set.insert("auditory.footsteps".to_string());
    let mut sel = SensorySelection::default();
    sel.visual_ids
        .push(VocabularyId::new("auditory.footsteps").unwrap());
    let result = validate(&sel, &set);
    assert_eq!(result.cleaned.visual_ids.len(), 0);
    assert_eq!(result.stripped, vec!["auditory.footsteps"]);
    assert!(result.all_empty);
}
