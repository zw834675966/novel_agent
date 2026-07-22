use crate::db::{MemoryRepo, PlotRepo, SensationRepo};
use crate::models::{
    CharacterId, CharacterMemoryDraft, PlotDevelopment, SceneId, SensorySelection, StoryError,
};
use chrono::{DateTime, Utc};
use sqlx::sqlite::SqlitePool;

/// 推导结果 Repository（组合写入）
/// ================================
/// 封装了一个"原子写入"操作：在一次事务中同时写入
/// 感官选择（character_sensations）和新记忆（character_memories）。
///
/// 这种设计确保：
///   - 要么感官和记忆都写入成功
///   - 要么都不写入（事务回滚）
///     不会出现"有感官没记忆"或"有记忆没感官"的不一致状态。
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
    /// # 参数
    /// - `sel`    - LLM 选择的五感词汇
    /// - `memory` - LLM 生成的新记忆草稿
    ///
    /// 失败时：任何一步出错 -> 全事务回滚，数据库状态不变
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

    /// 原子替换：删除某 `(character_id, scene_id)` 的旧状态并在同一事务中插入全部新状态
    ///
    /// 在一次 `pool.begin()` 事务中：
    ///   1. 删除 `character_sensations`、`character_memories`、
    ///      `character_plot_developments`、`character_derivation_context_tags`
    ///      中该 (character_id, scene_id) 的所有行
    ///   2. 调用 `SensationRepo::insert_in_tx` 写入新感官
    ///   3. 调用 `MemoryRepo::insert_in_tx` 写入新记忆
    ///   4. 对每个 plot 调用 `PlotRepo::insert_in_tx`
    ///   5. 调用 `PlotRepo::insert_context_tags_in_tx` 写入上下文标签
    ///   6. 全部成功后才 commit
    ///
    /// 失败时（任一步骤出错，包括外键约束失败）：全事务回滚，旧状态保留。
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
    ) -> Result<(), StoryError> {
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
        for dev in plots {
            PlotRepo::insert_in_tx(&mut tx, character_id, scene_id, dev, now).await?;
        }
        PlotRepo::insert_context_tags_in_tx(&mut tx, character_id, scene_id, context_tags).await?;

        tx.commit().await?;
        Ok(())
    }
}
