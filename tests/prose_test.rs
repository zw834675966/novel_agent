// prose 模块单元测试
// =====================
// 覆盖:拼装顺序(8 类)、非法 ref 剥离、pov 非参与者拒绝、
// 空 refs 只输出 action、空 narrative 硬错误、跨角色 ref 剥离、
// 缺失 derivation 的 pov 被拒绝、mock fallback 行为。

use novels::models::*;
use novels::prose::*;
use novels::vocab::Vocab;
use std::collections::HashSet;
use std::sync::Arc;

fn sample_vocab() -> Vocab {
    Vocab::load_from_str(
        r#"
visual:
  bloodstain:
    text: "血迹"
    tags: ["crime"]
auditory:
  footstep:
    text: "脚步声"
    tags: ["night"]
olfactory:
  bloodsmell:
    text: "血腥味"
    tags: ["crime"]
tactile:
  coldhand:
    text: "冰凉的手"
    tags: ["night"]
gustatory:
  bitter:
    text: "苦涩"
    tags: ["sadness"]
atmosphere:
  coldnight:
    text: "夜凉如水"
    tags: ["night"]
emotion:
  sorrow:
    text: "心中未免悔恨"
    tags: ["sorrow"]
gesture:
  weep:
    text: "她伸手把帕子绞了又绞"
    tags: ["grief"]
"#,
    )
    .unwrap()
}

fn make_character(id: &str, name: &str) -> Character {
    Character {
        id: CharacterId(uuid::Uuid::parse_str(id).unwrap()),
        name: name.into(),
        personality: vec![],
        skills: vec![],
    }
}

fn make_derivation(cid: &str, vid_strs: &[&str]) -> CharacterDerivation {
    let mut sel = SensorySelection::default();
    for s in vid_strs {
        let vid = VocabularyId::new(s).unwrap();
        match vid.sense() {
            "visual" => sel.visual_ids.push(vid),
            "auditory" => sel.auditory_ids.push(vid),
            "olfactory" => sel.olfactory_ids.push(vid),
            "tactile" => sel.tactile_ids.push(vid),
            "gustatory" => sel.gustatory_ids.push(vid),
            "emotion" => sel.emotion_ids.push(vid),
            "gesture" => sel.gesture_ids.push(vid),
            "atmosphere" => sel.atmosphere_ids.push(vid),
            _ => {}
        }
    }
    CharacterDerivation {
        character_id: CharacterId(uuid::Uuid::parse_str(cid).unwrap()),
        scene_id: SceneId(uuid::Uuid::new_v4()),
        sensations: sel,
        new_memory: CharacterMemoryDraft {
            content: "mock".into(),
            source: MemorySource::Witnessed,
            certainty: Certainty::Certain,
        },
        plot_development: vec![],
    }
}

fn make_candidates(cid: &str, vid_strs: &[&str], vocab: &Vocab) -> CharacterProseCandidates {
    let candidates: Vec<ProseCandidate> = vid_strs
        .iter()
        .filter_map(|raw| {
            let vid = VocabularyId::new(raw).ok()?;
            let entry = vocab.entries(vid.sense())?.get(vid.key())?;
            Some(ProseCandidate {
                id: raw.to_string(),
                sense: vid.sense().to_string(),
                text: entry.text.clone(),
                tags: entry.tags.clone(),
            })
        })
        .collect();
    CharacterProseCandidates {
        character_id: CharacterId(uuid::Uuid::parse_str(cid).unwrap()),
        candidates,
    }
}

// ---------- Fixture helpers for the new behavioral tests ----------

fn assemble_fixture(
    beat_refs: &[&str],
    derivation_vids: &[&str],
    action: &str,
) -> Result<AssembledProse, StoryError> {
    let vocab = sample_vocab();
    let cid = "00000000-0000-0000-0000-000000000010";
    let d = make_derivation(cid, derivation_vids);
    let mut participants = HashSet::new();
    participants.insert(cid.to_string());
    let refs: Vec<String> = beat_refs.iter().map(|s| s.to_string()).collect();
    let narrative = LlmNarrative {
        beats: vec![NarrativeBeat {
            pov: cid.into(),
            action: action.into(),
            sensation_refs: refs,
        }],
    };
    AssembledProse::assemble(&narrative, &vocab, std::slice::from_ref(&d), &participants)
}

