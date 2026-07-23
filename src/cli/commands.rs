// CLI 命令执行器
// ==============
// 把 `Commands` 枚举分发到具体的业务逻辑，返回结构化 `Observation`。
//
// Task 5 实现所有非 LLM 命令（character/scene/use/context）；
// LLM 命令（derive/narrate/show derivation/show prose）留桩，Task 6 接入。

use std::collections::BTreeMap;

use chrono::Utc;
use uuid::Uuid;

use crate::cli::args::{CharacterCmd, Commands, SceneCmd, ShowCmd, UseCmd};
use crate::cli::observation::{Observation, Status};
use crate::cli::prose_cache::ProseCache;
use crate::cli::session::{Session, resolve_character, resolve_scene};
use crate::models::{CharacterId, CreateScene};
use crate::scene::StoryService;

/// 命令执行上下文：把 service + 会话状态 + 缓存 + 模式标志打包给 execute。
pub struct CommandContext<'a> {
    pub service: &'a StoryService,
    pub session: &'a mut Session,
    pub prose_cache: &'a mut ProseCache,
    pub using_mock: bool,
    pub json: bool,
    pub one_shot: bool,
}

/// 分发命令并返回观测结果。
pub async fn execute(cmd: Commands, ctx: &mut CommandContext<'_>) -> Observation {
    match cmd {
        Commands::Character(c) => execute_character(c, ctx).await,
        Commands::Scene(s) => execute_scene(s, ctx).await,
        Commands::Use(u) => execute_use(u, ctx).await,
        Commands::Context => execute_context(ctx),
        // LLM 命令留桩（Task 6 接入）
        Commands::Derive { .. }
        | Commands::Narrate { .. }
        | Commands::Show(ShowCmd::Derivation { .. })
        | Commands::Show(ShowCmd::Prose { .. }) => err_observation("此命令尚未接入".into()),
        // REPL 循环仅在交互模式可用（Task 7）
        Commands::Repl => err_observation("REPL 仅在交互模式可用".into()),
    }
}

// ---- 角色命令 ----

async fn execute_character(cmd: CharacterCmd, ctx: &mut CommandContext<'_>) -> Observation {
    match cmd {
        CharacterCmd::Create { name, tags, skills } => {
            let id = CharacterId(Uuid::new_v4());
            let tags = tags.as_deref().map(split_csv).unwrap_or_default();
            let skills = skills.as_deref().map(split_csv).unwrap_or_default();
            match ctx
                .service
                .db()
                .characters()
                .create(id, &name, &tags, &skills)
                .await
            {
                Ok(()) => {
                    let mut artifacts = BTreeMap::new();
                    artifacts.insert("character_id".into(), id.0.to_string());
                    success_observation("角色已创建".into(), artifacts)
                }
                Err(e) => err_observation(e.to_string()),
            }
        }
        CharacterCmd::List => match ctx.service.list_characters().await {
            Ok(chars) => {
                let lines: Vec<String> = chars
                    .iter()
                    .map(|c| format!("{} ({})", c.name, c.id.0))
                    .collect();
                let mut artifacts = BTreeMap::new();
                artifacts.insert("characters".into(), lines.join("\n"));
                success_observation(format!("{} 个角色", chars.len()), artifacts)
            }
            Err(e) => err_observation(e.to_string()),
        },
        CharacterCmd::Show { id_or_name } => {
            match resolve_character(ctx.service.db(), &id_or_name).await {
                Ok(id) => match ctx.service.db().characters().get(id).await {
                    Ok(Some(c)) => {
                        let mut artifacts = BTreeMap::new();
                        artifacts.insert("name".into(), c.name.clone());
                        artifacts.insert("id".into(), c.id.0.to_string());
                        artifacts.insert("tags".into(), c.personality.join(", "));
                        artifacts.insert("skills".into(), c.skills.join(", "));
                        success_observation(format!("角色：{}", c.name), artifacts)
                    }
                    Ok(None) => err_observation(format!("未找到角色：{id_or_name}")),
                    Err(e) => err_observation(e.to_string()),
                },
                Err(e) => err_observation(e.summary()),
            }
        }
    }
}

// ---- 场景命令 ----

