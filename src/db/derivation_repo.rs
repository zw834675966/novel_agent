use crate::db::{MemoryRepo, SensationRepo};
use crate::models::{CharacterId, CharacterMemoryDraft, SceneId, SensorySelection, StoryError};
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
    /// - `sel`    — LLM 选择的五感词汇
    /// - `memory` — LLM 生成的新记忆草稿
    ///
    /// 失败时：任何一步出错 → 全事务回滚，数据库状态不变
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
