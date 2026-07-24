// CLI 命令执行器
// ==============
// 把 `Commands` 枚举分发到具体的业务逻辑，返回 `CommandOutput`（观测 + 可选正文）。
//
// 非 LLM 命令（character/scene/use/context）直接操作 DB；
// LLM 命令（derive/narrate）通过 StoryService 调用，show 展示持久化结果。

use std::collections::BTreeMap;

use chrono::Utc;
use uuid::Uuid;

use crate::cli::args::{CharacterCmd, Commands, SceneCmd, ShowCmd, UseCmd};
use crate::cli::observation::{Observation, Status, quality_from_prose};
use crate::cli::prose_cache::ProseCache;
use crate::cli::session::{Session, resolve_character, resolve_scene};
use crate::models::{
    CharacterDerivation, CharacterId, CreateScene, SceneDerivationDetail, SceneId, StoryError,
};
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

/// 命令输出：结构化观测 + 可选正文（narrate / show prose）。
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub observation: Observation,
    pub body: Option<String>,
}

/// 分发命令并返回命令输出。
pub async fn execute(cmd: Commands, ctx: &mut CommandContext<'_>) -> CommandOutput {
    match cmd {
        Commands::Character(c) => wrap(execute_character(c, ctx).await),
        Commands::Scene(s) => wrap(execute_scene(s, ctx).await),
        Commands::Use(u) => wrap(execute_use(u, ctx).await),
        Commands::Context => wrap(execute_context(ctx)),
        Commands::Derive { scene, character } => execute_derive(scene, character, ctx).await,
        Commands::Narrate { scene } => execute_narrate(scene, ctx).await,
        Commands::Show(ShowCmd::Derivation { scene, character }) => {
            execute_show_derivation(scene, character, ctx).await
        }
        Commands::Show(ShowCmd::Prose { scene }) => execute_show_prose(scene, ctx).await,
        // REPL 循环仅在交互模式可用（Task 7）
        Commands::Repl => wrap(err_observation("REPL 仅在交互模式可用".into())),
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
                Err(e) => err_from_story(e),
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
            Err(e) => err_from_story(e),
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
                    Err(e) => err_from_story(e),
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
            if tokens.is_empty() {
                return err_observation("场景至少需要一名参与者".into());
            }
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
                Err(e) => err_from_story(e),
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
            Err(e) => err_from_story(e),
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
                Err(e) => err_from_story(e),
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

// ---- Derive 命令 ----

async fn execute_derive(
    scene: Option<String>,
    character: Option<String>,
    ctx: &mut CommandContext<'_>,
) -> CommandOutput {
    // 1. 解析场景：flag > session
    let scene_id = match resolve_scene_for_cmd(scene.as_deref(), ctx).await {
        Ok(id) => id,
        Err(o) => return wrap(o),
    };

    // 2. 解析角色：flag > session
    let char_id = if let Some(name) = &character {
        match resolve_character(ctx.service.db(), name).await {
            Ok(id) => Some(id),
            Err(e) => return wrap(err_observation(e.summary())),
        }
    } else {
        ctx.session.current_character
    };

    // 3. 执行推导
    let status = if ctx.using_mock {
        Status::Warning
    } else {
        Status::Success
    };

    if let Some(cid) = char_id {
        // 单角色推导
        match ctx.service.derive_character(scene_id, cid).await {
            Ok(_) => {
                let summary = if ctx.using_mock {
                    "推导完成（非生产质量）".to_string()
                } else {
                    "推导完成".to_string()
                };
                let mut artifacts = BTreeMap::new();
                artifacts.insert("scene_id".into(), scene_id.0.to_string());
                artifacts.insert("character_id".into(), cid.0.to_string());
                wrap(Observation {
                    status,
                    summary,
                    artifacts,
                    quality: None,
                    next: vec![],
                })
            }
            Err(e) => wrap(err_from_story(e)),
        }
    } else {
        // 全场景推导
        let results = ctx.service.derive_scene(scene_id).await;
        let ok_count = results.iter().filter(|r| r.is_ok()).count();
        let err_count = results.len() - ok_count;

        let summary = if err_count > 0 {
            format!("推导完成：{ok_count} 成功，{err_count} 失败")
        } else if ctx.using_mock {
            format!("推导完成：{ok_count} 个角色（非生产质量）")
        } else {
            format!("推导完成：{ok_count} 个角色")
        };

        let mut artifacts = BTreeMap::new();
        artifacts.insert("scene_id".into(), scene_id.0.to_string());
        artifacts.insert("ok_count".into(), ok_count.to_string());
        artifacts.insert("err_count".into(), err_count.to_string());

        let final_status = if err_count > 0 && ok_count == 0 {
            Status::Error
        } else {
            status
        };

        wrap(Observation {
            status: final_status,
            summary,
            artifacts,
            quality: None,
            next: if ok_count > 0 {
                vec!["narrate".into()]
            } else {
                vec![]
            },
        })
    }
}

// ---- Narrate 命令 ----

async fn execute_narrate(scene: Option<String>, ctx: &mut CommandContext<'_>) -> CommandOutput {
    // 1. 解析场景
    let scene_id = match resolve_scene_for_cmd(scene.as_deref(), ctx).await {
        Ok(id) => id,
        Err(o) => return wrap(o),
    };

    // 2. 从 DB 加载推导详情
    //    scene_derivations 要求所有参与者都有推导记录；
    //    若部分角色未推导，会返回 CharacterNotFound -> 映射为「请先 derive」提示。
    let details = match ctx.service.scene_derivations(scene_id).await {
        Ok(d) => d,
        Err(StoryError::CharacterNotFound(_)) => {
            return wrap(Observation {
                status: Status::Error,
                summary: "该场景尚无完整推导记录（部分角色未推导）".into(),
                artifacts: BTreeMap::new(),
                quality: None,
                next: vec!["derive".into()],
            });
        }
        Err(e) => return wrap(err_from_story(e)),
    };

    // 3. 预检：无推导 -> 提示先 derive
    if details.is_empty() {
        return wrap(Observation {
            status: Status::Error,
            summary: "该场景尚无推导记录".into(),
            artifacts: BTreeMap::new(),
            quality: None,
            next: vec!["derive".into()],
        });
    }

    // 4. 映射 SceneDerivationDetail -> CharacterDerivation
    let derivations: Vec<CharacterDerivation> = details
        .iter()
        .map(|d| detail_to_derivation(d, scene_id))
        .collect();

    // 5. 调用 narrate
    match ctx.service.narrate_scene(scene_id, &derivations).await {
        Ok(prose) => {
            // 缓存
            ctx.prose_cache.scene_id = Some(scene_id);
            ctx.prose_cache.prose = Some(prose.clone());

            let status = if ctx.using_mock {
                Status::Warning
            } else {
                Status::Success
            };

            CommandOutput {
                observation: Observation {
                    status,
                    summary: if ctx.using_mock {
                        "正文已生成（非生产质量）".into()
                    } else {
                        "正文已生成".into()
                    },
                    artifacts: BTreeMap::new(),
                    quality: Some(quality_from_prose(&prose)),
                    next: vec![],
                },
                body: Some(prose.text.clone()),
            }
        }
        Err(e) => wrap(err_from_story(e)),
    }
}

// ---- Show Derivation 命令 ----

async fn execute_show_derivation(
    scene: Option<String>,
    character: Option<String>,
    ctx: &mut CommandContext<'_>,
) -> CommandOutput {
    let scene_id = match resolve_scene_for_cmd(scene.as_deref(), ctx).await {
        Ok(id) => id,
        Err(o) => return wrap(o),
    };

    let details = match ctx.service.scene_derivations(scene_id).await {
        Ok(d) => d,
        Err(e) => return wrap(err_from_story(e)),
    };

    if details.is_empty() {
        return wrap(Observation {
            status: Status::Error,
            summary: "该场景尚无推导记录".into(),
            artifacts: BTreeMap::new(),
            quality: None,
            next: vec!["derive".into()],
        });
    }

    // 可选过滤到单个角色
    let filtered: Vec<&SceneDerivationDetail> = if let Some(name) = &character {
        match resolve_character(ctx.service.db(), name).await {
            Ok(cid) => details.iter().filter(|d| d.character.id == cid).collect(),
            Err(e) => return wrap(err_observation(e.summary())),
        }
    } else {
        details.iter().collect()
    };

    let mut lines = Vec::new();
    for d in &filtered {
        let mem_preview = d.memory.content.render();
        let mem_preview = if mem_preview.chars().count() > 40 {
            let mut p: String = mem_preview.chars().take(40).collect();
            p.push('…');
            p
        } else {
            mem_preview
        };
        lines.push(format!(
            "  {}：记忆「{}」、剧情 {} 条、关系候选 {} 条",
            d.character.name,
            mem_preview,
            d.plot_developments.len(),
            d.relationship_candidates.len()
        ));
    }

    let mut artifacts = BTreeMap::new();
    artifacts.insert("scene_id".into(), scene_id.0.to_string());
    artifacts.insert("derivation_count".into(), filtered.len().to_string());
    artifacts.insert("details".into(), lines.join("\n"));

    wrap(success_observation(
        format!("场景有 {} 个角色推导", filtered.len()),
        artifacts,
    ))
}

// ---- Show Prose 命令 ----

async fn execute_show_prose(scene: Option<String>, ctx: &mut CommandContext<'_>) -> CommandOutput {
    // 如果有缓存且场景匹配，直接返回
    if let Some(prose) = &ctx.prose_cache.prose {
        let scene_matches = match (&ctx.prose_cache.scene_id, &scene) {
            (Some(cache_sid), Some(flag)) => match resolve_scene(ctx.service.db(), flag).await {
                Ok(sid) => *cache_sid == sid,
                Err(e) => return wrap(err_observation(e.summary())),
            },
            (Some(_), None) => true,
            _ => false,
        };
        if scene_matches {
            return CommandOutput {
                observation: success_observation("正文缓存".into(), BTreeMap::new()),
                body: Some(prose.text.clone()),
            };
        }
    }

    // 无缓存：区分 one-shot vs REPL
    if ctx.one_shot {
        wrap(Observation {
            status: Status::Error,
            summary: "正文仅在 REPL 进程内缓存；单次命令模式下请使用 novels narrate --scene <id> 直接输出正文。".into(),
            artifacts: BTreeMap::new(),
            quality: None,
            next: vec!["novels narrate --scene <id>".into()],
        })
    } else {
        wrap(Observation {
            status: Status::Error,
            summary: "当前进程尚无 narrate 缓存。请先执行 narrate。".into(),
            artifacts: BTreeMap::new(),
            quality: None,
            next: vec!["narrate".into()],
        })
    }
}

// ---- 辅助函数 ----

/// 逗号分隔 -> 逐项 trim -> 丢弃空串。
fn split_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

/// 把 SceneDerivationDetail 映射为 CharacterDerivation（只读，不调 LLM）。
fn detail_to_derivation(detail: &SceneDerivationDetail, scene_id: SceneId) -> CharacterDerivation {
    CharacterDerivation {
        character_id: detail.character.id,
        scene_id,
        sensations: detail.sensation.clone(),
        new_memory: detail.memory.clone(),
        plot_development: detail
            .plot_developments
            .iter()
            .map(|p| p.development.clone())
            .collect(),
        relationship_candidates: detail.relationship_candidates.clone(),
    }
}

/// 解析场景：flag > session.current_scene；都没有则返回 pre-gate 错误。
async fn resolve_scene_for_cmd(
    flag: Option<&str>,
    ctx: &mut CommandContext<'_>,
) -> Result<SceneId, Observation> {
    if let Some(s) = flag {
        resolve_scene(ctx.service.db(), s)
            .await
            .map_err(|e| err_observation(e.summary()))
    } else if let Some(id) = ctx.session.current_scene {
        Ok(id)
    } else {
        Err(Observation {
            status: Status::Error,
            summary: "未指定场景".into(),
            artifacts: BTreeMap::new(),
            quality: None,
            next: vec!["use scene <id>".into(), "derive --scene <id>".into()],
        })
    }
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

/// 把 StoryError 映射为中文错误摘要。
///
/// 对数据库锁定/忙碌给出专门的友好提示；
/// 其余变体的 Display 文本作为兜底（附带中文前缀）。
pub fn story_error_summary(e: &StoryError) -> String {
    match e {
        StoryError::Database(msg) => {
            let lower = msg.to_lowercase();
            if lower.contains("database is locked") || lower.contains("busy") {
                "数据库忙碌（可能与 API 进程同时写入 novels.db）；请稍后重试，MVP 不提供多写合并。"
                    .to_string()
            } else {
                format!("数据库错误：{msg}")
            }
        }
        StoryError::SceneNotFound(id) => format!("未找到场景：{id:?}"),
        StoryError::CharacterNotFound(id) => format!("未找到角色：{id:?}"),
        StoryError::NotSceneParticipant(cid, sid) => {
            format!("角色 {cid:?} 不是场景 {sid:?} 的参与者")
        }
        StoryError::InvalidParticipant(msg) => format!("无效参与者：{msg}"),
        StoryError::InvalidNarrationContext(msg) => format!("叙述上下文无效：{msg}"),
        StoryError::Llm(msg) => format!("LLM 调用失败：{msg}"),
        StoryError::InvalidVocabularySelection(msg) => format!("无效词汇选择：{msg}"),
        StoryError::VocabularyLoad(msg) => format!("词库加载失败：{msg}"),
        StoryError::RelationshipCandidateNotFound(id) => format!("未找到关系候选：{id:?}"),
        StoryError::RelationshipCandidateResolved(id) => format!("关系候选已确认：{id:?}"),
        StoryError::InvalidRelationshipCandidate(msg) => format!("无效关系候选：{msg}"),
    }
}

/// 从 StoryError 构造错误观测。
fn err_from_story(e: StoryError) -> Observation {
    err_observation(story_error_summary(&e))
}

/// 把 Observation 包成无正文的 CommandOutput。
fn wrap(observation: Observation) -> CommandOutput {
    CommandOutput {
        observation,
        body: None,
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

    #[test]
    fn story_error_busy_db_maps_to_chinese() {
        let e = StoryError::Database("database is locked".into());
        let msg = story_error_summary(&e);
        assert!(
            msg.contains("数据库忙碌"),
            "busy db should map to busy message: {msg}"
        );
    }

    #[test]
    fn story_error_not_found_maps_to_chinese() {
        let e = StoryError::CharacterNotFound(CharacterId(Uuid::nil()));
        let msg = story_error_summary(&e);
        assert!(
            msg.contains("未找到角色"),
            "character not found should be Chinese: {msg}"
        );
    }

    #[test]
    fn story_error_llm_maps_to_chinese() {
        let e = StoryError::Llm("timeout".into());
        let msg = story_error_summary(&e);
        assert!(msg.contains("LLM"), "llm error should mention LLM: {msg}");
    }
}
