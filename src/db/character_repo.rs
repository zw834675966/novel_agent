use crate::models::{Character, CharacterId, StoryError};
use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

#[derive(Clone)]
pub struct CharacterRepo {
    pool: SqlitePool,
}

impl CharacterRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        id: CharacterId,
        name: &str,
        personality: &[String],
        skills: &[String],
    ) -> Result<(), StoryError> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO characters (id, name, created_at, updated_at) VALUES (?, ?, ?, ?)")
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

    pub async fn update(
        &self,
        id: CharacterId,
        name: &str,
        personality: &[String],
        skills: &[String],
    ) -> Result<(), StoryError> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE characters SET name = ?, updated_at = ? WHERE id = ?")
            .bind(name)
            .bind(&now)
            .bind(id.0.to_string())
            .execute(&mut *tx)
            .await?;
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

    pub async fn get(&self, id: CharacterId) -> Result<Option<Character>, StoryError> {
        let row = sqlx::query("SELECT name FROM characters WHERE id = ?")
            .bind(id.0.to_string())
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let name: String = sqlx::Row::try_get(&row, "name")?;
        let personality: Vec<String> =
            sqlx::query("SELECT tag FROM character_personality_tags WHERE character_id = ? ORDER BY tag")
                .bind(id.0.to_string())
                .fetch_all(&self.pool)
                .await?
                .iter()
                .map(|r| sqlx::Row::try_get::<String, _>(r, "tag").unwrap_or_default())
                .collect();
        let skills: Vec<String> =
            sqlx::query("SELECT skill FROM character_skills WHERE character_id = ? ORDER BY skill")
                .bind(id.0.to_string())
                .fetch_all(&self.pool)
                .await?
                .iter()
                .map(|r| sqlx::Row::try_get::<String, _>(r, "skill").unwrap_or_default())
                .collect();
        Ok(Some(Character {
            id,
            name,
            personality,
            skills,
        }))
    }

    pub async fn exists(&self, id: CharacterId) -> Result<bool, StoryError> {
        let row = sqlx::query("SELECT 1 FROM characters WHERE id = ?")
            .bind(id.0.to_string())
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.is_some())
    }

    #[allow(dead_code)]
    fn parse_uuid(s: &str) -> Uuid {
        Uuid::parse_str(s).unwrap_or_default()
    }
}
