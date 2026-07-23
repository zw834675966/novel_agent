// CLI Harness 会话上下文 + 实体解析助手
// =====================================
// REPL 需要跟踪"当前场景/角色"上下文，并把用户输入的名称/UUID 统一解析成
// 类型安全的 `CharacterId` / `SceneId`。解析规则（spec）：
//   1. 若输入可被 `Uuid::parse_str` 解析 -> 当作 ID 精确查询；命中即 Ok，否则未找到。
//   2. 否则列出全部实体，按 `name`/`objective_event` 子串匹配（大小写敏感，中文 OK）。
//   3. 1 条匹配 -> Ok；0 条 -> 「未找到」错误；≥2 条 -> 列出候选的歧义错误，绝不静默取首条。

use uuid::Uuid;

use crate::db::Db;
use crate::models::{CharacterId, SceneId, StoryError};

/// REPL 会话上下文：跟踪当前选定的场景与角色。
#[derive(Debug, Clone, Default)]
pub struct Session {
    pub current_scene: Option<SceneId>,
    pub current_character: Option<CharacterId>,
}

/// CLI 错误枚举：可携带中文摘要 + 下一步建议，或透传领域错误。
#[derive(Debug)]
pub enum CliError {
    /// 自定义消息：摘要 + 可选的下一步建议。
    Message { summary: String, next: Vec<String> },
    /// 领域错误透传（DB/校验等）。
    Story(StoryError),
}

impl CliError {
    /// 返回中文错误摘要。
    pub fn summary(&self) -> String {
        match self {
            CliError::Message { summary, .. } => summary.clone(),
            // StoryError 实现了 thiserror::Error 的 Display（英文），直接透传。
            CliError::Story(e) => e.to_string(),
        }
    }
}

impl From<StoryError> for CliError {
    fn from(e: StoryError) -> Self {
        CliError::Story(e)
    }
}

/// 解析角色：UUID 精确查询 / 名称子串唯一匹配。
///
/// - 命中 UUID 且存在 -> `Ok(CharacterId)`
/// - 名称子串唯一匹配 -> `Ok(CharacterId)`
/// - 0 匹配 -> `CliError`（「未找到」）
/// - ≥2 匹配 -> `CliError`（列出候选 name + id，绝不静默取首条）
pub async fn resolve_character(db: &Db, id_or_name: &str) -> Result<CharacterId, CliError> {
    // 1. 若可解析为 UUID -> 当作 ID 精确查询
    if let Ok(uuid) = Uuid::parse_str(id_or_name) {
        let id = CharacterId(uuid);
        if db.characters().get(id).await?.is_some() {
            return Ok(id);
        }
        return Err(not_found("角色", id_or_name));
    }
    // 2. 否则按名称子串过滤
    let matches: Vec<_> = db
        .characters()
        .list()
        .await?
        .into_iter()
        .filter(|c| c.name.contains(id_or_name))
        .collect();
    resolve_unique(matches, "角色", id_or_name, |c| (c.name.clone(), c.id.0)).map(|c| c.id)
}

/// 解析场景：UUID 精确查询 / 事件子串唯一匹配。
///
/// 与 [`resolve_character`] 同构，但匹配字段是 `objective_event`。
pub async fn resolve_scene(db: &Db, id_or_name: &str) -> Result<SceneId, CliError> {
    // 1. 若可解析为 UUID -> 当作 ID 精确查询
    if let Ok(uuid) = Uuid::parse_str(id_or_name) {
        let id = SceneId(uuid);
        if db.scenes().get(id).await?.is_some() {
            return Ok(id);
        }
        return Err(not_found("场景", id_or_name));
    }
    // 2. 否则按 objective_event 子串过滤
    let matches: Vec<_> = db
        .scenes()
        .list()
        .await?
        .into_iter()
        .filter(|s| s.objective_event.contains(id_or_name))
        .collect();
    resolve_unique(matches, "场景", id_or_name, |s| {
        (s.objective_event.clone(), s.id.0)
    })
    .map(|s| s.id)
}

/// 构造「未找到」错误。
fn not_found(label: &str, query: &str) -> CliError {
    CliError::Message {
        summary: format!("未找到{label}：{query}"),
        next: vec![],
    }
}

