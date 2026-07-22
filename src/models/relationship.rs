use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::ids::{RelationshipCandidateId, RelationshipFactId};
use super::{CharacterId, MemoryId, SceneId, StoryError};

/// 关系类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    Knows,
    AlliedWith,
    Distrusts,
    Owes,
    FamilyOf,
    Mentors,
    ConflictsWith,
}

/// 候选状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CandidateStatus {
    Pending,
    Accepted,
    Rejected,
}

impl CandidateStatus {
    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Pending, Self::Accepted | Self::Rejected)
        )
    }
}

/// 关系事实（持久化）
#[derive(Debug, Clone)]
pub struct RelationshipFact {
    pub id: RelationshipFactId,
    pub from_character_id: CharacterId,
    pub to_character_id: CharacterId,
    pub relationship_type: RelationshipType,
    pub created_at: DateTime<Utc>,
}

/// 关系修订（持久化，历史记录）
#[derive(Debug, Clone)]
pub struct RelationshipRevision {
    pub id: RelationshipFactId,
    pub relationship_fact_id: RelationshipFactId,
    pub scene_id: SceneId,
    pub valid_from_scene_id: SceneId,
    pub valid_until_scene_id: Option<SceneId>,
    pub status: RevisionStatus,
    pub summary: String,
    pub tension_score: Option<u8>,
    pub trust_score: Option<u8>,
    pub affection_score: Option<u8>,
    pub power_score: Option<u8>,
    pub evidence_memory_id: Option<MemoryId>,
    pub created_at: DateTime<Utc>,
}

/// 修订状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RevisionStatus {
    Active,
    Superseded,
}

/// 关系候选（持久化）
#[derive(Debug, Clone)]
pub struct RelationshipCandidate {
    pub id: RelationshipCandidateId,
    pub scene_id: SceneId,
    pub from_character_id: CharacterId,
    pub to_character_id: CharacterId,
    pub relationship_type: RelationshipType,
    pub summary: String,
    pub tension_score: Option<u8>,
    pub trust_score: Option<u8>,
    pub affection_score: Option<u8>,
    pub power_score: Option<u8>,
    pub evidence_memory_id: Option<MemoryId>,
    pub confidence: f32,
    pub status: CandidateStatus,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

/// 图快照（用于可视化）
#[derive(Debug, Clone)]
pub struct GraphSnapshot {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// 图节点（角色）
#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: CharacterId,
    pub name: String,
}

/// 图边（关系）
#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub fact_id: RelationshipFactId,
    pub from: CharacterId,
    pub to: CharacterId,
    pub relationship_type: RelationshipType,
    pub summary: String,
    pub active: bool,
    pub tension_score: Option<u8>,
    pub trust_score: Option<u8>,
    pub affection_score: Option<u8>,
    pub power_score: Option<u8>,
}

/// 验证分数（0-100）
pub fn validate_score(score: Option<u8>) -> Result<(), StoryError> {
    if let Some(s) = score
        && s > 100
    {
        return Err(StoryError::InvalidRelationshipCandidate(format!(
            "score {s} exceeds maximum 100"
        )));
    }
    Ok(())
}
