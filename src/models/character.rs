use super::CharacterId;
use serde::{Deserialize, Serialize};

/// 角色定义
/// =============
/// 对应数据库 characters + character_personality_tags + character_skills 三张表。
/// 字段说明：
///   id          → UUID（CharacterId newtype）
///   name        → 角色显示名
///   personality → 性格标签列表（如 ["谨慎", "勇敢"]）
///   skills      → 能力标签列表（如 ["推理", "剑术"]）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: CharacterId,
    pub name: String,
    pub personality: Vec<String>,
    pub skills: Vec<String>,
}

/// 角色物理/叙事状态（N5 长线物理状态机）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, schemars::JsonSchema)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum CharacterState {
    /// 存活且正常
    #[default]
    Alive,
    /// 死亡
    Dead,
    /// 不在场/离场
    Absent,
    /// 负伤/受创
    Injured(String),
    /// 持有物品/兵刃
    Holding(String),
}