async fn execute_scene(cmd: SceneCmd, ctx: &mut CommandContext<'_>) -> Observation {
    match cmd {
        SceneCmd::Create { event, with } => {
            let tokens = split_csv(&with);
            let mut participant_ids = Vec::new();
            for token in &tokens {
                match resolve_character(ctx.service.db(), token).await {
                    Ok(id) => participant_ids.push(id),
                    Err(e) => return err_observation(e.summary()),
                }
            }
            let input = CreateScene {
                objective_event: event,
                participant_ids,
                occurred_at: Utc::now(),
            };
            match ctx.service.create_scene(input).await {
                Ok(scene_id) => {
                    // 场景创建成功后自动设为当前场景
                    ctx.session.current_scene = Some(scene_id);
                    let mut artifacts = BTreeMap::new();
                    artifacts.insert("scene_id".into(), scene_id.0.to_string());
                    success_observation("场景已创建".into(), artifacts)
                }
                Err(e) => err_observation(e.to_string()),
            }
        }
        SceneCmd::List => match ctx.service.list_scenes().await {
            Ok(scenes) => {
                let lines: Vec<String> = scenes
                    .iter()
                    .map(|s| format!("{} ({})", s.objective_event, s.id.0))
                    .collect();
                let mut artifacts = BTreeMap::new();
                artifacts.insert("scenes".into(), lines.join("\n"));
                success_observation(format!("{} 个场景", scenes.len()), artifacts)
            }
            Err(e) => err_observation(e.to_string()),
        },
        SceneCmd::Show { id_or_name } => match resolve_scene(ctx.service.db(), &id_or_name).await {
            Ok(id) => match ctx.service.db().scenes().get(id).await {
                Ok(Some(s)) => {
                    let mut artifacts = BTreeMap::new();
                    artifacts.insert("event".into(), s.objective_event.clone());
                    artifacts.insert("id".into(), s.id.0.to_string());
                    artifacts.insert("occurred_at".into(), s.occurred_at.to_rfc3339());
                    let participants: Vec<String> =
                        s.participant_ids.iter().map(|p| p.0.to_string()).collect();
                    artifacts.insert("participants".into(), participants.join(", "));
                    success_observation(format!("场景：{}", s.objective_event), artifacts)
                }
                Ok(None) => err_observation(format!("未找到场景：{id_or_name}")),
                Err(e) => err_observation(e.to_string()),
            },
            Err(e) => err_observation(e.summary()),
        },
    }
}

// ---- Use 命令（会话上下文切换） ----

async fn execute_use(cmd: UseCmd, ctx: &mut CommandContext<'_>) -> Observation {
    match cmd {
        UseCmd::Scene { id_or_name } => match resolve_scene(ctx.service.db(), &id_or_name).await {
            Ok(id) => {
                ctx.session.current_scene = Some(id);
                success_observation("已选定场景".into(), BTreeMap::new())
            }
            Err(e) => err_observation(e.summary()),
        },
        UseCmd::Character { id_or_name } => {
            match resolve_character(ctx.service.db(), &id_or_name).await {
                Ok(id) => {
                    ctx.session.current_character = Some(id);
                    success_observation("已选定角色".into(), BTreeMap::new())
                }
                Err(e) => err_observation(e.summary()),
            }
        }
        UseCmd::Clear => {
            ctx.session.current_scene = None;
            ctx.session.current_character = None;
            success_observation("已清除上下文".into(), BTreeMap::new())
        }
    }
}

// ---- Context 命令 ----

fn execute_context(ctx: &mut CommandContext<'_>) -> Observation {
    let mut artifacts = BTreeMap::new();
    let scene = ctx
        .session
        .current_scene
        .map(|s| s.0.to_string())
        .unwrap_or_default();
    let character = ctx
        .session
        .current_character
        .map(|c| c.0.to_string())
        .unwrap_or_default();
    artifacts.insert("current_scene".into(), scene);
    artifacts.insert("current_character".into(), character);

    let summary = if ctx.session.current_scene.is_none() && ctx.session.current_character.is_none()
    {
        "无上下文".to_string()
    } else {
        "当前上下文".to_string()
    };
    success_observation(summary, artifacts)
}

// ---- 辅助函数 ----

/// 逗号分隔 -> 逐项 trim -> 丢弃空串。
fn split_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

/// 构造成功观测。
fn success_observation(summary: String, artifacts: BTreeMap<String, String>) -> Observation {
    Observation {
        status: Status::Success,
        summary,
        artifacts,
        quality: None,
        next: vec![],
    }
}

/// 构造错误观测。
fn err_observation(summary: String) -> Observation {
    Observation {
        status: Status::Error,
        summary,
        artifacts: BTreeMap::new(),
        quality: None,
        next: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_csv_trims_and_drops_empty() {
        assert_eq!(split_csv("a, b , c"), vec!["a", "b", "c"]);
        assert_eq!(split_csv("a,, ,b"), vec!["a", "b"]);
        assert_eq!(split_csv(""), Vec::<String>::new());
        assert_eq!(split_csv("  "), Vec::<String>::new());
    }
}
