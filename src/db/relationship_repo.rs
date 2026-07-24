use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool, sqlite::SqliteRow};
use uuid::Uuid;

use crate::models::{
    CandidateStatus, CharacterId, GraphEdge, GraphNode, GraphSnapshot, MemoryId,
    RelationshipCandidate, RelationshipCandidateId, RelationshipFactId, RelationshipRevision,
    RelationshipType, RevisionStatus, SceneId, StoryError, validate_score,
};

/// 待插入的候选关系（LLM 产出，等待作者确认）
pub struct PendingCandidate {
    pub scene_id: SceneId,
    pub from: CharacterId,
    pub to: CharacterId,
    pub relationship_type: RelationshipType,
    pub summary: String,
    pub tension_score: Option<u8>,
    pub trust_score: Option<u8>,
    pub affection_score: Option<u8>,
    pub power_score: Option<u8>,
    pub evidence_memory_id: Option<MemoryId>,
    pub confidence: f32,
}

/// 关系评分（接受时可覆盖候选原始评分）
#[derive(Debug, Clone, Default)]
pub struct RelationshipScores {
    pub tension: Option<u8>,
    pub trust: Option<u8>,
    pub affection: Option<u8>,
    pub power: Option<u8>,
}

/// 候选解析动作
pub enum CandidateResolution {
    Accept {
        summary: String,
        scores: Option<RelationshipScores>,
    },
    Reject,
}

impl CandidateResolution {
    pub fn accept(summary: impl Into<String>, scores: Option<RelationshipScores>) -> Self {
        Self::Accept {
            summary: summary.into(),
            scores,
        }
    }

    pub fn reject() -> Self {
        Self::Reject
    }
}

/// 关系仓储
/// =========
/// 负责关系事实、修订历史、候选的持久化与查询。
/// 所有解析操作在单个事务中完成，保证原子性。
#[derive(Clone)]
pub struct RelationshipRepo {
    pool: SqlitePool,
}

