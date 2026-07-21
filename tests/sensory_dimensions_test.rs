use novels::models::{SensorySelection, VocabularyId};
use novels::vocab::validate;
use std::collections::HashSet;

fn id(value: &str) -> VocabularyId {
    VocabularyId::new(value).unwrap()
}

#[test]
fn validate_selection_keeps_candidates_in_all_eight_dimensions() {
    let selection = SensorySelection {
        visual_ids: vec![id("visual.x")],
        auditory_ids: vec![id("auditory.x")],
        olfactory_ids: vec![id("olfactory.x")],
        tactile_ids: vec![id("tactile.x")],
        gustatory_ids: vec![id("gustatory.x")],
        emotion_ids: vec![id("emotion.x")],
        gesture_ids: vec![id("gesture.x")],
        atmosphere_ids: vec![id("atmosphere.x")],
    };
    let candidates = [
        "visual.x",
        "auditory.x",
        "olfactory.x",
        "tactile.x",
        "gustatory.x",
        "emotion.x",
        "gesture.x",
        "atmosphere.x",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<HashSet<_>>();

    let result = validate(&selection, &candidates);

    assert!(result.stripped.is_empty());
    assert!(!result.all_empty);
    assert_eq!(result.cleaned.visual_ids, selection.visual_ids);
    assert_eq!(result.cleaned.auditory_ids, selection.auditory_ids);
    assert_eq!(result.cleaned.olfactory_ids, selection.olfactory_ids);
    assert_eq!(result.cleaned.tactile_ids, selection.tactile_ids);
    assert_eq!(result.cleaned.gustatory_ids, selection.gustatory_ids);
    assert_eq!(result.cleaned.emotion_ids, selection.emotion_ids);
    assert_eq!(result.cleaned.gesture_ids, selection.gesture_ids);
    assert_eq!(result.cleaned.atmosphere_ids, selection.atmosphere_ids);
}
