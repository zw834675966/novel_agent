//! SQLite row 列解析工具（消除 repo 间重复的 UUID / String / JSON / RFC3339 解析样板）。

use crate::models::StoryError;
use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::sqlite::SqliteRow;
use uuid::Uuid;

/// 从行读取字符串列。
pub fn get_string(row: &SqliteRow, col: &str) -> Result<String, StoryError> {
    row.try_get(col)
        .map_err(|e| StoryError::Database(format!("{col}: {e}")))
}

/// 从行读取 UUID 列。
pub fn get_uuid(row: &SqliteRow, col: &str) -> Result<Uuid, StoryError> {
    let s = get_string(row, col)?;
    Uuid::parse_str(&s).map_err(|e| StoryError::Database(format!("{col}: {e}")))
}

/// 从行读取 JSON 列并反序列化为 T。
pub fn get_json<T: serde::de::DeserializeOwned>(
    row: &SqliteRow,
    col: &str,
) -> Result<T, StoryError> {
    let s = get_string(row, col)?;
    serde_json::from_str(&s).map_err(|e| StoryError::Database(format!("{col}: {e}")))
}

/// 从行读取 RFC3339 时间戳列。
pub fn get_rfc3339(row: &SqliteRow, col: &str) -> Result<DateTime<Utc>, StoryError> {
    let s = get_string(row, col)?;
    DateTime::parse_from_rfc3339(&s)
        .map_err(|e| StoryError::Database(format!("{col}: {e}")))
        .map(|dt| dt.with_timezone(&Utc))
}
