use crate::models::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterDto {
    pub id: String,
    pub name: String,
    pub personality: Vec<String>,
    pub skills: Vec<String>,
}

impl From<Character> for CharacterDto {
    fn from(value: Character) -> Self {
        Self {
            id: value.id.0.to_string(),
            name: value.name,
            personality: value.personality,
            skills: value.skills,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCharacterRequest {
    pub name: String,
    pub personality: Vec<String>,
    pub skills: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneDto {
    pub id: String,
    pub objective_event: String,
    pub participant_ids: Vec<String>,
    pub occurred_at: String,
}

impl From<Scene> for SceneDto {
    fn from(value: Scene) -> Self {
        Self {
            id: value.id.0.to_string(),
            objective_event: value.objective_event,
            participant_ids: value
                .participant_ids
                .iter()
                .map(|id| id.0.to_string())
                .collect(),
            occurred_at: value.occurred_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSceneRequest {
    pub objective_event: String,
    pub participant_ids: Vec<String>,
    pub occurred_at: String,
}
