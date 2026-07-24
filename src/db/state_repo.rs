// N5 长线物理状态机 Repository 模式实现
// ============================================

use crate::models::{CharacterId, CharacterState, StoryError};
use chrono::Utc;
use sqlx::SqlitePool;

pub struct StateRepo {
    pool: SqlitePool,
}

impl StateRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 设置/更新角色的物理/叙事状态
    pub async fn set_state(
        &self,
        character_id: CharacterId,
        state: &CharacterState,
    ) -> Result<(), StoryError> {
        let cid_str = character_id.0.to_string();
        let state_json =
            serde_json::to_string(state).map_err(|e| StoryError::Database(e.to_string()))?;
        let now_str = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO character_states (character_id, state_json, updated_at)
            VALUES (?, ?, ?)
            ON CONFLICT(character_id) DO UPDATE SET
                state_json = excluded.state_json,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(cid_str)
        .bind(state_json)
        .bind(now_str)
        .execute(&self.pool)
        .await
        .map_err(|e| StoryError::Database(e.to_string()))?;

        Ok(())
    }

    /// 获取角色的当前物理/叙事状态
    pub async fn get_state(
        &self,
        character_id: CharacterId,
    ) -> Result<Option<CharacterState>, StoryError> {
        let cid_str = character_id.0.to_string();

        let row = sqlx::query_scalar::<_, String>(
            r#"SELECT state_json FROM character_states WHERE character_id = ?"#,
        )
        .bind(cid_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StoryError::Database(e.to_string()))?;

        match row {
            Some(state_json) => {
                let state: CharacterState = serde_json::from_str(&state_json)
                    .map_err(|e| StoryError::Database(e.to_string()))?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }
}
