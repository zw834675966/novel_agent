use super::{CharacterId, SceneId};
use super::{CharacterMemoryDraft, PlotDevelopment, SensorySelection};
use serde::{Deserialize, Serialize};

/// 推导结果（持久化版本）
/// =========================
/// 与 LlmCharacterDerivation 的区别：
///   前者是 LLM 输出的原始结构，包含 Schema 用于工具调用；
///   这里是业务层的持久化版本，附带了 character_id 和 scene_id 关联信息。
///
/// 用途：作为 derive_character() 的返回值，包含该角色在当前场景中的
///       感官、新记忆、剧情发展推导结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterDerivation {
    pub character_id: CharacterId,              // 被推导的角色
    pub scene_id: SceneId,                      // 当前场景
    pub sensations: SensorySelection,           // LLM 选择的五感词汇
    pub new_memory: CharacterMemoryDraft,       // 新形成的记忆（未持久化的草稿）
    pub plot_development: Vec<PlotDevelopment>, // 剧情发展方向
}
