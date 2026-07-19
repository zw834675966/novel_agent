use crate::models::{SensorySelection, VocabularyId};
use std::collections::HashSet;

/// 校验结果
/// ============
/// 记录哪些词汇通过校验（cleaned）、哪些被移除（stripped）、
/// 以及是否全部为空（all_empty，用于触发重试）。
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub cleaned: SensorySelection, // 通过校验的合法词汇
    pub stripped: Vec<String>,     // 被移除的非法词汇 ID
    pub all_empty: bool,           // 全部维度都为空 → 需要重试
}

/// 校验 LLM 输出的感官选择是否符合候选集
/// =========================================
///
/// # 规则
/// - 如果 ID 在候选集中 → 保留
/// - 如果 ID 不在候选集中 → 移入 stripped 列表
/// - 如果某维度全部被移除 → 该维度为空
/// - 如果所有维度都为空 → all_empty = true
///
/// # 设计理由
/// LLM（尤其是工具调用模式下）偶尔会"编造"不存在的词汇 ID。
/// 这一步是安全护栏：防止非法数据进入数据库。
pub fn validate_selection(
    sel: &SensorySelection,
    candidates: &HashSet<String>,
) -> ValidationResult {
    let mut cleaned = SensorySelection::default();
    let mut stripped = Vec::new();

    // 局部函数：校验单维度列表
    let mut clean = |ids: &[VocabularyId], prefix: &str| -> Vec<VocabularyId> {
        ids.iter()
            .filter_map(|id| {
                if id.as_str().starts_with(prefix) && candidates.contains(id.as_str()) {
                    Some(id.clone())
                } else {
                    stripped.push(id.as_str().to_string());
                    None
                }
            })
            .collect()
    };

    cleaned.visual_ids = clean(&sel.visual_ids, "visual.");
    cleaned.auditory_ids = clean(&sel.auditory_ids, "auditory.");
    cleaned.olfactory_ids = clean(&sel.olfactory_ids, "olfactory.");
    cleaned.tactile_ids = clean(&sel.tactile_ids, "tactile.");
    cleaned.gustatory_ids = clean(&sel.gustatory_ids, "gustatory.");

    let all_empty = cleaned.visual_ids.is_empty()
        && cleaned.auditory_ids.is_empty()
        && cleaned.olfactory_ids.is_empty()
        && cleaned.tactile_ids.is_empty()
        && cleaned.gustatory_ids.is_empty();

    ValidationResult {
        cleaned,
        stripped,
        all_empty,
    }
}
