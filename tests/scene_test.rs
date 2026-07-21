// 场景服务单元测试
// ==================
// 测试 StoryService 的核心逻辑：
//   - derive_character：正常推导 + 持久化
//   - derive_character：非参与者校验
//   - derive_scene：部分角色失败时的容错

use novels::db::Db;
use novels::llm::{
    ContextTagRequest, DerivationRequest, LlmCharacterDerivation, LlmContextTagSelection,
    MockSenseGenerator, SenseGenerator,
};
use novels::models::*;
use novels::scene::StoryService;
use novels::vocab::Vocab;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

struct SequenceGenerator {
    responses: Mutex<VecDeque<LlmCharacterDerivation>>,
}

struct OneFailureGenerator {
    failing_character: CharacterId,
}

/// Test-only generator that records every `ContextTagRequest` and
/// `DerivationRequest` it receives, then returns a fixed tag selection
/// followed by a fixed derivation. Used to assert that the semantic
/// vocabulary candidate metadata flows unchanged into the LLM prompt.
struct RecordingGenerator {
    context_tag_requests: Mutex<Vec<ContextTagRequest>>,
    derivation_requests: Mutex<Vec<DerivationRequest>>,
    tag_responses: Mutex<VecDeque<LlmContextTagSelection>>,
    derivation_responses: Mutex<VecDeque<LlmCharacterDerivation>>,
}

impl RecordingGenerator {
    fn new(
        tag_response: LlmContextTagSelection,
        derivation_responses: Vec<LlmCharacterDerivation>,
    ) -> Self {
        let tag_response_count = derivation_responses.len().max(1);
        Self {
            context_tag_requests: Mutex::new(vec![]),
            derivation_requests: Mutex::new(vec![]),
            tag_responses: Mutex::new(
                std::iter::repeat_n(tag_response, tag_response_count).collect(),
            ),
            derivation_responses: Mutex::new(derivation_responses.into()),
        }
    }

    async fn context_tag_requests(&self) -> Vec<ContextTagRequest> {
        self.context_tag_requests.lock().await.clone()
    }

    async fn derivation_requests(&self) -> Vec<DerivationRequest> {
        self.derivation_requests.lock().await.clone()
    }
}

impl SequenceGenerator {
    fn new(responses: Vec<LlmCharacterDerivation>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
        }
    }
}

#[async_trait::async_trait]
impl SenseGenerator for OneFailureGenerator {
    async fn derive(&self, req: &DerivationRequest) -> Result<LlmCharacterDerivation, StoryError> {
        if req.character.id == self.failing_character {
            Err(StoryError::Llm("configured participant failure".into()))
        } else {
            Ok(canned_derivation())
        }
    }

    async fn select_context_tags(
        &self,
        _req: &ContextTagRequest,
    ) -> Result<LlmContextTagSelection, StoryError> {
        Ok(LlmContextTagSelection::default())
    }
}

#[async_trait::async_trait]
impl SenseGenerator for SequenceGenerator {
    async fn derive(&self, _req: &DerivationRequest) -> Result<LlmCharacterDerivation, StoryError> {
        self.responses
            .lock()
            .await
            .pop_front()
            .ok_or_else(|| StoryError::Llm("no response configured".into()))
    }

    async fn select_context_tags(
        &self,
        _req: &ContextTagRequest,
    ) -> Result<LlmContextTagSelection, StoryError> {
        Ok(LlmContextTagSelection::default())
    }
}

#[async_trait::async_trait]
impl SenseGenerator for RecordingGenerator {
    async fn derive(&self, req: &DerivationRequest) -> Result<LlmCharacterDerivation, StoryError> {
        self.derivation_requests.lock().await.push(req.clone());
        self.derivation_responses
            .lock()
            .await
            .pop_front()
            .ok_or_else(|| StoryError::Llm("no derivation response configured".into()))
    }

    async fn select_context_tags(
        &self,
        req: &ContextTagRequest,
    ) -> Result<LlmContextTagSelection, StoryError> {
        self.context_tag_requests.lock().await.push(req.clone());
        self.tag_responses
            .lock()
            .await
            .pop_front()
            .ok_or_else(|| StoryError::Llm("no context-tag response configured".into()))
    }
}

fn canned_derivation() -> LlmCharacterDerivation {
    LlmCharacterDerivation {
        sensations: SensorySelection {
            visual_ids: vec![VocabularyId::new("visual.x").unwrap()],
            ..Default::default()
        },
        new_memory: CharacterMemoryDraft {
            content: "witnessed".into(),
            source: MemorySource::Witnessed,
            certainty: Certainty::Certain,
        },
        plot_development: vec![],
    }
}

