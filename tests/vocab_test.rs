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
    let v = Vocab::load_from_str(sample_yaml()).unwrap();
    assert!(v.has("visual", "bloodstain"));
    assert!(v.has("auditory", "footsteps"));
    assert!(!v.has("visual", "footsteps"));
}

#[test]
fn candidates_filter_by_tag() {
    let v = Vocab::load_from_str(sample_yaml()).unwrap();
    let cands = v.candidates("visual", &["injury"]);
    assert_eq!(cands.len(), 1);
    assert_eq!(cands[0].as_str(), "visual.bloodstain");
}

#[test]
fn validate_strips_unknown_ids() {
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
    let mut set = HashSet::new();
    set.insert("visual.bloodstain".to_string());
    let mut sel = SensorySelection::default();
    sel.visual_ids
        .push(VocabularyId::new("auditory.footsteps").unwrap());
    let result = validate(&sel, &set);
    assert_eq!(result.cleaned.visual_ids.len(), 0);
    assert!(result.all_empty);
}
