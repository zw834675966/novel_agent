use crate::llm::{DerivationRequest, LlmCharacterDerivation, SenseGenerator};
use crate::models::StoryError;

pub struct MockSenseGenerator {
    pub canned: LlmCharacterDerivation,
}

impl MockSenseGenerator {
    pub fn new(canned: LlmCharacterDerivation) -> Self {
        Self { canned }
    }
}

#[async_trait::async_trait]
impl SenseGenerator for MockSenseGenerator {
    async fn derive(&self, _req: &DerivationRequest) -> Result<LlmCharacterDerivation, StoryError> {
        Ok(self.canned.clone())
    }
}
