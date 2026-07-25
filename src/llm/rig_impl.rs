use crate::llm::{
    ContextTagRequest, DerivationRequest, LlmCharacterDerivation, LlmContextTagSelection,
    SenseGenerator,
};
use crate::models::{Certainty, MemorySource, PlotDevelopmentKind, SensorySelection, StoryError};
use rig::client::CompletionClient;
use rig::extractor::Extractor;
use rig::providers::deepseek;

/// 推导器系统指令（通过 rig `.preamble()` 注入为 system message）
const DERIVATION_PREAMBLE: &str = "\
你是小说人物视角推导器。\n\
你的任务：根据角色状态和场景事件，从候选词汇 ID 中为八个感官类别各选择合适的描写片段。\n\
约束：\n\
- 只能从候选词汇 ID 中选择，不得造词\n\
- 每类感官选 0-3 个最贴合场景的词汇\n\
- 依据角色性格、记忆和场景事件做选择\n\
- 记忆应反映角色对场景的主观反应\n\
- 剧情发展应自然承接已有线索";

/// 标签选择器系统指令（通过 rig `.preamble()` 注入为 system message）
const TAG_PREAMBLE: &str = "\
你是小说场景上下文标签选择器。\n\
你的任务：从可用标签中选择与本场景最相关的标签。\n\
约束：\n\
- 只能从可用标签列表中选择，不得造标签\n\
- 选 3-8 个最相关的标签\n\
- 依据角色性格、场景事件和已有剧情做选择";

fn memory_source_zh(source: MemorySource) -> &'static str {
    match source {
        MemorySource::Witnessed => "亲眼所见",
        MemorySource::Heard => "道听途说",
        MemorySource::Inferred => "推断",
    }
}

fn certainty_zh(certainty: Certainty) -> &'static str {
    match certainty {
        Certainty::Certain => "确定",
        Certainty::Suspected => "可能",
        Certainty::Uncertain => "不确定",
    }
}

fn plot_kind_zh(kind: PlotDevelopmentKind) -> &'static str {
    match kind {
        PlotDevelopmentKind::SuspicionRaised => "产生怀疑",
        PlotDevelopmentKind::ConflictEscalated => "冲突升级",
        PlotDevelopmentKind::GoalChanged => "目标改变",
        PlotDevelopmentKind::RelationshipShifted => "关系变化",
        PlotDevelopmentKind::NewClue => "新线索",
    }
}

/// 将上一场景感官选择格式化为可读摘要，如 "visual: visual.bloodstain; auditory: auditory.footsteps"。
fn format_last_sensation(s: &SensorySelection) -> String {
    let parts: Vec<String> = s
        .sense_fields()
        .into_iter()
        .filter(|(_, ids)| !ids.is_empty())
        .map(|(sense, ids)| {
            let ids_str = ids
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!("{sense}: {ids_str}")
        })
        .collect();
    parts.join("; ")
}

/// rig（DeepSeek）生产实现
/// ============================
/// 内部持有 rig `Extractor<deepseek::CompletionModel, LlmCharacterDerivation>`，
/// 每次 derive() 调用时：
///   1. 将请求上下文拼成文字 prompt
///   2. rig Extractor 自动注入 `submit` 工具的 JSON Schema
///   3. 发送到 DeepSeek API
///   4. 反序列化返回的 tool_call 为 LlmCharacterDerivation
///
pub struct RigSenseGenerator {
    tag_extractor: Extractor<deepseek::CompletionModel, LlmContextTagSelection>,
    derivation_extractor: Extractor<deepseek::CompletionModel, LlmCharacterDerivation>,
}

