use crate::models::{
    Certainty, CharacterId, CharacterMemory, MemoryId, MemorySource, SceneId, StoryError,
};
use chrono::{DateTime, Utc};
use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

#[derive(Clone)]
pub struct MemoryRepo {
    pool: SqlitePool,
}

impl MemoryRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list(
        &self,
        character_id: CharacterId,
        limit: i64,
    ) -> Result<Vec<CharacterMemory>, StoryError> {
        let rows = sqlx::query(
            "SELECT id, scene_id, content, source, certainty, created_at \
             FROM character_memories WHERE character_id = ? \
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(character_id.0.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::new();
        for r in rows {
            let id_str: String = sqlx::Row::try_get(&r, "id")?;
            let scene_id_str: String = sqlx::Row::try_get(&r, "scene_id")?;
            let content: String = sqlx::Row::try_get(&r, "content")?;
            let source_str: String = sqlx::Row::try_get(&r, "source")?;
            let certainty_str: String = sqlx::Row::try_get(&r, "certainty")?;
            let created_at_str: String = sqlx::Row::try_get(&r, "created_at")?;
            out.push(CharacterMemory {
                id: MemoryId(
                    Uuid::parse_str(&id_str).map_err(|e| StoryError::Database(e.to_string()))?,
                ),
                character_id,
                scene_id: SceneId(
                    Uuid::parse_str(&scene_id_str)
                        .map_err(|e| StoryError::Database(e.to_string()))?,
                ),
                content,
                source: serde_json::from_str(&source_str).unwrap_or(MemorySource::Witnessed),
                certainty: serde_json::from_str(&certainty_str).unwrap_or(Certainty::Uncertain),
                created_at: DateTime::parse_from_rfc3339(&created_at_str)
                    .map_err(|e| StoryError::Database(e.to_string()))?
                    .with_timezone(&Utc),
            });
        }
        Ok(out)
    }

    pub async fn insert_in_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        character_id: CharacterId,
        scene_id: SceneId,
        content: &str,
        source: MemorySource,
        certainty: Certainty,
        now: DateTime<Utc>,
    ) -> Result<(), StoryError> {
        sqlx::query(
            "INSERT INTO character_memories (id, character_id, scene_id, content, source, certainty, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(character_id.0.to_string())
        .bind(scene_id.0.to_string())
        .bind(content)
        .bind(serde_json::to_string(&source).unwrap_or_default())
        .bind(serde_json::to_string(&certainty).unwrap_or_default())
        .bind(now.to_rfc3339())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}
