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
    fn build_derivation_prompt(req: &DerivationRequest) -> String {
        let mut s = String::new();
        s.push_str("你是小说人物视角推导器。只从候选词汇 ID 中选择，不得造词。\n\n");
        s.push_str("【角色特质】\n");
        s.push_str(&format!(
            "人物: {} | 性格: {} | 技能: {}\n",
            req.character.name,
            req.character.personality.join(", "),
            req.character.skills.join(", ")
        ));
        s.push_str(&format!(
            "【场景情况】\n客观事件: {}\n",
            req.scene.objective_event
        ));
        Self::append_prior_plots(&mut s, &req.prior_plot_developments);
        if !req.recent_memories.is_empty() {
            s.push_str("【记忆线索】\n");
            for m in req.recent_memories.iter().rev() {
                s.push_str(&format!(
                    "- [来源: {}/确定度: {}] {}\n",
                    memory_source_label(m.source),
                    certainty_label(m.certainty),
                    m.content.display_narrative()
                ));
            }
        }
        if let Some(last) = &req.last_sensation {
            s.push_str(&format!(
                "上一场景感官: {}\n",
                render_sensory_selection(last)
            ));
        }
        let mut candidates: Vec<_> = req.candidates.iter().collect();
        candidates.sort_by(|left, right| left.id.cmp(&right.id));
        s.push_str("\n【候选感官库】\n");
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
        s.push_str("\n\n重要：请先在 sensory_analysis 字段中填写对此场景的感官焦点分析（中文，50字以内），再选择 sensations ID。");
        s
    }

    fn build_tag_prompt(req: &ContextTagRequest) -> String {
        let mut available_tags = req.available_tags.clone();
        available_tags.sort();
        available_tags.dedup();

        let mut s = String::new();
        s.push_str("你为小说人物选择本场景相关上下文标签。只能提交给定标签，不能造标签。\n\n");
        s.push_str(&format!(
            "人物: {} | 性格: {} | 技能: {}\n",
            req.character.name,
            req.character.personality.join(", "),
            req.character.skills.join(", ")
        ));
        s.push_str(&format!("客观事件: {}\n", req.scene.objective_event));
        Self::append_prior_plots(&mut s, &req.prior_plot_developments);
        s.push_str(&format!(
            "可用标签（只能从此列表选择）: {}\n",
            available_tags.join(", ")
        ));
        s.push_str("请调用 submit 提交结构化结果，tags 只能包含可用标签中的值。");
        s
    }

    fn append_prior_plots(s: &mut String, plots: &[crate::models::StoredPlotDevelopment]) {
        if plots.is_empty() {
            return;
        }

        s.push_str("此前剧情发展:\n");
        for plot in plots.iter().rev() {
            s.push_str(&format!(
                "- 类型: {} | 原因: {}\n",
                plot_kind_label(plot.development.kind),
                plot.development.reason
            ));
        }
    }
}

/// MemorySource -> Chinese label for prompt rendering (no Debug `{:?}`).
fn memory_source_label(source: crate::models::MemorySource) -> &'static str {
    use crate::models::MemorySource;
    match source {
        MemorySource::Witnessed => "亲眼目睹",
        MemorySource::Heard => "听人所说",
        MemorySource::Inferred => "推理得出",
    }
}

/// Certainty -> Chinese label for prompt rendering.
fn certainty_label(c: crate::models::Certainty) -> &'static str {
    use crate::models::Certainty;
    match c {
        Certainty::Certain => "确定",
        Certainty::Suspected => "推测",
        Certainty::Uncertain => "不确定",
    }
}

/// PlotDevelopmentKind -> Chinese label for prompt rendering.
fn plot_kind_label(k: crate::models::PlotDevelopmentKind) -> &'static str {
    use crate::models::PlotDevelopmentKind;
    match k {
        PlotDevelopmentKind::SuspicionRaised => "产生怀疑",
        PlotDevelopmentKind::ConflictEscalated => "冲突升级",
        PlotDevelopmentKind::GoalChanged => "目标改变",
        PlotDevelopmentKind::RelationshipShifted => "关系变化",
        PlotDevelopmentKind::NewClue => "获得新线索",
    }
}

