use crate::models::{CharacterMemoryDraft, PlotDevelopment, SensorySelection};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// LLM 强类型输出契约。
/// 通过 rig `Extractor<LlmCharacterDerivation>` 强制模型调用 submit 工具提交。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmCharacterDerivation {
    pub sensations: SensorySelection,
    pub new_memory: CharacterMemoryDraft,
    pub plot_development: Vec<PlotDevelopment>,
}
