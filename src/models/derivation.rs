use super::{
    CharacterId, CharacterMemory, PlotDevelopment, RelationshipCandidate, SceneId, SensorySelection,
};
use serde::{Deserialize, Serialize};

/// 推导结果（持久化版本）
/// =========================
/// 与 LlmCharacterDerivation 的区别：
///   前者是 LLM 输出的原始结构，包含 Schema 用于工具调用；
///   这里是业务层的持久化版本，附带了 character_id 和 scene_id 关联信息。
///
/// 用途：作为 derive_character() 的返回值，包含该角色在当前场景中的
///       感官、新记忆（已持久化含 ID）、剧情发展、关系候选推导结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterDerivation {
    pub character_id: CharacterId,                           // 被推导的角色
    pub scene_id: SceneId,                                   // 当前场景
    pub sensations: SensorySelection,                        // LLM 选择的五感词汇
    pub new_memory: CharacterMemory,                         // 已持久化的新记忆（含 MemoryId）
    pub plot_development: Vec<PlotDevelopment>,              // 剧情发展方向
    pub relationship_candidates: Vec<RelationshipCandidate>, // 关系候选（待作者确认）
}
