use crate::models::{SensorySelection, VocabularyId};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub cleaned: SensorySelection,
    pub stripped: Vec<String>,
    pub all_empty: bool,
}

pub fn validate_selection(
    sel: &SensorySelection,
    candidates: &HashSet<String>,
) -> ValidationResult {
    let mut cleaned = SensorySelection::default();
    let mut stripped = Vec::new();

    let mut clean = |ids: &[VocabularyId]| -> Vec<VocabularyId> {
        ids.iter()
            .filter_map(|id| {
                if candidates.contains(id.as_str()) {
                    Some(id.clone())
                } else {
                    stripped.push(id.as_str().to_string());
                    None
                }
            })
            .collect()
    };

    cleaned.visual_ids = clean(&sel.visual_ids);
    cleaned.auditory_ids = clean(&sel.auditory_ids);
    cleaned.olfactory_ids = clean(&sel.olfactory_ids);
    cleaned.tactile_ids = clean(&sel.tactile_ids);
    cleaned.gustatory_ids = clean(&sel.gustatory_ids);

    let all_empty = cleaned.visual_ids.is_empty()
        && cleaned.auditory_ids.is_empty()
        && cleaned.olfactory_ids.is_empty()
        && cleaned.tactile_ids.is_empty()
        && cleaned.gustatory_ids.is_empty();

    ValidationResult {
        cleaned,
        stripped,
        all_empty,
    }
}
