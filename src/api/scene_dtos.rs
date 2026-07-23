use crate::api::dto::CharacterDto;
use crate::api::relationship_dtos::CandidateDto;
use crate::models::*;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneDerivationDto {
    pub character: CharacterDto,
    pub memory: MemoryDto,
    pub sensation: SensorySelectionDto,
    pub plot_developments: Vec<PlotDto>,
    pub relationship_candidates: Vec<CandidateDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DerivationDto {
    pub character_id: String,
    pub sensations: SensorySelectionDto,
    pub new_memory: MemoryDto,
    pub plot_development: Vec<PlotDto>,
    pub relationship_candidates: Vec<CandidateDto>,
}

impl From<CharacterDerivation> for DerivationDto {
    fn from(value: CharacterDerivation) -> Self {
        Self {
            character_id: value.character_id.0.to_string(),
            sensations: value.sensations.into(),
            new_memory: value.new_memory.into(),
            plot_development: value
                .plot_development
                .into_iter()
                .map(PlotDto::from)
                .collect(),
            relationship_candidates: value
                .relationship_candidates
                .into_iter()
                .map(CandidateDto::from)
                .collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SensorySelectionDto {
    pub visual: Vec<String>,
    pub auditory: Vec<String>,
    pub olfactory: Vec<String>,
    pub tactile: Vec<String>,
    pub gustatory: Vec<String>,
    pub emotion: Vec<String>,
    pub gesture: Vec<String>,
    pub atmosphere: Vec<String>,
}

impl From<SensorySelection> for SensorySelectionDto {
    fn from(value: SensorySelection) -> Self {
        Self {
            visual: value
                .visual_ids
                .into_iter()
                .map(|id| id.as_str().to_string())
                .collect(),
            auditory: value
                .auditory_ids
                .into_iter()
                .map(|id| id.as_str().to_string())
                .collect(),
            olfactory: value
                .olfactory_ids
                .into_iter()
                .map(|id| id.as_str().to_string())
                .collect(),
            tactile: value
                .tactile_ids
                .into_iter()
                .map(|id| id.as_str().to_string())
                .collect(),
            gustatory: value
                .gustatory_ids
                .into_iter()
                .map(|id| id.as_str().to_string())
                .collect(),
            emotion: value
                .emotion_ids
                .into_iter()
                .map(|id| id.as_str().to_string())
                .collect(),
            gesture: value
                .gesture_ids
                .into_iter()
                .map(|id| id.as_str().to_string())
                .collect(),
            atmosphere: value
                .atmosphere_ids
                .into_iter()
                .map(|id| id.as_str().to_string())
                .collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryDto {
    pub character_id: String,
    pub content: String,
    pub source: String,
    pub certainty: String,
    pub created_at: String,
}

impl From<CharacterMemory> for MemoryDto {
    fn from(value: CharacterMemory) -> Self {
        Self {
            character_id: value.character_id.0.to_string(),
            content: value.content.render(),
            source: format!("{:?}", value.source),
            certainty: format!("{:?}", value.certainty),
            created_at: value.created_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlotDto {
    pub kind: String,
    pub reason: String,
}

impl From<PlotDevelopment> for PlotDto {
    fn from(value: PlotDevelopment) -> Self {
        Self {
            kind: format!("{:?}", value.kind),
            reason: value.reason.render(),
        }
    }
}

impl From<StoredPlotDevelopment> for PlotDto {
    fn from(value: StoredPlotDevelopment) -> Self {
        Self {
            kind: format!("{:?}", value.development.kind),
            reason: value.development.reason.render(),
        }
    }
}
