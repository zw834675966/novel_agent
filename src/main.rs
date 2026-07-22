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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
                Arc::new(RigSenseGenerator::new(client.clone(), vocab.clone())),
                Arc::new(RigProseGenerator::new(client)),
            ),
            Err(_) => {
                eprintln!("DEEPSEEK_API_KEY not set, using mock generators");
                (
                    Arc::new(MockSenseGenerator::new(
                        LlmContextTagSelection::default(),
                        LlmCharacterDerivation {
                            sensations: SensorySelection::default(),
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
        .create(cid, "侦探", &["谨慎".to_string()], &["推理".to_string()])
        .await?;

    // 2. 创建一个场景
    let sid = svc
        .create_scene(CreateScene {
            objective_event: "古宅发现一具尸体".into(),
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
    match svc.narrate_scene(sid, &derivations).await {
        Ok(prose) => {
            println!(
                "
=== 正文 ===
{}
=== stripped refs: {} | rejected beats: {} | action-only beats: {} | quote density: {:.2} ===",
                prose.text,
                prose.stripped_refs,
                prose.rejected_beats,
                prose.action_only_beats,
                prose.quote_density()
            );
        }
        Err(e) => eprintln!("narrate err: {e}"),
    }

    Ok(())
}