/// Render a SensorySelection as a compact human-readable string (no Debug `{:?}`).
fn render_sensory_selection(s: &crate::models::SensorySelection) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !s.visual_ids.is_empty() {
        parts.push(format!(
            "视觉: {}",
            s.visual_ids
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !s.auditory_ids.is_empty() {
        parts.push(format!(
            "听觉: {}",
            s.auditory_ids
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !s.olfactory_ids.is_empty() {
        parts.push(format!(
            "嗅觉: {}",
            s.olfactory_ids
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !s.tactile_ids.is_empty() {
        parts.push(format!(
            "触觉: {}",
            s.tactile_ids
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !s.gustatory_ids.is_empty() {
        parts.push(format!(
            "味觉: {}",
            s.gustatory_ids
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !s.emotion_ids.is_empty() {
        parts.push(format!(
            "情绪: {}",
            s.emotion_ids
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !s.gesture_ids.is_empty() {
        parts.push(format!(
            "动作: {}",
            s.gesture_ids
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !s.atmosphere_ids.is_empty() {
        parts.push(format!(
            "氛围: {}",
            s.atmosphere_ids
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if parts.is_empty() {
        "无".to_string()
    } else {
        parts.join("; ")
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
            .map_err(|e| StoryError::Llm(format!("{e:?}")))
    }

    async fn select_context_tags(
        &self,
        req: &ContextTagRequest,
    ) -> Result<LlmContextTagSelection, StoryError> {
        let prompt = Self::build_tag_prompt(req);
        self.tag_extractor
            .extract(&prompt)
            .await
            .map_err(|e| StoryError::Llm(format!("{e:?}")))
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
            scene_participants: vec![Character {
                id: character_id,
                name: "A".into(),
                personality: vec![],
                skills: vec![],
            }],
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

    #[test]
    fn derivation_prompt_has_no_debug_syntax() {
        use crate::models::{
            Certainty, CharacterMemory, MemoryContentSlot, MemoryId, MemorySource,
            SensorySelection, VocabularyId,
        };

        let character_id = CharacterId(Uuid::new_v4());
        let scene_id = SceneId(Uuid::new_v4());
        let memory = CharacterMemory {
            id: MemoryId(Uuid::new_v4()),
            character_id,
            scene_id,
            content: MemoryContentSlot::Observation("见到血迹".into()),
            source: MemorySource::Witnessed,
            certainty: Certainty::Certain,
            created_at: Utc::now(),
        };
        let last_sensation = SensorySelection {
            visual_ids: vec![VocabularyId::new("visual.bloodstain").unwrap()],
            ..Default::default()
        };
        let request = DerivationRequest {
            character: Character {
                id: character_id,
                name: "侦探".into(),
                personality: vec!["谨慎".into(), "细心".into()],
                skills: vec!["推理".into()],
            },
            scene: Scene {
                id: scene_id,
                objective_event: "古宅发现尸体".into(),
                participant_ids: vec![character_id],
                occurred_at: Utc::now(),
            },
            recent_memories: vec![memory],
            last_sensation: Some(last_sensation),
            candidates: vec![],
            prior_plot_developments: vec![],
            scene_participants: vec![],
        };

        let prompt = RigSenseGenerator::build_derivation_prompt(&request);

        // No Rust Debug syntax: enum variant names, bracketed arrays, etc.
        assert!(
            !prompt.contains("Witnessed"),
            "prompt must not contain Debug enum name"
        );
        assert!(
            !prompt.contains("Certain"),
            "prompt must not contain Debug enum name"
        );
        assert!(
            !prompt.contains("Observation("),
            "prompt must not contain Debug enum name"
        );
        assert!(
            !prompt.contains("NewClue"),
            "prompt must not contain Debug enum name"
        );

        // Chinese labels present instead.
        assert!(
            prompt.contains("亲眼目睹"),
            "prompt should render MemorySource in Chinese"
        );
        assert!(
            prompt.contains("确定"),
            "prompt should render Certainty in Chinese"
        );
        assert!(
            prompt.contains("【亲历/目击】"),
            "prompt should use display_narrative"
        );
        assert!(
            prompt.contains("谨慎, 细心"),
            "prompt should join personality with commas"
        );
    }
}
