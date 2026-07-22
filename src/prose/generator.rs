use crate::models::{Character, CharacterDerivation, CharacterId, Scene, StoryError};

use super::contract::LlmNarrative;

/// 候选描写片段(语义版,携带 sense/text/tags)
/// ============================================
/// 与 `VocabularyCandidate` 同构,但归属 prose 模块。
/// LLM 通过引用 `id` 字段(`"<sense>.<key>"`)拉取原著原文。
#[derive(Debug, Clone)]
pub struct ProseCandidate {
    pub id: String,
    pub sense: String,
    pub text: String,
    pub tags: Vec<String>,
}

/// 单个角色的全部候选片段
/// ========================
/// `candidates` 字段携带该角色在本场景 derivation 中选出的全部描写片段,
/// 供 LLM 在 prompt 中看到原文并按 id 引用。
#[derive(Debug, Clone)]
pub struct CharacterProseCandidates {
    pub character_id: CharacterId,
    pub candidates: Vec<ProseCandidate>,
}

/// 叙事编排请求(语义版)
/// =======================
/// 打包一个场景的完整结构化上下文,供 `ProseGenerator::narrate` 使用。
/// `candidates` 按 `characters` 顺序对齐,每角色一组语义候选片段。
#[derive(Debug, Clone)]
pub struct NarrateRequest {
    pub scene: Scene,
    pub characters: Vec<Character>,
    pub derivations: Vec<CharacterDerivation>,
    pub candidates: Vec<CharacterProseCandidates>,
}

/// 叙事生成器抽象
/// =================
/// 与 `SenseGenerator` 对称:生产用 rig(DeepSeek),测试用 mock。
/// 正文编排测试不依赖网络。
#[async_trait::async_trait]
pub trait ProseGenerator: Send + Sync {
    async fn narrate(&self, req: &NarrateRequest) -> Result<LlmNarrative, StoryError>;
}