/// 从子串匹配结果中选出唯一实体。
///
/// - 1 条 -> `Ok`
/// - 0 条 -> 「未找到」错误
/// - ≥2 条 -> 列出候选（描述 + id）的歧义错误，绝不静默取首条
fn resolve_unique<T, F>(
    matches: Vec<T>,
    label: &str,
    query: &str,
    descriptor: F,
) -> Result<T, CliError>
where
    F: Fn(&T) -> (String, Uuid),
{
    match matches.len() {
        0 => Err(not_found(label, query)),
        1 => Ok(matches.into_iter().next().expect("len == 1")),
        _ => {
            let candidates: Vec<String> = matches
                .iter()
                .map(|m| {
                    let (desc, id) = descriptor(m);
                    format!("{desc} ({id})")
                })
                .collect();
            Err(CliError::Message {
                summary: format!(
                    "多个{label}匹配「{query}」，候选：{}",
                    candidates.join("、")
                ),
                next: vec![],
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // ---- 角色解析 ----

    #[tokio::test]
    async fn resolve_character_unique_name() {
        let db = Db::open_in_memory().await.unwrap();
        let id = CharacterId(Uuid::new_v4());
        db.characters().create(id, "宝玉", &[], &[]).await.unwrap();
        let got = resolve_character(&db, "宝玉").await.unwrap();
        assert_eq!(got, id);
    }

    #[tokio::test]
    async fn resolve_character_ambiguous_errors() {
        let db = Db::open_in_memory().await.unwrap();
        let a = CharacterId(Uuid::new_v4());
        let b = CharacterId(Uuid::new_v4());
        db.characters().create(a, "宝玉甲", &[], &[]).await.unwrap();
        db.characters().create(b, "宝玉乙", &[], &[]).await.unwrap();
        let err = resolve_character(&db, "宝玉").await.unwrap_err();
        let msg = err.summary();
        assert!(
            msg.contains("多个") || msg.contains("歧义") || msg.contains("候选"),
            "ambiguous msg should flag ambiguity, got: {msg}"
        );
        // 歧义信息必须列出至少两个候选（name + id）
        let a_id = a.0.to_string();
        let b_id = b.0.to_string();
        assert!(msg.contains(&a_id), "missing candidate a id: {msg}");
        assert!(msg.contains(&b_id), "missing candidate b id: {msg}");
    }

    #[tokio::test]
    async fn resolve_character_uuid_exact() {
        let db = Db::open_in_memory().await.unwrap();
        let id = CharacterId(Uuid::new_v4());
        db.characters().create(id, "黛玉", &[], &[]).await.unwrap();
        // 用 UUID 字符串解析应精确命中，且不靠名称子串
        let got = resolve_character(&db, &id.0.to_string()).await.unwrap();
        assert_eq!(got, id);
    }

    #[tokio::test]
    async fn resolve_character_uuid_not_found() {
        let db = Db::open_in_memory().await.unwrap();
        let stranger = Uuid::new_v4();
        let err = resolve_character(&db, &stranger.to_string())
            .await
            .unwrap_err();
        let msg = err.summary();
        assert!(msg.contains("未找到"), "uuid not-found msg: {msg}");
    }

    #[tokio::test]
    async fn resolve_character_name_not_found() {
        let db = Db::open_in_memory().await.unwrap();
        let err = resolve_character(&db, "不存在的人").await.unwrap_err();
        let msg = err.summary();
        assert!(msg.contains("未找到"), "name not-found msg: {msg}");
    }

    #[tokio::test]
    async fn resolve_character_substring_match() {
        // 子串匹配，而非全等
        let db = Db::open_in_memory().await.unwrap();
        let id = CharacterId(Uuid::new_v4());
        db.characters()
            .create(id, "林黛玉", &[], &[])
            .await
            .unwrap();
        let got = resolve_character(&db, "黛玉").await.unwrap();
        assert_eq!(got, id);
    }

    // ---- 场景解析 ----

    #[tokio::test]
    async fn resolve_scene_unique_event() {
        let db = Db::open_in_memory().await.unwrap();
        let id = SceneId(Uuid::new_v4());
        db.scenes()
            .create(id, "宝玉挨打", &[], Utc::now())
            .await
            .unwrap();
        let got = resolve_scene(&db, "宝玉挨打").await.unwrap();
        assert_eq!(got, id);
    }

    #[tokio::test]
    async fn resolve_scene_substring_match() {
        let db = Db::open_in_memory().await.unwrap();
        let id = SceneId(Uuid::new_v4());
        db.scenes()
            .create(id, "林黛玉进贾府", &[], Utc::now())
            .await
            .unwrap();
        let got = resolve_scene(&db, "进贾府").await.unwrap();
        assert_eq!(got, id);
    }

    #[tokio::test]
    async fn resolve_scene_ambiguous_errors() {
        let db = Db::open_in_memory().await.unwrap();
        let a = SceneId(Uuid::new_v4());
        let b = SceneId(Uuid::new_v4());
        db.scenes()
            .create(a, "宝玉挨打第一次", &[], Utc::now())
            .await
            .unwrap();
        db.scenes()
            .create(b, "宝玉挨打第二次", &[], Utc::now())
            .await
            .unwrap();
        let err = resolve_scene(&db, "宝玉挨打").await.unwrap_err();
        let msg = err.summary();
        assert!(
            msg.contains("多个") || msg.contains("歧义") || msg.contains("候选"),
            "ambiguous scene msg: {msg}"
        );
        let a_id = a.0.to_string();
        let b_id = b.0.to_string();
        assert!(msg.contains(&a_id));
        assert!(msg.contains(&b_id));
    }

    #[tokio::test]
    async fn resolve_scene_uuid_exact() {
        let db = Db::open_in_memory().await.unwrap();
        let id = SceneId(Uuid::new_v4());
        db.scenes()
            .create(id, "元妃省亲", &[], Utc::now())
            .await
            .unwrap();
        let got = resolve_scene(&db, &id.0.to_string()).await.unwrap();
        assert_eq!(got, id);
    }

    #[tokio::test]
    async fn resolve_scene_not_found() {
        let db = Db::open_in_memory().await.unwrap();
        let err = resolve_scene(&db, "不存在的场景").await.unwrap_err();
        let msg = err.summary();
        assert!(msg.contains("未找到"), "scene not-found msg: {msg}");
    }

    // ---- Session ----

    #[test]
    fn session_default_is_empty() {
        let s = Session::default();
        assert!(s.current_scene.is_none());
        assert!(s.current_character.is_none());
    }

    #[test]
    fn session_clone_debug_roundtrip() {
        let cid = CharacterId(Uuid::new_v4());
        let sid = SceneId(Uuid::new_v4());
        let s = Session {
            current_scene: Some(sid),
            current_character: Some(cid),
        };
        let cloned = s.clone();
        assert_eq!(cloned.current_scene, Some(sid));
        assert_eq!(cloned.current_character, Some(cid));
        let dbg = format!("{s:?}");
        assert!(dbg.contains("Session"));
    }

    #[test]
    fn cli_error_story_summary_uses_domain_message() {
        let err = CliError::Story(StoryError::CharacterNotFound(CharacterId(Uuid::nil())));
        let msg = err.summary();
        assert!(!msg.is_empty());
    }
}
