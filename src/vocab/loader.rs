use serde::Deserialize;
use std::collections::HashMap;

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
}

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
        for sense in ["visual", "auditory", "olfactory", "tactile", "gustatory"] {
            for id in self.candidates(sense, tags) {
                set.insert(id.as_str().to_string());
            }
        }
        set
    }
}
