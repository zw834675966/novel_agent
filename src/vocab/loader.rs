use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};

use crate::llm::VocabularyCandidate;

/// 词库条目
/// ============
/// 对应 YAML 中每个感官类别下的每个键值对。
/// 如 YAML 中 "visual.bloodstain" 对应的值：
///   text: "血迹"
///   tags: ["injury", "crime"]
#[derive(Debug, Clone, Deserialize)]
pub struct VocabEntry {
    pub text: String, // 显示文本（中文，如"血迹"）
    #[serde(default)]
    pub tags: Vec<String>, // 标签列表（用于过滤，如 ["injury", "crime"]）
}

/// 词库文件顶层结构
/// ===================
/// 对应 YAML 文件的顶级字段，按五感分类。
/// 每个分类是一个 HashMap<String, VocabEntry>，键为词汇 ID。
#[derive(Debug, Clone, Deserialize, Default)]
pub struct VocabFile {
    #[serde(default)]
    pub visual: HashMap<String, VocabEntry>, // 视觉
    #[serde(default)]
    pub auditory: HashMap<String, VocabEntry>, // 听觉
    #[serde(default)]
    pub olfactory: HashMap<String, VocabEntry>, // 嗅觉
    #[serde(default)]
    pub tactile: HashMap<String, VocabEntry>, // 触觉
    #[serde(default)]
    pub gustatory: HashMap<String, VocabEntry>, // 味觉
    #[serde(default)]
    pub emotion: HashMap<String, VocabEntry>, // 情绪心理
    #[serde(default)]
    pub gesture: HashMap<String, VocabEntry>, // 动作神态
    #[serde(default)]
    pub atmosphere: HashMap<String, VocabEntry>, // 氛围环境
}

/// 全部感官/描写类别(候选集与校验共用同一份定义,防止遗漏)
pub const SENSES: [&str; 8] = [
    "visual",
    "auditory",
    "olfactory",
    "tactile",
    "gustatory",
    "emotion",
    "gesture",
    "atmosphere",
];

/// 每个 sense 类别最多进入 prompt 的候选数（确定性 top-k）
pub const DEFAULT_PER_SENSE_CAP: usize = 24;
/// 候选总数上限（跨 sense 合计）
pub const DEFAULT_TOTAL_CAP: usize = 96;
/// 传给 LLM 的 known tags 上限
pub const DEFAULT_TAG_CAP: usize = 80;

/// Weak-sense floor: each of auditory/olfactory/tactile/gustatory gets at least
/// this many candidates (if available) in quota retrieval.
pub const WEAK_SENSE_FLOOR: usize = 5;
/// Per-sense cap for gesture/emotion in quota retrieval.
pub const GESTURE_EMOTION_CAP: usize = 15;
/// Senses that receive floor protection in quota retrieval.
pub const WEAK_SENSES: [&str; 4] = ["auditory", "olfactory", "tactile", "gustatory"];

/// 词库（已加载状态）
/// =====================
/// 提供按感官类别查询、按标签过滤、生成候选集等功能。
#[derive(Debug, Clone)]
pub struct Vocab {
    pub file: VocabFile,
}

/// BM25 scores for a candidate pool (zero-dependency IR ranking).
///
/// Documents = each candidate's `text` + tags. Query terms use **whole-term
/// substring TF** (`text.matches(term).count()` + exact tag hit), not char-level
/// tokenization, so multi-char names like 「宝玉」 stay atomic.
struct Bm25Scores {
    /// candidate id → BM25 score (0 if missing / empty query)
    scores: HashMap<String, f64>,
}

impl Bm25Scores {
    const K1: f64 = 1.2;
    const B: f64 = 0.75;

