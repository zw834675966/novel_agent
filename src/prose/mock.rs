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

        // 兜底：严格按 plan.scene_card.camera_beats 一一对应生成 beat。
        // 每个 camera beat 解析 pov_name -> CharacterId，填入 camera_beat_id，
        // 并从该角色候选集中取首个 ref（若有），实现 1:1 镜头覆盖。

        // name -> CharacterId(UUID 字符串) 查找表
        let name_to_id: std::collections::HashMap<&str, String> = req
            .characters
            .iter()
            .map(|c| (c.name.as_str(), c.id.0.to_string()))
            .collect();

        // character_id(UUID 字符串) -> 首个候选片段 id
        let id_to_first_candidate: std::collections::HashMap<String, Option<String>> = req
            .candidates
            .iter()
            .map(|c| {
                (
                    c.character_id.0.to_string(),
                    c.candidates.first().map(|x| x.id.clone()),
                )
            })
            .collect();

        let beats: Vec<NarrativeBeat> = req
            .plan
            .scene_card
            .camera_beats
            .iter()
            .filter_map(|cb| {
                // pov_name 必须能解析到参与者 UUID，否则跳过该镜头
                let pov = name_to_id.get(cb.pov_name.as_str())?.clone();
                // 取该角色首个候选 ref（无候选则为空）
                let sensation_refs: Vec<String> = id_to_first_candidate
                    .get(&pov)
                    .and_then(|opt| opt.clone())
                    .map(|id| vec![id])
                    .unwrap_or_default();
                // intent 截断 ≤6 字作为动作对象（render 时再经 text_guard 清理）
                let target_text: String = cb.intent.chars().take(6).collect();
                Some(NarrativeBeat {
                    pov,
                    action: crate::models::StructuredAction {
                        kind: crate::models::ActionKind::LookAt,
                        target: Some(target_text),
                        dialogue: None,
                    },
                    sensation_refs,
                    camera_beat_id: cb.beat_id.clone(),
                })
            })
            .collect();

        Ok(LlmNarrative { beats })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Character, CharacterId, Scene, SceneId};
    use crate::prose::{CharacterProseCandidates, LlmScenePlan};
    use chrono::Utc;
    use uuid::Uuid;

    #[tokio::test]
    async fn mock_narrate_emits_one_beat_per_camera_beat() {
        let cid_a = CharacterId(Uuid::new_v4());
        let cid_b = CharacterId(Uuid::new_v4());
        let char_a = Character {
            id: cid_a,
            name: "甲".into(),
            personality: vec![],
            skills: vec![],
        };
        let char_b = Character {
            id: cid_b,
            name: "乙".into(),
            personality: vec![],
            skills: vec![],
        };
        let scene = Scene {
            id: SceneId(Uuid::new_v4()),
            objective_event: "测试事件".into(),
            occurred_at: Utc::now(),
            participant_ids: vec![cid_a, cid_b],
        };
        // minimal 计划：每个角色名一个镜头（b1=甲, b2=乙）
        let plan = LlmScenePlan::minimal("测试事件", &["甲", "乙"]);

        let req = NarrateRequest {
            scene,
            characters: vec![char_a, char_b],
            derivations: vec![],
            candidates: vec![
                CharacterProseCandidates {
                    character_id: cid_a,
                    candidates: vec![],
                },
                CharacterProseCandidates {
                    character_id: cid_b,
                    candidates: vec![],
                },
            ],
            plan,
        };

        let generator = MockProseGenerator::fallback();
        let narrative = generator.narrate(&req).await.unwrap();

        assert_eq!(
            narrative.beats.len(),
            2,
            "should emit one beat per camera beat"
        );
        assert_eq!(narrative.beats[0].camera_beat_id, "b1");
        assert_eq!(narrative.beats[1].camera_beat_id, "b2");
        // pov 应解析为角色 UUID，而非名字
        assert_ne!(narrative.beats[0].pov, narrative.beats[1].pov);
    }
}