impl RigSenseGenerator {
    /// 构造 RigSenseGenerator
    ///
    /// # 参数
    /// - `client` — DeepSeek 客户端（从 DEEPSEEK_API_KEY 创建）
    pub fn new(client: deepseek::Client) -> Self {
        let tag_extractor = client
            .extractor::<LlmContextTagSelection>(deepseek::DEEPSEEK_V4_FLASH)
            .preamble(TAG_PREAMBLE)
            .retries(1)
            .build();
        let derivation_extractor = client
            .extractor::<LlmCharacterDerivation>(deepseek::DEEPSEEK_V4_FLASH)
            .preamble(DERIVATION_PREAMBLE)
            .retries(1)
            .build();
        Self {
            tag_extractor,
            derivation_extractor,
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
    fn build_derivation_prompt(req: &DerivationRequest) -> String {
        let mut s = String::new();
        s.push_str(&format!(
            "人物: {} | 性格: {} | 技能: {}\n",
            req.character.name,
            req.character.personality.join("、"),
            req.character.skills.join("、")
        ));
        s.push_str(&format!("客观事件: {}\n", req.scene.objective_event));
        Self::append_prior_plots(&mut s, &req.prior_plot_developments);
        if !req.recent_memories.is_empty() {
            s.push_str("该人物已知记忆:\n");
            for m in req.recent_memories.iter().rev() {
                s.push_str(&format!(
                    "- [{}/{}] {}\n",
                    memory_source_zh(m.source),
                    certainty_zh(m.certainty),
                    m.content
                ));
            }
        }
        if let Some(last) = &req.last_sensation {
            let summary = format_last_sensation(last);
            if !summary.is_empty() {
                s.push_str(&format!("上一场景感官: {}\n", summary));
            }
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
        s.push_str("\n示例（仅供参考格式）:\n");
        s.push_str("- 视觉: visual.bloodstain, visual.moonlight\n");
        s.push_str("- 听觉: auditory.footsteps\n");
        s.push_str("- 记忆: 角色注意到地上的血迹，想起之前的冲突\n");
        s
    }

    fn build_tag_prompt(req: &ContextTagRequest) -> String {
        let mut available_tags = req.available_tags.clone();
        available_tags.sort();
        available_tags.dedup();

        let mut s = String::new();
        s.push_str(&format!(
            "人物: {} | 性格: {} | 技能: {}\n",
            req.character.name,
            req.character.personality.join("、"),
            req.character.skills.join("、")
        ));
        s.push_str(&format!("客观事件: {}\n", req.scene.objective_event));
        Self::append_prior_plots(&mut s, &req.prior_plot_developments);
        s.push_str(&format!(
            "可用标签（只能从此列表选择）: {}\n",
            available_tags.join(", ")
        ));
        s
    }

    fn append_prior_plots(s: &mut String, plots: &[crate::models::StoredPlotDevelopment]) {
        if plots.is_empty() {
            return;
        }

        s.push_str("此前剧情发展:\n");
        for plot in plots.iter().rev() {
            s.push_str(&format!(
                "- kind: {} | reason: {}\n",
                plot_kind_zh(plot.development.kind),
                plot.development.reason
            ));
        }
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
        let prompt = Self::build_derivation_prompt(req);
        self.derivation_extractor
            .extract(&prompt)
            .await
            .map_err(StoryError::llm_from_debug)
    }

    async fn select_context_tags(
        &self,
        req: &ContextTagRequest,
    ) -> Result<LlmContextTagSelection, StoryError> {
        let prompt = Self::build_tag_prompt(req);
        self.tag_extractor
            .extract(&prompt)
            .await
            .map_err(StoryError::llm_from_debug)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        Character, CharacterId, PlotDevelopment, PlotDevelopmentKind, Scene, SceneId,
        StoredPlotDevelopment,
    };
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn prompts_list_prior_plots_oldest_first() {
        let character_id = CharacterId(Uuid::new_v4());
        let scene_id = SceneId(Uuid::new_v4());
        let older = StoredPlotDevelopment {
            character_id,
            scene_id,
            development: PlotDevelopment {
                kind: PlotDevelopmentKind::NewClue,
                reason: "older plot".into(),
            },
            created_at: Utc::now(),
        };
        let newer = StoredPlotDevelopment {
            character_id,
            scene_id,
            development: PlotDevelopment {
                kind: PlotDevelopmentKind::ConflictEscalated,
                reason: "newer plot".into(),
            },
            created_at: Utc::now(),
        };
        let request = DerivationRequest {
            character: Character {
                id: character_id,
                name: "A".into(),
                personality: vec![],
                skills: vec![],
            },
            scene: Scene {
                id: scene_id,
                objective_event: "event".into(),
                participant_ids: vec![character_id],
                occurred_at: Utc::now(),
            },
            recent_memories: vec![],
            last_sensation: None,
            candidates: vec![],
            prior_plot_developments: vec![newer, older],
        };

        let derivation_prompt = RigSenseGenerator::build_derivation_prompt(&request);
        let tag_prompt = RigSenseGenerator::build_tag_prompt(&ContextTagRequest {
            character: request.character.clone(),
            scene: request.scene.clone(),
            prior_plot_developments: request.prior_plot_developments.clone(),
            available_tags: vec![],
        });

        for prompt in [derivation_prompt, tag_prompt] {
            assert!(prompt.find("older plot").unwrap() < prompt.find("newer plot").unwrap());
        }
    }
}
