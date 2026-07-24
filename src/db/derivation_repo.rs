use crate::db::{MemoryRepo, PlotRepo, SensationRepo};
use crate::models::{
    CharacterId, CharacterMemory, CharacterMemoryDraft, MemoryId, PlotDevelopment, SceneId,
    SensorySelection, StoryError,
};
use chrono::{DateTime, Utc};
use sqlx::sqlite::SqlitePool;

/// 推导结果 Repository（组合写入）
/// ================================
/// 封装了一个"原子写入"操作：在一次事务中同时写入
/// 感官选择（character_sensations）和新记忆（character_memories）。
#[derive(Clone)]
pub struct DerivationRepo {
    pool: SqlitePool,
}

impl DerivationRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 原子写入：感官 + 新记忆同一事务
    ///
    /// # 返回
    /// 生成的 CharacterMemory（含 MemoryId）。
    pub async fn insert_derivation(
        &self,
        character_id: CharacterId,
        scene_id: SceneId,
        sel: &SensorySelection,
        memory: &CharacterMemoryDraft,
        now: DateTime<Utc>,
    ) -> Result<CharacterMemory, StoryError> {
        let mut tx = self.pool.begin().await?;
        SensationRepo::insert_in_tx(&mut tx, character_id, scene_id, sel, now).await?;
        let memory_id = MemoryRepo::insert_in_tx(
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
        Ok(CharacterMemory {
            id: memory_id,
            character_id,
            scene_id,
            content: memory.content.clone(),
            source: memory.source,
            certainty: memory.certainty,
            created_at: now,
        })
    }

    /// 原子替换：删除某 `(character_id, scene_id)` 的旧状态并在同一事务中插入全部新状态
    ///
    /// # 返回
    /// 生成的 CharacterMemory（含 MemoryId，供关系候选 evidence 使用）。
    #[allow(clippy::too_many_arguments)]
    pub async fn replace_derivation(
        &self,
        character_id: CharacterId,
        scene_id: SceneId,
        sensations: &SensorySelection,
        memory: &CharacterMemoryDraft,
        plots: &[PlotDevelopment],
        context_tags: &[String],
        now: DateTime<Utc>,
    ) -> Result<CharacterMemory, StoryError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM character_sensations WHERE character_id = ? AND scene_id = ?")
            .bind(character_id.0.to_string())
            .bind(scene_id.0.to_string())
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM character_memories WHERE character_id = ? AND scene_id = ?")
            .bind(character_id.0.to_string())
            .bind(scene_id.0.to_string())
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "DELETE FROM character_plot_developments WHERE character_id = ? AND scene_id = ?",
        )
        .bind(character_id.0.to_string())
        .bind(scene_id.0.to_string())
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "DELETE FROM character_derivation_context_tags WHERE character_id = ? AND scene_id = ?",
        )
        .bind(character_id.0.to_string())
        .bind(scene_id.0.to_string())
        .execute(&mut *tx)
        .await?;

        SensationRepo::insert_in_tx(&mut tx, character_id, scene_id, sensations, now).await?;
        let memory_id = MemoryRepo::insert_in_tx(
            &mut tx,
            character_id,
            scene_id,
            &memory.content,
            memory.source,
            memory.certainty,
            now,
        )
        .await?;
        for dev in plots {
            PlotRepo::insert_in_tx(&mut tx, character_id, scene_id, dev, now).await?;
        }
        PlotRepo::insert_context_tags_in_tx(&mut tx, character_id, scene_id, context_tags).await?;

        tx.commit().await?;
        Ok(CharacterMemory {
            id: memory_id,
            character_id,
            scene_id,
            content: memory.content.clone(),
            source: memory.source,
            certainty: memory.certainty,
            created_at: now,
        })
    }

    /// 获取指定角色的最新记忆 ID（供关系候选 evidence 关联）
    pub async fn latest_memory_id(
        &self,
        character_id: CharacterId,
    ) -> Result<Option<MemoryId>, StoryError> {
        let row = sqlx::query(
            "SELECT id FROM character_memories WHERE character_id = ? \
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(character_id.0.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => {
                let id_str: String = sqlx::Row::try_get(&r, "id")?;
                let uuid = uuid::Uuid::parse_str(&id_str)
                    .map_err(|e| StoryError::Database(e.to_string()))?;
                Ok(Some(MemoryId(uuid)))
            }
            None => Ok(None),
        }
    }
}
