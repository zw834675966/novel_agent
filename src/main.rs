// novels 应用入口
// ==================
// 启动流程：
//   1. 从环境变量加载 DEEPSEEK_API_KEY
//   2. 初始化 SQLite 数据库（novels.db）
//   3. 加载感官词库（assets/vocab.yaml）
//   4. 初始化 LLM 生成器（无 API Key 时降级为 Mock）
//   5. 创建 StoryService
//   6. 创建角色 + 场景 + 推导 + 叙事编排演示

use novels::db::Db;
use novels::llm::{
    LlmCharacterDerivation, LlmContextTagSelection, MockSenseGenerator, RigSenseGenerator,
    SenseGenerator,
};
use novels::models::*;
use novels::prose::{MockProseGenerator, ProseGenerator, RigProseGenerator};
use novels::scene::StoryService;
use rig::client::ProviderClient;
use rig::providers::deepseek;
use std::sync::Arc;

/// 演示 CLI 参数（clap derive）
/// ============================
/// 允许在不重新编译的前提下调整演示角色/事件/性格/技能，并支持跳过叙事编排。
#[derive(clap::Parser, Debug)]
#[command(
    version,
    about = "novels - AI-driven novel character perception engine demo"
)]
struct Cli {
    /// 演示角色姓名
    #[arg(long, default_value = "侦探")]
    character: String,

    /// 场景客观事件
    #[arg(long, default_value = "古宅发现一具尸体")]
    event: String,

    /// 角色性格标签（可多次传入覆盖默认）
    #[arg(long, default_value = "谨慎")]
    personality: Vec<String>,

    /// 角色技能标签（可多次传入覆盖默认）
    #[arg(long, default_value = "推理")]
    skill: Vec<String>,

    /// 跳过叙事编排（仅执行 derive）
    #[arg(long, default_value_t = false)]
    skip_narrate: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 解析 CLI 参数
    let cli = <Cli as clap::Parser>::parse();

    // 加载 .env 文件（如果存在）
    // 可以在此文件中设置 DEEPSEEK_API_KEY 等环境变量
    dotenv::dotenv().ok();

    // 初始化底层依赖
    let db = Db::open("novels.db").await?;

    // 词库：base (assets/vocab.yaml) + 可选 distilled 目录合并
    // - 默认：若 assets/distilled 存在则合并
    // - NOVELS_DISTILLED_DIR=<path>：覆盖默认 distilled 目录（须为目录）
    // - NOVELS_SKIP_DISTILLED=1：仅加载 base，跳过 distilled
    let skip_distilled = std::env::var("NOVELS_SKIP_DISTILLED")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let distilled = if skip_distilled {
        None
    } else {
        std::env::var("NOVELS_DISTILLED_DIR")
            .ok()
            .filter(|s| !s.is_empty())
            .map(std::path::PathBuf::from)
            .filter(|p| p.is_dir())
            .or_else(|| {
                let p = std::path::PathBuf::from("assets/distilled");
                p.is_dir().then_some(p)
            })
    };
    let (vocab, report) = novels::vocab::load_runtime_vocab(
        std::path::Path::new("assets/vocab.yaml"),
        distilled.as_deref(),
    )?;
    eprintln!(
        "vocab loaded: base={} distilled_files={} total={}",
        report.base_entries, report.distilled_files, report.total_entries
    );

    // 初始化 LLM 生成器
    // rig::providers::deepseek::Client::from_env() 从 DEEPSEEK_API_KEY 环境变量创建客户端
    // 如果环境变量未设置，使用 Mock 生成器（返回固定数据，不调用外部 API）
    //
    // 单次 from_env() 调用同时构造 sense + prose 两套适配器；
    // 失败分支同时降级为两套 Mock，保证不会出现"只配了一个生产适配器"的半成品状态。
    let (sense_generator, prose_generator): (Arc<dyn SenseGenerator>, Arc<dyn ProseGenerator>) =
        match deepseek::Client::from_env() {
            Ok(client) => (
                Arc::new(RigSenseGenerator::new(client.clone())),
                Arc::new(RigProseGenerator::new(client)),
            ),
            Err(_) => {
                eprintln!("DEEPSEEK_API_KEY not set, using mock generators");
                (
                    Arc::new(MockSenseGenerator::new(
                        LlmContextTagSelection::default(),
                        LlmCharacterDerivation {
                            // 注入真实 base vocab id,避免 mock 路径产生空感官,
                            // 使无 API Key 演示也能走通 derive -> narrate 全链路。
                            sensations: SensorySelection {
                                visual_ids: vec![
                                    VocabularyId::new("visual.bloodstain")
                                        .expect("visual.bloodstain is a valid base vocab id"),
                                ],
                                ..Default::default()
                            },
                            new_memory: CharacterMemoryDraft {
                                content: "mock".into(),
                                source: MemorySource::Witnessed,
                                certainty: Certainty::Certain,
                            },
                            plot_development: vec![],
                        },
                    )),
                    Arc::new(MockProseGenerator::fallback()),
                )
            }
        };

    // 创建故事服务
    let svc = StoryService::new(db, vocab, sense_generator, prose_generator);

    // ---- 演示流程 ----
    // 1. 创建一个角色
    let cid = CharacterId(uuid::Uuid::new_v4());
    svc.db()
        .characters()
        .create(cid, &cli.character, &cli.personality, &cli.skill)
        .await?;

    // 2. 创建一个场景
    let sid = svc
        .create_scene(CreateScene {
            objective_event: cli.event.clone(),
            participant_ids: vec![cid],
            occurred_at: chrono::Utc::now(),
        })
        .await?;

    // 3. 对场景中的角色进行推导
    let results = svc.derive_scene(sid).await;
    let derivations: Vec<_> = results
        .iter()
        .filter_map(|r| r.as_ref().ok().cloned())
        .collect();
    for d in &derivations {
        println!("{:?}", d);
    }

    // 4. 把结构化推导编排成小说正文
    //    LLM 只写叙事骨架,描写从红楼梦/甄嬛传素材库拉取原著片段
    if cli.skip_narrate {
        eprintln!("--skip-narrate: skipping narration");
        return Ok(());
    }
    match svc.narrate_scene(sid, &derivations).await {
        Ok(prose) => {
            println!(
                "
=== 正文 ===
{}
=== stripped refs: {} | rejected beats: {} | action-only beats: {} | quote density: {:.2} | unverified quotes: {} | low_quote_density: {} ===",
                prose.text,
                prose.stripped_refs,
                prose.rejected_beats,
                prose.action_only_beats,
                prose.quote_density(),
                prose.unverified_quotes,
                prose.quality.low_quote_density
            );
        }
        Err(e) => eprintln!("narrate err: {e}"),
    }

    Ok(())
}
