use novels::db::Db;
use novels::llm::{LlmCharacterDerivation, LlmContextTagSelection, MockSenseGenerator};
use novels::models::*;
use novels::prose::*;
use novels::scene::StoryService;
use novels::vocab::Vocab;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

fn sample_vocab() -> Vocab {
    Vocab::load_from_str(
        r#"
visual:
  bloodstain:
    text: "血迹"
    tags: ["crime"]
"#,
    )
    .unwrap()
}

struct RecordingProseGenerator {
    calls: AtomicUsize,
    requests: Mutex<Vec<NarrateRequest>>,
    response: LlmNarrative,
}

impl RecordingProseGenerator {
    fn new(response: LlmNarrative) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            requests: Mutex::new(vec![]),
            response,
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Default for RecordingProseGenerator {
    fn default() -> Self {
        Self::new(LlmNarrative { beats: vec![] })
    }
}

#[async_trait::async_trait]
impl ProseGenerator for RecordingProseGenerator {
    async fn narrate(&self, req: &NarrateRequest) -> Result<LlmNarrative, StoryError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.requests.lock().unwrap().push(req.clone());
        Ok(self.response.clone())
    }
}

async fn service_fixture(prose: Arc<dyn ProseGenerator>) -> StoryService {
    let db = Db::open_in_memory().await.unwrap();
    let sense = Arc::new(MockSenseGenerator::new(
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
    ));
    StoryService::new(db, sample_vocab(), sense, prose)
}

fn derivation_for(scene_id: SceneId, character_id: CharacterId) -> CharacterDerivation {
    CharacterDerivation {
        character_id,
        scene_id,
        sensations: SensorySelection::default(),
        new_memory: CharacterMemoryDraft {
            content: "mock".into(),
            source: MemorySource::Witnessed,
            certainty: Certainty::Certain,
        },
        plot_development: vec![],
    }
}

struct SceneFixture {
    service: StoryService,
    scene_id: SceneId,
    character_id: CharacterId,
    generator: Arc<RecordingProseGenerator>,
}

async fn service_with_scene() -> SceneFixture {
    let generator = Arc::new(RecordingProseGenerator::default());
    let service = service_fixture(generator.clone()).await;
    let character_id = CharacterId(Uuid::new_v4());
    service
        .db()
        .characters()
        .create(character_id, "甲", &[], &[])
        .await
        .unwrap();
    let scene_id = service
        .create_scene(CreateScene {
            objective_event: "事件".into(),
            participant_ids: vec![character_id],
            occurred_at: chrono::Utc::now(),
        })
        .await
        .unwrap();
    SceneFixture {
        service,
        scene_id,
        character_id,
        generator,
    }
}

struct PartialFixture {
    service: StoryService,
    scene_id: SceneId,
    derivations: Vec<CharacterDerivation>,
}

async fn service_with_partial_success_response() -> PartialFixture {
    let character_id = CharacterId(Uuid::new_v4());
    let cid_str = character_id.0.to_string();
    let response = LlmNarrative {
        beats: vec![
            NarrativeBeat {
                pov: cid_str.clone(),
                action: "她俯身查看。".into(),
                sensation_refs: vec!["visual.bloodstain".into()],
            },
            NarrativeBeat {
                pov: cid_str,
                action: "她继续调查。".into(),
                sensation_refs: vec!["emotion.fabricated".into()],
            },
        ],
    };
    let service = service_fixture(Arc::new(RecordingProseGenerator::new(response))).await;
    service
        .db()
        .characters()
        .create(character_id, "侦探", &[], &[])
        .await
        .unwrap();
    let scene_id = service
        .create_scene(CreateScene {
            objective_event: "古宅发现一具尸体".into(),
            participant_ids: vec![character_id],
            occurred_at: chrono::Utc::now(),
        })
        .await
        .unwrap();
    let mut sensations = SensorySelection::default();
    sensations
        .visual_ids
        .push(VocabularyId::new("visual.bloodstain").unwrap());
    PartialFixture {
        service,
        scene_id,
        derivations: vec![CharacterDerivation {
            character_id,
            scene_id,
            sensations,
            new_memory: CharacterMemoryDraft {
                content: "saw blood".into(),
                source: MemorySource::Witnessed,
                certainty: Certainty::Certain,
            },
            plot_development: vec![],
        }],
    }
}

