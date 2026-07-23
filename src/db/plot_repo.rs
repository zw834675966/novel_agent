use crate::db::row_util::{get_json, get_rfc3339, get_uuid};
use crate::models::{CharacterId, PlotDevelopment, SceneId, StoredPlotDevelopment, StoryError};
use chrono::{DateTime, Utc};
use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

/// 剧情发展 Repository
/// ======================
/// 负责 character_plot_developments 表的查询与事务内插入。
/// 查询 `list_before_scene` 通过 JOIN scenes 表取早于指定场景的剧情发展，
/// 按 (s.occurred_at DESC, p.created_at DESC) 排序，与记忆连续性语义一致。
///
/// 枚举 `PlotDevelopmentKind` 以 serde JSON 字符串存储（与 MemorySource/Certainty 一致）。
/// 解析失败返回 `StoryError::Database`，不静默降级。
#[derive(Clone)]
pub struct PlotRepo {
    pool: SqlitePool,
}

impl PlotRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 查询某角色在指定场景之前（按 scenes.occurred_at 比较）发生的剧情发展。
    ///
    /// # 参数
    /// - `character_id` - 角色 ID
    /// - `before` - 该场景的 `occurred_at` 时间戳；早于该时间的剧情才返回
    ///
    /// # 排序
    /// `s.occurred_at DESC, p.created_at DESC`：场景时间倒序，同一场景内按写入顺序倒序。
    ///
    /// # 错误
    /// - `StoryError::Database` - 任何 UUID、时间戳或枚举 JSON 解析失败
    pub async fn list_before_scene(
        &self,
        character_id: CharacterId,
        before: DateTime<Utc>,
    ) -> Result<Vec<StoredPlotDevelopment>, StoryError> {
        let rows = sqlx::query(
            "SELECT p.id, p.character_id, p.scene_id, p.kind, p.reason, p.created_at, s.occurred_at \
             FROM character_plot_developments p \
             JOIN scenes s ON s.id = p.scene_id \
             WHERE p.character_id = ? AND s.occurred_at < ? \
             ORDER BY s.occurred_at DESC, p.created_at DESC",
        )
        .bind(character_id.0.to_string())
        .bind(before.to_rfc3339())
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::new();
        for r in rows {
            let cid = get_uuid(&r, "character_id")?;
            let sid = get_uuid(&r, "scene_id")?;
            let kind = get_json(&r, "kind")?;
            let reason = get_json(&r, "reason")?;
            let created_at = get_rfc3339(&r, "created_at")?;

            out.push(StoredPlotDevelopment {
                character_id: CharacterId(cid),
                scene_id: SceneId(sid),
                development: PlotDevelopment { kind, reason },
                created_at,
            });
        }
        Ok(out)
    }

    /// 列出指定场景的所有剧情发展（按 created_at ASC）
    pub async fn list_for_scene(
        &self,
        scene_id: SceneId,
    ) -> Result<Vec<StoredPlotDevelopment>, StoryError> {
        let rows = sqlx::query(
            "SELECT id, character_id, scene_id, kind, reason, created_at \
             FROM character_plot_developments WHERE scene_id = ? \
             ORDER BY created_at ASC",
        )
        .bind(scene_id.0.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::new();
        for r in rows {
            let cid = get_uuid(&r, "character_id")?;
            let kind = get_json(&r, "kind")?;
            let reason = get_json(&r, "reason")?;
            let created_at = get_rfc3339(&r, "created_at")?;

            out.push(StoredPlotDevelopment {
                character_id: CharacterId(cid),
                scene_id,
                development: PlotDevelopment { kind, reason },
                created_at,
            });
        }
        Ok(out)
    }

    /// 在已有事务中插入一条剧情发展（供 DerivationRepo 跨表事务调用）
    ///
    /// # 设计说明
    /// pub(crate) 而非 pub：只允许 DerivationRepo 在事务中调用，
    /// 不允许单独插入剧情（必须与感官+记忆一起写入以保持推导结果一致性）。
    pub async fn insert_in_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        character_id: CharacterId,
        scene_id: SceneId,
        development: &PlotDevelopment,
        now: DateTime<Utc>,
    ) -> Result<(), StoryError> {
        sqlx::query(
            "INSERT INTO character_plot_developments (id, character_id, scene_id, kind, reason, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(character_id.0.to_string())
        .bind(scene_id.0.to_string())
        .bind(
            serde_json::to_string(&development.kind)
                .map_err(|e| StoryError::Database(e.to_string()))?,
        )
        .bind(
            serde_json::to_string(&development.reason)
                .map_err(|e| StoryError::Database(e.to_string()))?,
        )
        .bind(now.to_rfc3339())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// 在已有事务中插入一组推导上下文标签（character_derivation_context_tags）。
    ///
    /// # 设计说明
    /// pub(crate) 而非 pub：只允许 DerivationRepo 在事务中调用，
    /// 标签必须与推导结果同事务写入，避免孤立标签污染下一轮上下文选择。
    pub async fn insert_context_tags_in_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        character_id: CharacterId,
        scene_id: SceneId,
        tags: &[String],
    ) -> Result<(), StoryError> {
        for tag in tags {
            sqlx::query(
                "INSERT OR IGNORE INTO character_derivation_context_tags (character_id, scene_id, tag) \
                 VALUES (?, ?, ?)",
            )
            .bind(character_id.0.to_string())
            .bind(scene_id.0.to_string())
            .bind(tag)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }
}
