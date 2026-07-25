use super::{CharacterId, SceneId};

/// LLM 错误分类，用于重试/运维决策（按字符串启发式归类，不依赖具体 provider 类型）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LlmErrorKind {
    #[error("auth")]
    Auth,
    #[error("rate_limit")]
    RateLimit,
    #[error("timeout")]
    Timeout,
    #[error("schema")]
    Schema,
    #[error("empty_beats")]
    EmptyBeats,
    #[error("other")]
    Other,
}

/// 全局错误枚举（thiserror）
/// ============================
/// 覆盖项目中的所有错误类型，从数据访问到 LLM 调用。
/// 通过 `From<sqlx::Error>` 和 `From<sqlx::migrate::MigrateError>` 实现自动转换，
/// 使得 `?` 操作符可以直接在 DB 操作后使用。
#[derive(Debug, thiserror::Error)]
pub enum StoryError {
    #[error("scene not found: {0:?}")]
    SceneNotFound(SceneId),

    #[error("character not found: {0:?}")]
    CharacterNotFound(CharacterId),

    #[error("character {0:?} is not a participant of scene {1:?}")]
    NotSceneParticipant(CharacterId, SceneId),

    #[error("vocabulary load error: {0}")]
    VocabularyLoad(String),

    #[error("invalid vocabulary selection: {0}")]
    InvalidVocabularySelection(String),

    #[error("invalid narration context: {0}")]
    InvalidNarrationContext(String),

    #[error("llm error ({kind}): {message}")]
    Llm { kind: LlmErrorKind, message: String },

    #[error("database error: {0}")]
    Database(String),
}

impl StoryError {
    /// 用 `Other` 分类构造一个 LLM 错误。
    pub fn llm(message: impl Into<String>) -> Self {
        StoryError::Llm {
            kind: LlmErrorKind::Other,
            message: message.into(),
        }
    }

    /// 从任意 `Debug` 错误构造 LLM 错误，并按 `format!("{e:?}")` 启发式分类。
    ///
    /// 启发式（大小写不敏感，匹配即返回）：
    /// - `401` / `unauthorized` -> Auth
    /// - `429` / `rate` -> RateLimit
    /// - `timeout` / `timed out` -> Timeout
    /// - `schema` / `json` / `deserialize` -> Schema
    /// - 其余 -> Other
    pub fn llm_from_debug(e: impl std::fmt::Debug) -> Self {
        let message = format!("{e:?}");
        let kind = classify_debug(&message);
        StoryError::Llm { kind, message }
    }
}

/// 按 debug 字符串启发式归类 LLM 错误。
fn classify_debug(message: &str) -> LlmErrorKind {
    let lower = message.to_ascii_lowercase();
    if lower.contains("401") || lower.contains("unauthorized") {
        LlmErrorKind::Auth
    } else if lower.contains("429") || lower.contains("rate") {
        LlmErrorKind::RateLimit
    } else if lower.contains("timeout") || lower.contains("timed out") {
        LlmErrorKind::Timeout
    } else if lower.contains("schema") || lower.contains("json") || lower.contains("deserialize") {
        LlmErrorKind::Schema
    } else {
        LlmErrorKind::Other
    }
}

/// sqlx 错误 -> StoryError 自动转换
impl From<sqlx::Error> for StoryError {
    fn from(e: sqlx::Error) -> Self {
        StoryError::Database(e.to_string())
    }
}

/// 数据库迁移错误 -> StoryError 自动转换
impl From<sqlx::migrate::MigrateError> for StoryError {
    fn from(e: sqlx::migrate::MigrateError) -> Self {
        StoryError::Database(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_helper_defaults_to_other() {
        let e = StoryError::llm("boom");
        assert!(matches!(
            e,
            StoryError::Llm {
                kind: LlmErrorKind::Other,
                message
            } if message == "boom"
        ));
    }

    #[test]
    fn classify_auth_variants() {
        assert_eq!(classify_debug("HTTP 401 Unauthorized"), LlmErrorKind::Auth);
        assert_eq!(classify_debug("unauthorized api key"), LlmErrorKind::Auth);
    }

    #[test]
    fn classify_rate_limit_variants() {
        assert_eq!(
            classify_debug("429 Too Many Requests"),
            LlmErrorKind::RateLimit
        );
        assert_eq!(
            classify_debug("rate limit exceeded"),
            LlmErrorKind::RateLimit
        );
    }

    #[test]
    fn classify_timeout_variants() {
        assert_eq!(classify_debug("request timeout"), LlmErrorKind::Timeout);
        assert_eq!(classify_debug("operation timed out"), LlmErrorKind::Timeout);
    }

    #[test]
    fn classify_schema_variants() {
        assert_eq!(
            classify_debug("json deserialize error"),
            LlmErrorKind::Schema
        );
        assert_eq!(classify_debug("schema mismatch"), LlmErrorKind::Schema);
    }

    #[test]
    fn classify_other_for_unknown() {
        assert_eq!(
            classify_debug("connection reset by peer"),
            LlmErrorKind::Other
        );
    }

    #[test]
    fn llm_from_debug_classifies_and_preserves_message() {
        let e = StoryError::llm_from_debug(401_u16);
        assert!(matches!(
            e,
            StoryError::Llm {
                kind: LlmErrorKind::Auth,
                ..
            }
        ));
    }

    #[test]
    fn display_includes_kind_and_message() {
        let e = StoryError::llm("boom");
        let s = format!("{e}");
        assert!(s.contains("other"), "display: {s}");
        assert!(s.contains("boom"), "display: {s}");
    }
}
