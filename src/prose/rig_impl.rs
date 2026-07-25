use rig::client::CompletionClient;
use rig::extractor::Extractor;
use rig::providers::deepseek;

use crate::models::{PlotDevelopmentKind, StoryError};
use crate::text_guard::{MAX_MEMORY_CHARS, MAX_PLOT_REASON_CHARS, sanitize_free_text};

use super::contract::LlmNarrative;
use super::generator::{NarrateRequest, ProseGenerator};

/// 叙事编排器系统指令（通过 rig `.preamble()` 注入为 system message）
const NARRATE_PREAMBLE: &str = "\
你是小说叙事编排器。只写叙事骨架,描写一律通过引用片段 ID 实现。\n\
铁律:\n\
1. action 只写客观动作和对话,严禁感官/情绪/环境/神态修饰词。\n\
   禁止:悲伤地、愤怒地、冰冷地、香喷喷、泪流满面(这些是描写,用 sensation_refs 引用)\n\
   允许:她起身、他推开门、黛玉说\"……\"\n\
2. sensation_refs 只能从提供的候选片段 ID 中选,不得造词。\n\
3. pov 必须是场景参与者 UUID,复制完整 UUID,不要手打。\n\
4. 用多个 beat 呈现场景,每个 beat 一个视角,引用该视角角色的描写片段。";

/// rig(DeepSeek)生产实现
/// ========================
/// 复用现有 rig + DeepSeek 基建。内部持有 `Extractor<LlmNarrative>`,
/// 与 `RigSenseGenerator` 同构。不持有 `Vocab`,候选语义由
/// `NarrateRequest.candidates` 直接携带。
pub struct RigProseGenerator {
    extractor: Extractor<deepseek::CompletionModel, LlmNarrative>,
}

impl RigProseGenerator {
    pub fn new(client: deepseek::Client) -> Self {
        let extractor = client
            .extractor::<LlmNarrative>(deepseek::DEEPSEEK_V4_FLASH)
            .preamble(NARRATE_PREAMBLE)
            .retries(1)
            .build();
        Self { extractor }
    }
}

#[async_trait::async_trait]
impl ProseGenerator for RigProseGenerator {
    async fn narrate(&self, req: &NarrateRequest) -> Result<LlmNarrative, StoryError> {
        let prompt = build_prompt(req);
        self.extractor
            .extract(&prompt)
            .await
            .map_err(StoryError::llm_from_debug)
    }
}

