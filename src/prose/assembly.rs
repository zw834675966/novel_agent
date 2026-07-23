use std::collections::HashSet;

use crate::models::{CharacterDerivation, StoryError};
use crate::vocab::Vocab;

use super::contract::LlmNarrative;

/// Minimum quote density before [`ProseQualityReport::low_quote_density`] is set.
/// Observational only — does not fail assemble (mock/action-only flows may sit below this).
pub const MIN_QUOTE_DENSITY: f64 = 0.30;

/// Aggregated quality KPIs for a single assemble (VeriCite-style post-gen observability).
#[derive(Debug, Clone, PartialEq)]
pub struct ProseQualityReport {
    pub quote_density: f64,
    /// `action_only_beats / accepted_beats` (0 if no accepted beats).
    pub action_only_rate: f64,
    /// `stripped_refs / total_refs_seen` (0 if no refs seen).
    pub stripped_ref_rate: f64,
    /// Injected quote texts not found in final `text` after assemble (should be 0).
    pub unverified_quotes: usize,
    /// `quote_density < MIN_QUOTE_DENSITY`.
    pub low_quote_density: bool,
}

/// 拼装结果:正文 + 剥离/拒绝/动作-唯一计数 + quote density 可观测
/// ================================================================
/// `action_only_beats` 统计"被接受但描写为空、只输出 action"的 beat 数。
/// `quote_chars` / `total_chars` 用于 quote density（原著描写占比）。
/// `unverified_quotes` / `quality`：装配后回源重扫 + 聚合 KPI。
#[derive(Debug, Clone, PartialEq)]
pub struct AssembledProse {
    pub text: String,
    pub stripped_refs: usize,
    pub rejected_beats: usize,
    pub action_only_beats: usize,
    /// Characters from injected vocab `text` (soul-bearing spans).
    pub quote_chars: usize,
    /// Total characters in final `text` (including actions and newlines).
    pub total_chars: usize,
    /// Post-assemble provenance failures (injected text missing from final body).
    pub unverified_quotes: usize,
    /// Aggregated KPIs (density, rates, density threshold flag).
    pub quality: ProseQualityReport,
}

impl AssembledProse {
    /// `quote_chars / total_chars` in \[0, 1\]; 0 if empty.
    pub fn quote_density(&self) -> f64 {
        if self.total_chars == 0 {
            0.0
        } else {
            self.quote_chars as f64 / self.total_chars as f64
        }
    }

    /// Render + sanitize a structured action.
    /// N3: 舞台提示级渲染，subject 由角色名确定。
    pub fn render_action(action: &crate::models::StructuredAction, subject: &str) -> String {
        // StructuredAction.render 内部已调用 sanitize_free_text
        let rendered = action.render(subject);
        // 额外长度安全网
        crate::text_guard::sanitize_free_text(&rendered, crate::text_guard::MAX_ACTION_CHARS)
    }

    /// Legacy string sanitizer (for backward compat / non-N3 code paths).
    pub fn sanitize_action(action: &str) -> String {
        crate::text_guard::sanitize_free_text(action, crate::text_guard::MAX_ACTION_CHARS)
    }

    /// 把 `LlmNarrative` 拼装成正文
    /// =============================
    /// 规则:
    ///   1. `beats` 为空 -> `StoryError::Llm("prose generator returned no beats")`
    ///   2. beat 的 pov 非参与者,或 pov 无匹配 derivation -> 整 beat 拒绝,`rejected_beats`++
    ///   3. ref 不属于该 pov 的 derivation 候选集,或无法通过 Vocab 解析 -> 剥离,`stripped_refs`++
    ///   4. 合法 ref 按 sensation_refs 出现顺序拼装(Hard 原句;镜头序由 beat 序列表达)
    ///   5. beat 间保持叙事顺序(同类内 ref 出现顺序随 ref 序自然保留)
    ///   6. 被接受的 beat 若描写为空但 action 非空 -> `action_only_beats`++
    ///   7. action 经 [`Self::sanitize_action`] 后输出
    ///   8. 所有被接受的非空 beat 用单个 `\n` 连接
    ///   9. 装配后对每条注入 quote 做 `text.contains` 回源重扫 → `unverified_quotes`
    pub(crate) fn assemble(
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
        let mut quote_chars = 0usize;
        let mut accepted_beats = 0usize;
        let mut total_refs_seen = 0usize;
        let mut injected_quotes: Vec<String> = Vec::new();

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
            total_refs_seen += refs.len();
            let (desc, stripped, quotes) = assemble_beat_descriptions(&refs, vocab, &allowed);
            total_stripped += stripped;
            quote_chars += desc.chars().count();
            injected_quotes.extend(quotes);

            // N3: 结构化动作渲染，主语用「他/她」（视角中性，实际应用可按角色性别替换）
            let action = Self::render_action(&action, "她");
            let para = if desc.is_empty() {
                if !action.trim().is_empty() {
                    action_only += 1;
                }
                action
            } else {
                join_desc_action(&desc, &action)
            };
            // Count accepted beats that contribute to output (or empty accepted with empty action).
            accepted_beats += 1;
            if !para.trim().is_empty() {
                paragraphs.push(para);
            }
        }

