use crate::llm::LlmCharacterDerivation;
use crate::models::{Character, CharacterMemory, Scene, SensorySelection, StoryError};
use std::collections::HashSet;

/// 推导请求：注入到 SenseGenerator 的上下文。
#[derive(Debug, Clone)]
pub struct DerivationRequest {
    pub character: Character,
    pub scene: Scene,
    pub recent_memories: Vec<CharacterMemory>,
    pub last_sensation: Option<SensorySelection>,
    pub candidate_ids: HashSet<String>,
    pub candidate_tags: Vec<String>,
}

/// 感官生成器抽象。生产实现包装 rig Extractor；测试实现返回固定数据。
#[async_trait::async_trait]
pub trait SenseGenerator: Send + Sync {
    async fn derive(&self, req: &DerivationRequest) -> Result<LlmCharacterDerivation, StoryError>;
}
