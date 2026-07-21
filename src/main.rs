// novels 应用入口
// ==================
// 启动流程：
//   1. 从环境变量加载 DEEPSEEK_API_KEY
//   2. 初始化 SQLite 数据库（novels.db）
//   3. 加载感官词库（assets/vocab.yaml）
//   4. 初始化 LLM 生成器（无 API Key 时降级为 Mock）
//   5. 创建 StoryService
//   6. 创建角色 + 场景 + 推导演示

use novels::db::Db;
use novels::llm::{
    LlmCharacterDerivation, LlmContextTagSelection, MockSenseGenerator, RigSenseGenerator,
    SenseGenerator,
};
use novels::models::*;
use novels::scene::StoryService;
use novels::vocab::Vocab;
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
    let vocab = Vocab::load_from_path(std::path::Path::new("assets/vocab.yaml"))?;

    // 初始化 LLM 生成器
    // rig::providers::deepseek::Client::from_env() 从 DEEPSEEK_API_KEY 环境变量创建客户端
    // 如果环境变量未设置，使用 Mock 生成器（返回固定数据，不调用外部 API）
    let generator: Arc<dyn SenseGenerator> = match deepseek::Client::from_env() {
        Ok(client) => Arc::new(RigSenseGenerator::new(client, vocab.clone())),
        Err(_) => {
            eprintln!("DEEPSEEK_API_KEY not set, using mock generator");
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
            ))
        }
    };

    // 创建故事服务
    let svc = StoryService::new(db, vocab, generator);

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
    for r in results {
        match r {
            Ok(d) => println!("{:?}", d),
            Err(e) => eprintln!("err: {e}"),
        }
    }

    Ok(())
}
