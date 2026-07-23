// CLI 命令执行器集成测试
// ========================
// 覆盖非 LLM 命令（character/scene/use/context）和 LLM 命令（derive/narrate/show）。
// 全部使用内存 SQLite + Mock 生成器，不触碰真实 LLM。

use novels::cli::{
    CharacterCmd, CommandContext, Commands, ProseCache, SceneCmd, Session, ShowCmd, Status, UseCmd,
    execute,
};
use novels::db::Db;
use novels::llm::{LlmCharacterDerivation, LlmContextTagSelection, MockSenseGenerator};
use novels::models::*;
use novels::prose::MockProseGenerator;
use novels::scene::StoryService;
use novels::vocab::Vocab;
use std::sync::Arc;

/// 构造一个带 Mock 生成器的 StoryService（内存 DB + 真实词库）。
async fn make_service() -> StoryService {
    let db = Db::open_in_memory().await.unwrap();
    let yaml = std::include_str!("../assets/vocab.yaml");
    let vocab = Vocab::load_from_str(yaml).unwrap();
    let canned = LlmCharacterDerivation {
        sensations: SensorySelection {
            visual_ids: vec![VocabularyId::new("visual.bloodstain").unwrap()],
            ..Default::default()
        },
        new_memory: CharacterMemoryDraft {
            content: "看到血迹".into(),
            source: MemorySource::Witnessed,
            certainty: Certainty::Certain,
        },
        plot_development: vec![],
        relationship_candidates: vec![],
    };
    let sense_gen = Arc::new(MockSenseGenerator::new(
        LlmContextTagSelection::default(),
        canned,
    ));
    let prose_gen = Arc::new(MockProseGenerator::fallback());
    StoryService::new(db, vocab, sense_gen, prose_gen)
}

/// 构造一个默认的 CommandContext（mock 模式，非 json，one-shot）。
fn make_ctx<'a>(
    service: &'a StoryService,
    session: &'a mut Session,
    prose_cache: &'a mut ProseCache,
) -> CommandContext<'a> {
    CommandContext {
        service,
        session,
        prose_cache,
        using_mock: true,
        json: false,
        one_shot: true,
    }
}

#[tokio::test]
async fn character_create_via_execute() {
    let service = make_service().await;
    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    let output = execute(
        Commands::Character(CharacterCmd::Create {
            name: "宝玉".into(),
            tags: Some("痴情,贵公子".into()),
            skills: Some("诗词".into()),
        }),
        &mut ctx,
    )
    .await;

    let obs = &output.observation;
    assert_eq!(obs.status, Status::Success, "summary: {}", obs.summary);
    assert_eq!(obs.summary, "角色已创建");
    let id_str = obs
        .artifacts
        .get("character_id")
        .expect("character_id artifact missing");
    assert!(
        uuid::Uuid::parse_str(id_str).is_ok(),
        "not a uuid: {id_str}"
    );

    // 角色应出现在列表中
    let list_output = execute(Commands::Character(CharacterCmd::List), &mut ctx).await;
    assert_eq!(list_output.observation.status, Status::Success);
    assert!(
        list_output.observation.summary.contains("1"),
        "count: {}",
        list_output.observation.summary
    );
    let chars = list_output
        .observation
        .artifacts
        .get("characters")
        .expect("characters artifact missing");
    assert!(chars.contains("宝玉"), "list: {chars}");
}

#[tokio::test]
async fn character_list_via_execute() {
    let service = make_service().await;
    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    for name in ["宝玉", "黛玉"] {
        let output = execute(
            Commands::Character(CharacterCmd::Create {
                name: name.into(),
                tags: None,
                skills: None,
            }),
            &mut ctx,
        )
        .await;
        assert_eq!(output.observation.status, Status::Success);
    }

    let list_output = execute(Commands::Character(CharacterCmd::List), &mut ctx).await;
    assert_eq!(list_output.observation.status, Status::Success);
    assert!(
        list_output.observation.summary.contains("2"),
        "expected 2 chars, got: {}",
        list_output.observation.summary
    );
}

#[tokio::test]
async fn scene_create_via_execute() {
    let service = make_service().await;
    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    // 先创建一个角色
    execute(
        Commands::Character(CharacterCmd::Create {
            name: "宝玉".into(),
            tags: None,
            skills: None,
        }),
        &mut ctx,
    )
    .await;

    let output = execute(
        Commands::Scene(SceneCmd::Create {
            event: "宝玉挨打".into(),
            with: "宝玉".into(),
        }),
        &mut ctx,
    )
    .await;

    assert_eq!(
        output.observation.status,
        Status::Success,
        "summary: {}",
        output.observation.summary
    );
    assert_eq!(output.observation.summary, "场景已创建");
    assert!(output.observation.artifacts.contains_key("scene_id"));

    // 场景创建成功后应自动设置 session.current_scene
    assert!(
        ctx.session.current_scene.is_some(),
        "current_scene should be set after scene create"
    );
}