#[tokio::test]
async fn derive_character_persists_and_returns() {
    // 正常流程：推导结果同时以返回值返回并持久化到数据库
    let db = Db::open_in_memory().await.unwrap();
    let yaml = "visual:\n  x:\n    text: x\n    tags: []\n";
    let vocab = Vocab::load_from_str(yaml).unwrap();
    let generator = Arc::new(MockSenseGenerator::new(
        LlmContextTagSelection::default(),
        canned_derivation(),
    ));
    let svc = StoryService::new(db.clone(), vocab, generator);

    let cid = CharacterId(uuid::Uuid::new_v4());
    let sid = SceneId(uuid::Uuid::new_v4());
    db.characters().create(cid, "A", &[], &[]).await.unwrap();
    db.scenes()
        .create(sid, "事件X", &[cid], chrono::Utc::now())
        .await
        .unwrap();

    let r = svc.derive_character(sid, cid).await;
    assert!(r.is_ok(), "{:?}", r.err());
    let d = r.unwrap();
    assert_eq!(d.character_id, cid);
    assert_eq!(d.scene_id, sid);
    let mems = db.memories().list(cid, 50).await.unwrap();
    assert_eq!(mems.len(), 1);
    assert_eq!(mems[0].content, "witnessed");
    let latest = db.sensations().latest(cid).await.unwrap();
    assert!(latest.is_some());
}

#[tokio::test]
async fn derive_character_rejects_non_participant() {
    // 非参与者推导应返回 NotSceneParticipant 错误
    let db = Db::open_in_memory().await.unwrap();
    let vocab = Vocab::load_from_str("visual:\n  x:\n    text: x\n    tags: []\n").unwrap();
    let generator = Arc::new(MockSenseGenerator::new(
        LlmContextTagSelection::default(),
        canned_derivation(),
    ));
    let svc = StoryService::new(db.clone(), vocab, generator);
    let cid = CharacterId(uuid::Uuid::new_v4());
    let sid = SceneId(uuid::Uuid::new_v4());
    db.characters().create(cid, "A", &[], &[]).await.unwrap();
    db.scenes()
        .create(sid, "e", &[cid], chrono::Utc::now())
        .await
        .unwrap();
    let other = CharacterId(uuid::Uuid::new_v4());
    db.characters().create(other, "B", &[], &[]).await.unwrap();
    let err = svc.derive_character(sid, other).await.unwrap_err();
    assert!(matches!(err, StoryError::NotSceneParticipant(_, _)));
}

#[tokio::test]
async fn derive_scene_returns_partial_on_one_failure() {
    // 批量推导时个别角色失败不影响其他角色的结果
    let db = Db::open_in_memory().await.unwrap();
    let vocab = Vocab::load_from_str("visual:\n  x:\n    text: x\n    tags: []\n").unwrap();
    let c1 = CharacterId(uuid::Uuid::new_v4());
    let c2 = CharacterId(uuid::Uuid::new_v4());
    let generator = Arc::new(OneFailureGenerator {
        failing_character: c1,
    });
    let svc = StoryService::new(db.clone(), vocab, generator);
    let s = SceneId(uuid::Uuid::new_v4());
    db.characters().create(c1, "A", &[], &[]).await.unwrap();
    db.characters().create(c2, "B", &[], &[]).await.unwrap();
    db.scenes()
        .create(s, "e", &[c1, c2], chrono::Utc::now())
        .await
        .unwrap();
    let results = svc.derive_scene(s).await;
    assert_eq!(results.len(), 2);
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
    assert!(
        results
            .iter()
            .any(|result| { matches!(result, Ok(derivation) if derivation.character_id == c2) })
    );
    assert!(results
        .iter()
        .any(|result| matches!(result, Err(StoryError::Llm(message)) if message == "configured participant failure")));
}

