use crate::models::{CharacterId, Scene, SceneId, StoryError};
use chrono::{DateTime, Utc};
use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

/// 场景 Repository
/// =================
/// 负责 scenes 主表 + scene_participants 关联表的 CRUD。
#[derive(Clone)]
pub struct SceneRepo {
    pool: SqlitePool,
}

impl SceneRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 创建场景（事务内写 1 主表 + N 参与者关联）
    pub async fn create(
        &self,
        id: SceneId,
        objective_event: &str,
        participant_ids: &[CharacterId],
        occurred_at: DateTime<Utc>,
    ) -> Result<(), StoryError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO scenes (id, objective_event, occurred_at) VALUES (?, ?, ?)")
            .bind(id.0.to_string())
            .bind(objective_event)
            .bind(occurred_at.to_rfc3339())
            .execute(&mut *tx)
            .await?;
        for cid in participant_ids {
            sqlx::query("INSERT INTO scene_participants (scene_id, character_id) VALUES (?, ?)")
                .bind(id.0.to_string())
                .bind(cid.0.to_string())
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// 按 ID 查询场景（含参与者列表，返回 None 表示不存在）
    pub async fn get(&self, id: SceneId) -> Result<Option<Scene>, StoryError> {
        let row = sqlx::query("SELECT objective_event, occurred_at FROM scenes WHERE id = ?")
            .bind(id.0.to_string())
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let objective_event: String = sqlx::Row::try_get(&row, "objective_event")?;
        let occurred_at_str: String = sqlx::Row::try_get(&row, "occurred_at")?;
        let occurred_at = DateTime::parse_from_rfc3339(&occurred_at_str)
            .map_err(|e| StoryError::Database(e.to_string()))?
            .with_timezone(&Utc);
        let participant_ids: Vec<CharacterId> =
            sqlx::query("SELECT character_id FROM scene_participants WHERE scene_id = ?")
                .bind(id.0.to_string())
                .fetch_all(&self.pool)
                .await?
                .iter()
                .map(|r| {
                    let s: String = sqlx::Row::try_get(r, "character_id")?;
                    let character_id =
                        Uuid::parse_str(&s).map_err(|e| StoryError::Database(e.to_string()))?;
                    Ok(CharacterId(character_id))
                })
                .collect::<Result<Vec<_>, StoryError>>()?;
        Ok(Some(Scene {
            id,
            objective_event,
            participant_ids,
            occurred_at,
        }))
    }

    /// 检查角色是否参与某场景
    pub async fn is_participant(
        &self,
        scene_id: SceneId,
        character_id: CharacterId,
    ) -> Result<bool, StoryError> {
        let row =
            sqlx::query("SELECT 1 FROM scene_participants WHERE scene_id = ? AND character_id = ?")
                .bind(scene_id.0.to_string())
                .bind(character_id.0.to_string())
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.is_some())
    }
}
