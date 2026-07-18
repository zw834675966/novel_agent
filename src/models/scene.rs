use super::CharacterId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub id: super::SceneId,
    pub objective_event: String,
    pub participant_ids: Vec<CharacterId>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateScene {
    pub objective_event: String,
    pub participant_ids: Vec<CharacterId>,
    pub occurred_at: DateTime<Utc>,
}
