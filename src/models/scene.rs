use super::CharacterId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 场景（持久化实体）
/// =====================
/// 对应数据库 scenes + scene_participants 两张表。
///
/// 字段说明：
///   id                → UUID 主键
///   objective_event   → 客观事件描述（不包含任何角色的主观感受）
///   participant_ids   → 参与该场景的角色 ID 列表
///   occurred_at       → 场景发生时间
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub id: super::SceneId,
    pub objective_event: String,
    pub participant_ids: Vec<CharacterId>,
    pub occurred_at: DateTime<Utc>,
}

/// 创建场景请求
/// ==============
/// 与 Scene 的区别：不包含 id（由系统在 create 时生成）。
/// 用于 StoryService::create_scene() 的入参。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateScene {
    pub objective_event: String,
    pub participant_ids: Vec<CharacterId>,
    pub occurred_at: DateTime<Utc>,
}
