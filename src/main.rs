use novels::db::Db;
use novels::llm::{LlmCharacterDerivation, MockSenseGenerator, RigSenseGenerator, SenseGenerator};
use novels::models::*;
use novels::scene::StoryService;
use novels::vocab::Vocab;
use rig::client::ProviderClient;
use rig::providers::deepseek;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let db = Db::open("novels.db").await?;
    let vocab = Vocab::load_from_path(std::path::Path::new("assets/vocab.yaml"))?;

    let generator: Arc<dyn SenseGenerator> = match deepseek::Client::from_env() {
        Ok(client) => Arc::new(RigSenseGenerator::new(client, vocab.clone())),
        Err(_) => {
            eprintln!("DEEPSEEK_API_KEY not set, using mock generator");
            Arc::new(MockSenseGenerator::new(LlmCharacterDerivation {
                sensations: SensorySelection::default(),
                new_memory: CharacterMemoryDraft {
                    content: "mock".into(),
                    source: MemorySource::Witnessed,
                    certainty: Certainty::Certain,
                },
                plot_development: vec![],
            }))
        }
    };

    let svc = StoryService::new(db, vocab, generator);

    let cid = CharacterId(uuid::Uuid::new_v4());
    svc.db()
        .characters()
        .create(cid, "侦探", &["谨慎".to_string()], &["推理".to_string()])
        .await?;
    let sid = svc
        .create_scene(CreateScene {
            objective_event: "古宅发现一具尸体".into(),
            participant_ids: vec![cid],
            occurred_at: chrono::Utc::now(),
        })
        .await?;

    let results = svc.derive_scene(sid).await;
    for r in results {
        match r {
            Ok(d) => println!("{:?}", d),
            Err(e) => eprintln!("err: {e}"),
        }
    }

    Ok(())
}
