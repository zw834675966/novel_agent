use crate::models::{Character, CharacterId, StoryError};
use sqlx::sqlite::SqlitePool;

/// 角色 Repository
/// =================
/// 负责 characters 主表 + character_personality_tags + character_skills 子表的 CRUD。
/// 创建和更新都在同一个事务中完成，确保主表和标签表一致。
///
/// 子表设计原因：personality 和 skills 是可变长度的标签列表，
/// 不适合存在主表的单字段中（避免序列化/反序列化开销，支持 SQL 级联删除）。
#[derive(Clone)]
pub struct CharacterRepo {
    pool: SqlitePool,
}

impl CharacterRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 创建角色（事务内写 1 主表 + N 标签 + M 技能）
    pub async fn create(
        &self,
        id: CharacterId,
        name: &str,
        personality: &[String],
        skills: &[String],
    ) -> Result<(), StoryError> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO characters (id, name, created_at, updated_at) VALUES (?, ?, ?, ?)",
        )
        .bind(id.0.to_string())
        .bind(name)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
        for t in personality {
            sqlx::query("INSERT INTO character_personality_tags (character_id, tag) VALUES (?, ?)")
                .bind(id.0.to_string())
                .bind(t)
                .execute(&mut *tx)
                .await?;
        }
        for s in skills {
            sqlx::query("INSERT INTO character_skills (character_id, skill) VALUES (?, ?)")
                .bind(id.0.to_string())
                .bind(s)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// 更新角色（先删旧标签再插新标签，实现"全量替换"语义）
    pub async fn update(
        &self,
        id: CharacterId,
        name: &str,
        personality: &[String],
        skills: &[String],
    ) -> Result<(), StoryError> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query("UPDATE characters SET name = ?, updated_at = ? WHERE id = ?")
            .bind(name)
            .bind(&now)
            .bind(id.0.to_string())
            .execute(&mut *tx)
            .await?;
        if result.rows_affected() == 0 {
            return Err(StoryError::CharacterNotFound(id));
        }
        sqlx::query("DELETE FROM character_personality_tags WHERE character_id = ?")
            .bind(id.0.to_string())
            .execute(&mut *tx)
            .await?;
        for t in personality {
            sqlx::query("INSERT INTO character_personality_tags (character_id, tag) VALUES (?, ?)")
                .bind(id.0.to_string())
                .bind(t)
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("DELETE FROM character_skills WHERE character_id = ?")
            .bind(id.0.to_string())
            .execute(&mut *tx)
            .await?;
        for s in skills {
            sqlx::query("INSERT INTO character_skills (character_id, skill) VALUES (?, ?)")
                .bind(id.0.to_string())
                .bind(s)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// 按 ID 查询角色（含标签，返回 None 表示不存在）
    pub async fn get(&self, id: CharacterId) -> Result<Option<Character>, StoryError> {
        let row = sqlx::query("SELECT name FROM characters WHERE id = ?")
            .bind(id.0.to_string())
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let name: String = sqlx::Row::try_get(&row, "name")?;
        let personality: Vec<String> = sqlx::query(
            "SELECT tag FROM character_personality_tags WHERE character_id = ? ORDER BY tag",
        )
        .bind(id.0.to_string())
        .fetch_all(&self.pool)
        .await?
        .iter()
        .map(|r| sqlx::Row::try_get::<String, _>(r, "tag"))
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
        let skills: Vec<String> =
            sqlx::query("SELECT skill FROM character_skills WHERE character_id = ? ORDER BY skill")
                .bind(id.0.to_string())
                .fetch_all(&self.pool)
                .await?
                .iter()
                .map(|r| sqlx::Row::try_get::<String, _>(r, "skill"))
                .collect::<Result<Vec<_>, sqlx::Error>>()?;
        Ok(Some(Character {
            id,
            name,
            personality,
            skills,
        }))
    }

    /// 检查角色是否存在
    pub async fn exists(&self, id: CharacterId) -> Result<bool, StoryError> {
        let row = sqlx::query("SELECT 1 FROM characters WHERE id = ?")
            .bind(id.0.to_string())
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.is_some())
    }
}
