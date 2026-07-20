use crate::models::{CharacterId, SceneId};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 剧情发展方向枚举
/// ===================
/// 表示当前场景可能引发的叙事类型。
/// LLM 需要针对每个角色选择一种或多种发展。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlotDevelopmentKind {
    SuspicionRaised,     // 产生怀疑
    ConflictEscalated,   // 冲突升级
    GoalChanged,         // 目标改变
    RelationshipShifted, // 关系变化
    NewClue,             // 获得新线索
}

/// 剧情发展（含原因）
/// ====================
/// LLM 不仅要标记发展类型，还要给出简短的文本理由。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlotDevelopment {
    pub kind: PlotDevelopmentKind, // 发展类型
    pub reason: String,            // 原因描述（LLM 生成的文本）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPlotDevelopment {
    pub character_id: CharacterId,
    pub scene_id: SceneId,
    pub development: PlotDevelopment,
    pub created_at: DateTime<Utc>,
}
