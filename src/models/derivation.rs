use super::{
    Character, CharacterId, CharacterMemory, PlotDevelopment, RelationshipCandidate, SceneId,
    SensorySelection, StoredPlotDevelopment,
};
use serde::{Deserialize, Serialize};

/// 推导结果（持久化版本）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterDerivation {
    pub character_id: CharacterId,
    pub scene_id: SceneId,
    pub sensations: SensorySelection,
    pub new_memory: CharacterMemory,
    pub plot_development: Vec<PlotDevelopment>,
    pub relationship_candidates: Vec<RelationshipCandidate>,
}

/// 场景推导详情（工作台读取模型）
/// ================================
/// 聚合某场景中单个角色的全部持久化状态：
///   - 角色 Character
///   - 持久化记忆 CharacterMemory
///   - 感官选择 SensorySelection
///   - 该场景的剧情发展（已持久化）
///   - 该场景的待确认关系候选
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDerivationDetail {
    pub character: Character,
    pub memory: CharacterMemory,
    pub sensation: SensorySelection,
    pub plot_developments: Vec<StoredPlotDevelopment>,
    pub relationship_candidates: Vec<RelationshipCandidate>,
}
