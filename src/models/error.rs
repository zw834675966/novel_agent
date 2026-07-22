use super::{CharacterId, SceneId};

/// 全局错误枚举（thiserror）
/// ============================
/// 覆盖项目中的所有错误类型，从数据访问到 LLM 调用。
/// 通过 `From<sqlx::Error>` 和 `From<sqlx::migrate::MigrateError>` 实现自动转换，
/// 使得 `?` 操作符可以直接在 DB 操作后使用。
#[derive(Debug, thiserror::Error)]
pub enum StoryError {
    #[error("scene not found: {0:?}")]
    SceneNotFound(SceneId),

    #[error("character not found: {0:?}")]
    CharacterNotFound(CharacterId),

    #[error("character {0:?} is not a participant of scene {1:?}")]
    NotSceneParticipant(CharacterId, SceneId),

    #[error("vocabulary load error: {0}")]
    VocabularyLoad(String),

    #[error("invalid vocabulary selection: {0}")]
    InvalidVocabularySelection(String),

    #[error("invalid narration context: {0}")]
    InvalidNarrationContext(String),

    #[error("llm error: {0}")]
    Llm(String),

    #[error("database error: {0}")]
    Database(String),

    #[error("relationship candidate not found: {0:?}")]
    RelationshipCandidateNotFound(crate::models::RelationshipCandidateId),

    #[error("relationship candidate already resolved: {0:?}")]
    RelationshipCandidateResolved(crate::models::RelationshipCandidateId),

    #[error("invalid relationship candidate: {0}")]
    InvalidRelationshipCandidate(String),
}

/// sqlx 错误 → StoryError 自动转换
impl From<sqlx::Error> for StoryError {
    fn from(e: sqlx::Error) -> Self {
        StoryError::Database(e.to_string())
    }
}

/// 数据库迁移错误 → StoryError 自动转换
impl From<sqlx::migrate::MigrateError> for StoryError {
    fn from(e: sqlx::migrate::MigrateError) -> Self {
        StoryError::Database(e.to_string())
    }
}
