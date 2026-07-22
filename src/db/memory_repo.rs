use crate::models::{
    Certainty, CharacterId, CharacterMemory, MemoryId, MemorySource, SceneId, StoryError,
};
use chrono::{DateTime, Utc};
use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

/// 角色记忆 Repository
/// =====================
/// 负责 character_memories 表的查询和插入。
/// 查询按 (character_id, created_at DESC) 索引高效取最近 N 条。
/// 插入作为静态方法提供给 DerivationRepo 跨表事务复用。
#[derive(Clone)]
pub struct MemoryRepo {
    pool: SqlitePool,
}

impl MemoryRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 按角色获取最近 N 条记忆（按时间倒序）
    ///
    /// # 参数
    /// - `character_id` - 角色 ID
    /// - `limit` - 返回上限（主业务流程传 50）
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
                source: serde_json::from_str(&source_str)
                    .map_err(|e| StoryError::Database(e.to_string()))?,
                certainty: serde_json::from_str(&certainty_str)
                    .map_err(|e| StoryError::Database(e.to_string()))?,
                created_at: DateTime::parse_from_rfc3339(&created_at_str)
                    .map_err(|e| StoryError::Database(e.to_string()))?
                    .with_timezone(&Utc),
            });
        }
        Ok(out)
    }

    /// 按角色获取发生在指定场景之前的记忆（按 scenes.occurred_at 比较）
    ///
    /// # 参数
    /// - `character_id` - 角色 ID
    /// - `before` - 该场景的 `occurred_at` 时间戳；早于该时间的记忆才返回
    /// - `limit` - 返回上限
    ///
    /// # 排序
    /// `s.occurred_at DESC, m.created_at DESC`：场景时间倒序，同一场景内按写入顺序倒序。
    ///
    /// # 错误
    /// - `StoryError::Database` - 任何 UUID、时间戳或枚举 JSON 解析失败
    pub async fn list_before_scene(
        &self,
        character_id: CharacterId,
        before: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<CharacterMemory>, StoryError> {
        let rows = sqlx::query(
            "SELECT m.id, m.scene_id, m.content, m.source, m.certainty, m.created_at \
             FROM character_memories m \
             JOIN scenes s ON s.id = m.scene_id \
             WHERE m.character_id = ? AND s.occurred_at < ? \
             ORDER BY s.occurred_at DESC, m.created_at DESC \
             LIMIT ?",
        )
        .bind(character_id.0.to_string())
        .bind(before.to_rfc3339())
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
                source: serde_json::from_str(&source_str)
                    .map_err(|e| StoryError::Database(e.to_string()))?,
                certainty: serde_json::from_str(&certainty_str)
                    .map_err(|e| StoryError::Database(e.to_string()))?,
                created_at: DateTime::parse_from_rfc3339(&created_at_str)
                    .map_err(|e| StoryError::Database(e.to_string()))?
                    .with_timezone(&Utc),
            });
        }
        Ok(out)
    }

    /// 在已有事务中插入记忆（供 DerivationRepo 跨表事务调用）
    ///
    /// # 返回
    /// 生成的 MemoryId（供调用方构造 CharacterMemory 或关系候选 evidence）。
    pub async fn insert_in_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        character_id: CharacterId,
        scene_id: SceneId,
        content: &str,
        source: MemorySource,
        certainty: Certainty,
        now: DateTime<Utc>,
    ) -> Result<MemoryId, StoryError> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO character_memories (id, character_id, scene_id, content, source, certainty, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(character_id.0.to_string())
        .bind(scene_id.0.to_string())
        .bind(content)
        .bind(
            serde_json::to_string(&source)
                .map_err(|e| StoryError::Database(e.to_string()))?,
        )
        .bind(
            serde_json::to_string(&certainty)
                .map_err(|e| StoryError::Database(e.to_string()))?,
        )
        .bind(now.to_rfc3339())
        .execute(&mut **tx)
        .await?;
        Ok(MemoryId(id))
    }
}