    fn build(candidates: &[VocabularyCandidate], query_terms: &[String]) -> Self {
        let terms: Vec<&str> = query_terms
            .iter()
            .map(|q| q.trim())
            .filter(|q| !q.is_empty())
            .collect();
        if terms.is_empty() || candidates.is_empty() {
            return Self {
                scores: HashMap::new(),
            };
        }

        let n = candidates.len() as f64;
        let lens: Vec<f64> = candidates.iter().map(bm25_doc_len).collect();
        let avgdl = (lens.iter().sum::<f64>() / n).max(1e-9);

        let mut dfs = vec![0usize; terms.len()];
        for c in candidates {
            for (i, term) in terms.iter().enumerate() {
                if bm25_term_tf(c, term) > 0.0 {
                    dfs[i] += 1;
                }
            }
        }
        let idfs: Vec<f64> = dfs
            .iter()
            .map(|&df| {
                let df = df as f64;
                ((n - df + 0.5) / (df + 0.5) + 1.0).ln()
            })
            .collect();

        let mut scores = HashMap::with_capacity(candidates.len());
        for (di, c) in candidates.iter().enumerate() {
            let mut s = 0.0_f64;
            let dl = lens[di];
            for (i, term) in terms.iter().enumerate() {
                let tf = bm25_term_tf(c, term);
                if tf <= 0.0 {
                    continue;
                }
                let denom = tf + Self::K1 * (1.0 - Self::B + Self::B * dl / avgdl);
                s += idfs[i] * (tf * (Self::K1 + 1.0)) / denom;
            }
            scores.insert(c.id.clone(), s);
        }
        Self { scores }
    }

    fn get(&self, id: &str) -> f64 {
        self.scores.get(id).copied().unwrap_or(0.0)
    }
}

/// Scene focus weighting: query terms matching sensory keywords boost related senses.
///
/// Example: query containing "血" boosts `visual` and `olfactory`; "冷" boosts
/// `tactile`. This counteracts corpus imbalance where gesture/emotion dominate.
struct SenseFocus {
    boosts: HashMap<&'static str, i64>,
}

/// Keyword -> sense mapping for focus weighting.
const FOCUS_MAP: &[(&str, &str)] = &[
    ("血", "visual"),
    ("血", "olfactory"),
    ("暗", "visual"),
    ("黑", "visual"),
    ("冷", "tactile"),
    ("寒", "tactile"),
    ("热", "tactile"),
    ("湿", "tactile"),
    ("香", "olfactory"),
    ("臭", "olfactory"),
    ("腥", "olfactory"),
    ("味", "gustatory"),
    ("苦", "gustatory"),
    ("甜", "gustatory"),
    ("声", "auditory"),
    ("响", "auditory"),
    ("光", "visual"),
    ("亮", "visual"),
    // Scene / weather / body cues (anti-AI sense focus)
    ("雨", "visual"),
    ("雨", "auditory"),
    ("雨", "tactile"),
    ("风", "auditory"),
    ("风", "tactile"),
    ("火", "visual"),
    ("火", "tactile"),
    ("烟", "visual"),
    ("烟", "olfactory"),
    ("酒", "olfactory"),
    ("酒", "gustatory"),
    ("茶", "olfactory"),
    ("茶", "gustatory"),
    ("哭", "auditory"),
    ("笑", "auditory"),
    ("门", "auditory"),
    ("门", "visual"),
    ("脚步", "auditory"),
    ("汗", "olfactory"),
    ("汗", "tactile"),
    ("雾", "visual"),
    ("雾", "tactile"),
    ("雪", "visual"),
    ("雪", "tactile"),
    ("花", "visual"),
    ("花", "olfactory"),
    ("雷", "auditory"),
    ("灯", "visual"),
    ("水", "visual"),
    ("水", "auditory"),
    ("夜", "visual"),
];

impl SenseFocus {
    const BOOST: i64 = 10;

    fn from_query(query_terms: &[String]) -> Self {
        let mut boosts: HashMap<&'static str, i64> = HashMap::new();
        for q in query_terms {
            let q = q.trim();
            if q.is_empty() {
                continue;
            }
            for (keyword, sense) in FOCUS_MAP {
                if q.contains(keyword) {
                    *boosts.entry(sense).or_insert(0) += Self::BOOST;
                }
            }
        }
        Self { boosts }
    }

