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
    created_at TEXT NOT NULL,
    FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE,
    FOREIGN KEY (scene_id) REFERENCES scenes(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_sensations_character ON character_sensations(character_id, created_at DESC);
"#;

/// 执行数据库迁移
/// ================
/// 当前简化实现：直接执行 DDL 建表语句。
/// 未来可替换为 sqlx::migrate! 的 SQL 文件迁移。
pub async fn migrate(pool: &sqlx::sqlite::SqlitePool) -> Result<(), crate::models::StoryError> {
    sqlx::query(SCHEMA_SQL).execute(pool).await?;
    Ok(())
}
