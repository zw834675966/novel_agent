use crate::llm::{
    ContextTagRequest, DerivationRequest, LlmCharacterDerivation, LlmContextTagSelection,
    SenseGenerator,
};
use crate::models::StoryError;

/// Mock 感官生成器（测试用）
/// ============================
/// 忽略输入的 DerivationRequest，始终返回预设的 `canned` 数据。
/// 用于：
///   - 单元测试（不依赖 LLM API）
///   - 环境无 DEEPSEEK_API_KEY 时的降级运行
pub struct MockSenseGenerator {
    pub canned: LlmCharacterDerivation,
}

impl MockSenseGenerator {
    /// 创建 Mock 生成器
    ///
    /// # 参数
    /// - `canned` — 固定的推导结果，每次 derive() 都返回这个值
    pub fn new(canned: LlmCharacterDerivation) -> Self {
        Self { canned }
    }
}

#[async_trait::async_trait]
impl SenseGenerator for MockSenseGenerator {
    /// 忽略请求上下文，直接返回预设数据
    async fn derive(&self, _req: &DerivationRequest) -> Result<LlmCharacterDerivation, StoryError> {
        Ok(self.canned.clone())
    }

    async fn select_context_tags(
        &self,
        _req: &ContextTagRequest,
    ) -> Result<LlmContextTagSelection, StoryError> {
        Ok(LlmContextTagSelection::default())
    }
}
