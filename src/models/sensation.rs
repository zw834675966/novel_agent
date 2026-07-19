use super::VocabularyId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 五感选择结果
/// ==============
/// 包含视觉/听觉/嗅觉/触觉/味觉五个维度的词汇选择。
/// 每个字段是一个 VocabularyId 列表，每个 ID 指向 vocab.yaml 中的一条词汇。
///
/// 设计要点：
///   - 每个维度可以选多个词汇（Vec<VocabularyId>）
///   - Default 实现支持在无选择时使用默认空列表
///   - JsonSchema 标注使 rig Extractor 能生成 LLM 工具调用的 JSON Schema
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct SensorySelection {
    pub visual_ids: Vec<VocabularyId>,    // 视觉（如 "血迹"、"烛光"）
    pub auditory_ids: Vec<VocabularyId>,  // 听觉（如 "脚步声"）
    pub olfactory_ids: Vec<VocabularyId>, // 嗅觉（如 "血腥味"）
    pub tactile_ids: Vec<VocabularyId>,   // 触觉（如 "冰凉"）
    pub gustatory_ids: Vec<VocabularyId>, // 味觉（如 "苦涩"）
}