#[tokio::test]
async fn scene_create_empty_participants_errors() {
    let service = make_service().await;
    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    let output = execute(
        Commands::Scene(SceneCmd::Create {
            event: "空场景".into(),
            with: "  ,  ".into(),
        }),
        &mut ctx,
    )
    .await;

    assert_eq!(output.observation.status, Status::Error);
    assert!(
        output.observation.summary.contains("参与者"),
        "should mention participants: {}",
        output.observation.summary
    );
}

#[tokio::test]
async fn scene_create_resolves_participants() {
    let service = make_service().await;
    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    // 创建两个角色
    for name in ["宝玉", "黛玉"] {
        execute(
            Commands::Character(CharacterCmd::Create {
                name: name.into(),
                tags: None,
                skills: None,
            }),
            &mut ctx,
        )
        .await;
    }

    let output = execute(
        Commands::Scene(SceneCmd::Create {
            event: "共读西厢".into(),
            with: "宝玉,黛玉".into(),
        }),
        &mut ctx,
    )
    .await;

    assert_eq!(
        output.observation.status,
        Status::Success,
        "summary: {}",
        output.observation.summary
    );
    let scene_id_str = output
        .observation
        .artifacts
        .get("scene_id")
        .expect("scene_id artifact missing");

    // 验证场景确实有两个参与者
    let scene_id = SceneId(uuid::Uuid::parse_str(scene_id_str).unwrap());
    let scene = service
        .db()
        .scenes()
        .get(scene_id)
        .await
        .unwrap()
        .expect("scene should exist");
    assert_eq!(
        scene.participant_ids.len(),
        2,
        "expected 2 participants, got {}",
        scene.participant_ids.len()
    );
}

#[tokio::test]
async fn scene_show_via_execute() {
    let service = make_service().await;
    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    execute(
        Commands::Character(CharacterCmd::Create {
            name: "宝玉".into(),
            tags: None,
            skills: None,
        }),
        &mut ctx,
    )
    .await;

    execute(
        Commands::Scene(SceneCmd::Create {
            event: "宝玉挨打".into(),
            with: "宝玉".into(),
        }),
        &mut ctx,
    )
    .await;

    let output = execute(
        Commands::Scene(SceneCmd::Show {
            id_or_name: "宝玉挨打".into(),
        }),
        &mut ctx,
    )
    .await;

    assert_eq!(
        output.observation.status,
        Status::Success,
        "summary: {}",
        output.observation.summary
    );
    assert_eq!(
        output
            .observation
            .artifacts
            .get("event")
            .map(|s| s.as_str()),
        Some("宝玉挨打")
    );
}