        let text = paragraphs.join("\n");
        let total_chars = text.chars().count();
        // Post-assemble provenance: every injected quote must appear in final body.
        let unverified_quotes = injected_quotes
            .iter()
            .filter(|q| !q.is_empty() && !text.contains(q.as_str()))
            .count();
        let quote_density = if total_chars == 0 {
            0.0
        } else {
            quote_chars as f64 / total_chars as f64
        };
        let quality = ProseQualityReport {
            quote_density,
            action_only_rate: if accepted_beats == 0 {
                0.0
            } else {
                action_only as f64 / accepted_beats as f64
            },
            stripped_ref_rate: if total_refs_seen == 0 {
                0.0
            } else {
                total_stripped as f64 / total_refs_seen as f64
            },
            unverified_quotes,
            low_quote_density: quote_density < MIN_QUOTE_DENSITY,
        };
        Ok(Self {
            text,
            stripped_refs: total_stripped,
            rejected_beats: rejected,
            action_only_beats: action_only,
            quote_chars,
            total_chars,
            unverified_quotes,
            quality,
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

/// Resolved quote candidate before source filter + rhythm join.
struct ResolvedQuote {
    text: String,
    /// Book corpus tag when known (`hlm` / `zhz`); base vocab is `None`.
    source: Option<String>,
}

/// Infer book source from distilled key (`hlm-c103-46`) or tags (`hlm`/`zhz`).
fn book_source(key: &str, tags: &[String]) -> Option<String> {
    for t in tags {
        if t == "hlm" || t == "zhz" {
            return Some(t.clone());
        }
    }
    if let Some(prefix) = key.split('-').next()
        && (prefix == "hlm" || prefix == "zhz")
    {
        return Some(prefix.to_string());
    }
    None
}

fn ends_with_cjk_punct(s: &str) -> bool {
    matches!(
        s.chars().last(),
        Some(
            '。' | '，'
                | '、'
                | '！'
                | '？'
                | '；'
                | '：'
                | '…'
                | '”'
                | '」'
                | '』'
                | '.'
                | ','
                | '!'
                | '?'
        )
    )
}

fn starts_with_cjk_punct(s: &str) -> bool {
    matches!(
        s.chars().next(),
        Some('。' | '，' | '、' | '！' | '？' | '；' | '：' | '…' | '“' | '「' | '『')
    )
}

/// Join injected quotes with light punctuation so short lemmas are not bare-pasted
/// (platform P2: list / 注水 texture — "血迹血腥味侦探…").
fn join_quotes_with_rhythm(parts: &[String]) -> String {
    if parts.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for (i, p) in parts.iter().enumerate() {
        if i == 0 {
            out.push_str(p);
            continue;
        }
        if !ends_with_cjk_punct(&out) && !starts_with_cjk_punct(p) {
            // Prefer comma between short sensory lemmas; period between longer spans.
            let prev_short = parts[i - 1].chars().count() <= 6;
            let cur_short = p.chars().count() <= 6;
            if prev_short || cur_short {
                out.push('，');
            } else {
                out.push('。');
            }
        }
        out.push_str(p);
    }
    out
}

/// Join description block to action without bare glue.
fn join_desc_action(desc: &str, action: &str) -> String {
    if desc.is_empty() {
        return action.to_string();
    }
    if action.is_empty() {
        return desc.to_string();
    }
    if ends_with_cjk_punct(desc) || starts_with_cjk_punct(action) {
        format!("{desc}{action}")
    } else {
        format!("{desc}。{action}")
    }
}

/// Majority book source among hlm/zhz quotes; ties → first-seen.
fn dominant_book_source(quotes: &[ResolvedQuote]) -> Option<String> {
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    for q in quotes {
        if let Some(s) = &q.source {
            if !counts.contains_key(s) {
                order.push(s.clone());
            }
            *counts.entry(s.clone()).or_insert(0) += 1;
        }
    }
    if counts.is_empty() {
        return None;
    }
    let max = *counts.values().max().unwrap_or(&0);
    order
        .into_iter()
        .find(|s| counts.get(s).copied().unwrap_or(0) == max)
}

/// 按 sensation_refs 出现顺序拼装单 beat 的描写片段。
///
/// 返回 `(描写段, 剥离计数, 注入的原文列表)`。
/// - 顺序：保持 refs 遇到顺序（镜头序），不按感官类别重排
/// - 书源隔离：同 beat 内 hlm/zhz 冲突时只保留多数源（基座词无源，始终可留）
/// - 节奏：quote 间按需插入 ，/。，避免短词裸贴
fn assemble_beat_descriptions(
    refs: &[String],
    vocab: &Vocab,
    allowed: &HashSet<String>,
) -> (String, usize, Vec<String>) {
    let mut resolved: Vec<ResolvedQuote> = Vec::new();
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
        resolved.push(ResolvedQuote {
            text: entry.text.clone(),
            source: book_source(vid.key(), &entry.tags),
        });
    }

    // Source isolation (platform P3: 跨书串味).
    let dominant = dominant_book_source(&resolved);
    if let Some(dom) = &dominant {
        let before = resolved.len();
        resolved.retain(|q| q.source.as_ref().map(|s| s == dom).unwrap_or(true));
        stripped += before - resolved.len();
    }

    // Preserve ref input order (camera-beat order); do NOT sort by sense category.
    // `resolved` is already in ref-encounter order, and the source-isolation
    // `retain` above is stable, so relative order survives.
    let parts: Vec<String> = resolved.iter().map(|q| q.text.clone()).collect();
    let quotes = parts.clone();
    let desc = join_quotes_with_rhythm(&parts);
    (desc, stripped, quotes)
}

struct Beat {
    pov: String,
    action: crate::models::StructuredAction,
    refs: Vec<String>,
}

fn unpack_beat(b: &super::contract::NarrativeBeat) -> Beat {
    Beat {
        pov: b.pov.clone(),
        action: b.action.clone(),
        refs: b.sensation_refs.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        Certainty, CharacterId, CharacterMemory, MemoryId, MemorySource, SceneId, SensorySelection,
        VocabularyId,
    };
    use crate::prose::NarrativeBeat;

    const CHARACTER_A: &str = "00000000-0000-0000-0000-000000000001";
    const CHARACTER_B: &str = "00000000-0000-0000-0000-000000000002";

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

    fn derivation(character_id: &str, ids: &[&str]) -> CharacterDerivation {
        let mut sensations = SensorySelection::default();
        for raw in ids {
            let id = VocabularyId::new(raw).unwrap();
            match id.sense() {
                "visual" => sensations.visual_ids.push(id),
                "auditory" => sensations.auditory_ids.push(id),
                "olfactory" => sensations.olfactory_ids.push(id),
                "tactile" => sensations.tactile_ids.push(id),
                "gustatory" => sensations.gustatory_ids.push(id),
                "emotion" => sensations.emotion_ids.push(id),
                "gesture" => sensations.gesture_ids.push(id),
                "atmosphere" => sensations.atmosphere_ids.push(id),
                _ => unreachable!(),
            }
        }
        CharacterDerivation {
            character_id: CharacterId(uuid::Uuid::parse_str(character_id).unwrap()),
            scene_id: SceneId(uuid::Uuid::new_v4()),
            sensations,
            new_memory: CharacterMemory {
                id: MemoryId(uuid::Uuid::new_v4()),
                character_id: CharacterId(uuid::Uuid::parse_str(character_id).unwrap()),
                scene_id: SceneId(uuid::Uuid::new_v4()),
                content: "mock".into(),
                source: MemorySource::Witnessed,
                certainty: Certainty::Certain,
                created_at: chrono::Utc::now(),
            },
            plot_development: vec![],
            relationship_candidates: vec![],
        }
    }

    fn participants(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|id| (*id).to_string()).collect()
    }

    fn narrative(beats: Vec<(&str, crate::models::StructuredAction, &[&str])>) -> LlmNarrative {
        LlmNarrative {
            beats: beats
                .into_iter()
                .map(|(pov, action, refs)| NarrativeBeat {
                    pov: pov.into(),
                    action,
                    sensation_refs: refs.iter().map(|raw| (*raw).to_string()).collect(),
                })
                .collect(),
        }
    }

    #[test]
    fn preserves_ref_input_order_all_categories() {
        // Refs are deliberately given in REVERSE sense order to prove assemble
        // preserves ref-input order rather than the old fixed 8-category order.
        let ids = [
            "gesture.weep",
            "emotion.sorrow",
            "gustatory.bitter",
            "tactile.coldhand",
            "olfactory.bloodsmell",
            "auditory.footstep",
            "visual.bloodstain",
            "atmosphere.coldnight",
        ];
        let action = crate::models::StructuredAction {
            kind: crate::models::ActionKind::Enter,
            target: Some("房间".into()),
            dialogue: None,
        };
        let prose = AssembledProse::assemble(
            &narrative(vec![(CHARACTER_A, action, &ids)]),
            &sample_vocab(),
            &[derivation(CHARACTER_A, &ids)],
            &participants(&[CHARACTER_A]),
        )
        .unwrap();

        // N3: quotes joined in ref-input order (gesture -> emotion -> ... -> atmosphere),
        // 动作渲染为 "她进入房间"。All sample texts <=6 chars => comma separators.
        assert_eq!(
            prose.text,
            "她伸手把帕子绞了又绞，心中未免悔恨，苦涩，冰凉的手，血腥味，脚步声，血迹，夜凉如水。她进入房间"
        );
        assert_eq!(prose.stripped_refs, 0);
    }

    #[test]
    fn assemble_preserves_ref_order_not_sense_order() {
        // vocab: visual.bloodstain="血迹", atmosphere.coldnight="夜凉如水"
        // beat refs: [visual.bloodstain, atmosphere.coldnight]  // visual before atmosphere
        // OLD behavior: atmosphere first -> "夜凉如水…血迹"
        // NEW: preserve ref order -> starts with "血迹"
        let action = crate::models::StructuredAction {
            kind: crate::models::ActionKind::LookAt,
            target: Some("地面".into()),
            dialogue: None,
        };
        let prose = AssembledProse::assemble(
            &narrative(vec![(
                CHARACTER_A,
                action,
                &["visual.bloodstain", "atmosphere.coldnight"],
            )]),
            &sample_vocab(),
            &[derivation(
                CHARACTER_A,
                &["visual.bloodstain", "atmosphere.coldnight"],
            )],
            &participants(&[CHARACTER_A]),
        )
        .unwrap();
        let pos_blood = prose.text.find("血迹").unwrap();
        let pos_night = prose.text.find("夜凉如水").unwrap();
        assert!(
            pos_blood < pos_night,
            "ref order must win over sense order: {}",
            prose.text
        );
    }

    #[test]
    fn preserves_beat_order() {
        let action1 = crate::models::StructuredAction {
            kind: crate::models::ActionKind::Pause,
            target: None,
            dialogue: Some("第一拍。".into()),
        };
        let action2 = crate::models::StructuredAction {
            kind: crate::models::ActionKind::Pause,
            target: None,
            dialogue: Some("第二拍。".into()),
        };
        let prose = AssembledProse::assemble(
            &narrative(vec![
                (CHARACTER_A, action1, &[]),
                (CHARACTER_A, action2, &[]),
            ]),
            &sample_vocab(),
            &[derivation(CHARACTER_A, &[])],
            &participants(&[CHARACTER_A]),
        )
        .unwrap();

        // N3: 动作渲染为 "她停步，道：..."
        assert!(prose.text.contains("第一拍。"));
        assert!(prose.text.contains("第二拍。"));
        assert!(prose.text.contains('\n'));
    }

    #[test]
    fn strips_unknown_and_malformed_refs() {
        let action = crate::models::StructuredAction {
            kind: crate::models::ActionKind::SitDown,
            target: None,
            dialogue: None,
        };
        let prose = AssembledProse::assemble(
            &narrative(vec![(
                CHARACTER_A,
                action,
                &["emotion.sorrow", "emotion.fabricated", "malformed"],
            )]),
            &sample_vocab(),
            &[derivation(CHARACTER_A, &["emotion.sorrow"])],
            &participants(&[CHARACTER_A]),
        )
        .unwrap();

        // N3: "她坐下" 而非 "她落座。"
        assert_eq!(prose.text, "心中未免悔恨。她坐下");
        assert_eq!(prose.stripped_refs, 2);
    }

    #[test]
    fn strips_cross_character_refs() {
        let action = crate::models::StructuredAction {
            kind: crate::models::ActionKind::LookAt,
            target: Some("上方".into()),
            dialogue: None,
        };
        let prose = AssembledProse::assemble(
            &narrative(vec![(CHARACTER_A, action, &["emotion.sorrow"])]),
            &sample_vocab(),
            &[
                derivation(CHARACTER_A, &[]),
                derivation(CHARACTER_B, &["emotion.sorrow"]),
            ],
            &participants(&[CHARACTER_A, CHARACTER_B]),
        )
        .unwrap();

        assert_eq!(prose.text, "她看向上方");
        assert_eq!(prose.stripped_refs, 1);
        assert_eq!(prose.action_only_beats, 1);
    }

    #[test]
    fn strips_refs_missing_from_vocab() {
        let action = crate::models::StructuredAction {
            kind: crate::models::ActionKind::SitDown,
            target: None,
            dialogue: None,
        };
        let prose = AssembledProse::assemble(
            &narrative(vec![(CHARACTER_A, action, &["emotion.missing"])]),
            &sample_vocab(),
            &[derivation(CHARACTER_A, &["emotion.missing"])],
            &participants(&[CHARACTER_A]),
        )
        .unwrap();

        assert_eq!(prose.text, "她坐下");
        assert_eq!(prose.stripped_refs, 1);
    }

    #[test]
    fn rejects_non_participant_pov() {
        let action_b = crate::models::StructuredAction {
            kind: crate::models::ActionKind::Enter,
            target: Some("房间".into()),
            dialogue: None,
        };
        let action_a = crate::models::StructuredAction {
            kind: crate::models::ActionKind::LookAt,
            target: Some("上方".into()),
            dialogue: None,
        };
        let prose = AssembledProse::assemble(
            &narrative(vec![
                (CHARACTER_B, action_b, &[]),
                (CHARACTER_A, action_a, &["emotion.sorrow"]),
            ]),
            &sample_vocab(),
            &[derivation(CHARACTER_A, &["emotion.sorrow"])],
            &participants(&[CHARACTER_A]),
        )
        .unwrap();

        assert_eq!(prose.rejected_beats, 1);
        assert_eq!(prose.text, "心中未免悔恨。她看向上方");
    }

    #[test]
    fn rejects_pov_without_derivation() {
        let action = crate::models::StructuredAction {
            kind: crate::models::ActionKind::Enter,
            target: None,
            dialogue: None,
        };
        let prose = AssembledProse::assemble(
            &narrative(vec![(CHARACTER_A, action, &[])]),
            &sample_vocab(),
            &[],
            &participants(&[CHARACTER_A]),
        )
        .unwrap();

        assert_eq!(prose.rejected_beats, 1);
        assert!(prose.text.is_empty());
    }

    #[test]
    fn empty_refs_keep_action_and_count_action_only() {
        let action = crate::models::StructuredAction {
            kind: crate::models::ActionKind::Turn,
            target: Some("离开".into()),
            dialogue: None,
        };
        let prose = AssembledProse::assemble(
            &narrative(vec![(CHARACTER_A, action, &[])]),
            &sample_vocab(),
            &[derivation(CHARACTER_A, &[])],
            &participants(&[CHARACTER_A]),
        )
        .unwrap();

        assert_eq!(prose.text, "她转身离开");
        assert_eq!(prose.action_only_beats, 1);
    }

    #[test]
    fn all_invalid_refs_keep_action_and_count_action_only() {
        let action = crate::models::StructuredAction {
            kind: crate::models::ActionKind::SitDown,
            target: None,
            dialogue: None,
        };
        let prose = AssembledProse::assemble(
            &narrative(vec![(CHARACTER_A, action, &["emotion.fabricated"])]),
            &sample_vocab(),
            &[derivation(CHARACTER_A, &["emotion.sorrow"])],
            &participants(&[CHARACTER_A]),
        )
        .unwrap();

        assert_eq!(prose.text, "她坐下");
        assert_eq!(prose.stripped_refs, 1);
        assert_eq!(prose.action_only_beats, 1);
    }

    #[test]
    fn empty_beats_are_a_hard_error() {
        let error = AssembledProse::assemble(
            &LlmNarrative { beats: vec![] },
            &sample_vocab(),
            &[derivation(CHARACTER_A, &[])],
            &participants(&[CHARACTER_A]),
        )
        .unwrap_err();

        assert!(matches!(error, StoryError::Llm(message) if message.contains("no beats")));
    }

    #[test]
    fn sanitize_action_strips_causal_fillers_and_clamps_length() {
        let long = "于是".repeat(50);
        let out = AssembledProse::sanitize_action(&format!("因此{long}不禁"));
        assert!(!out.contains("因此"));
        assert!(!out.contains("不禁"));
        assert!(!out.contains("于是"));
        assert!(out.chars().count() <= crate::text_guard::MAX_ACTION_CHARS);
    }

    #[test]
    fn assemble_reports_quote_density_from_injected_text() {
        let action = crate::models::StructuredAction {
            kind: crate::models::ActionKind::SitDown,
            target: None,
            dialogue: None,
        };
        let prose = AssembledProse::assemble(
            &narrative(vec![(CHARACTER_A, action, &["emotion.sorrow"])]),
            &sample_vocab(),
            &[derivation(CHARACTER_A, &["emotion.sorrow"])],
            &participants(&[CHARACTER_A]),
        )
        .unwrap();
        // "心中未免悔恨" + "她坐下"
        assert!(prose.quote_chars > 0);
        assert!(prose.total_chars >= prose.quote_chars);
        assert!(prose.quote_density() > 0.0);
        assert!(prose.quote_density() <= 1.0);
        assert!(prose.text.contains("心中未免悔恨"));
    }

    #[test]
    fn assemble_sanitizes_action_causal_glue() {
        // N3: 结构化动作在 render 内已清理套话；target 含套话也会清理
        let action = crate::models::StructuredAction {
            kind: crate::models::ActionKind::Turn,
            target: Some("因此身".into()), // 含套话，render 时会清理
            dialogue: None,
        };
        let prose = AssembledProse::assemble(
            &narrative(vec![(CHARACTER_A, action, &[])]),
            &sample_vocab(),
            &[derivation(CHARACTER_A, &[])],
            &participants(&[CHARACTER_A]),
        )
        .unwrap();
        assert!(!prose.text.contains("因此"));
        assert!(prose.text.contains("她转身"));
    }

    #[test]
    fn join_rhythm_separates_short_sensory_lemmas() {
        let action = crate::models::StructuredAction {
            kind: crate::models::ActionKind::Pause,
            target: None,
            dialogue: None,
        };
        let prose = AssembledProse::assemble(
            &narrative(vec![(
                CHARACTER_A,
                action,
                &["visual.bloodstain", "olfactory.bloodsmell"],
            )]),
            &sample_vocab(),
            &[derivation(
                CHARACTER_A,
                &["visual.bloodstain", "olfactory.bloodsmell"],
            )],
            &participants(&[CHARACTER_A]),
        )
        .unwrap();
        // Must not bare-paste "血迹血腥味"
        assert!(!prose.text.contains("血迹血腥味"));
        assert!(prose.text.contains("血迹"));
        assert!(prose.text.contains("血腥味"));
        assert!(prose.text.contains('，') || prose.text.contains('。'));
    }

    #[test]
    fn book_source_isolation_drops_minority_corpus() {
        let yaml = r#"
atmosphere:
  hlm-c001-01:
    text: "满脸黑血直挺挺躺着"
    tags: ["hlm", "尸体"]
  zhz-c001-01:
    text: "修竹千竿之后有个人影一闪"
    tags: ["zhz", "竹林"]
visual:
  bloodstain:
    text: "血迹"
    tags: ["crime"]
"#;
        let v = Vocab::load_from_str(yaml).unwrap();
        let ids = [
            "atmosphere.hlm-c001-01",
            "atmosphere.zhz-c001-01",
            "visual.bloodstain",
        ];
        let action = crate::models::StructuredAction {
            kind: crate::models::ActionKind::Inspect,
            target: Some("地面".into()),
            dialogue: None,
        };
        let prose = AssembledProse::assemble(
            &narrative(vec![(CHARACTER_A, action, &ids)]),
            &v,
            &[derivation(CHARACTER_A, &ids)],
            &participants(&[CHARACTER_A]),
        )
        .unwrap();
        // Tie hlm vs zhz (1 each) → first-seen wins (hlm); zhz stripped.
        assert!(prose.text.contains("满脸黑血"));
        assert!(!prose.text.contains("修竹千竿"));
        assert!(prose.text.contains("血迹"), "base vocab kept");
        assert_eq!(prose.stripped_refs, 1);
    }

    #[test]
    fn book_source_from_key_prefix_without_tag() {
        assert_eq!(book_source("hlm-c103-46", &[]), Some("hlm".into()));
        assert_eq!(book_source("bloodstain", &[]), None);
        assert_eq!(book_source("x", &["zhz".into()]), Some("zhz".into()));
    }

    #[test]
    fn verify_provenance_zero_for_clean_assemble() {
        let action = crate::models::StructuredAction {
            kind: crate::models::ActionKind::SitDown,
            target: None,
            dialogue: None,
        };
        let prose = AssembledProse::assemble(
            &narrative(vec![(CHARACTER_A, action, &["emotion.sorrow"])]),
            &sample_vocab(),
            &[derivation(CHARACTER_A, &["emotion.sorrow"])],
            &participants(&[CHARACTER_A]),
        )
        .unwrap();
        assert_eq!(prose.unverified_quotes, 0);
        assert_eq!(prose.quality.unverified_quotes, 0);
        assert!(prose.text.contains("心中未免悔恨"));
    }

    #[test]
    fn quality_report_aggregates_kpis() {
        // One clean quote beat + one stripped-ref action-only beat.
        let action1 = crate::models::StructuredAction {
            kind: crate::models::ActionKind::Inspect,
            target: Some("下方".into()),
            dialogue: None,
        };
        let action2 = crate::models::StructuredAction {
            kind: crate::models::ActionKind::Pause,
            target: None,
            dialogue: None,
        };
        let prose = AssembledProse::assemble(
            &narrative(vec![
                (CHARACTER_A, action1, &["emotion.sorrow"]),
                (CHARACTER_A, action2, &["emotion.fabricated"]),
            ]),
            &sample_vocab(),
            &[derivation(CHARACTER_A, &["emotion.sorrow"])],
            &participants(&[CHARACTER_A]),
        )
        .unwrap();
        assert_eq!(prose.stripped_refs, 1);
        assert_eq!(prose.action_only_beats, 1);
        assert_eq!(prose.unverified_quotes, 0);
        // 2 accepted beats, 1 action-only → rate 0.5
        assert!((prose.quality.action_only_rate - 0.5).abs() < 1e-9);
        // 2 refs seen, 1 stripped → rate 0.5
        assert!((prose.quality.stripped_ref_rate - 0.5).abs() < 1e-9);
        assert!((prose.quality.quote_density - prose.quote_density()).abs() < 1e-9);
    }

    #[test]
    fn low_quote_density_flag_when_sparse() {
        let action = crate::models::StructuredAction {
            kind: crate::models::ActionKind::Turn,
            target: Some("离开".into()),
            dialogue: None,
        };
        let prose = AssembledProse::assemble(
            &narrative(vec![(CHARACTER_A, action, &[])]),
            &sample_vocab(),
            &[derivation(CHARACTER_A, &[])],
            &participants(&[CHARACTER_A]),
        )
        .unwrap();
        assert_eq!(prose.quote_chars, 0);
        assert!(prose.quality.low_quote_density);
        assert!(prose.quote_density() < MIN_QUOTE_DENSITY);
    }
}