#[tokio::test]
async fn missing_scene_fails_before_prose_generator() {
    let recorder = Arc::new(RecordingProseGenerator::default());
    let service = service_fixture(recorder.clone()).await;

    let result = service.narrate_scene(SceneId(Uuid::new_v4()), &[]).await;

    assert!(matches!(result, Err(StoryError::SceneNotFound(_))));
    assert_eq!(recorder.calls(), 0);
}

#[tokio::test]
async fn derivation_from_another_scene_fails_before_generator() {
    let fixture = service_with_scene().await;
    let wrong = derivation_for(SceneId(Uuid::new_v4()), fixture.character_id);
    let result = fixture
        .service
        .narrate_scene(fixture.scene_id, &[wrong])
        .await;

    assert!(matches!(
        result,
        Err(StoryError::InvalidNarrationContext(_))
    ));
    assert_eq!(fixture.generator.calls(), 0);
}

#[tokio::test]
async fn duplicate_character_derivations_fail_before_generator() {
    let fixture = service_with_scene().await;
    let derivation = derivation_for(fixture.scene_id, fixture.character_id);
    let result = fixture
        .service
        .narrate_scene(fixture.scene_id, &[derivation.clone(), derivation])
        .await;

    assert!(matches!(
        result,
        Err(StoryError::InvalidNarrationContext(_))
    ));
    assert_eq!(fixture.generator.calls(), 0);
}

#[tokio::test]
async fn narration_returns_source_text_and_quality_counters() {
    let fixture = service_with_partial_success_response().await;
    let prose = fixture
        .service
        .narrate_scene(fixture.scene_id, &fixture.derivations)
        .await
        .unwrap();

    assert!(prose.text.contains("血迹"));
    assert!(prose.text.contains("她俯身查看。"));
    assert_eq!(prose.stripped_refs, 1);
    assert_eq!(prose.action_only_beats, 1);
}

/// 闭环烟雾:base vocab merge 蒸馏 emotion 片段后,拼装正文解析出原著句。
#[tokio::test]
async fn assemble_resolves_distilled_emotion_text() {
    let mut vocab = Vocab::load_from_str(include_str!("../assets/vocab.yaml")).unwrap();
    vocab.merge(
        Vocab::load_from_str(
            r#"
emotion:
  hlm-c001-01:
    text: "心中无限凄凉"
    tags: ["hlm"]
"#,
        )
        .unwrap(),
    );

    let character_id = CharacterId(Uuid::new_v4());
    let cid_str = character_id.0.to_string();
    let response = LlmNarrative {
        beats: vec![NarrativeBeat {
            pov: cid_str,
            action: "她缓缓起身。".into(),
            sensation_refs: vec!["emotion.hlm-c001-01".into()],
        }],
    };
    let prose_gen = Arc::new(RecordingProseGenerator::new(response));
    let sense = Arc::new(MockSenseGenerator::new(
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
    ));
    let db = Db::open_in_memory().await.unwrap();
    let service = StoryService::new(db, vocab, sense, prose_gen);

    service
        .db()
        .characters()
        .create(character_id, "黛玉", &[], &[])
        .await
        .unwrap();
    let scene_id = service
        .create_scene(CreateScene {
            objective_event: "雨夜独坐".into(),
            participant_ids: vec![character_id],
            occurred_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    let mut sensations = SensorySelection::default();
    sensations
        .emotion_ids
        .push(VocabularyId::new("emotion.hlm-c001-01").unwrap());
    let derivation = CharacterDerivation {
        character_id,
        scene_id,
        sensations,
        new_memory: CharacterMemoryDraft {
            content: "心中凄凉".into(),
            source: MemorySource::Witnessed,
            certainty: Certainty::Certain,
        },
        plot_development: vec![],
    };

    let prose = service
        .narrate_scene(scene_id, &[derivation])
        .await
        .unwrap();

    assert!(
        prose.text.contains("心中无限凄凉"),
        "assembled text should resolve distilled emotion fragment, got: {}",
        prose.text
    );
    assert!(prose.text.contains("她缓缓起身。"));
    assert_eq!(prose.stripped_refs, 0);
}
