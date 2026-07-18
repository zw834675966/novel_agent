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

pub async fn migrate(pool: &sqlx::sqlite::SqlitePool) -> Result<(), crate::models::StoryError> {
    sqlx::query(SCHEMA_SQL).execute(pool).await?;
    Ok(())
}
