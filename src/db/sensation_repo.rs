use crate::db::row_util::{get_string, get_uuid};
use crate::models::{CharacterId, SceneId, SensorySelection, StoryError, VocabularyId};
use chrono::{DateTime, Utc};
use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

/// 五感 Repository
/// ==================
/// 负责 character_sensations 表的查询和插入。
/// 五感数据以 JSON TEXT 列存储（每个感官维度一个 JSON 字符串列表）。
///
/// 存储格式选择因由：
///   - 五感是 LLM 输出的结构化数据，查询模式固定（只按 character_id 取最新）
///   - 不需要对感官 ID 做关系型查询
///   - JSON 存储避免了多张子表的 JOIN 开销
#[derive(Clone)]
pub struct SensationRepo {
    pool: SqlitePool,
}

/// 将 VocabularyId 列表序列化为 JSON 字符串数组
fn ids_to_json(ids: &[VocabularyId]) -> String {
    serde_json::to_string(
        &ids.iter()
            .map(|i| i.as_str().to_string())
            .collect::<Vec<_>>(),
    )
    .unwrap_or_default()
}

/// 将 JSON 字符串数组反序列化为 VocabularyId 列表
fn parse_ids(s: &str) -> Result<Vec<VocabularyId>, StoryError> {
    serde_json::from_str::<Vec<String>>(s)
        .map_err(|e| StoryError::Database(e.to_string()))?
        .into_iter()
        .map(|id| VocabularyId::new(&id).map_err(|e| StoryError::Database(e.to_string())))
        .collect::<Result<Vec<_>, StoryError>>()
}

impl SensationRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 获取某个角色最近一次感官选择（用于下一场景的连续性参考）
    ///
    /// # 返回
    /// - `Ok(Some((SensorySelection, SceneId)))` - 最近感官 + 对应场景 ID
    /// - `Ok(None)` - 该角色还没有感官记录
    pub async fn latest(
        &self,
        character_id: CharacterId,
    ) -> Result<Option<(SensorySelection, SceneId)>, StoryError> {
        let row = sqlx::query(
            "SELECT scene_id, visual_ids_json, auditory_ids_json, olfactory_ids_json, \
             tactile_ids_json, gustatory_ids_json, emotion_ids_json, gesture_ids_json, \
             atmosphere_ids_json FROM character_sensations \
             WHERE character_id = ? ORDER BY created_at DESC LIMIT 1",
        )
        .bind(character_id.0.to_string())
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(Self::parse_row(&row)?))
    }

    /// 获取某个角色在指定场景之前（按 scenes.occurred_at 比较）最近一次感官选择
    ///
    /// # 参数
    /// - `character_id` - 角色 ID
    /// - `before` - 当前场景的 `occurred_at` 时间戳；早于该时间的感官才返回
    ///
    /// # 排序
    /// `s.occurred_at DESC, cs.created_at DESC LIMIT 1`
    pub async fn latest_before_scene(
        &self,
        character_id: CharacterId,
        before: DateTime<Utc>,
    ) -> Result<Option<(SensorySelection, SceneId)>, StoryError> {
        let row = sqlx::query(
            "SELECT cs.scene_id, cs.visual_ids_json, cs.auditory_ids_json, cs.olfactory_ids_json, \
             cs.tactile_ids_json, cs.gustatory_ids_json, cs.emotion_ids_json, \
             cs.gesture_ids_json, cs.atmosphere_ids_json FROM character_sensations cs \
             JOIN scenes s ON s.id = cs.scene_id \
             WHERE cs.character_id = ? AND s.occurred_at < ? \
             ORDER BY s.occurred_at DESC, cs.created_at DESC LIMIT 1",
        )
        .bind(character_id.0.to_string())
        .bind(before.to_rfc3339())
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(Self::parse_row(&row)?))
    }

    /// 列出指定场景的所有感官选择（含 character_id，按 created_at ASC）
    pub async fn list_for_scene(
        &self,
        scene_id: SceneId,
    ) -> Result<Vec<(CharacterId, SensorySelection)>, StoryError> {
        let rows = sqlx::query(
            "SELECT character_id, scene_id, visual_ids_json, auditory_ids_json, olfactory_ids_json, \
             tactile_ids_json, gustatory_ids_json, emotion_ids_json, gesture_ids_json, \
             atmosphere_ids_json FROM character_sensations \
             WHERE scene_id = ? ORDER BY created_at ASC",
        )
        .bind(scene_id.0.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::new();
        for row in &rows {
            let character_id = CharacterId(get_uuid(row, "character_id")?);
            let (sel, _) = Self::parse_row(row)?;
            out.push((character_id, sel));
        }
        Ok(out)
    }

    /// 将一行 character_sensations 记录解析为 (SensorySelection, SceneId)
    /// 严格 JSON 解析：任一维度解析失败 -> StoryError::Database
    fn parse_row(row: &sqlx::sqlite::SqliteRow) -> Result<(SensorySelection, SceneId), StoryError> {
        let scene_id = SceneId(get_uuid(row, "scene_id")?);
        let visual: String = get_string(row, "visual_ids_json")?;
        let auditory: String = get_string(row, "auditory_ids_json")?;
        let olfactory: String = get_string(row, "olfactory_ids_json")?;
        let tactile: String = get_string(row, "tactile_ids_json")?;
        let gustatory: String = get_string(row, "gustatory_ids_json")?;
        let emotion: String = get_string(row, "emotion_ids_json")?;
        let gesture: String = get_string(row, "gesture_ids_json")?;
        let atmosphere: String = get_string(row, "atmosphere_ids_json")?;
        let sel = SensorySelection {
            visual_ids: parse_ids(&visual)?,
            auditory_ids: parse_ids(&auditory)?,
            olfactory_ids: parse_ids(&olfactory)?,
            tactile_ids: parse_ids(&tactile)?,
            gustatory_ids: parse_ids(&gustatory)?,
            emotion_ids: parse_ids(&emotion)?,
            gesture_ids: parse_ids(&gesture)?,
            atmosphere_ids: parse_ids(&atmosphere)?,
        };
        Ok((sel, scene_id))
    }

    /// 在已有事务中插入五感数据（供 DerivationRepo 跨表事务调用）
    pub async fn insert_in_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        character_id: CharacterId,
        scene_id: SceneId,
        sel: &SensorySelection,
        now: DateTime<Utc>,
    ) -> Result<(), StoryError> {
        sqlx::query(
            "INSERT INTO character_sensations \
             (id, character_id, scene_id, visual_ids_json, auditory_ids_json, olfactory_ids_json, \
              tactile_ids_json, gustatory_ids_json, emotion_ids_json, gesture_ids_json, \
              atmosphere_ids_json, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(character_id.0.to_string())
        .bind(scene_id.0.to_string())
        .bind(ids_to_json(&sel.visual_ids))
        .bind(ids_to_json(&sel.auditory_ids))
        .bind(ids_to_json(&sel.olfactory_ids))
        .bind(ids_to_json(&sel.tactile_ids))
        .bind(ids_to_json(&sel.gustatory_ids))
        .bind(ids_to_json(&sel.emotion_ids))
        .bind(ids_to_json(&sel.gesture_ids))
        .bind(ids_to_json(&sel.atmosphere_ids))
        .bind(now.to_rfc3339())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}
