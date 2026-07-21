use std::collections::HashSet;

use crate::models::{CharacterDerivation, StoryError};
use crate::vocab::Vocab;

use super::contract::LlmNarrative;

/// 拼装结果:正文 + 剥离/拒绝/动作-唯一计数(可观测)
/// =================================================
/// `action_only_beats` 统计"被接受但描写为空、只输出 action"的 beat 数,
/// 用于衡量 LLM 输出的"描写退化"程度。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProse {
    pub text: String,
    pub stripped_refs: usize,
    pub rejected_beats: usize,
    pub action_only_beats: usize,
}

impl AssembledProse {
    /// 把 `LlmNarrative` 拼装成正文
    /// =============================
    /// 规则:
    ///   1. `beats` 为空 -> `StoryError::Llm("prose generator returned no beats")`
    ///   2. beat 的 pov 非参与者,或 pov 无匹配 derivation -> 整 beat 拒绝,`rejected_beats`++
    ///   3. ref 不属于该 pov 的 derivation 候选集,或无法通过 Vocab 解析 -> 剥离,`stripped_refs`++
    ///   4. 合法 ref 按固定 8 类顺序拼装:atmosphere -> visual -> auditory -> olfactory -> tactile -> gustatory -> emotion -> gesture
    ///   5. 同类内保持 ref 出现顺序,beat 间保持叙事顺序
    ///   6. 被接受的 beat 若描写为空但 action 非空 -> `action_only_beats`++
    ///   7. action 非空时即使描写为空也输出 action 段
    ///   8. 所有被接受的非空 beat 用单个 `\n` 连接
    pub fn assemble(
        narrative: &LlmNarrative,
        vocab: &Vocab,
        derivations: &[CharacterDerivation],
        participant_ids: &HashSet<String>,
    ) -> Result<Self, StoryError> {
        if narrative.beats.is_empty() {
            return Err(StoryError::Llm("prose generator returned no beats".into()));
        }

        // pov -> 该角色候选片段集(8 类合并)
        let mut cand_map: std::collections::HashMap<String, HashSet<String>> =
            std::collections::HashMap::new();
        for d in derivations {
            cand_map.insert(d.character_id.0.to_string(), candidate_refs_for(d));
        }

        let mut paragraphs: Vec<String> = Vec::new();
        let mut total_stripped = 0usize;
        let mut rejected = 0usize;
        let mut action_only = 0usize;

        for beat in &narrative.beats {
            let Beat { pov, action, refs } = unpack_beat(beat);
            if !participant_ids.contains(&pov) {
                rejected += 1;
                continue;
            }
            // pov 必须有匹配的 derivation;缺失则拒绝整 beat
            let allowed = match cand_map.get(&pov) {
                Some(set) => set.clone(),
                None => {
                    rejected += 1;
                    continue;
                }
            };
            let (desc, stripped) = assemble_beat_descriptions(&refs, vocab, &allowed);
            total_stripped += stripped;

            let para = if desc.is_empty() {
                if !action.trim().is_empty() {
                    action_only += 1;
                }
                action.clone()
            } else {
                format!("{desc}{action}")
            };
            if !para.trim().is_empty() {
                paragraphs.push(para);
            }
        }

        Ok(Self {
            text: paragraphs.join("\n"),
            stripped_refs: total_stripped,
            rejected_beats: rejected,
            action_only_beats: action_only,
        })
    }
}

/// 从该角色的 derivation 收集全部合法候选片段 ID(8 类合并)
pub(crate) fn candidate_refs_for(derivation: &CharacterDerivation) -> HashSet<String> {
    let s = &derivation.sensations;
    s.visual_ids
        .iter()
        .chain(s.auditory_ids.iter())
        .chain(s.olfactory_ids.iter())
        .chain(s.tactile_ids.iter())
        .chain(s.gustatory_ids.iter())
        .chain(s.emotion_ids.iter())
        .chain(s.gesture_ids.iter())
        .chain(s.atmosphere_ids.iter())
        .map(|id| id.as_str().to_string())
        .collect()
}

/// 按类别有序拼装单 beat 的描写片段,返回拼好的描写段(可能为空)+ 剥离计数
///
/// 顺序模拟小说段落节奏:氛围开头 -> 感官穿插 -> 情绪 -> 神态
fn assemble_beat_descriptions(
    refs: &[String],
    vocab: &Vocab,
    allowed: &HashSet<String>,
) -> (String, usize) {
    let mut by_cat: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut stripped = 0;
    for raw in refs {
        if !allowed.contains(raw) {
            stripped += 1;
            continue;
        }
        let Ok(vid) = crate::models::VocabularyId::new(raw) else {
            stripped += 1;
            continue;
        };
        let Some(entry) = vocab.entries(vid.sense()).and_then(|m| m.get(vid.key())) else {
            stripped += 1;
            continue;
        };
        by_cat
            .entry(vid.sense().to_string())
            .or_default()
            .push(entry.text.clone());
    }

    let order = [
        "atmosphere",
        "visual",
        "auditory",
        "olfactory",
        "tactile",
        "gustatory",
        "emotion",
        "gesture",
    ];
    let mut parts: Vec<String> = Vec::new();
    for cat in order {
        if let Some(texts) = by_cat.get(cat) {
            for t in texts {
                parts.push(t.clone());
            }
        }
    }
    (parts.join(""), stripped)
}

struct Beat {
    pov: String,
    action: String,
    refs: Vec<String>,
}

fn unpack_beat(b: &super::contract::NarrativeBeat) -> Beat {
    Beat {
        pov: b.pov.clone(),
        action: b.action.clone(),
        refs: b.sensation_refs.clone(),
    }
}
