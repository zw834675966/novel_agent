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

/// 词库（已加载状态）
/// =====================
/// 提供按感官类别查询、按标签过滤、生成候选集等功能。
#[derive(Debug, Clone)]
pub struct Vocab {
    pub file: VocabFile,
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

    /// Lexical score for scene/character-aware ranking (critique P0/P1).
    /// Higher is better. Pure function; deterministic.
    ///
    /// Voice isolation: exact tag == query term (e.g. character name) gets a large boost
    /// so other characters' fragments rank lower when name is in query_terms.
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
            } else if c
                .tags
                .iter()
                .any(|t| t.contains(q) || q.contains(t.as_str()))
            {
                score += 12 * weight;
            }
            if c.text.contains(q) {
                score += 8 * weight;
            }
        }
        score
    }

    /// Rank by [`Self::score_candidate`] (DESC) then id (ASC), then apply caps.
    pub fn candidates_ranked_limited(
        &self,
        selected: &[String],
        query_terms: &[String],
        per_sense: usize,
        total_max: usize,
    ) -> Vec<VocabularyCandidate> {
        let mut all = self.candidates_for_tags(selected);
        all.sort_by(|a, b| {
            let sa = Self::score_candidate(a, selected, query_terms);
            let sb = Self::score_candidate(b, selected, query_terms);
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