fn assemble_empty_narrative_fixture() -> Result<AssembledProse, StoryError> {
    let vocab = sample_vocab();
    let cid = "00000000-0000-0000-0000-000000000020";
    let d = make_derivation(cid, &["emotion.sorrow"]);
    let mut participants = HashSet::new();
    participants.insert(cid.to_string());
    let narrative = LlmNarrative { beats: vec![] };
    AssembledProse::assemble(&narrative, &vocab, std::slice::from_ref(&d), &participants)
}

fn assemble_cross_character_fixture() -> Result<AssembledProse, StoryError> {
    let vocab = sample_vocab();
    let cid_a = "00000000-0000-0000-0000-000000000030";
    let cid_b = "00000000-0000-0000-0000-000000000031";
    let d_a = make_derivation(cid_a, &[]);
    let d_b = make_derivation(cid_b, &["emotion.sorrow"]);
    let derivations = vec![d_a, d_b];
    let mut participants = HashSet::new();
    participants.insert(cid_a.to_string());
    participants.insert(cid_b.to_string());
    let narrative = LlmNarrative {
        beats: vec![NarrativeBeat {
            pov: cid_a.into(),
            action: "她抬头。".into(),
            sensation_refs: vec!["emotion.sorrow".into()],
        }],
    };
    AssembledProse::assemble(&narrative, &vocab, &derivations, &participants)
}

fn assemble_missing_derivation_fixture() -> Result<AssembledProse, StoryError> {
    let vocab = sample_vocab();
    let cid = "00000000-0000-0000-0000-000000000040";
    let mut participants = HashSet::new();
    participants.insert(cid.to_string());
    let narrative = LlmNarrative {
        beats: vec![NarrativeBeat {
            pov: cid.into(),
            action: "她出现。".into(),
            sensation_refs: vec![],
        }],
    };
    AssembledProse::assemble(&narrative, &vocab, &[], &participants)
}

// ---------- Existing tests (updated for new API) ----------

#[test]
fn assemble_orders_by_category() {
    let vocab = sample_vocab();
    let cid = "00000000-0000-0000-0000-000000000001";
    let d = make_derivation(
        cid,
        &[
            "gesture.weep",
            "emotion.sorrow",
            "gustatory.bitter",
            "tactile.coldhand",
            "olfactory.bloodsmell",
            "auditory.footstep",
            "visual.bloodstain",
            "atmosphere.coldnight",
        ],
    );
    let mut participants = HashSet::new();
    participants.insert(cid.to_string());
    let narrative = LlmNarrative {
        beats: vec![NarrativeBeat {
            pov: cid.into(),
            action: "她起身推门。".into(),
            sensation_refs: vec![
                "gesture.weep".into(),
                "emotion.sorrow".into(),
                "gustatory.bitter".into(),
                "tactile.coldhand".into(),
                "olfactory.bloodsmell".into(),
                "auditory.footstep".into(),
                "visual.bloodstain".into(),
                "atmosphere.coldnight".into(),
            ],
        }],
    };
    let prose =
        AssembledProse::assemble(&narrative, &vocab, std::slice::from_ref(&d), &participants)
            .unwrap();
    let atm = prose.text.find("夜凉如水").unwrap();
    let vis = prose.text.find("血迹").unwrap();
    let aud = prose.text.find("脚步声").unwrap();
    let olf = prose.text.find("血腥味").unwrap();
    let tac = prose.text.find("冰凉的手").unwrap();
    let gus = prose.text.find("苦涩").unwrap();
    let emo = prose.text.find("心中未免悔恨").unwrap();
    let ges = prose.text.find("她伸手把帕子绞了又绞").unwrap();
    assert!(atm < vis);
    assert!(vis < aud);
    assert!(aud < olf);
    assert!(olf < tac);
    assert!(tac < gus);
    assert!(gus < emo);
    assert!(emo < ges);
    assert_eq!(prose.stripped_refs, 0);
    assert!(prose.text.ends_with("她起身推门。"));
}