    fn boost(&self, sense: &str) -> i64 {
        self.boosts.get(sense).copied().unwrap_or(0)
    }
}

/// Document length proxy: chars in text + chars in tags.
fn bm25_doc_len(c: &VocabularyCandidate) -> f64 {
    let tag_chars: usize = c.tags.iter().map(|t| t.chars().count()).sum();
    (c.text.chars().count() + tag_chars) as f64
}

/// Whole-term TF: non-overlapping substring hits in text + 1 if any tag == term.
fn bm25_term_tf(c: &VocabularyCandidate, term: &str) -> f64 {
    if term.is_empty() {
        return 0.0;
    }
    let mut tf = c.text.matches(term).count() as f64;
    if c.tags.iter().any(|t| t == term) {
        tf += 1.0;
    }
    tf
}

impl Vocab {
    /// 从 YAML 字符串加载词库
    pub fn load_from_str(s: &str) -> Result<Self, super::super::models::StoryError> {
        let file: VocabFile = serde_yaml::from_str(s)
            .map_err(|e| super::super::models::StoryError::VocabularyLoad(e.to_string()))?;
        Ok(Self { file })
    }

    /// 从 YAML 文件路径加载词库
    pub fn load_from_path(p: &std::path::Path) -> Result<Self, super::super::models::StoryError> {
        let s = std::fs::read_to_string(p)
            .map_err(|e| super::super::models::StoryError::VocabularyLoad(e.to_string()))?;
        Self::load_from_str(&s)
    }

    /// 获取某感官类别的所有条目（返回引用，不拷贝）
    pub fn entries(&self, sense: &str) -> Option<&HashMap<String, VocabEntry>> {
        match sense {
            "visual" => Some(&self.file.visual),
            "auditory" => Some(&self.file.auditory),
            "olfactory" => Some(&self.file.olfactory),
            "tactile" => Some(&self.file.tactile),
            "gustatory" => Some(&self.file.gustatory),
            "emotion" => Some(&self.file.emotion),
            "gesture" => Some(&self.file.gesture),
            "atmosphere" => Some(&self.file.atmosphere),
            _ => None,
        }
    }

    /// 检查某个 sense.key 是否存在
    pub fn has(&self, sense: &str, key: &str) -> bool {
        self.entries(sense)
            .map(|m| m.contains_key(key))
            .unwrap_or(false)
    }

    /// 按感官类别和标签过滤，返回匹配的 VocabularyId 列表
    ///
    /// # 参数
    /// - `sense` — 五感类别名称
    /// - `tags`  — 目标标签（指定标签时只返回同时匹配的条目；为空时返回全部）
    ///
    /// # 返回
    /// 格式如 ["visual.bloodstain", "visual.candlelight"]
    pub fn candidates(&self, sense: &str, tags: &[&str]) -> Vec<crate::models::VocabularyId> {
        let Some(map) = self.entries(sense) else {
            return vec![];
        };
        map.iter()
            .filter_map(|(k, e)| {
                if tags.is_empty() || tags.iter().any(|t| e.tags.iter().any(|et| et == t)) {
                    crate::models::VocabularyId::new(&format!("{sense}.{k}")).ok()
                } else {
                    None
                }
            })
            .collect()
    }

    /// 生成全感官候选集（HashSet<String>，用于快速校验 LLM 输出）
    ///
    /// # 用途
    /// 将结果传给 VocabularyId::new() 的 validate 函数，
    /// 检查 LLM 输出的每个 ID 是否在候选集中。
    pub fn candidate_set(&self, tags: &[&str]) -> std::collections::HashSet<String> {
        let mut set = std::collections::HashSet::new();
        for sense in SENSES {
            for id in self.candidates(sense, tags) {
                set.insert(id.as_str().to_string());
            }
        }
        set
    }

