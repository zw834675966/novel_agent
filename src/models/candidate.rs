/// 候选描写片段（语义版，携带 sense/text/tags）
/// ============================================
/// 统一类型：llm 模块的 DerivationRequest 与 prose 模块的 NarrateRequest
/// 共用此结构，消除 VocabularyCandidate/ProseCandidate 重复。
#[derive(Debug, Clone)]
pub struct VocabularyCandidate {
    pub id: String,
    pub sense: String,
    pub text: String,
    pub tags: Vec<String>,
}
