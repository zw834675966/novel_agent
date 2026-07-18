use super::{CharacterId, SceneId};
use super::{CharacterMemoryDraft, PlotDevelopment, SensorySelection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterDerivation {
    pub character_id: CharacterId,
    pub scene_id: SceneId,
    pub sensations: SensorySelection,
    pub new_memory: CharacterMemoryDraft,
    pub plot_development: Vec<PlotDevelopment>,
}
