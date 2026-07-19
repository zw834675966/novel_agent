use super::{Certainty, MemorySource};
use super::{CharacterId, MemoryId, SceneId};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 角色记忆（持久化实体）
/// =========================
/// 对应数据库 character_memories 表的一行。
/// 每次推导后，新记忆会被写入此表。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterMemory {
    pub id: MemoryId,              // UUID 主键
    pub character_id: CharacterId, // 所属角色
    pub scene_id: SceneId,         // 产生此记忆的场景
    pub content: String,           // 记忆内容
    pub source: MemorySource,      // 获取来源（亲眼所见/听说/推断）
    pub certainty: Certainty,      // 确定性级别（确定/怀疑/不确定）
    pub created_at: DateTime<Utc>, // 创建时间
}

/// 记忆草稿（LLM 输出结构）
/// ===========================
/// 不包含 id 和时间戳（由系统在持久化时生成）。
/// 用 JsonSchema 标注以支持 rig Extractor 的 JSON Schema 生成。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CharacterMemoryDraft {
    pub content: String,      // 记忆内容
    pub source: MemorySource, // 获取来源
    pub certainty: Certainty, // 确定性级别
}