#[tokio::test]
async fn use_scene_sets_session() {
    let service = make_service().await;
    // 通过直接 DB 调用创建角色和场景，保持 session 干净
    let cid = CharacterId(uuid::Uuid::new_v4());
    service
        .db()
        .characters()
        .create(cid, "宝玉", &[], &[])
        .await
        .unwrap();
    let sid = service
        .create_scene(CreateScene {
            objective_event: "宝玉挨打".into(),
            participant_ids: vec![cid],
            occurred_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    let output = execute(
        Commands::Use(UseCmd::Scene {
            id_or_name: sid.0.to_string(),
        }),
        &mut ctx,
    )
    .await;

    assert_eq!(output.observation.status, Status::Success);
    assert_eq!(
        ctx.session.current_scene,
        Some(sid),
        "current_scene should be set after Use::Scene"
    );
}

#[tokio::test]
async fn use_clear_resets_session() {
    let service = make_service().await;
    // 预设 session 字段（在创建 ctx 之前）
    let mut session = Session {
        current_scene: Some(SceneId(uuid::Uuid::new_v4())),
        current_character: Some(CharacterId(uuid::Uuid::new_v4())),
    };
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    let output = execute(Commands::Use(UseCmd::Clear), &mut ctx).await;
    assert_eq!(output.observation.status, Status::Success);

    assert!(
        ctx.session.current_scene.is_none(),
        "scene should be cleared"
    );
    assert!(
        ctx.session.current_character.is_none(),
        "character should be cleared"
    );
}

#[tokio::test]
async fn context_shows_empty() {
    let service = make_service().await;
    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    let output = execute(Commands::Context, &mut ctx).await;
    assert_eq!(output.observation.status, Status::Success);
    assert!(
        output.observation.summary.contains("无"),
        "empty context summary should mention 无, got: {}",
        output.observation.summary
    );
}

#[tokio::test]
async fn derive_without_scene_is_pregate_error() {
    let service = make_service().await;
    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    let output = execute(
        Commands::Derive {
            scene: None,
            character: None,
        },
        &mut ctx,
    )
    .await;

    assert_eq!(
        output.observation.status,
        Status::Error,
        "derive without scene should error: {}",
        output.observation.summary
    );
    // next 应该提示 use scene / derive --scene
    assert!(
        !output.observation.next.is_empty(),
        "should have next suggestions"
    );
    let next_joined = output.observation.next.join(" ");
    assert!(
        next_joined.contains("scene") || next_joined.contains("场景"),
        "next should mention scene: {next_joined}"
    );
}

#[tokio::test]
async fn closed_loop_derive_narrate_has_quality() {
    let service = make_service().await;
    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    // 1. 创建角色
    let char_output = execute(
        Commands::Character(CharacterCmd::Create {
            name: "宝玉".into(),
            tags: None,
            skills: None,
        }),
        &mut ctx,
    )
    .await;
    assert_eq!(char_output.observation.status, Status::Success);

    // 2. 创建场景（自动设为 session.current_scene）
    let scene_output = execute(
        Commands::Scene(SceneCmd::Create {
            event: "宝玉挨打".into(),
            with: "宝玉".into(),
        }),
        &mut ctx,
    )
    .await;
    assert_eq!(scene_output.observation.status, Status::Success);

    // 3. 推导（using_mock -> Warning）
    let derive_output = execute(
        Commands::Derive {
            scene: None,
            character: None,
        },
        &mut ctx,
    )
    .await;
    assert_eq!(
        derive_output.observation.status,
        Status::Warning,
        "derive with mock should be warning: {}",
        derive_output.observation.summary
    );

    // 4. 叙述
    let narrate_output = execute(Commands::Narrate { scene: None }, &mut ctx).await;
    assert_eq!(
        narrate_output.observation.status,
        Status::Warning,
        "narrate with mock should be warning: {}",
        narrate_output.observation.summary
    );
    assert!(
        narrate_output.observation.quality.is_some(),
        "narrate should include quality KPIs"
    );
    assert!(
        narrate_output.body.is_some(),
        "narrate should produce body text"
    );

    // 5. 缓存应被设置
    assert!(ctx.prose_cache.prose.is_some());
    assert!(ctx.prose_cache.scene_id.is_some());
}

#[tokio::test]
async fn one_shot_show_prose_explains_repl_cache() {
    let service = make_service().await;
    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    let output = execute(Commands::Show(ShowCmd::Prose { scene: None }), &mut ctx).await;

    assert_eq!(output.observation.status, Status::Error);
    assert!(
        output.observation.summary.contains("REPL")
            || output.observation.summary.contains("narrate"),
        "one-shot show prose should explain REPL cache: {}",
        output.observation.summary
    );
    assert!(
        !output.observation.next.is_empty(),
        "should suggest narrate next"
    );
}

#[tokio::test]
async fn show_derivation_summarizes_counts() {
    let service = make_service().await;
    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    // 创建角色 + 场景 + 推导
    execute(
        Commands::Character(CharacterCmd::Create {
            name: "宝玉".into(),
            tags: None,
            skills: None,
        }),
        &mut ctx,
    )
    .await;

    execute(
        Commands::Scene(SceneCmd::Create {
            event: "宝玉挨打".into(),
            with: "宝玉".into(),
        }),
        &mut ctx,
    )
    .await;

    execute(
        Commands::Derive {
            scene: None,
            character: None,
        },
        &mut ctx,
    )
    .await;

    // 查看推导
    let output = execute(
        Commands::Show(ShowCmd::Derivation {
            scene: None,
            character: None,
        }),
        &mut ctx,
    )
    .await;

    assert_eq!(
        output.observation.status,
        Status::Success,
        "summary: {}",
        output.observation.summary
    );
    assert!(
        output.observation.summary.contains("1"),
        "should mention 1 derivation: {}",
        output.observation.summary
    );
    assert!(
        output
            .observation
            .artifacts
            .contains_key("derivation_count"),
        "should have derivation_count artifact"
    );
}

#[tokio::test]
async fn show_prose_after_narrate_returns_body() {
    let service = make_service().await;
    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    // 完整流程：创建 -> 场景 -> 推导 -> 叙述 -> 查看正文
    execute(
        Commands::Character(CharacterCmd::Create {
            name: "宝玉".into(),
            tags: None,
            skills: None,
        }),
        &mut ctx,
    )
    .await;

    execute(
        Commands::Scene(SceneCmd::Create {
            event: "宝玉挨打".into(),
            with: "宝玉".into(),
        }),
        &mut ctx,
    )
    .await;

    execute(
        Commands::Derive {
            scene: None,
            character: None,
        },
        &mut ctx,
    )
    .await;

    let narrate_output = execute(Commands::Narrate { scene: None }, &mut ctx).await;
    assert!(narrate_output.body.is_some());

    let show_output = execute(Commands::Show(ShowCmd::Prose { scene: None }), &mut ctx).await;
    assert_eq!(show_output.observation.status, Status::Success);
    assert!(show_output.body.is_some(), "show prose should return body");
}

#[tokio::test]
async fn character_show_via_execute() {
    let service = make_service().await;
    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    let create_output = execute(
        Commands::Character(CharacterCmd::Create {
            name: "宝玉".into(),
            tags: Some("痴情".into()),
            skills: Some("诗词".into()),
        }),
        &mut ctx,
    )
    .await;
    assert_eq!(create_output.observation.status, Status::Success);

    let show_output = execute(
        Commands::Character(CharacterCmd::Show {
            id_or_name: "宝玉".into(),
        }),
        &mut ctx,
    )
    .await;

    assert_eq!(
        show_output.observation.status,
        Status::Success,
        "summary: {}",
        show_output.observation.summary
    );
    assert_eq!(
        show_output
            .observation
            .artifacts
            .get("name")
            .map(|s| s.as_str()),
        Some("宝玉")
    );
    assert!(show_output.observation.artifacts.contains_key("id"));
}

#[tokio::test]
async fn scene_list_via_execute() {
    let service = make_service().await;
    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    execute(
        Commands::Character(CharacterCmd::Create {
            name: "宝玉".into(),
            tags: None,
            skills: None,
        }),
        &mut ctx,
    )
    .await;

    for event in ["事件甲", "事件乙"] {
        execute(
            Commands::Scene(SceneCmd::Create {
                event: event.into(),
                with: "宝玉".into(),
            }),
            &mut ctx,
        )
        .await;
    }

    let list_output = execute(Commands::Scene(SceneCmd::List), &mut ctx).await;
    assert_eq!(list_output.observation.status, Status::Success);
    assert!(
        list_output.observation.summary.contains("2"),
        "expected 2 scenes, got: {}",
        list_output.observation.summary
    );
}

#[tokio::test]
async fn use_character_sets_session() {
    let service = make_service().await;
    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    execute(
        Commands::Character(CharacterCmd::Create {
            name: "黛玉".into(),
            tags: None,
            skills: None,
        }),
        &mut ctx,
    )
    .await;

    let output = execute(
        Commands::Use(UseCmd::Character {
            id_or_name: "黛玉".into(),
        }),
        &mut ctx,
    )
    .await;

    assert_eq!(output.observation.status, Status::Success);
    assert!(
        ctx.session.current_character.is_some(),
        "current_character should be set after Use::Character"
    );
}

#[tokio::test]
async fn context_shows_populated() {
    let service = make_service().await;
    let sid = SceneId(uuid::Uuid::new_v4());
    let cid = CharacterId(uuid::Uuid::new_v4());
    // 预设 session 字段（在创建 ctx 之前）
    let mut session = Session {
        current_scene: Some(sid),
        current_character: Some(cid),
    };
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    let output = execute(Commands::Context, &mut ctx).await;
    assert_eq!(output.observation.status, Status::Success);
    assert!(
        !output.observation.summary.contains("无"),
        "populated context should not say 无, got: {}",
        output.observation.summary
    );
    assert_eq!(
        output
            .observation
            .artifacts
            .get("current_scene")
            .map(|s| s.as_str()),
        Some(sid.0.to_string()).as_deref()
    );
    assert_eq!(
        output
            .observation
            .artifacts
            .get("current_character")
            .map(|s| s.as_str()),
        Some(cid.0.to_string()).as_deref()
    );
}

#[tokio::test]
async fn repl_returns_error() {
    let service = make_service().await;
    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    let output = execute(Commands::Repl, &mut ctx).await;
    assert_eq!(output.observation.status, Status::Error);
}

#[tokio::test]
async fn command_output_body_is_none_for_non_narrate() {
    let service = make_service().await;
    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    let output = execute(Commands::Character(CharacterCmd::List), &mut ctx).await;
    assert!(
        output.body.is_none(),
        "non-narrate commands should have body=None"
    );
}