#[tokio::test]
async fn retry_persists_complete_second_response() {
    let db = Db::open_in_memory().await.unwrap();
    let vocab = Vocab::load_from_str("visual:\n  x:\n    text: x\n    tags: []\n").unwrap();
    let first = LlmCharacterDerivation {
        sensations: SensorySelection {
            auditory_ids: vec![VocabularyId::new("auditory.x").unwrap()],
            ..Default::default()
        },
        new_memory: CharacterMemoryDraft {
            content: "first memory".into(),
            source: MemorySource::Witnessed,
            certainty: Certainty::Certain,
        },
        plot_development: vec![PlotDevelopment {
            kind: PlotDevelopmentKind::SuspicionRaised,
            reason: "first plot".into(),
        }],
    };
    let second = LlmCharacterDerivation {
        sensations: SensorySelection {
            visual_ids: vec![VocabularyId::new("visual.x").unwrap()],
            ..Default::default()
        },
        new_memory: CharacterMemoryDraft {
            content: "second memory".into(),
            source: MemorySource::Inferred,
            certainty: Certainty::Suspected,
        },
        plot_development: vec![PlotDevelopment {
            kind: PlotDevelopmentKind::NewClue,
            reason: "second plot".into(),
        }],
    };
    let generator = Arc::new(SequenceGenerator::new(vec![first, second]));
    let svc = StoryService::new(db.clone(), vocab, generator);
    let cid = CharacterId(uuid::Uuid::new_v4());
    let sid = SceneId(uuid::Uuid::new_v4());
    db.characters().create(cid, "A", &[], &[]).await.unwrap();
    db.scenes()
        .create(sid, "e", &[cid], chrono::Utc::now())
        .await
        .unwrap();

    let result = svc.derive_character(sid, cid).await.unwrap();
    assert_eq!(result.sensations.visual_ids[0].as_str(), "visual.x");
    assert_eq!(result.new_memory.content, "second memory");
    assert_eq!(result.new_memory.source, MemorySource::Inferred);
    assert_eq!(result.new_memory.certainty, Certainty::Suspected);
    assert_eq!(result.plot_development[0].reason, "second plot");

    let memories = db.memories().list(cid, 50).await.unwrap();
    assert_eq!(memories.len(), 1);
    assert_eq!(memories[0].content, "second memory");
    assert_eq!(memories[0].source, MemorySource::Inferred);
    assert_eq!(memories[0].certainty, Certainty::Suspected);
    let latest = db.sensations().latest(cid).await.unwrap().unwrap();
    assert_eq!(latest.0.visual_ids[0].as_str(), "visual.x");
}

#[tokio::test]
async fn retry_rejects_two_invalid_responses_without_persisting() {
    let db = Db::open_in_memory().await.unwrap();
    let vocab = Vocab::load_from_str("visual:\n  x:\n    text: x\n    tags: []\n").unwrap();
    let invalid = || LlmCharacterDerivation {
        sensations: SensorySelection {
            auditory_ids: vec![VocabularyId::new("auditory.x").unwrap()],
            ..Default::default()
        },
        new_memory: CharacterMemoryDraft {
            content: "should not persist".into(),
            source: MemorySource::Witnessed,
            certainty: Certainty::Certain,
        },
        plot_development: vec![],
    };
    let generator = Arc::new(SequenceGenerator::new(vec![invalid(), invalid()]));
    let svc = StoryService::new(db.clone(), vocab, generator);
    let cid = CharacterId(uuid::Uuid::new_v4());
    let sid = SceneId(uuid::Uuid::new_v4());
    db.characters().create(cid, "A", &[], &[]).await.unwrap();
    db.scenes()
        .create(sid, "e", &[cid], chrono::Utc::now())
        .await
        .unwrap();

    let error = svc.derive_character(sid, cid).await.unwrap_err();
    assert!(matches!(error, StoryError::InvalidVocabularySelection(_)));
    assert!(db.memories().list(cid, 50).await.unwrap().is_empty());
    assert!(db.sensations().latest(cid).await.unwrap().is_none());
}

#[tokio::test]
async fn derivation_passes_semantic_vocabulary_candidates() {
    // The derivation request that reaches the LLM must carry full semantic
    // candidate metadata (id / sense / text / tags) so the model can reason
    // over text rather than opaque IDs. It also asserts that the available
    // tag list supplied by the service reaches `select_context_tags`.
    let db = Db::open_in_memory().await.unwrap();
    let yaml = "visual:\n  bloodstain:\n    text: 血迹\n    tags: [\"injury\"]\n";
    let vocab = Vocab::load_from_str(yaml).unwrap();

    let derivation = LlmCharacterDerivation {
        sensations: SensorySelection {
            visual_ids: vec![VocabularyId::new("visual.bloodstain").unwrap()],
            ..Default::default()
        },
        new_memory: CharacterMemoryDraft {
            content: "saw blood".into(),
            source: MemorySource::Witnessed,
            certainty: Certainty::Certain,
        },
        plot_development: vec![],
    };
    let tag_selection = LlmContextTagSelection {
        tags: vec!["injury".into()],
    };
    let generator = Arc::new(RecordingGenerator::new(tag_selection, vec![derivation]));
    let svc = StoryService::new(db.clone(), vocab, generator.clone());

    let cid = CharacterId(uuid::Uuid::new_v4());
    let sid = SceneId(uuid::Uuid::new_v4());
    db.characters()
        .create(cid, "侦探", &["谨慎".to_string()], &["推理".to_string()])
        .await
        .unwrap();
    db.scenes()
        .create(sid, "古宅发现一具尸体", &[cid], chrono::Utc::now())
        .await
        .unwrap();

    let result = svc.derive_character(sid, cid).await.unwrap();
    assert_eq!(result.character_id, cid);
    assert_eq!(result.scene_id, sid);

    let tag_request = generator
        .context_tag_requests()
        .await
        .pop()
        .expect("select_context_tags was not invoked");
    assert_eq!(
        tag_request.available_tags,
        vec!["injury".to_string()],
        "available_tags must include every known vocab tag in stable order"
    );

    let request = generator
        .derivation_requests()
        .await
        .pop()
        .expect("derive was not invoked");
    assert!(!request.candidates.is_empty());
    assert_eq!(request.candidates[0].id, "visual.bloodstain");
    assert_eq!(request.candidates[0].sense, "visual");
    assert_eq!(request.candidates[0].text, "血迹");
    assert_eq!(request.candidates[0].tags, vec!["injury"]);
}

