use super::{Certainty, MemorySource};
use super::{CharacterId, MemoryId, SceneId};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterMemory {
    pub id: MemoryId,
    pub character_id: CharacterId,
    pub scene_id: SceneId,
    pub content: String,
    pub source: MemorySource,
    pub certainty: Certainty,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CharacterMemoryDraft {
    pub content: String,
    pub source: MemorySource,
    pub certainty: Certainty,
}
