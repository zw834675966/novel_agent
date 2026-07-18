use super::{CharacterId, SceneId};

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
    #[error("llm error: {0}")]
    Llm(String),
    #[error("database error: {0}")]
    Database(String),
}

impl From<sqlx::Error> for StoryError {
    fn from(e: sqlx::Error) -> Self {
        StoryError::Database(e.to_string())
    }
}

impl From<sqlx::migrate::MigrateError> for StoryError {
    fn from(e: sqlx::migrate::MigrateError) -> Self {
        StoryError::Database(e.to_string())
    }
}