fn derivation(memory: &str, plot: &str) -> LlmCharacterDerivation {
    LlmCharacterDerivation {
        sensations: SensorySelection {
            visual_ids: vec![VocabularyId::new("visual.bloodstain").unwrap()],
            ..Default::default()
        },
        new_memory: CharacterMemoryDraft {
            content: memory.into(),
            source: MemorySource::Witnessed,
            certainty: Certainty::Certain,
        },
        plot_development: vec![PlotDevelopment {
            kind: PlotDevelopmentKind::NewClue,
            reason: plot.into(),
        }],
    }
}

async fn narrative_service_fixture(
    tag_selection: LlmContextTagSelection,
    derivations: Vec<LlmCharacterDerivation>,
) -> (
    Db,
    StoryService,
    Arc<RecordingGenerator>,
    CharacterId,
    SceneId,
    SceneId,
) {
    let db = Db::open_in_memory().await.unwrap();
    let vocab = Vocab::load_from_str(
        "visual:\n  bloodstain:\n    text: bloodstain\n    tags: [injury]\nauditory:\n  footsteps:\n    text: footsteps\n    tags: [movement]\n",
    )
    .unwrap();
    let generator = Arc::new(RecordingGenerator::new(tag_selection, derivations));
    let service = StoryService::new(db.clone(), vocab, generator.clone());
    let cid = CharacterId(uuid::Uuid::new_v4());
    db.characters().create(cid, "A", &[], &[]).await.unwrap();

    let early = service
        .create_scene(CreateScene {
            objective_event: "early".into(),
            participant_ids: vec![cid],
            occurred_at: chrono::Utc::now(),
        })
        .await
        .unwrap();
    let later = service
        .create_scene(CreateScene {
            objective_event: "later".into(),
            participant_ids: vec![cid],
            occurred_at: chrono::Utc::now() + chrono::Duration::minutes(1),
        })
        .await
        .unwrap();

    (db, service, generator, cid, early, later)
}

#[tokio::test]
async fn derive_character_uses_earlier_plot_and_replaces_same_scene() {
    let (db, service, generator, cid, early, later) = narrative_service_fixture(
        LlmContextTagSelection {
            tags: vec!["injury".into()],
        },
        vec![
            derivation("early memory", "early plot"),
            derivation("first later", "first plot"),
            derivation("second later", "second plot"),
        ],
    )
    .await;
    service.derive_character(early, cid).await.unwrap();
    service.derive_character(later, cid).await.unwrap();
    service.derive_character(later, cid).await.unwrap();

    let requests = generator.derivation_requests().await;
    assert_eq!(
        requests[1].prior_plot_developments[0].development.reason,
        "early plot"
    );
    assert_eq!(
        db.memories().list(cid, 50).await.unwrap()[0].content,
        "second later"
    );
}

#[tokio::test]
async fn empty_selected_tags_use_all_vocabulary_candidates() {
    let (_db, service, generator, cid, early, _later) = narrative_service_fixture(
        LlmContextTagSelection::default(),
        vec![derivation("memory", "plot")],
    )
    .await;
    service.derive_character(early, cid).await.unwrap();

    let request = generator.derivation_requests().await.pop().unwrap();
    assert_eq!(request.candidates.len(), 2);
    assert!(
        request
            .candidates
            .iter()
            .any(|candidate| candidate.id == "visual.bloodstain")
    );
    assert!(
        request
            .candidates
            .iter()
            .any(|candidate| candidate.id == "auditory.footsteps")
    );
}

#[tokio::test]
async fn later_derivation_never_receives_future_scene_context() {
    let (db, service, generator, cid, early, later) = narrative_service_fixture(
        LlmContextTagSelection::default(),
        vec![
            derivation("future memory", "future plot"),
            derivation("early memory", "early plot"),
        ],
    )
    .await;
    service.derive_character(later, cid).await.unwrap();
    service.derive_character(early, cid).await.unwrap();

    let request = generator.derivation_requests().await.pop().unwrap();
    assert!(request.recent_memories.is_empty());
    assert!(request.last_sensation.is_none());
    assert!(request.prior_plot_developments.is_empty());
    assert_eq!(
        db.memories().list(cid, 50).await.unwrap()[0].content,
        "early memory"
    );
}
