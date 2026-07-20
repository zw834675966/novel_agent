use crate::models::{CharacterMemoryDraft, PlotDevelopment, SensorySelection};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// LLM 强类型输出契约
/// ======================
/// 这个结构体是 rig `Extractor` 的泛型参数 T。
/// rig 会在调用 DeepSeek API 时自动注入 `submit` 工具的 JSON Schema，
/// 强制 LLM 按此结构返回数据，并反序列化为 Rust 类型。
///
/// 字段说明：
///   sensations      → LLM 选择的五感词汇 ID（只能从候选集中选）
///   new_memory      → 角色在当前场景中形成的新记忆
///   plot_development → 由此情节可能引发的剧情走向
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmCharacterDerivation {
    pub sensations: SensorySelection,
    pub new_memory: CharacterMemoryDraft,
    pub plot_development: Vec<PlotDevelopment>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct LlmContextTagSelection {
    #[serde(default)]
    pub tags: Vec<String>,
}
