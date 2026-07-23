// 真实题目：“青梅竹马商战破产多重逢爱恨情仇”配置模型全链路体验与质量报告生成
// =================================================================================

use chrono::{Duration, Utc};
use novels::{
    db::Db,
    llm::{
        LlmCharacterDerivation, LlmContextTagSelection, MockSenseGenerator, RigSenseGenerator,
        SenseGenerator,
    },
    models::{
        Certainty, CharacterId, CharacterMemoryDraft, CreateScene, MemorySource, PlotDevelopment,
        PlotDevelopmentKind, PlotReasonSlot, SensorySelection, VocabularyId,
    },
    prose::{
        MockProseGenerator, MockScenePlanner, ProseGenerator, RigProseGenerator, RigScenePlanner,
        ScenePlanner,
    },
    scene::StoryService,
};
use rig::client::ProviderClient;
use rig::providers::deepseek;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    // 1. 初始化 SQLite 内存/磁盘数据库与运行时蒸馏词库（全量 1.2 万条原著片段）
    let db = Db::open_in_memory().await?;
    let distilled_path = std::path::Path::new("assets/distilled");
    let distilled_dir = distilled_path.is_dir().then_some(distilled_path);
    let (vocab, report) = novels::vocab::load_runtime_vocab(
        std::path::Path::new("assets/vocab.yaml"),
        distilled_dir,
    )?;
    println!(
        "=== [全链路初始化] ===\n加载词库条目：Base={} Distilled_Files={} Total={}\n",
        report.base_entries, report.distilled_files, report.total_entries
    );

    // 2. 使用项目配置模型：DeepSeek Flash / 缺失 API Key 时备用安全 Mock
    let (sense_generator, prose_generator, scene_planner): (
        Arc<dyn SenseGenerator>,
        Arc<dyn ProseGenerator>,
        Arc<dyn ScenePlanner>,
    ) = match deepseek::Client::from_env() {
        Ok(client) => {
            println!(
                "=== [模型配置] ===\n已启用生产模型 provider: rig::providers::deepseek (deepseek-chat v4 flash)"
            );
            (
                Arc::new(RigSenseGenerator::new(client.clone(), vocab.clone())),
                Arc::new(RigProseGenerator::new(client.clone())),
                Arc::new(RigScenePlanner::new(client)),
            )
        }
        Err(_) => {
            println!("=== [模型配置] ===\nDEEPSEEK_API_KEY 未设置，使用零配置安全 Mock 生成器演示");
            (
                Arc::new(MockSenseGenerator::new(
                    LlmContextTagSelection {
                        tags: vec!["hlm".into(), "zhz".into(), "悲伤".into(), "冷漠".into()],
                    },
                    LlmCharacterDerivation {
                        sensory_analysis: String::new(),
                        sensations: SensorySelection {
                            visual_ids: vec![VocabularyId::new("visual.bloodstain").unwrap()],
                            auditory_ids: vec![],
                            olfactory_ids: vec![],
                            tactile_ids: vec![],
                            gustatory_ids: vec![],
                            emotion_ids: vec![VocabularyId::new("emotion.hlm-c001-01").unwrap()],
                            gesture_ids: vec![VocabularyId::new("gesture.hlm-c001-02").unwrap()],
                            atmosphere_ids: vec![
                                VocabularyId::new("atmosphere.hlm-c001-06").unwrap(),
                            ],
                        },
                        new_memory: CharacterMemoryDraft {
                            content:
                                "慈善拍卖会上与陆沉正面交锋，十年前家道破产落魄离去的画面历历在目"
                                    .into(),
                            source: MemorySource::Witnessed,
                            certainty: Certainty::Certain,
                        },
                        plot_development: vec![PlotDevelopment {
                            kind: PlotDevelopmentKind::ConflictEscalated,
                            reason: PlotReasonSlot::Observed("对方眼底藏纳的愧意".into()),
                        }],
                        relationship_candidates: vec![],
                    },
                )),
                Arc::new(MockProseGenerator::fallback()),
                Arc::new(MockScenePlanner::fallback()),
            )
        }
    };

    let service = StoryService::new(
        db.clone(),
        vocab,
        sense_generator,
        prose_generator,
        scene_planner,
    );

    // 3. 建模角色与场景
    let lu_chen_id = CharacterId(uuid::Uuid::new_v4());
    let jiang_ning_id = CharacterId(uuid::Uuid::new_v4());

    db.characters()
        .create(
            lu_chen_id,
            "陆沉",
            &["冷峻离群".into(), "陆氏财阀继承人".into()],
            &["资本控制".into()],
        )
        .await?;

    db.characters()
        .create(
            jiang_ning_id,
            "姜宁",
            &["清冷坚韧".into(), "新锐顶尖投资人".into()],
            &["恶意并购反制".into()],
        )
        .await?;

    // 场景 1：十年前破产清算
    let past_scene_id = service
        .create_scene(CreateScene {
            objective_event:
                "十年前两家商战，陆氏集团恶性打压竞争导致姜家破产清算，姜宁随父搬迁乡下".into(),
            participant_ids: vec![lu_chen_id, jiang_ning_id],
            occurred_at: Utc::now() - Duration::days(3650),
        })
        .await?;
    println!("--> 正在生成场景1推导（十年前商战破产）...");
    let past_results = service.derive_scene(past_scene_id).await;
    for (idx, res) in past_results.iter().enumerate() {
        match res {
            Ok(_) => println!("    [场景1 角色{}] 成功推导感官与记忆", idx + 1),
            Err(e) => println!("    [场景1 角色{}] 推导异常: {}", idx + 1, e),
        }
    }

    // 场景 2：多年后慈善拍卖会崛起重逢
    let present_scene_id = service
        .create_scene(CreateScene {
            objective_event:
                "多年后顶级慈善拍卖会上，姜宁以神秘资方掌门人身份天价举牌，与陆沉隔空重逢".into(),
            participant_ids: vec![lu_chen_id, jiang_ning_id],
            occurred_at: Utc::now(),
        })
        .await?;
    println!("--> 正在生成场景2推导（十年后拍卖会重逢）...");
    let present_results = service.derive_scene(present_scene_id).await;
    let mut present_derivations = Vec::new();
    for (idx, res) in present_results.into_iter().enumerate() {
        match res {
            Ok(d) => {
                println!(
                    "    [场景2 角色{}] 成功推导，剧情原因槽: {}",
                    idx + 1,
                    d.plot_development
                        .first()
                        .map(|p| p.reason.render())
                        .unwrap_or_default()
                );
                present_derivations.push(d);
            }
            Err(e) => println!("    [场景2 角色{}] 推导异常: {}", idx + 1, e),
        }
    }

    println!("--> 正在拼装叙事正文...");
    let assembled = service
        .narrate_scene(present_scene_id, &present_derivations)
        .await?;

    println!("=== [输出正文片段] ===");
    println!("{}\n", assembled.text);

    println!("=== [质量分析指标 (Prose Quality Report)] ===");
    println!("* 物理字数 (Total Chars): {}", assembled.total_chars);
    println!("* 独占硬引用字数 (Quote Chars): {}", assembled.quote_chars);
    println!(
        "* 引用密度 (Quote Density): {:.2}%",
        assembled.quality.quote_density * 100.0
    );
    println!(
        "* 纯动作段落占比 (Action-Only Rate): {:.2}%",
        assembled.quality.action_only_rate * 100.0
    );
    println!(
        "* 低密度警告标志 (Low Quote Density Flag): {}",
        assembled.quality.low_quote_density
    );
    println!(
        "* 未回源引用数 (Unverified Quotes): {}",
        assembled.unverified_quotes
    );

    Ok(())
}
