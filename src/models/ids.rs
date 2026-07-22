use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 角色 ID（UUID newtype）
/// ==========================
/// 类型安全的角色标识，避免与 SceneId/MemoryId 混淆。
/// 派生 Copy 以支持在集合中按值传递。
/// 业务上用于主键和外键绑定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CharacterId(pub Uuid);

/// 场景 ID（UUID newtype）
/// 对应场景发生的时间点与参与者聚合，避免与角色/记忆 ID 混淆。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SceneId(pub Uuid);

/// 记忆 ID（UUID newtype）
/// 用于 character_memories 主键，避免与其他实体 ID 混淆。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryId(pub Uuid);

/// 关系事实 ID（UUID newtype）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelationshipFactId(pub Uuid);

/// 关系候选 ID（UUID newtype）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelationshipCandidateId(pub Uuid);

/// 词汇 ID（字符串 newtype）
/// =============================
/// 格式："{sense}.{key}"，如 "visual.bloodstain"
///   sense → 五感类别（visual/auditory/olfactory/tactile/gustatory）
///   key   → 词汇表内的唯一键
///
/// 构建时校验：
///   - 必须包含一个 '.' 分隔符
///   - 分隔符前后都不能为空
///     此校验确保 LLM 无法捏造不合法的词汇引用。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct VocabularyId(String);

impl VocabularyId {
    /// 从字符串创建 VocabularyId，同时校验格式
    ///
    /// # 错误
    /// - `StoryError::InvalidVocabularySelection` — 格式不正确
    pub fn new(raw: &str) -> Result<Self, super::StoryError> {
        let (sense, key) = raw.split_once('.').ok_or_else(|| {
            super::StoryError::InvalidVocabularySelection(format!("missing '.' in {raw}"))
        })?;
        if sense.is_empty() || key.is_empty() {
            return Err(super::StoryError::InvalidVocabularySelection(format!(
                "empty segment in {raw}"
            )));
        }
        Ok(Self(raw.to_string()))
    }

    /// 获取完整的 "sense.key" 字符串（用于候选集比较）
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 获取感官类别（"visual"/"auditory"/等）
    pub fn sense(&self) -> &str {
        self.0.split_once('.').unwrap().0
    }

    /// 获取词汇键名
    pub fn key(&self) -> &str {
        self.0.split_once('.').unwrap().1
    }
}

impl std::fmt::Display for VocabularyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
