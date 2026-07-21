use std::collections::HashSet;

use rig::client::CompletionClient;
use rig::extractor::Extractor;
use rig::providers::deepseek;
use std::sync::Arc;

use crate::models::{CharacterDerivation, StoryError};
use crate::vocab::Vocab;

use super::assembly::candidate_refs_for;
use super::contract::LlmNarrative;
use super::generator::{CharacterProseCandidates, NarrateRequest, ProseCandidate, ProseGenerator};

/// rig(DeepSeek)生产实现
/// ========================
/// 复用现有 rig + DeepSeek 基建。内部持有 `Extractor<LlmNarrative>`,
/// 与 `RigSenseGenerator` 同构。
#[allow(dead_code)]
pub struct RigProseGenerator {
    extractor: Extractor<deepseek::CompletionModel, LlmNarrative>,
    vocab: Arc<Vocab>,
}

impl RigProseGenerator {
    #[allow(dead_code)]
    pub fn new(client: deepseek::Client, vocab: Arc<Vocab>) -> Self {
        let extractor = client
            .extractor::<LlmNarrative>(deepseek::DEEPSEEK_V4_FLASH)
            .retries(1)
            .build();
        Self { extractor, vocab }
    }
}

#[async_trait::async_trait]
impl ProseGenerator for RigProseGenerator {
    async fn narrate(&self, req: &NarrateRequest) -> Result<LlmNarrative, StoryError> {
        let prompt = build_prompt(req);
        self.extractor
            .extract(&prompt)
            .await
            .map_err(|e| StoryError::Llm(format!("{e:?}")))
    }
}

/// 构建叙事编排 prompt
/// =====================
/// 候选片段按角色分组列出 ID + 原文,LLM 只能引用这些 ID。
#[allow(dead_code)]
fn build_prompt(req: &NarrateRequest) -> String {
    let mut s = String::new();
    s.push_str("你是小说叙事编排器。只写叙事骨架,描写一律通过引用片段 ID 实现。\n\n");
    s.push_str("铁律:\n");
    s.push_str("1. action 只写客观动作和对话,严禁感官/情绪/环境/神态修饰词。\n");
    s.push_str(
        "   禁止:悲伤地、愤怒地、冰冷地、香喷喷、泪流满面(这些是描写,用 sensation_refs 引用)\n",
    );
    s.push_str("   允许:她起身、他推开门、黛玉说\"……\"\n");
    s.push_str("2. sensation_refs 只能从提供的候选片段 ID 中选,不得造词。\n");
    s.push_str("3. pov 必须是场景参与者 UUID。\n");
    s.push_str("4. 用多个 beat 呈现场景,每个 beat 一个视角,引用该视角角色的描写片段。\n\n");
    s.push_str(&format!("客观事件: {}\n", req.scene.objective_event));
    s.push_str("参与者:\n");
    for c in &req.characters {
        s.push_str(&format!(
            "- {} | 名字: {} | 性格: {:?}\n",
            c.id.0, c.name, c.personality
        ));
    }
    s.push_str("\n候选片段(按角色分组,格式 id=\"原文\"):\n");
    for cr in &req.candidates {
        s.push_str(&format!("[{}]:\n", cr.character_id.0));
        for cand in &cr.candidates {
            s.push_str(&format!("  {}=\"{}\"\n", cand.id, cand.text));
        }
    }
    s.push_str("\n请调用 submit 提交结构化结果。");
    s
}

/// 从 derivations 构造每角色的语义候选引用(供 NarrateRequest 使用)
///
/// 需要词库以解析每个 VocabularyId 的 text/tags/sense。
pub fn build_candidate_refs(
    derivations: &[CharacterDerivation],
    vocab: &Vocab,
) -> Vec<CharacterProseCandidates> {
    derivations
        .iter()
        .map(|d| {
            let ids = candidate_refs_for(d);
            let candidates = ids
                .iter()
                .filter_map(|raw| {
                    let vid = crate::models::VocabularyId::new(raw).ok()?;
                    let entry = vocab.entries(vid.sense())?.get(vid.key())?;
                    Some(ProseCandidate {
                        id: raw.to_string(),
                        sense: vid.sense().to_string(),
                        text: entry.text.clone(),
                        tags: entry.tags.clone(),
                    })
                })
                .collect::<Vec<_>>();
            CharacterProseCandidates {
                character_id: d.character_id,
                candidates,
            }
        })
        .collect()
}

/// 参与者 ID 集合(供 assemble 校验 pov)
pub fn participant_set(participants: &[crate::models::CharacterId]) -> HashSet<String> {
    participants.iter().map(|id| id.0.to_string()).collect()
}
