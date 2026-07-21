use crate::llm::{
    ContextTagRequest, DerivationRequest, LlmCharacterDerivation, LlmContextTagSelection,
    SenseGenerator,
};
use crate::models::StoryError;
use crate::vocab::Vocab;
use rig::client::CompletionClient;
use rig::extractor::Extractor;
use rig::providers::deepseek;

/// rig（DeepSeek）生产实现
/// ============================
/// 内部持有 rig `Extractor<deepseek::CompletionModel, LlmCharacterDerivation>`，
/// 每次 derive() 调用时：
///   1. 将请求上下文拼成文字 prompt
///   2. rig Extractor 自动注入 `submit` 工具的 JSON Schema
///   3. 发送到 DeepSeek API
///   4. 反序列化返回的 tool_call 为 LlmCharacterDerivation
///
/// vocab 字段用于后续增强（如将词汇描述注入 prompt），当前保留但未使用。
pub struct RigSenseGenerator {
    tag_extractor: Extractor<deepseek::CompletionModel, LlmContextTagSelection>,
    derivation_extractor: Extractor<deepseek::CompletionModel, LlmCharacterDerivation>,
    #[allow(dead_code)]
    vocab: Vocab,
}

impl RigSenseGenerator {
    /// 构造 RigSenseGenerator
    ///
    /// # 参数
    /// - `client` — DeepSeek 客户端（从 DEEPSEEK_API_KEY 创建）
    /// - `vocab`  — 感官词库（后续用于注入词汇描述）
    pub fn new(client: deepseek::Client, vocab: Vocab) -> Self {
        let tag_extractor = client
            .extractor::<LlmContextTagSelection>(deepseek::DEEPSEEK_V4_FLASH)
            .retries(1)
            .build();
        let derivation_extractor = client
            .extractor::<LlmCharacterDerivation>(deepseek::DEEPSEEK_V4_FLASH)
            .retries(1)
            .build();
        Self {
            tag_extractor,
            derivation_extractor,
            vocab,
        }
    }

    /// 构建发送给 LLM 的 prompt
    ///
    /// prompt 包含：
    /// - 角色姓名/性格/技能
    /// - 当前场景的客观事件
    /// - 角色已有的近期记忆（按时间倒序）
    /// - 上一场景的感官（如果有）
    /// - 候选词汇 ID 列表
    ///
    /// 约束：LLM 只能从候选词汇 ID 中选择，不得自行造词。
    fn build_derivation_prompt(&self, req: &DerivationRequest) -> String {
        let mut s = String::new();
        s.push_str("你是小说人物视角推导器。只从候选词汇 ID 中选择，不得造词。\n\n");
        s.push_str(&format!(
            "人物: {} | 性格: {:?} | 技能: {:?}\n",
            req.character.name, req.character.personality, req.character.skills
        ));
        s.push_str(&format!("客观事件: {}\n", req.scene.objective_event));
        if !req.prior_plot_developments.is_empty() {
            s.push_str("此前剧情发展:\n");
            for plot in &req.prior_plot_developments {
                s.push_str(&format!(
                    "- kind: {:?} | reason: {}\n",
                    plot.development.kind, plot.development.reason
                ));
            }
        }
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
        let mut candidates: Vec<_> = req.candidates.iter().collect();
        candidates.sort_by(|left, right| left.id.cmp(&right.id));
        s.push_str("\n候选词汇:\n");
        for candidate in candidates {
            let mut tags = candidate.tags.clone();
            tags.sort();
            s.push_str(&format!(
                "- id: {} | sense: {} | text: {} | tags: {}\n",
                candidate.id,
                candidate.sense,
                candidate.text,
                tags.join(", ")
            ));
        }
        s.push_str("\n请调用 submit 提交结构化结果。sensations 各字段只能包含候选 ID。");
        s
    }

    fn build_tag_prompt(&self, req: &ContextTagRequest) -> String {
        let mut available_tags = req.available_tags.clone();
        available_tags.sort();
        available_tags.dedup();

        let mut s = String::new();
        s.push_str("你为小说人物选择本场景相关上下文标签。只能提交给定标签，不能造标签。\n\n");
        s.push_str(&format!(
            "人物: {} | 性格: {:?} | 技能: {:?}\n",
            req.character.name, req.character.personality, req.character.skills
        ));
        s.push_str(&format!("客观事件: {}\n", req.scene.objective_event));
        if !req.prior_plot_developments.is_empty() {
            s.push_str("此前剧情发展:\n");
            for plot in &req.prior_plot_developments {
                s.push_str(&format!(
                    "- kind: {:?} | reason: {}\n",
                    plot.development.kind, plot.development.reason
                ));
            }
        }
        s.push_str(&format!(
            "可用标签（只能从此列表选择）: {:?}\n",
            available_tags
        ));
        s.push_str("请调用 submit 提交结构化结果，tags 只能包含可用标签中的值。");
        s
    }
}

#[async_trait::async_trait]
impl SenseGenerator for RigSenseGenerator {
    /// 执行 LLM 推导
    ///
    /// 流程：
    /// 1. 构建 prompt
    /// 2. 调用 rig Extractor::extract()
    /// 3. 将错误转换为 StoryError::Llm
    async fn derive(&self, req: &DerivationRequest) -> Result<LlmCharacterDerivation, StoryError> {
        let prompt = self.build_derivation_prompt(req);
        self.derivation_extractor
            .extract(&prompt)
            .await
            .map_err(|e| StoryError::Llm(format!("{e:?}")))
    }

    async fn select_context_tags(
        &self,
        req: &ContextTagRequest,
    ) -> Result<LlmContextTagSelection, StoryError> {
        let prompt = self.build_tag_prompt(req);
        self.tag_extractor
            .extract(&prompt)
            .await
            .map_err(|e| StoryError::Llm(format!("{e:?}")))
    }
}
