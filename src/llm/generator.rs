use crate::llm::{LlmCharacterDerivation, LlmContextTagSelection};
use crate::models::{
    Character, CharacterMemory, Scene, SensorySelection, StoredPlotDevelopment, StoryError,
    VocabularyCandidate,
};

#[derive(Debug, Clone)]
pub struct ContextTagRequest {
    pub character: Character,
    pub scene: Scene,
    pub prior_plot_developments: Vec<StoredPlotDevelopment>,
    pub available_tags: Vec<String>,
}

/// 推导请求：调用 LLM 前需要注入的全部上下文
/// ================================================
/// 这个结构体打包了某个角色在某个场景中的完整状态，
/// 供 SenseGenerator::derive() 使用。
#[derive(Debug, Clone)]
pub struct DerivationRequest {
    pub character: Character,                     // 角色定义（名字/性格/技能）
    pub scene: Scene,                             // 当前客观场景
    pub recent_memories: Vec<CharacterMemory>,    // 该角色最近的记忆（上限 50 条）
    pub last_sensation: Option<SensorySelection>, // 上一场景的五感（用于连续性）
    pub candidates: Vec<VocabularyCandidate>,
    pub prior_plot_developments: Vec<StoredPlotDevelopment>,
}

/// 感官生成器抽象（Trait）
/// ============================
/// 生产实现（RigSenseGenerator）调用 DeepSeek 模型；
/// 测试实现（MockSenseGenerator）返回固定数据。
/// 泛型 + Send + Sync + async 使其在各种运行时中安全共享。
#[async_trait::async_trait]
pub trait SenseGenerator: Send + Sync {
    /// 对角色在场景中的感官/记忆/剧情进行 AI 推导
    ///
    /// # 参数
    /// - `req` — 包含角色、场景、记忆、候选词等全部上下文
    ///
    /// # 返回
    /// - `Ok(LlmCharacterDerivation)` — 结构化推导结果
    /// - `Err(StoryError::Llm(...))` — LLM 调用失败
    async fn derive(&self, req: &DerivationRequest) -> Result<LlmCharacterDerivation, StoryError>;

    async fn select_context_tags(
        &self,
        req: &ContextTagRequest,
    ) -> Result<LlmContextTagSelection, StoryError>;
}
