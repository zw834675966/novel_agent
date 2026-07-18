use super::VocabularyId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct SensorySelection {
    pub visual_ids: Vec<VocabularyId>,
    pub auditory_ids: Vec<VocabularyId>,
    pub olfactory_ids: Vec<VocabularyId>,
    pub tactile_ids: Vec<VocabularyId>,
    pub gustatory_ids: Vec<VocabularyId>,
}
