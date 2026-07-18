use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CharacterId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SceneId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct VocabularyId(String);

impl VocabularyId {
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

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn sense(&self) -> &str {
        self.0.split_once('.').unwrap().0
    }

    pub fn key(&self) -> &str {
        self.0.split_once('.').unwrap().1
    }
}

impl std::fmt::Display for VocabularyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