#[test]
fn assemble_strips_unknown_refs() {
    let vocab = sample_vocab();
    let cid = "00000000-0000-0000-0000-000000000002";
    let d = make_derivation(cid, &["emotion.sorrow"]);
    let mut participants = HashSet::new();
    participants.insert(cid.to_string());
    let narrative = LlmNarrative {
        beats: vec![NarrativeBeat {
            pov: cid.into(),
            action: "她落座。".into(),
            sensation_refs: vec!["emotion.sorrow".into(), "emotion.fabricated".into()],
        }],
    };
    let prose =
        AssembledProse::assemble(&narrative, &vocab, std::slice::from_ref(&d), &participants)
            .unwrap();
    assert_eq!(prose.stripped_refs, 1);
    assert!(prose.text.contains("心中未免悔恨"));
}

#[test]
fn assemble_rejects_non_participant_pov() {
    let vocab = sample_vocab();
    let cid = "00000000-0000-0000-0000-000000000003";
    let outsider = "00000000-0000-0000-0000-000000000099";
    let d = make_derivation(cid, &["emotion.sorrow"]);
    let mut participants = HashSet::new();
    participants.insert(cid.to_string());
    let narrative = LlmNarrative {
        beats: vec![
            NarrativeBeat {
                pov: outsider.into(),
                action: "他闯入。".into(),
                sensation_refs: vec![],
            },
            NarrativeBeat {
                pov: cid.into(),
                action: "她抬头。".into(),
                sensation_refs: vec!["emotion.sorrow".into()],
            },
        ],
    };
    let prose =
        AssembledProse::assemble(&narrative, &vocab, std::slice::from_ref(&d), &participants)
            .unwrap();
    assert_eq!(prose.rejected_beats, 1);
    assert!(!prose.text.contains("他闯入"));
    assert!(prose.text.contains("她抬头"));
}

#[test]
fn assemble_empty_refs_outputs_action_only() {
    let vocab = sample_vocab();
    let cid = "00000000-0000-0000-0000-000000000004";
    let d = make_derivation(cid, &[]);
    let mut participants = HashSet::new();
    participants.insert(cid.to_string());
    let narrative = LlmNarrative {
        beats: vec![NarrativeBeat {
            pov: cid.into(),
            action: "她默默转身离去。".into(),
            sensation_refs: vec![],
        }],
    };
    let prose =
        AssembledProse::assemble(&narrative, &vocab, std::slice::from_ref(&d), &participants)
            .unwrap();
    assert_eq!(prose.text, "她默默转身离去。");
}

// ---------- New behavioral tests ----------

#[test]
fn all_invalid_refs_keep_action_and_count_degradation() {
    let prose = assemble_fixture(&["emotion.fabricated"], &["emotion.sorrow"], "她落座。").unwrap();

    assert_eq!(prose.text, "她落座。");
    assert_eq!(prose.stripped_refs, 1);
    assert_eq!(prose.action_only_beats, 1);
}

#[test]
fn empty_refs_keep_action_and_count_degradation() {
    let prose = assemble_fixture(&[], &[], "她转身离开。").unwrap();
    assert_eq!(prose.text, "她转身离开。");
    assert_eq!(prose.action_only_beats, 1);
}

#[test]
fn empty_narrative_is_an_llm_error() {
    let err = assemble_empty_narrative_fixture().unwrap_err();
    assert!(matches!(err, StoryError::Llm(message) if message.contains("no beats")));
}

#[test]
fn cross_character_ref_is_stripped() {
    let prose = assemble_cross_character_fixture().unwrap();
    assert_eq!(prose.stripped_refs, 1);
    assert_eq!(prose.action_only_beats, 1);
}

#[test]
fn missing_derivation_pov_is_rejected() {
    let prose = assemble_missing_derivation_fixture().unwrap();
    assert_eq!(prose.rejected_beats, 1);
    assert!(prose.text.is_empty());
}

// ---------- Mock generator tests ----------