impl RelationshipRepo {
    pub(crate) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 插入待处理候选
    pub async fn insert_pending(
        &self,
        c: PendingCandidate,
    ) -> Result<RelationshipCandidate, StoryError> {
        validate_score(c.tension_score)?;
        validate_score(c.trust_score)?;
        validate_score(c.affection_score)?;
        validate_score(c.power_score)?;

        let id = RelationshipCandidateId(Uuid::new_v4());
        let now = Utc::now();
        let rt_json = serde_json::to_string(&c.relationship_type)
            .map_err(|e| StoryError::Database(e.to_string()))?;

        sqlx::query(
            "INSERT INTO relationship_candidates
             (id, scene_id, from_character_id, to_character_id, relationship_type,
              summary, tension_score, trust_score, affection_score, power_score,
              evidence_memory_id, confidence, status, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.0.to_string())
        .bind(c.scene_id.0.to_string())
        .bind(c.from.0.to_string())
        .bind(c.to.0.to_string())
        .bind(&rt_json)
        .bind(&c.summary)
        .bind(c.tension_score.map(|v| v as i64))
        .bind(c.trust_score.map(|v| v as i64))
        .bind(c.affection_score.map(|v| v as i64))
        .bind(c.power_score.map(|v| v as i64))
        .bind(c.evidence_memory_id.map(|m| m.0.to_string()))
        .bind(c.confidence as f64)
        .bind("pending")
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(RelationshipCandidate {
            id,
            scene_id: c.scene_id,
            from_character_id: c.from,
            to_character_id: c.to,
            relationship_type: c.relationship_type,
            summary: c.summary,
            tension_score: c.tension_score,
            trust_score: c.trust_score,
            affection_score: c.affection_score,
            power_score: c.power_score,
            evidence_memory_id: c.evidence_memory_id,
            confidence: c.confidence,
            status: CandidateStatus::Pending,
            created_at: now,
            resolved_at: None,
        })
    }

    /// 解析候选（接受或拒绝），单个事务保证原子性
    pub async fn resolve_candidate(
        &self,
        candidate_id: RelationshipCandidateId,
        resolution: CandidateResolution,
    ) -> Result<(), StoryError> {
        let mut tx = self.pool.begin().await?;

        // 获取候选
        let row = sqlx::query(
            "SELECT scene_id, from_character_id, to_character_id, relationship_type,
                    summary, tension_score, trust_score, affection_score, power_score,
                    evidence_memory_id, confidence, status
             FROM relationship_candidates WHERE id = ?",
        )
        .bind(candidate_id.0.to_string())
        .fetch_optional(&mut *tx)
        .await?;

        let row = row.ok_or(StoryError::RelationshipCandidateNotFound(candidate_id))?;
        let status: String = Row::try_get(&row, "status")?;
        if status != "pending" {
            return Err(StoryError::RelationshipCandidateResolved(candidate_id));
        }

        let scene_id: String = Row::try_get(&row, "scene_id")?;
        let from_character_id: String = Row::try_get(&row, "from_character_id")?;
        let to_character_id: String = Row::try_get(&row, "to_character_id")?;
        let relationship_type: String = Row::try_get(&row, "relationship_type")?;
        let evidence_memory_id: Option<String> = Row::try_get(&row, "evidence_memory_id")?;
        let now = Utc::now();

        match resolution {
            CandidateResolution::Reject => {
                sqlx::query(
                    "UPDATE relationship_candidates SET status = ?, resolved_at = ? WHERE id = ?",
                )
                .bind("rejected")
                .bind(now.to_rfc3339())
                .bind(candidate_id.0.to_string())
                .execute(&mut *tx)
                .await?;
            }
            CandidateResolution::Accept { summary, scores } => {
                let (tension, trust, affection, power) = match &scores {
                    Some(s) => (s.tension, s.trust, s.affection, s.power),
                    None => {
                        let t: Option<i64> = Row::try_get(&row, "tension_score")?;
                        let tr: Option<i64> = Row::try_get(&row, "trust_score")?;
                        let a: Option<i64> = Row::try_get(&row, "affection_score")?;
                        let p: Option<i64> = Row::try_get(&row, "power_score")?;
                        (
                            t.map(|v| v as u8),
                            tr.map(|v| v as u8),
                            a.map(|v| v as u8),
                            p.map(|v| v as u8),
                        )
                    }
                };
                validate_score(tension)?;
                validate_score(trust)?;
                validate_score(affection)?;
                validate_score(power)?;

                let new_fact_id = RelationshipFactId(Uuid::new_v4());

                // 查找或创建事实（ON CONFLICT DO NOTHING，再 SELECT 获取 id）
                sqlx::query(
                    "INSERT INTO relationship_facts
                     (id, from_character_id, to_character_id, relationship_type, created_at, created_by)
                     VALUES (?, ?, ?, ?, ?, 'author')
                     ON CONFLICT(from_character_id, to_character_id, relationship_type) DO NOTHING",
                )
                .bind(new_fact_id.0.to_string())
                .bind(&from_character_id)
                .bind(&to_character_id)
                .bind(&relationship_type)
                .bind(now.to_rfc3339())
                .execute(&mut *tx)
                .await?;

                let fact_row = sqlx::query(
                    "SELECT id FROM relationship_facts
                     WHERE from_character_id = ? AND to_character_id = ? AND relationship_type = ?",
                )
                .bind(&from_character_id)
                .bind(&to_character_id)
                .bind(&relationship_type)
                .fetch_one(&mut *tx)
                .await?;
                let fact_id_str: String = Row::try_get(&fact_row, "id")?;

                // 更新现有活跃修订为 superseded（valid_until_scene_id 设为当前候选场景）
                sqlx::query(
                    "UPDATE relationship_revisions
                     SET status = 'superseded', valid_until_scene_id = ?
                     WHERE relationship_fact_id = ? AND status = 'active'",
                )
                .bind(&scene_id)
                .bind(&fact_id_str)
                .execute(&mut *tx)
                .await?;

                // 插入新活跃修订
                let rev_id = Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT INTO relationship_revisions
                     (id, relationship_fact_id, scene_id, valid_from_scene_id, status, summary,
                      tension_score, trust_score, affection_score, power_score,
                      evidence_memory_id, created_at)
                     VALUES (?, ?, ?, ?, 'active', ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&rev_id)
                .bind(&fact_id_str)
                .bind(&scene_id)
                .bind(&scene_id)
                .bind(&summary)
                .bind(tension.map(|v| v as i64))
                .bind(trust.map(|v| v as i64))
                .bind(affection.map(|v| v as i64))
                .bind(power.map(|v| v as i64))
                .bind(&evidence_memory_id)
                .bind(now.to_rfc3339())
                .execute(&mut *tx)
                .await?;

                // 更新候选为 accepted
                sqlx::query(
                    "UPDATE relationship_candidates SET status = ?, resolved_at = ? WHERE id = ?",
                )
                .bind("accepted")
                .bind(now.to_rfc3339())
                .bind(candidate_id.0.to_string())
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }

    /// 列出场景的所有候选（包括 pending/accepted/rejected）
    pub async fn list_candidates_for_scene(
        &self,
        scene_id: SceneId,
    ) -> Result<Vec<RelationshipCandidate>, StoryError> {
        let rows = sqlx::query(
            "SELECT id, scene_id, from_character_id, to_character_id, relationship_type,
                    summary, tension_score, trust_score, affection_score, power_score,
                    evidence_memory_id, confidence, status, created_at, resolved_at
             FROM relationship_candidates WHERE scene_id = ?
             ORDER BY created_at ASC",
        )
        .bind(scene_id.0.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(parse_candidate_row).collect()
    }

    /// 获取通过场景的图快照（时序：只包含在目标场景时间点有效的修订）
    pub async fn graph_snapshot_through(
        &self,
        scene_id: SceneId,
    ) -> Result<GraphSnapshot, StoryError> {
        // 获取目标场景的 occurred_at
        let scene_row = sqlx::query("SELECT occurred_at FROM scenes WHERE id = ?")
            .bind(scene_id.0.to_string())
            .fetch_optional(&self.pool)
            .await?;
        let scene_row = scene_row.ok_or(StoryError::SceneNotFound(scene_id))?;
        let target_time: String = Row::try_get(&scene_row, "occurred_at")?;

        // 查询有效修订
        // valid_from_scene.occurred_at <= target AND
        // (valid_until_scene_id IS NULL OR valid_until_scene.occurred_at > target)
        let edge_rows = sqlx::query(
            "SELECT rr.id, rr.relationship_fact_id, rr.scene_id, rr.valid_from_scene_id,
                    rr.valid_until_scene_id, rr.status, rr.summary,
                    rr.tension_score, rr.trust_score, rr.affection_score, rr.power_score,
                    rr.evidence_memory_id, rr.created_at,
                    rf.from_character_id, rf.to_character_id, rf.relationship_type
             FROM relationship_revisions rr
             JOIN relationship_facts rf ON rf.id = rr.relationship_fact_id
             JOIN scenes vf ON vf.id = rr.valid_from_scene_id
             LEFT JOIN scenes vu ON vu.id = rr.valid_until_scene_id
             WHERE vf.occurred_at <= ?
               AND (rr.valid_until_scene_id IS NULL OR vu.occurred_at > ?)
             ORDER BY vf.occurred_at DESC, rr.created_at DESC",
        )
        .bind(&target_time)
        .bind(&target_time)
        .fetch_all(&self.pool)
        .await?;

        let mut edges: Vec<GraphEdge> = Vec::new();
        let mut node_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

        for row in &edge_rows {
            let fact_id_str: String = Row::try_get(row, "relationship_fact_id")?;
            let from_str: String = Row::try_get(row, "from_character_id")?;
            let to_str: String = Row::try_get(row, "to_character_id")?;
            let rt_str: String = Row::try_get(row, "relationship_type")?;
            let summary: String = Row::try_get(row, "summary")?;
            let status_str: String = Row::try_get(row, "status")?;
            let tension: Option<i64> = Row::try_get(row, "tension_score")?;
            let trust: Option<i64> = Row::try_get(row, "trust_score")?;
            let affection: Option<i64> = Row::try_get(row, "affection_score")?;
            let power: Option<i64> = Row::try_get(row, "power_score")?;

            let fact_id = parse_uuid(&fact_id_str)?;
            let from = CharacterId(parse_uuid(&from_str)?);
            let to = CharacterId(parse_uuid(&to_str)?);
            let rt: RelationshipType =
                serde_json::from_str(&rt_str).map_err(|e| StoryError::Database(e.to_string()))?;

            node_ids.insert(from_str);
            node_ids.insert(to_str);

            edges.push(GraphEdge {
                fact_id: RelationshipFactId(fact_id),
                from,
                to,
                relationship_type: rt,
                summary,
                active: status_str == "active",
                tension_score: tension.map(|v| v as u8),
                trust_score: trust.map(|v| v as u8),
                affection_score: affection.map(|v| v as u8),
                power_score: power.map(|v| v as u8),
            });
        }

        // 投影角色名称作为节点
        let mut nodes: Vec<GraphNode> = Vec::new();
        for id_str in &node_ids {
            let row = sqlx::query("SELECT name FROM characters WHERE id = ?")
                .bind(id_str)
                .fetch_one(&self.pool)
                .await?;
            let name: String = Row::try_get(&row, "name")?;
            nodes.push(GraphNode {
                id: CharacterId(parse_uuid(id_str)?),
                name,
            });
        }
        nodes.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(GraphSnapshot { nodes, edges })
    }

    /// 获取关系事实的修订历史
    pub async fn history(
        &self,
        fact_id: RelationshipFactId,
    ) -> Result<Vec<RelationshipRevision>, StoryError> {
        let rows = sqlx::query(
            "SELECT id, relationship_fact_id, scene_id, valid_from_scene_id, valid_until_scene_id,
                    status, summary, tension_score, trust_score, affection_score, power_score,
                    evidence_memory_id, created_at
             FROM relationship_revisions
             WHERE relationship_fact_id = ?
             ORDER BY created_at DESC",
        )
        .bind(fact_id.0.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(parse_revision_row).collect()
    }
}

fn parse_candidate_row(row: &SqliteRow) -> Result<RelationshipCandidate, StoryError> {
    let id_str: String = Row::try_get(row, "id")?;
    let scene_id_str: String = Row::try_get(row, "scene_id")?;
    let from_str: String = Row::try_get(row, "from_character_id")?;
    let to_str: String = Row::try_get(row, "to_character_id")?;
    let rt_str: String = Row::try_get(row, "relationship_type")?;
    let summary: String = Row::try_get(row, "summary")?;
    let tension: Option<i64> = Row::try_get(row, "tension_score")?;
    let trust: Option<i64> = Row::try_get(row, "trust_score")?;
    let affection: Option<i64> = Row::try_get(row, "affection_score")?;
    let power: Option<i64> = Row::try_get(row, "power_score")?;
    let evidence_str: Option<String> = Row::try_get(row, "evidence_memory_id")?;
    let confidence: f64 = Row::try_get(row, "confidence")?;
    let status_str: String = Row::try_get(row, "status")?;
    let created_str: String = Row::try_get(row, "created_at")?;
    let resolved_str: Option<String> = Row::try_get(row, "resolved_at")?;

    let rt: RelationshipType =
        serde_json::from_str(&rt_str).map_err(|e| StoryError::Database(e.to_string()))?;
    let status = match status_str.as_str() {
        "pending" => CandidateStatus::Pending,
        "accepted" => CandidateStatus::Accepted,
        "rejected" => CandidateStatus::Rejected,
        other => {
            return Err(StoryError::Database(format!(
                "unknown candidate status: {other}"
            )));
        }
    };
    let created_at = parse_rfc3339(&created_str)?;
    let resolved_at = match resolved_str {
        Some(s) => Some(parse_rfc3339(&s)?),
        None => None,
    };

    Ok(RelationshipCandidate {
        id: RelationshipCandidateId(parse_uuid(&id_str)?),
        scene_id: SceneId(parse_uuid(&scene_id_str)?),
        from_character_id: CharacterId(parse_uuid(&from_str)?),
        to_character_id: CharacterId(parse_uuid(&to_str)?),
        relationship_type: rt,
        summary,
        tension_score: tension.map(|v| v as u8),
        trust_score: trust.map(|v| v as u8),
        affection_score: affection.map(|v| v as u8),
        power_score: power.map(|v| v as u8),
        evidence_memory_id: evidence_str
            .map(|s| parse_uuid(&s).map(MemoryId))
            .transpose()?,
        confidence: confidence as f32,
        status,
        created_at,
        resolved_at,
    })
}

fn parse_revision_row(row: &SqliteRow) -> Result<RelationshipRevision, StoryError> {
    let id_str: String = Row::try_get(row, "id")?;
    let fact_id_str: String = Row::try_get(row, "relationship_fact_id")?;
    let scene_id_str: String = Row::try_get(row, "scene_id")?;
    let valid_from_str: String = Row::try_get(row, "valid_from_scene_id")?;
    let valid_until_str: Option<String> = Row::try_get(row, "valid_until_scene_id")?;
    let status_str: String = Row::try_get(row, "status")?;
    let summary: String = Row::try_get(row, "summary")?;
    let tension: Option<i64> = Row::try_get(row, "tension_score")?;
    let trust: Option<i64> = Row::try_get(row, "trust_score")?;
    let affection: Option<i64> = Row::try_get(row, "affection_score")?;
    let power: Option<i64> = Row::try_get(row, "power_score")?;
    let evidence_str: Option<String> = Row::try_get(row, "evidence_memory_id")?;
    let created_str: String = Row::try_get(row, "created_at")?;

    let status = match status_str.as_str() {
        "active" => RevisionStatus::Active,
        "superseded" => RevisionStatus::Superseded,
        other => {
            return Err(StoryError::Database(format!(
                "unknown revision status: {other}"
            )));
        }
    };

    Ok(RelationshipRevision {
        id: RelationshipFactId(parse_uuid(&id_str)?),
        relationship_fact_id: RelationshipFactId(parse_uuid(&fact_id_str)?),
        scene_id: SceneId(parse_uuid(&scene_id_str)?),
        valid_from_scene_id: SceneId(parse_uuid(&valid_from_str)?),
        valid_until_scene_id: match &valid_until_str {
            Some(s) => Some(SceneId(parse_uuid(s)?)),
            None => None,
        },
        status,
        summary,
        tension_score: tension.map(|v| v as u8),
        trust_score: trust.map(|v| v as u8),
        affection_score: affection.map(|v| v as u8),
        power_score: power.map(|v| v as u8),
        evidence_memory_id: evidence_str
            .map(|s| parse_uuid(&s).map(MemoryId))
            .transpose()?,
        created_at: parse_rfc3339(&created_str)?,
    })
}

fn parse_uuid(s: &str) -> Result<Uuid, StoryError> {
    Uuid::parse_str(s).map_err(|e| StoryError::Database(format!("invalid UUID {s}: {e}")))
}

fn parse_rfc3339(s: &str) -> Result<DateTime<Utc>, StoryError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| StoryError::Database(format!("invalid timestamp {s}: {e}")))
}
