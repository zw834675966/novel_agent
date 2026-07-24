use crate::models::{CharacterId, PlotReasonSlot, SceneId};
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

/// 剧情发展（含结构化原因 N4）
/// =================================
/// 原因使用 `PlotReasonSlot` 枚举槽，禁止 LLM 生成开放抒情或因果套话。
/// 平台 P1 信号治理：消除 AI 因果说明文风格输出。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlotDevelopment {
    pub kind: PlotDevelopmentKind, // 发展类型
    pub reason: PlotReasonSlot,    // 结构化原因短槽（N4: 防AI套话）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPlotDevelopment {
    pub character_id: CharacterId,
    pub scene_id: SceneId,
    pub development: PlotDevelopment,
    pub created_at: DateTime<Utc>,
}
