use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};

use crate::models::VocabularyCandidate;

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

impl VocabFile {
    /// 返回 8 个感官 HashMap 的 `(sense_name, &map)` 数组，用于统一迭代。
    pub(crate) fn sense_maps(&self) -> [(&'static str, &HashMap<String, VocabEntry>); 8] {
        [
            ("visual", &self.visual),
            ("auditory", &self.auditory),
            ("olfactory", &self.olfactory),
            ("tactile", &self.tactile),
            ("gustatory", &self.gustatory),
            ("emotion", &self.emotion),
            ("gesture", &self.gesture),
            ("atmosphere", &self.atmosphere),
        ]
    }

    /// 返回 8 个感官 HashMap 的可变引用数组，用于统一写入。
    pub(crate) fn sense_maps_mut(
        &mut self,
    ) -> [(&'static str, &mut HashMap<String, VocabEntry>); 8] {
        [
            ("visual", &mut self.visual),
            ("auditory", &mut self.auditory),
            ("olfactory", &mut self.olfactory),
            ("tactile", &mut self.tactile),
            ("gustatory", &mut self.gustatory),
            ("emotion", &mut self.emotion),
            ("gesture", &mut self.gesture),
            ("atmosphere", &mut self.atmosphere),
        ]
    }
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

/// Position of `sense` in SENSES, or `usize::MAX` if unknown.
/// Replaces 6 inline copies of `SENSES.iter().position(...).unwrap_or(usize::MAX)`.
pub(crate) fn sense_order(sense: &str) -> usize {
    SENSES
        .iter()
        .position(|s| *s == sense)
        .unwrap_or(usize::MAX)
}

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
    /// candidate id -> BM25 score (0 if missing / empty query)
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
        self.file
            .sense_maps()
            .into_iter()
            .find(|(s, _)| *s == sense)
            .map(|(_, m)| m)
    }

    /// 检查某个 sense.key 是否存在
    pub fn has(&self, sense: &str, key: &str) -> bool {
        self.entries(sense)
            .map(|m| m.contains_key(key))
            .unwrap_or(false)
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

    /// Score a vocabulary tag for shortlist relevance (higher is better).
    pub(crate) fn score_tag(tag: &str, query_terms: &[String]) -> i64 {
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

    /// Selected-tag hits + exact name/tag voice isolation (critique P0/P1).
    /// Higher is better. Pure function; deterministic.
    ///
    /// Text/substring relevance is scored by BM25 in [`Self::candidates_ranked_limited`];
    /// this function only adds discrete boosts that BM25 should not replace
    /// (selected-tag preference + exact character-name tag isolation).
    pub(crate) fn score_candidate(
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
            // Exact character-name / term on tag -> strong voice boost (P1).
            if c.tags.iter().any(|t| t == q) {
                score += 40 * weight;
            }
        }
        score
    }

    /// BM25 base (×1000, rounded) + [`Self::score_candidate`] boosts.
    /// Used by ranked selection; exposed for tests.
    pub(crate) fn score_candidate_with_bm25(
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
    /// whole-term substring TF). Empty `query_terms` -> BM25=0, order falls back to
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
                let ia = sense_order(&a.sense);
                let ib = sense_order(&b.sense);
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
    pub fn merge(&mut self, mut other: Vocab) {
        for ((_, dst), (_, src)) in self
            .file
            .sense_maps_mut()
            .into_iter()
            .zip(other.file.sense_maps_mut())
        {
            dst.extend(std::mem::take(src));
        }
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
