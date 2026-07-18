use crate::models::{CharacterId, SceneId, SensorySelection, StoryError, VocabularyId};
use chrono::{DateTime, Utc};
use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

#[derive(Clone)]
pub struct SensationRepo {
    pool: SqlitePool,
}

fn ids_to_json(ids: &[VocabularyId]) -> String {
    serde_json::to_string(&ids.iter().map(|i| i.as_str().to_string()).collect::<Vec<_>>())
        .unwrap_or_default()
}

fn parse_ids(s: &str) -> Vec<VocabularyId> {
    serde_json::from_str::<Vec<String>>(s)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|id| VocabularyId::new(&id).ok())
        .collect()
}

impl SensationRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn latest(
        &self,
        character_id: CharacterId,
    ) -> Result<Option<(SensorySelection, SceneId)>, StoryError> {
        let row = sqlx::query(
            "SELECT scene_id, visual_ids_json, auditory_ids_json, olfactory_ids_json, \
             tactile_ids_json, gustatory_ids_json FROM character_sensations \
             WHERE character_id = ? ORDER BY created_at DESC LIMIT 1",
        )
        .bind(character_id.0.to_string())
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let scene_id_str: String = sqlx::Row::try_get(&row, "scene_id")?;
        let visual: String = sqlx::Row::try_get(&row, "visual_ids_json")?;
        let auditory: String = sqlx::Row::try_get(&row, "auditory_ids_json")?;
        let olfactory: String = sqlx::Row::try_get(&row, "olfactory_ids_json")?;
        let tactile: String = sqlx::Row::try_get(&row, "tactile_ids_json")?;
        let gustatory: String = sqlx::Row::try_get(&row, "gustatory_ids_json")?;
        let sel = SensorySelection {
            visual_ids: parse_ids(&visual),
            auditory_ids: parse_ids(&auditory),
            olfactory_ids: parse_ids(&olfactory),
            tactile_ids: parse_ids(&tactile),
            gustatory_ids: parse_ids(&gustatory),
        };
        let scene_id = SceneId(Uuid::parse_str(&scene_id_str).map_err(|e| StoryError::Database(e.to_string()))?);
        Ok(Some((sel, scene_id)))
    }

    pub async fn insert_in_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        character_id: CharacterId,
        scene_id: SceneId,
        sel: &SensorySelection,
        now: DateTime<Utc>,
    ) -> Result<(), StoryError> {
        sqlx::query(
            "INSERT INTO character_sensations \
             (id, character_id, scene_id, visual_ids_json, auditory_ids_json, olfactory_ids_json, \
              tactile_ids_json, gustatory_ids_json, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(character_id.0.to_string())
        .bind(scene_id.0.to_string())
        .bind(ids_to_json(&sel.visual_ids))
        .bind(ids_to_json(&sel.auditory_ids))
        .bind(ids_to_json(&sel.olfactory_ids))
        .bind(ids_to_json(&sel.tactile_ids))
        .bind(ids_to_json(&sel.gustatory_ids))
        .bind(now.to_rfc3339())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}
