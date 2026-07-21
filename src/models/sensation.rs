use super::VocabularyId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 感官与描写选择结果
/// ====================
/// 包含视觉/听觉/嗅觉/触觉/味觉五感,以及情绪心理/动作神态/氛围环境
/// 三个细粒度描写维度。每个字段是一个 VocabularyId 列表,
/// 每个 ID 指向词库(vocab.yaml / 蒸馏素材库)中的一条条目。
///
/// 设计要点：
///   - 每个维度可以选多个词汇（Vec<VocabularyId>）
///   - Default 实现支持在无选择时使用默认空列表
///   - JsonSchema 标注使 rig Extractor 能生成 LLM 工具调用的 JSON Schema
///   - serde(default) 让旧数据/旧 LLM 输出缺少新字段时反序列化为空列表
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct SensorySelection {
    pub visual_ids: Vec<VocabularyId>,    // 视觉（如 "血迹"、"烛光"）
    pub auditory_ids: Vec<VocabularyId>,  // 听觉（如 "脚步声"）
    pub olfactory_ids: Vec<VocabularyId>, // 嗅觉（如 "血腥味"）
    pub tactile_ids: Vec<VocabularyId>,   // 触觉（如 "冰凉"）
    pub gustatory_ids: Vec<VocabularyId>, // 味觉（如 "苦涩"）
    #[serde(default)]
    pub emotion_ids: Vec<VocabularyId>, // 情绪心理（原著情感描写片段）
    #[serde(default)]
    pub gesture_ids: Vec<VocabularyId>, // 动作神态（原著动作描写片段）
    #[serde(default)]
    pub atmosphere_ids: Vec<VocabularyId>, // 氛围环境（原著环境描写片段）
}