    /// Returns every vocabulary tag in stable order.
    pub fn known_tags(&self) -> Vec<String> {
        SENSES
            .iter()
            .flat_map(|sense| {
                self.entries(sense)
                    .into_iter()
                    .flat_map(|entries| entries.values())
            })
            .flat_map(|entry| entry.tags.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Retains known tags, removing duplicates and returning stable order.
    pub fn filter_known_tags(&self, tags: &[String]) -> Vec<String> {
        let known = self.known_tags().into_iter().collect::<BTreeSet<_>>();
        tags.iter()
            .filter(|tag| known.contains(*tag))
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Produces semantic candidates for selected tags, falling back to all entries on no match.
    pub fn candidates_for_tags(&self, selected: &[String]) -> Vec<VocabularyCandidate> {
        let matched = self.collect_candidates(Some(selected));
        if matched.is_empty() {
            self.collect_candidates(None)
        } else {
            matched
        }
    }

    /// Returns known tags truncated to `max` (stable order from [`Self::known_tags`]).
    pub fn known_tags_limited(&self, max: usize) -> Vec<String> {
        self.known_tags_ranked_limited(&[], max)
    }

    /// Score a vocabulary tag for shortlist relevance (higher is better).
    pub fn score_tag(tag: &str, query_terms: &[String]) -> i64 {
        let mut score: i64 = 0;
        for q in query_terms {
            let q = q.trim();
            if q.is_empty() {
                continue;
            }
            let weight = (q.chars().count() as i64).clamp(1, 8);
            if tag == q {
                score += 50 * weight;
            } else if tag.contains(q) || q.contains(tag) {
                score += 15 * weight;
            }
        }
        score
    }

    /// Rank tags by query relevance then take top `max` (deterministic).
    pub fn known_tags_ranked_limited(&self, query_terms: &[String], max: usize) -> Vec<String> {
        let mut tags = self.known_tags();
        tags.sort_by(|a, b| {
            let sa = Self::score_tag(a, query_terms);
            let sb = Self::score_tag(b, query_terms);
            sb.cmp(&sa).then_with(|| a.cmp(b))
        });
        if tags.len() > max {
            tags.truncate(max);
        }
        tags
    }

    /// Like [`Self::candidates_for_tags`], then applies per-sense and total caps.
    ///
    /// When `query_terms` is empty, ranking is score-0 with id order (stable).
    /// Prefer [`Self::candidates_ranked_limited`] when scene/character context exists.
    pub fn candidates_for_tags_limited(
        &self,
        selected: &[String],
        per_sense: usize,
        total_max: usize,
    ) -> Vec<VocabularyCandidate> {
        self.candidates_ranked_limited(selected, &[], per_sense, total_max)
    }

    /// Selected-tag hits + exact name/tag voice isolation (critique P0/P1).
    /// Higher is better. Pure function; deterministic.
    ///
    /// Text/substring relevance is scored by BM25 in [`Self::candidates_ranked_limited`];
    /// this function only adds discrete boosts that BM25 should not replace
    /// (selected-tag preference + exact character-name tag isolation).
    pub fn score_candidate(
        c: &VocabularyCandidate,
        selected: &[String],
        query_terms: &[String],
    ) -> i64 {
        let mut score: i64 = 0;
        for tag in &c.tags {
            if selected.iter().any(|s| s == tag) {
                score += 20;
            }
        }
        for q in query_terms {
            let q = q.trim();
            if q.is_empty() {
                continue;
            }
            let weight = (q.chars().count() as i64).clamp(1, 8);
            // Exact character-name / term on tag → strong voice boost (P1).
            if c.tags.iter().any(|t| t == q) {
                score += 40 * weight;
            }
        }
        score
    }

    /// BM25 base (×1000, rounded) + [`Self::score_candidate`] boosts.
    /// Used by ranked selection; exposed for tests.
    pub fn score_candidate_with_bm25(
        c: &VocabularyCandidate,
        selected: &[String],
        query_terms: &[String],
        bm25: f64,
    ) -> i64 {
        let base = (bm25 * 1000.0).round() as i64;
        base + Self::score_candidate(c, selected, query_terms)
    }

    /// Rank by BM25 + voice/selected boosts (DESC), then SENSES order + id (ASC); apply caps.
    ///
    /// Builds a zero-dependency BM25 index over the candidate pool (text + tags as docs,
    /// whole-term substring TF). Empty `query_terms` → BM25=0, order falls back to
    /// selected-tag boosts then stable sense/id order.
    pub fn candidates_ranked_limited(
        &self,
        selected: &[String],
        query_terms: &[String],
        per_sense: usize,
        total_max: usize,
    ) -> Vec<VocabularyCandidate> {
        let mut all = self.candidates_for_tags(selected);
        let bm25 = Bm25Scores::build(&all, query_terms);
        all.sort_by(|a, b| {
            let sa = Self::score_candidate_with_bm25(a, selected, query_terms, bm25.get(&a.id));
            let sb = Self::score_candidate_with_bm25(b, selected, query_terms, bm25.get(&b.id));
            // Score DESC; ties break by SENSES order then id (stable with prior dict-cap tests).
            sb.cmp(&sa).then_with(|| {
                let ia = SENSES
                    .iter()
                    .position(|s| *s == a.sense)
                    .unwrap_or(usize::MAX);
                let ib = SENSES
                    .iter()
                    .position(|s| *s == b.sense)
                    .unwrap_or(usize::MAX);
                ia.cmp(&ib).then_with(|| a.id.cmp(&b.id))
            })
        });
        let mut out = Vec::new();
        let mut per: HashMap<String, usize> = HashMap::new();
        for c in all.drain(..) {
            let n = per.entry(c.sense.clone()).or_insert(0);
            if *n >= per_sense {
                continue;
            }
            *n += 1;
            out.push(c);
            if out.len() >= total_max {
                break;
            }
        }
        out
    }

    /// Like [`Self::candidates_ranked_limited`] but with **sensory quota** enforcement:
    ///
    /// - Weak senses (auditory/olfactory/tactile/gustatory) get a **floor** of
    ///   [`WEAK_SENSE_FLOOR`] candidates each (if available), preventing gesture/emotion
    ///   dominance from starving them.
    /// - `gesture` and `emotion` are each capped at [`GESTURE_EMOTION_CAP`].
    /// - Scene **focus weighting**: query terms matching keyword triggers (血/暗/冷...)
    ///   boost the score of related senses (visual/olfactory/tactile).
    /// - Total never exceeds `total_max`; stable sort score DESC -> SENSES -> id.
    pub fn candidates_ranked_limited_with_quotas(
        &self,
        selected: &[String],
        query_terms: &[String],
        per_sense_cap: usize,
        total_max: usize,
    ) -> Vec<VocabularyCandidate> {
        let mut all = self.candidates_for_tags(selected);
        let bm25 = Bm25Scores::build(&all, query_terms);
        let focus = SenseFocus::from_query(query_terms);

        all.sort_by(|a, b| {
            let sa = Self::score_candidate_with_bm25(a, selected, query_terms, bm25.get(&a.id))
                + focus.boost(&a.sense);
            let sb = Self::score_candidate_with_bm25(b, selected, query_terms, bm25.get(&b.id))
                + focus.boost(&b.sense);
            sb.cmp(&sa).then_with(|| {
                let ia = SENSES
                    .iter()
                    .position(|s| *s == a.sense)
                    .unwrap_or(usize::MAX);
                let ib = SENSES
                    .iter()
                    .position(|s| *s == b.sense)
                    .unwrap_or(usize::MAX);
                ia.cmp(&ib).then_with(|| a.id.cmp(&b.id))
            })
        });

        // Count available per weak sense for floor computation.
        let mut available: HashMap<&str, usize> = HashMap::new();
        for c in &all {
            *available.entry(c.sense.as_str()).or_insert(0) += 1;
        }

        let mut out: Vec<VocabularyCandidate> = Vec::new();
        let mut per: HashMap<String, usize> = HashMap::new();
        let mut taken: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Phase 1: reserve weak-sense floor (top-scored per weak sense).
        for ws in WEAK_SENSES {
            let floor = WEAK_SENSE_FLOOR.min(available.get(ws).copied().unwrap_or(0));
            let mut count = 0;
            for c in &all {
                if count >= floor {
                    break;
                }
                if c.sense == ws && !taken.contains(&c.id) {
                    taken.insert(c.id.clone());
                    out.push(c.clone());
                    count += 1;
                }
            }
            per.insert(ws.to_string(), count);
        }

        // Phase 2: fill remaining capacity with all candidates (score order).
        for c in &all {
            if out.len() >= total_max {
                break;
            }
            if taken.contains(&c.id) {
                continue;
            }
            let cap = if c.sense == "gesture" || c.sense == "emotion" {
                GESTURE_EMOTION_CAP
            } else {
                per_sense_cap
            };
            let n = per.entry(c.sense.clone()).or_insert(0);
            if *n >= cap {
                continue;
            }
            *n += 1;
            taken.insert(c.id.clone());
            out.push(c.clone());
        }

        // Re-sort output for stability: score DESC -> SENSES -> id.
        out.sort_by(|a, b| {
            let sa = Self::score_candidate_with_bm25(a, selected, query_terms, bm25.get(&a.id))
                + focus.boost(&a.sense);
            let sb = Self::score_candidate_with_bm25(b, selected, query_terms, bm25.get(&b.id))
                + focus.boost(&b.sense);
            sb.cmp(&sa).then_with(|| {
                let ia = SENSES
                    .iter()
                    .position(|s| *s == a.sense)
                    .unwrap_or(usize::MAX);
                let ib = SENSES
                    .iter()
                    .position(|s| *s == b.sense)
                    .unwrap_or(usize::MAX);
                ia.cmp(&ib).then_with(|| a.id.cmp(&b.id))
            })
        });

        out
    }

    fn collect_candidates(&self, selected: Option<&[String]>) -> Vec<VocabularyCandidate> {
        let mut candidates = Vec::new();

        for sense in SENSES {
            let Some(entries) = self.entries(sense) else {
                continue;
            };
            let mut keys = entries.keys().collect::<Vec<_>>();
            keys.sort_unstable();

            for key in keys {
                let entry = &entries[key];
                if selected
                    .is_none_or(|selected| entry.tags.iter().any(|tag| selected.contains(tag)))
                {
                    candidates.push(VocabularyCandidate {
                        id: format!("{sense}.{key}"),
                        sense: sense.to_string(),
                        text: entry.text.clone(),
                        tags: entry.tags.clone(),
                    });
                }
            }
        }

        candidates
    }

    /// 合并另一份词库(蒸馏素材库叠加到手写基础词库上;键冲突时以 other 为准)
    pub fn merge(&mut self, other: Vocab) {
        self.file.visual.extend(other.file.visual);
        self.file.auditory.extend(other.file.auditory);
        self.file.olfactory.extend(other.file.olfactory);
        self.file.tactile.extend(other.file.tactile);
        self.file.gustatory.extend(other.file.gustatory);
        self.file.emotion.extend(other.file.emotion);
        self.file.gesture.extend(other.file.gesture);
        self.file.atmosphere.extend(other.file.atmosphere);
    }

    /// 从目录加载所有 *.yaml 并合并(用于 assets/distilled/ 素材库)
    pub fn load_dir_merged(
        &mut self,
        dir: &std::path::Path,
    ) -> Result<usize, super::super::models::StoryError> {
        let mut n = 0;
        let entries = std::fs::read_dir(dir)
            .map_err(|e| super::super::models::StoryError::VocabularyLoad(e.to_string()))?;
        let mut paths: Vec<_> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("yaml"))
            .collect();
        paths.sort();
        for p in paths {
            self.merge(Self::load_from_path(&p)?);
            n += 1;
        }
        Ok(n)
    }
}
