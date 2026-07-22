use crate::models::StoryError;

use super::contract::{LlmNarrative, NarrativeBeat};
use super::generator::{NarrateRequest, ProseGenerator};

/// 测试用 mock 叙事生成器
pub struct MockProseGenerator {
    response: LlmNarrative,
    is_fallback: bool,
}

impl MockProseGenerator {
    pub fn new(response: LlmNarrative) -> Self {
        Self {
            response,
            is_fallback: false,
        }
    }

    pub fn fallback() -> Self {
        Self {
            response: LlmNarrative { beats: vec![] },
            is_fallback: true,
        }
    }
}

#[async_trait::async_trait]
impl ProseGenerator for MockProseGenerator {
    async fn narrate(&self, req: &NarrateRequest) -> Result<LlmNarrative, StoryError> {
        if !self.is_fallback {
            return Ok(self.response.clone());
        }

        let Some(first) = req.characters.first() else {
            return Ok(LlmNarrative { beats: vec![] });
        };
        let pov = first.id.0.to_string();

        let sensation_refs: Vec<String> = req
            .candidates
            .iter()
            .find(|c| c.character_id == first.id)
            .and_then(|c| c.candidates.first())
            .map(|c| vec![c.id.clone()])
            .unwrap_or_default();

        Ok(LlmNarrative {
            beats: vec![NarrativeBeat {
                pov,
                action: crate::models::StructuredAction {
                    kind: crate::models::ActionKind::Enter,
                    target: Some(req.scene.objective_event.clone()),
                    dialogue: None,
                },
                sensation_refs,
            }],
        })
    }
}
