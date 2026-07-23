// CLI 命令执行器集成测试
// ========================
// 覆盖 Task 5 的非 LLM 命令：character create/list/show, scene create/list,
// use scene/character/clear, context, 以及 derive/repl 桩。
// 全部使用内存 SQLite + Mock 生成器，不触碰真实 LLM。

use novels::cli::{
    CharacterCmd, CommandContext, Commands, ProseCache, SceneCmd, Session, Status, UseCmd, execute,
};
use novels::db::Db;
use novels::llm::{LlmCharacterDerivation, LlmContextTagSelection, MockSenseGenerator};
use novels::models::*;
use novels::prose::MockProseGenerator;
use novels::scene::StoryService;
use novels::vocab::Vocab;
use std::sync::Arc;

/// 构造一个带 Mock 生成器的 StoryService（内存 DB）。
async fn make_service() -> StoryService {
    let db = Db::open_in_memory().await.unwrap();
    let yaml = "visual:\n  x:\n    text: x\n    tags: []\n";
    let vocab = Vocab::load_from_str(yaml).unwrap();
    let canned = LlmCharacterDerivation {
        sensations: SensorySelection::default(),
        new_memory: CharacterMemoryDraft {
            content: "memory".into(),
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

    let obs = execute(
        Commands::Character(CharacterCmd::Create {
            name: "宝玉".into(),
            tags: Some("痴情,贵公子".into()),
            skills: Some("诗词".into()),
        }),
        &mut ctx,
    )
    .await;

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
    let list_obs = execute(Commands::Character(CharacterCmd::List), &mut ctx).await;
    assert_eq!(list_obs.status, Status::Success);
    assert!(
        list_obs.summary.contains("1"),
        "count: {}",
        list_obs.summary
    );
    let chars = list_obs
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
        let obs = execute(
            Commands::Character(CharacterCmd::Create {
                name: name.into(),
                tags: None,
                skills: None,
            }),
            &mut ctx,
        )
        .await;
        assert_eq!(obs.status, Status::Success);
    }

    let list_obs = execute(Commands::Character(CharacterCmd::List), &mut ctx).await;
    assert_eq!(list_obs.status, Status::Success);
    assert!(
        list_obs.summary.contains("2"),
        "expected 2 chars, got: {}",
        list_obs.summary
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

    let obs = execute(
        Commands::Scene(SceneCmd::Create {
            event: "宝玉挨打".into(),
            with: "宝玉".into(),
        }),
        &mut ctx,
    )
    .await;

    assert_eq!(obs.status, Status::Success, "summary: {}", obs.summary);
    assert_eq!(obs.summary, "场景已创建");
    assert!(obs.artifacts.contains_key("scene_id"));

    // 场景创建成功后应自动设置 session.current_scene
    assert!(
        ctx.session.current_scene.is_some(),
        "current_scene should be set after scene create"
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

    let obs = execute(
        Commands::Scene(SceneCmd::Create {
            event: "共读西厢".into(),
            with: "宝玉,黛玉".into(),
        }),
        &mut ctx,
    )
    .await;

    assert_eq!(obs.status, Status::Success, "summary: {}", obs.summary);
    let scene_id_str = obs
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

    let obs = execute(
        Commands::Use(UseCmd::Scene {
            id_or_name: sid.0.to_string(),
        }),
        &mut ctx,
    )
    .await;

    assert_eq!(obs.status, Status::Success);
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

    let obs = execute(Commands::Use(UseCmd::Clear), &mut ctx).await;
    assert_eq!(obs.status, Status::Success);

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

    let obs = execute(Commands::Context, &mut ctx).await;
    assert_eq!(obs.status, Status::Success);
    assert!(
        obs.summary.contains("无"),
        "empty context summary should mention 无, got: {}",
        obs.summary
    );
}

#[tokio::test]
async fn derive_stub_returns_error() {
    let service = make_service().await;
    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    let obs = execute(
        Commands::Derive {
            scene: None,
            character: None,
        },
        &mut ctx,
    )
    .await;

    assert_eq!(
        obs.status,
        Status::Error,
        "derive should be stubbed: {}",
        obs.summary
    );
}

#[tokio::test]
async fn character_show_via_execute() {
    let service = make_service().await;
    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    let create_obs = execute(
        Commands::Character(CharacterCmd::Create {
            name: "宝玉".into(),
            tags: Some("痴情".into()),
            skills: Some("诗词".into()),
        }),
        &mut ctx,
    )
    .await;
    assert_eq!(create_obs.status, Status::Success);

    let show_obs = execute(
        Commands::Character(CharacterCmd::Show {
            id_or_name: "宝玉".into(),
        }),
        &mut ctx,
    )
    .await;

    assert_eq!(
        show_obs.status,
        Status::Success,
        "summary: {}",
        show_obs.summary
    );
    assert_eq!(
        show_obs.artifacts.get("name").map(|s| s.as_str()),
        Some("宝玉")
    );
    assert!(show_obs.artifacts.contains_key("id"));
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

    let list_obs = execute(Commands::Scene(SceneCmd::List), &mut ctx).await;
    assert_eq!(list_obs.status, Status::Success);
    assert!(
        list_obs.summary.contains("2"),
        "expected 2 scenes, got: {}",
        list_obs.summary
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

    let obs = execute(
        Commands::Use(UseCmd::Character {
            id_or_name: "黛玉".into(),
        }),
        &mut ctx,
    )
    .await;

    assert_eq!(obs.status, Status::Success);
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

    let obs = execute(Commands::Context, &mut ctx).await;
    assert_eq!(obs.status, Status::Success);
    assert!(
        !obs.summary.contains("无"),
        "populated context should not say 无, got: {}",
        obs.summary
    );
    assert_eq!(
        obs.artifacts.get("current_scene").map(|s| s.as_str()),
        Some(sid.0.to_string()).as_deref()
    );
    assert_eq!(
        obs.artifacts.get("current_character").map(|s| s.as_str()),
        Some(cid.0.to_string()).as_deref()
    );
}

#[tokio::test]
async fn repl_returns_error() {
    let service = make_service().await;
    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();
    let mut ctx = make_ctx(&service, &mut session, &mut prose_cache);

    let obs = execute(Commands::Repl, &mut ctx).await;
    assert_eq!(obs.status, Status::Error);
}