/// 构建叙事编排 prompt
/// =====================
/// 候选片段按角色分组,以 `id | sense | text | tags` 形式列出。
/// 排序规则:
///   - 角色按 `CharacterId` 升序
///   - 每角色候选组按 `CharacterId` 升序
///   - 每组内候选按 `id` 升序
///   - 每个候选的 tags 按字典序排序
#[allow(dead_code)]
fn build_prompt(req: &NarrateRequest) -> String {
    // 克隆并按 typed CharacterId 排序角色与候选组,保证 prompt 稳定。
    // CharacterId 未派生 Ord,所以按内部 Uuid 比较。
    let mut characters = req.characters.clone();
    characters.sort_by_key(|c| c.id.0);
    let mut groups = req.candidates.clone();
    groups.sort_by_key(|g| g.character_id.0);

    let mut s = String::new();
    s.push_str(&format!("客观事件: {}\n", req.scene.objective_event));
    s.push_str("参与者:\n");
    for c in &characters {
        s.push_str(&format!("- 角色: {} (pov: {})\n", c.name, c.id.0));
        if !c.personality.is_empty() {
            s.push_str(&format!("  性格: {}\n", c.personality.join(", ")));
        }
        if !c.skills.is_empty() {
            s.push_str(&format!("  技能: {}\n", c.skills.join(", ")));
        }
    }
    // 推导上下文:每角色的最新记忆与剧情发展(自由文本复用 text_guard 限长,
    // 与 service 层 memory/plot reason 护栏一致),让 narrate 不再丢失 derive 语义。
    if !req.derivations.is_empty() {
        let mut derivations = req.derivations.clone();
        derivations.sort_by_key(|d| d.character_id.0);
        s.push_str("\n推导上下文(记忆与剧情):\n");
        for d in &derivations {
            let name = characters
                .iter()
                .find(|c| c.id == d.character_id)
                .map(|c| c.name.as_str())
                .unwrap_or("未知角色");
            let memory = sanitize_free_text(&d.new_memory.content, MAX_MEMORY_CHARS);
            s.push_str(&format!("- 角色: {name}\n  记忆: {memory}\n"));
            for plot in &d.plot_development {
                let reason = sanitize_free_text(&plot.reason, MAX_PLOT_REASON_CHARS);
                s.push_str(&format!(
                    "  剧情: {} - {}\n",
                    plot_kind_label(plot.kind),
                    reason
                ));
            }
        }
    }
    s.push_str("\n候选片段(按角色分组,格式 id | sense | text | tags):\n");
    for cr in &groups {
        let name = characters
            .iter()
            .find(|c| c.id == cr.character_id)
            .map(|c| c.name.as_str())
            .unwrap_or("未知角色");
        s.push_str(&format!("角色 {name} (pov: {}):\n", cr.character_id.0));
        // 按 VocabularyId 的规范顺序排序:先按 sense 在 SENSES 中的位置,
        // 再按 key。与 vocab::loader::collect_candidates 一致,保证稳定。
        let mut cands = cr.candidates.clone();
        cands.sort_by(|a, b| {
            let sa = crate::vocab::sense_order(&a.sense);
            let sb = crate::vocab::sense_order(&b.sense);
            sa.cmp(&sb).then_with(|| a.id.cmp(&b.id))
        });
        for cand in &cands {
            let mut tags = cand.tags.clone();
            tags.sort();
            s.push_str(&format!(
                "  id: {} | sense: {} | text: {} | tags: {}\n",
                cand.id,
                cand.sense,
                cand.text,
                tags.join(", ")
            ));
        }
    }
    s.push_str("\n示例 beat:\n");
    s.push_str("- pov: <UUID> | action: 黛玉转身走向窗前，低声道：\"我没事。\" | sensation_refs: [\"emotion.grief\", \"visual.moonlight\"]\n");
    s.push_str("注意：action 纯动作对话，描写全靠 sensation_refs 引用。\n");
    s
}

