use crate::llm::{DerivationRequest, LlmCharacterDerivation, SenseGenerator};
use crate::models::StoryError;
use crate::vocab::Vocab;
use rig::client::CompletionClient;
use rig::extractor::Extractor;
use rig::providers::deepseek;

pub struct RigSenseGenerator {
    extractor: Extractor<deepseek::CompletionModel, LlmCharacterDerivation>,
    #[allow(dead_code)]
    vocab: Vocab,
}

impl RigSenseGenerator {
    pub fn new(client: deepseek::Client, vocab: Vocab) -> Self {
        let extractor = client
            .extractor::<LlmCharacterDerivation>(deepseek::DEEPSEEK_V4_FLASH)
            .retries(1)
            .build();
        Self { extractor, vocab }
    }

    fn build_prompt(&self, req: &DerivationRequest) -> String {
        let mut s = String::new();
        s.push_str("你是小说人物视角推导器。只从候选词汇 ID 中选择，不得造词。\n\n");
        s.push_str(&format!(
            "人物: {} | 性格: {:?} | 技能: {:?}\n",
            req.character.name, req.character.personality, req.character.skills
        ));
        s.push_str(&format!("客观事件: {}\n", req.scene.objective_event));
        if !req.recent_memories.is_empty() {
            s.push_str("该人物已知记忆:\n");
            for m in req.recent_memories.iter().rev() {
                s.push_str(&format!(
                    "- [{:?}/{:?}] {}\n",
                    m.source, m.certainty, m.content
                ));
            }
        }
        if let Some(last) = &req.last_sensation {
            s.push_str(&format!("上一场景感官: {:?}\n", last));
        }
        let mut cands: Vec<&String> = req.candidate_ids.iter().collect();
        cands.sort();
        s.push_str(&format!("\n候选词汇 ID: {:?}\n", cands));
        s.push_str("\n请调用 submit 提交结构化结果。sensations 各字段只能包含候选 ID。");
        s
    }
}

#[async_trait::async_trait]
impl SenseGenerator for RigSenseGenerator {
    async fn derive(&self, req: &DerivationRequest) -> Result<LlmCharacterDerivation, StoryError> {
        let prompt = self.build_prompt(req);
        self.extractor
            .extract(&prompt)
            .await
            .map_err(|e| StoryError::Llm(format!("{e:?}")))
    }
}
