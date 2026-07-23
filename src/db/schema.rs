/// SQLite 数据库 DDL
/// ====================
/// 共 5 张表 + 2 个索引：
///
/// characters              — 角色主表
/// character_personality_tags — 角色性格标签（子表，一对多）
/// character_skills        — 角色技能（子表，一对多）
/// scenes                  — 场景主表
/// scene_participants      — 场景参与者关联（多对多）
/// character_memories      — 角色记忆
/// character_sensations    — 角色五感
///
/// 设计要点：
///   - 所有外键 ON DELETE CASCADE（删除角色/场景时自动清理关联数据）
///   - 时间和 ID 均以 TEXT 存储（ISO 8601 / UUID 字符串）
///   - 五感数据以 JSON TEXT 存储（vocabulary_id 字符串列表）
///   - 记忆按 (character_id, created_at DESC) 建索引，支持快速取最近 N 条
///   - 五感按 (character_id, created_at DESC) 建索引，支持取最新一条
pub const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS characters (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS character_personality_tags (
    character_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    PRIMARY KEY (character_id, tag),
    FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS character_skills (
    character_id TEXT NOT NULL,
    skill TEXT NOT NULL,
    PRIMARY KEY (character_id, skill),
    FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS scenes (
    id TEXT PRIMARY KEY,
    objective_event TEXT NOT NULL,
    occurred_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS scene_participants (
    scene_id TEXT NOT NULL,
    character_id TEXT NOT NULL,
    PRIMARY KEY (scene_id, character_id),
    FOREIGN KEY (scene_id) REFERENCES scenes(id) ON DELETE CASCADE,
    FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS character_states (
    character_id TEXT PRIMARY KEY,
    state_json TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS character_memories (
    id TEXT PRIMARY KEY,
    character_id TEXT NOT NULL,
    scene_id TEXT NOT NULL,
    content TEXT NOT NULL,
    source TEXT NOT NULL,
    certainty TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE,
    FOREIGN KEY (scene_id) REFERENCES scenes(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_memories_character ON character_memories(character_id, created_at DESC);
CREATE TABLE IF NOT EXISTS character_sensations (
    id TEXT PRIMARY KEY,
    character_id TEXT NOT NULL,
    scene_id TEXT NOT NULL,
    visual_ids_json TEXT NOT NULL,
    auditory_ids_json TEXT NOT NULL,
    olfactory_ids_json TEXT NOT NULL,
    tactile_ids_json TEXT NOT NULL,
    gustatory_ids_json TEXT NOT NULL,
    emotion_ids_json TEXT NOT NULL DEFAULT '[]',
    gesture_ids_json TEXT NOT NULL DEFAULT '[]',
    atmosphere_ids_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE,
    FOREIGN KEY (scene_id) REFERENCES scenes(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_sensations_character ON character_sensations(character_id, created_at DESC);
CREATE TABLE IF NOT EXISTS character_plot_developments (
    id TEXT PRIMARY KEY,
    character_id TEXT NOT NULL,
    scene_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE,
    FOREIGN KEY (scene_id) REFERENCES scenes(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_plot_developments_character_scene
    ON character_plot_developments(character_id, scene_id);
CREATE TABLE IF NOT EXISTS character_derivation_context_tags (
    character_id TEXT NOT NULL,
    scene_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    PRIMARY KEY (character_id, scene_id, tag),
    FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE,
    FOREIGN KEY (scene_id) REFERENCES scenes(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS relationship_facts (
    id TEXT PRIMARY KEY,
    from_character_id TEXT NOT NULL,
    to_character_id TEXT NOT NULL,
    relationship_type TEXT NOT NULL,
    created_at TEXT NOT NULL,
    created_by TEXT NOT NULL CHECK (created_by = 'author'),
    UNIQUE(from_character_id, to_character_id, relationship_type),
    FOREIGN KEY (from_character_id) REFERENCES characters(id) ON DELETE CASCADE,
    FOREIGN KEY (to_character_id) REFERENCES characters(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS relationship_revisions (
    id TEXT PRIMARY KEY,
    relationship_fact_id TEXT NOT NULL,
    scene_id TEXT NOT NULL,
    valid_from_scene_id TEXT NOT NULL,
    valid_until_scene_id TEXT,
    status TEXT NOT NULL CHECK (status IN ('active', 'superseded')),
    summary TEXT NOT NULL,
    tension_score INTEGER,
    trust_score INTEGER,
    affection_score INTEGER,
    power_score INTEGER,
    evidence_memory_id TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (relationship_fact_id) REFERENCES relationship_facts(id) ON DELETE CASCADE,
    FOREIGN KEY (scene_id) REFERENCES scenes(id) ON DELETE CASCADE,
    FOREIGN KEY (evidence_memory_id) REFERENCES character_memories(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_relationship_revisions_fact ON relationship_revisions(relationship_fact_id, created_at DESC);
CREATE TABLE IF NOT EXISTS relationship_candidates (
    id TEXT PRIMARY KEY,
    scene_id TEXT NOT NULL,
    from_character_id TEXT NOT NULL,
    to_character_id TEXT NOT NULL,
    relationship_type TEXT NOT NULL,
    summary TEXT NOT NULL,
    tension_score INTEGER,
    trust_score INTEGER,
    affection_score INTEGER,
    power_score INTEGER,
    evidence_memory_id TEXT,
    confidence REAL NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'accepted', 'rejected')),
    created_at TEXT NOT NULL,
    resolved_at TEXT,
    FOREIGN KEY (scene_id) REFERENCES scenes(id) ON DELETE CASCADE,
    FOREIGN KEY (from_character_id) REFERENCES characters(id) ON DELETE CASCADE,
    FOREIGN KEY (to_character_id) REFERENCES characters(id) ON DELETE CASCADE,
    FOREIGN KEY (evidence_memory_id) REFERENCES character_memories(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_relationship_candidates_scene ON relationship_candidates(scene_id, status);
"#;

/// 执行数据库迁移
/// ================
/// 当前简化实现：直接执行 DDL 建表语句。
/// 未来可替换为 sqlx::migrate! 的 SQL 文件迁移。
pub async fn migrate(pool: &sqlx::sqlite::SqlitePool) -> Result<(), crate::models::StoryError> {
    sqlx::query(SCHEMA_SQL).execute(pool).await?;
    ensure_sensation_dimension_columns(pool).await?;
    Ok(())
}

async fn ensure_sensation_dimension_columns(
    pool: &sqlx::sqlite::SqlitePool,
) -> Result<(), crate::models::StoryError> {
    let columns = sqlx::query("PRAGMA table_info(character_sensations)")
        .fetch_all(pool)
        .await?;
    let existing: std::collections::HashSet<String> = columns
        .iter()
        .map(|row| sqlx::Row::try_get(row, "name"))
        .collect::<Result<_, _>>()?;

    for column in [
        "emotion_ids_json",
        "gesture_ids_json",
        "atmosphere_ids_json",
    ] {
        if !existing.contains(column) {
            sqlx::query(&format!(
                "ALTER TABLE character_sensations ADD COLUMN {column} TEXT NOT NULL DEFAULT '[]'"
            ))
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}
