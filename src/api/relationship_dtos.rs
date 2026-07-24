use crate::api::errors::ApiError;
use crate::api::scene_dtos::DerivationDto;
use crate::models::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateDto {
    pub id: String,
    pub from_character_id: String,
    pub to_character_id: String,
    pub relationship_type: String,
    pub summary: String,
    pub tension_score: Option<u8>,
    pub trust_score: Option<u8>,
    pub affection_score: Option<u8>,
    pub power_score: Option<u8>,
    pub confidence: f32,
    pub status: String,
    pub created_at: String,
}

impl From<RelationshipCandidate> for CandidateDto {
    fn from(value: RelationshipCandidate) -> Self {
        Self {
            id: value.id.0.to_string(),
            from_character_id: value.from_character_id.0.to_string(),
            to_character_id: value.to_character_id.0.to_string(),
            relationship_type: format!("{:?}", value.relationship_type),
            summary: value.summary,
            tension_score: value.tension_score,
            trust_score: value.trust_score,
            affection_score: value.affection_score,
            power_score: value.power_score,
            confidence: value.confidence,
            status: format!("{:?}", value.status),
            created_at: value.created_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchDerivationResultDto {
    pub derivation: Option<DerivationDto>,
    pub error: Option<ApiError>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeDto {
    pub id: String,
    pub name: String,
}

impl From<GraphNode> for GraphNodeDto {
    fn from(value: GraphNode) -> Self {
        Self {
            id: value.id.0.to_string(),
            name: value.name,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdgeDto {
    pub fact_id: String,
    pub from_character_id: String,
    pub to_character_id: String,
    pub relationship_type: String,
    pub summary: String,
    pub scores: ScoresDto,
    pub active: bool,
}

impl From<GraphEdge> for GraphEdgeDto {
    fn from(value: GraphEdge) -> Self {
        Self {
            fact_id: value.fact_id.0.to_string(),
            from_character_id: value.from.0.to_string(),
            to_character_id: value.to.0.to_string(),
            relationship_type: format!("{:?}", value.relationship_type),
            summary: value.summary,
            scores: ScoresDto {
                tension: value.tension_score,
                trust: value.trust_score,
                affection: value.affection_score,
                power: value.power_score,
            },
            active: value.active,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoresDto {
    pub tension: Option<u8>,
    pub trust: Option<u8>,
    pub affection: Option<u8>,
    pub power: Option<u8>,
}

impl From<RelationshipRevision> for ScoresDto {
    fn from(value: RelationshipRevision) -> Self {
        Self {
            tension: value.tension_score,
            trust: value.trust_score,
            affection: value.affection_score,
            power: value.power_score,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphSnapshotDto {
    pub nodes: Vec<GraphNodeDto>,
    pub edges: Vec<GraphEdgeDto>,
}

impl From<GraphSnapshot> for GraphSnapshotDto {
    fn from(value: GraphSnapshot) -> Self {
        Self {
            nodes: value.nodes.into_iter().map(Into::into).collect(),
            edges: value.edges.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipRevisionDto {
    pub id: String,
    pub relationship_fact_id: String,
    pub scene_id: String,
    pub valid_from_scene_id: String,
    pub valid_until_scene_id: Option<String>,
    pub status: String,
    pub summary: String,
    pub scores: ScoresDto,
    pub evidence_memory_id: Option<String>,
    pub created_at: String,
}

impl From<RelationshipRevision> for RelationshipRevisionDto {
    fn from(value: RelationshipRevision) -> Self {
        Self {
            id: value.id.0.to_string(),
            relationship_fact_id: value.relationship_fact_id.0.to_string(),
            scene_id: value.scene_id.0.to_string(),
            valid_from_scene_id: value.valid_from_scene_id.0.to_string(),
            valid_until_scene_id: value.valid_until_scene_id.map(|id| id.0.to_string()),
            status: format!("{:?}", value.status),
            summary: value.summary,
            scores: ScoresDto {
                tension: value.tension_score,
                trust: value.trust_score,
                affection: value.affection_score,
                power: value.power_score,
            },
            evidence_memory_id: value.evidence_memory_id.map(|id| id.0.to_string()),
            created_at: value.created_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveCandidateRequest {
    pub decision: CandidateDecision,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateDecision {
    pub action: String,
    pub summary: Option<String>,
    pub scores: Option<ScoresDto>,
}