#[tokio::test]
async fn mock_generator_end_to_end() {
    let vocab = sample_vocab();
    let cid = "00000000-0000-0000-0000-000000000005";
    let d = make_derivation(cid, &["atmosphere.coldnight", "gesture.weep"]);
    let mut participants = HashSet::new();
    participants.insert(cid.to_string());
    let mock = MockProseGenerator::new(LlmNarrative {
        beats: vec![NarrativeBeat {
            pov: cid.into(),
            action: "她推门而入。".into(),
            sensation_refs: vec!["atmosphere.coldnight".into(), "gesture.weep".into()],
        }],
    });
    let candidates = vec![make_candidates(
        cid,
        &["atmosphere.coldnight", "gesture.weep"],
        &vocab,
    )];
    let req = NarrateRequest {
        scene: Scene {
            id: SceneId(uuid::Uuid::new_v4()),
            objective_event: "深夜来访".into(),
            participant_ids: vec![CharacterId(uuid::Uuid::parse_str(cid).unwrap())],
            occurred_at: chrono::Utc::now(),
        },
        characters: vec![make_character(cid, "来客")],
        derivations: vec![d.clone()],
        candidates,
    };
    let narrative = Arc::new(mock).narrate(&req).await.unwrap();
    let prose =
        AssembledProse::assemble(&narrative, &vocab, std::slice::from_ref(&d), &participants)
            .unwrap();
    assert!(prose.text.contains("夜凉如水"));
    assert!(prose.text.contains("她伸手把帕子绞了又绞"));
    assert!(prose.text.contains("她推门而入。"));
}

#[tokio::test]
async fn mock_fallback_uses_first_character_and_scene_event() {
    let vocab = sample_vocab();
    let cid = "00000000-0000-0000-0000-000000000006";
    let d = make_derivation(cid, &["atmosphere.coldnight"]);
    let candidates = vec![make_candidates(cid, &["atmosphere.coldnight"], &vocab)];
    let req = NarrateRequest {
        scene: Scene {
            id: SceneId(uuid::Uuid::new_v4()),
            objective_event: "寒夜叩门".into(),
            participant_ids: vec![CharacterId(uuid::Uuid::parse_str(cid).unwrap())],
            occurred_at: chrono::Utc::now(),
        },
        characters: vec![make_character(cid, "夜客")],
        derivations: vec![d.clone()],
        candidates,
    };
    let mock = MockProseGenerator::fallback();
    let narrative = Arc::new(mock).narrate(&req).await.unwrap();
    assert_eq!(narrative.beats.len(), 1);
    let beat = &narrative.beats[0];
    assert_eq!(beat.pov, cid);
    assert_eq!(beat.action, "寒夜叩门");
    // fallback selects the first candidate ID for the first character
    assert_eq!(
        beat.sensation_refs,
        vec!["atmosphere.coldnight".to_string()]
    );
}

#[tokio::test]
async fn mock_fallback_with_no_characters_returns_empty_beats() {
    let req = NarrateRequest {
        scene: Scene {
            id: SceneId(uuid::Uuid::new_v4()),
            objective_event: "空场景".into(),
            participant_ids: vec![],
            occurred_at: chrono::Utc::now(),
        },
        characters: vec![],
        derivations: vec![],
        candidates: vec![],
    };
    let mock = MockProseGenerator::fallback();
    let narrative = Arc::new(mock).narrate(&req).await.unwrap();
    assert!(narrative.beats.is_empty());
}

// ---------- Service-level narration tests ----------

use novels::db::Db;
use novels::llm::{LlmCharacterDerivation, LlmContextTagSelection, MockSenseGenerator};
use novels::scene::StoryService;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use uuid::Uuid;

/// Test-only ProseGenerator that counts calls, records requests,
/// and returns a configured LlmNarrative.
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
    let vocab = sample_vocab();
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
    StoryService::new(db, vocab, sense, prose)
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
    let generator = Arc::new(RecordingProseGenerator::new(response));
    let service = service_fixture(generator).await;
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
    let mut sel = SensorySelection::default();
    sel.visual_ids
        .push(VocabularyId::new("visual.bloodstain").unwrap());
    let derivation = CharacterDerivation {
        character_id,
        scene_id,
        sensations: sel,
        new_memory: CharacterMemoryDraft {
            content: "saw blood".into(),
            source: MemorySource::Witnessed,
            certainty: Certainty::Certain,
        },
        plot_development: vec![],
    };
    PartialFixture {
        service,
        scene_id,
        derivations: vec![derivation],
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
