use crate::models::{
    CharacterId, CharacterMemoryDraft, PlotDevelopment, RelationshipType, SensorySelection,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// LLM 关系候选（待作者确认）
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmRelationshipCandidate {
    pub target_character_id: CharacterId,
    pub relationship_type: RelationshipType,
    pub summary: String,
    #[serde(default)]
    pub tension_score: Option<u8>,
    #[serde(default)]
    pub trust_score: Option<u8>,
    #[serde(default)]
    pub affection_score: Option<u8>,
    #[serde(default)]
    pub power_score: Option<u8>,
    pub confidence: f32,
}

/// LLM 强类型输出契约
/// ======================
/// rig `Extractor` 的泛型参数 T，强制 LLM 按此结构返回数据。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmCharacterDerivation {
    /// 感官焦点分析（CoT）：LLM 在选择 ID 前先分析此场景的感官焦点。
    #[serde(default)]
    pub sensory_analysis: String,
    pub sensations: SensorySelection,
    pub new_memory: CharacterMemoryDraft,
    pub plot_development: Vec<PlotDevelopment>,
    #[serde(default)]
    pub relationship_candidates: Vec<LlmRelationshipCandidate>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct LlmContextTagSelection {
    #[serde(default)]
    pub tags: Vec<String>,
}