/// `PlotDevelopmentKind` 的 snake_case 标签(与 serde `rename_all` 一致),供 prompt 展示。
fn plot_kind_label(kind: PlotDevelopmentKind) -> &'static str {
    match kind {
        PlotDevelopmentKind::SuspicionRaised => "suspicion_raised",
        PlotDevelopmentKind::ConflictEscalated => "conflict_escalated",
        PlotDevelopmentKind::GoalChanged => "goal_changed",
        PlotDevelopmentKind::RelationshipShifted => "relationship_shifted",
        PlotDevelopmentKind::NewClue => "new_clue",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        Certainty, Character, CharacterDerivation, CharacterId, CharacterMemoryDraft, MemorySource,
        PlotDevelopment, PlotDevelopmentKind, Scene, SceneId, SensorySelection,
    };
    use crate::prose::{CharacterProseCandidates, VocabularyCandidate};
    use chrono::Utc;
    use uuid::Uuid;

    fn uuid_from(s: &str) -> Uuid {
        Uuid::parse_str(s).unwrap()
    }

    fn make_character(id: &str, name: &str) -> Character {
        Character {
            id: CharacterId(uuid_from(id)),
            name: name.into(),
            personality: vec![],
            skills: vec![],
        }
    }

    fn make_candidate(id: &str, sense: &str, text: &str, tags: &[&str]) -> VocabularyCandidate {
        VocabularyCandidate {
            id: id.into(),
            sense: sense.into(),
            text: text.into(),
            tags: tags.iter().map(|t| (*t).to_string()).collect(),
        }
    }

    /// 构造一个候选顺序混乱、tags 顺序混乱的请求,用于验证 prompt 排序稳定性。
    fn request_with_unsorted_candidates() -> NarrateRequest {
        // 故意让 cid_b 的 UUID 小于 cid_a,验证角色按 typed ID 排序
        let cid_b = "00000000-0000-0000-0000-000000000001";
        let cid_a = "00000000-0000-0000-0000-000000000002";
        NarrateRequest {
            scene: Scene {
                id: SceneId(uuid_from(cid_a)),
                objective_event: "深夜来访".into(),
                participant_ids: vec![CharacterId(uuid_from(cid_a)), CharacterId(uuid_from(cid_b))],
                occurred_at: Utc::now(),
            },
            // characters 顺序与 sorted-by-id 相反
            characters: vec![make_character(cid_a, "甲"), make_character(cid_b, "乙")],
            derivations: vec![],
            // candidates 分组顺序与 sorted-by-id 相反;组内候选顺序也故意逆序;
            // tags 顺序故意逆序。
            candidates: vec![
                CharacterProseCandidates {
                    character_id: CharacterId(uuid_from(cid_a)),
                    candidates: vec![
                        // gesture 在 visual 之后(sort by id),这里故意放前面
                        make_candidate(
                            "gesture.weep",
                            "gesture",
                            "她伸手把帕子绞了又绞",
                            &["grief", "crime"],
                        ),
                        // tags 顺序:injury 在 crime 之前(字典序),这里故意放后
                        make_candidate("visual.bloodstain", "visual", "血迹", &["injury", "crime"]),
                    ],
                },
                CharacterProseCandidates {
                    character_id: CharacterId(uuid_from(cid_b)),
                    candidates: vec![],
                },
            ],
        }
    }

    #[test]
    fn prompt_contains_stable_semantic_candidates() {
        let request = request_with_unsorted_candidates();
        let prompt = build_prompt(&request);

        // tags 按字典序排序后是 "crime, grief" / "crime, injury"
        assert!(
            prompt.contains(
                "id: visual.bloodstain | sense: visual | text: 血迹 | tags: crime, injury"
            )
        );
        assert!(prompt.contains(
            "id: gesture.weep | sense: gesture | text: 她伸手把帕子绞了又绞 | tags: crime, grief"
        ));
        // 候选组内按 id 升序:visual.bloodstain 在 gesture.weep 之前
        assert!(prompt.find("visual.bloodstain").unwrap() < prompt.find("gesture.weep").unwrap());
        // 角色分组按 CharacterId 升序:cid_b (....001) 在 cid_a (....002) 之前
        assert!(prompt.find(cid_b_str()).unwrap() < prompt.find(cid_a_str()).unwrap());
    }

    fn cid_a_str() -> &'static str {
        "00000000-0000-0000-0000-000000000002"
    }

    fn cid_b_str() -> &'static str {
        "00000000-0000-0000-0000-000000000001"
    }

    #[test]
    fn prompt_restricts_action_and_reference_output() {
        // 铁律已移入 NARRATE_PREAMBLE 系统指令,验证其在 preamble 中存在。
        assert!(NARRATE_PREAMBLE.contains("action 只写客观动作和对话"));
        assert!(NARRATE_PREAMBLE.contains("sensation_refs 只能从提供的候选"));
    }

    #[test]
    fn prompt_includes_personality_memory_and_plot() {
        let cid = "00000000-0000-0000-0000-00000000000a";
        let scene = Scene {
            id: SceneId(uuid_from("00000000-0000-0000-0000-00000000000b")),
            objective_event: "古宅发现一具尸体".into(),
            participant_ids: vec![CharacterId(uuid_from(cid))],
            occurred_at: Utc::now(),
        };
        let character = Character {
            id: CharacterId(uuid_from(cid)),
            name: "侦探".into(),
            personality: vec!["谨慎".into()],
            skills: vec!["推理".into()],
        };
        let derivation = CharacterDerivation {
            character_id: CharacterId(uuid_from(cid)),
            scene_id: scene.id,
            sensations: SensorySelection::default(),
            new_memory: CharacterMemoryDraft {
                content: "看到地上有血迹".into(),
                source: MemorySource::Witnessed,
                certainty: Certainty::Certain,
            },
            plot_development: vec![PlotDevelopment {
                kind: PlotDevelopmentKind::NewClue,
                reason: "血迹指向凶手".into(),
            }],
        };
        let req = NarrateRequest {
            scene,
            characters: vec![character],
            derivations: vec![derivation],
            candidates: vec![],
        };
        let prompt = build_prompt(&req);

        assert!(
            prompt.contains("性格: 谨慎"),
            "missing personality: {prompt}"
        );
        assert!(prompt.contains("技能: 推理"), "missing skills: {prompt}");
        assert!(
            prompt.contains("看到地上有血迹"),
            "missing memory content: {prompt}"
        );
        assert!(
            prompt.contains("血迹指向凶手"),
            "missing plot reason: {prompt}"
        );
        assert!(prompt.contains("new_clue"), "missing plot kind: {prompt}");
    }
}
