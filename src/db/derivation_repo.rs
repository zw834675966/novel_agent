use crate::models::{
    CharacterId, CharacterMemoryDraft, SceneId, SensorySelection, StoryError,
};
use crate::db::{MemoryRepo, SensationRepo};
use chrono::{DateTime, Utc};
use sqlx::sqlite::SqlitePool;

#[derive(Clone)]
pub struct DerivationRepo {
    pool: SqlitePool,
}

impl DerivationRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 原子写入：感官 + 新记忆同一事务。任一失败回滚。
    pub async fn insert_derivation(
        &self,
        character_id: CharacterId,
        scene_id: SceneId,
        sel: &SensorySelection,
        memory: &CharacterMemoryDraft,
        now: DateTime<Utc>,
    ) -> Result<(), StoryError> {
        let mut tx = self.pool.begin().await?;
        SensationRepo::insert_in_tx(&mut tx, character_id, scene_id, sel, now).await?;
        MemoryRepo::insert_in_tx(
            &mut tx,
            character_id,
            scene_id,
            &memory.content,
            memory.source,
            memory.certainty,
            now,
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
