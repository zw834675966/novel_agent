use crate::models::StoryError;

use super::contract::{LlmNarrative, NarrativeBeat};
use super::generator::{NarrateRequest, ProseGenerator};

/// 测试用 mock 叙事生成器
/// ========================
/// `new` -> 返回固定 `LlmNarrative`。
/// `fallback` -> 根据请求确定性地产出一个 beat(首角色 + 场景事件 + 首候选 ID),
///               无角色时返回空 beat 列表,让 service 层的硬错误兜底。
pub struct MockProseGenerator {
    response: LlmNarrative,
    is_fallback: bool,
}

impl MockProseGenerator {
    /// 固定响应构造器
    pub fn new(response: LlmNarrative) -> Self {
        Self {
            response,
            is_fallback: false,
        }
    }

    /// 确定性兜底构造器
    /// =================
    /// - 取 `req.characters` 的第一个角色作为 pov
    /// - action 使用 `req.scene.objective_event`
    /// - sensation_refs 取该角色在 `req.candidates` 中的第一个候选 ID(若有)
    /// - 无角色时返回空 beat 列表
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
        let action = req.scene.objective_event.clone();

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
                action,
                sensation_refs,
            }],
        })
    }
}
