use crate::models::{Character, StoryError};

use super::plan_contract::{CameraBeat, LlmScenePlan, OutlineAct, SceneCard, StoryOutline};
use super::planner::{PlanRequest, ScenePlanner};

/// 测试用 mock 场景规划器
/// ========================
/// `response = Some(plan)` 时返回固定计划（用于断言精确输出）；
/// `response = None`（`fallback()`）时按角色名构造兜底计划
/// （每个角色一个镜头，按名字升序）。
pub struct MockScenePlanner {
    pub response: Option<LlmScenePlan>,
}

impl MockScenePlanner {
    /// 返回固定计划，用于断言精确输出。
    pub fn new(plan: LlmScenePlan) -> Self {
        Self {
            response: Some(plan),
        }
    }

    /// 兜底构造：每个角色一个镜头，按角色名升序排序。
    pub fn fallback() -> Self {
        Self { response: None }
    }
}

#[async_trait::async_trait]
impl ScenePlanner for MockScenePlanner {
    async fn plan_scene(&self, req: &PlanRequest) -> Result<LlmScenePlan, StoryError> {
        if let Some(plan) = &self.response {
            return Ok(plan.clone());
        }

        // 兜底：从客观事件截取一句话作为梗概（premise 与 act summary 复用）。
        let premise = truncate_chars(&req.scene.objective_event, 40);

        // 按角色名升序排序后逐个生成镜头，保证输出稳定可断言。
        let mut ordered: Vec<&Character> = req.characters.iter().collect();
        ordered.sort_by(|a, b| a.name.cmp(&b.name));

        let camera_beats: Vec<CameraBeat> = ordered
            .iter()
            .enumerate()
            .map(|(i, c)| CameraBeat {
                beat_id: format!("b{}", i + 1),
                pov_name: c.name.clone(),
                intent: format!("回应：{}", premise),
                must_show: vec![],
                location_hint: String::new(),
            })
            .collect();

        let on_stage: Vec<String> = ordered.iter().map(|c| c.name.clone()).collect();

        Ok(LlmScenePlan {
            outline: StoryOutline {
                premise_one_liner: premise.clone(),
                acts: vec![OutlineAct {
                    act_id: "a1".into(),
                    summary: premise,
                    emotional_beat: "推进".into(),
                    stakes: "未定".into(),
                }],
            },
            scene_card: SceneCard {
                when: "场景当下".into(),
                where_place: "未标注".into(),
                on_stage,
                camera_beats,
            },
        })
    }
}

/// 按 char 截断到 `max` 个字符（中文字符安全，不截断 UTF-8 边界）。
fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Character, CharacterId, Scene, SceneId};
    use chrono::Utc;
    use uuid::Uuid;

    fn char(name: &str) -> Character {
        Character {
            id: CharacterId(Uuid::new_v4()),
            name: name.into(),
            personality: vec![],
            skills: vec![],
        }
    }

    #[tokio::test]
    async fn fallback_one_beat_per_character_sorted_by_name() {
        let a = char("苏念卿");
        let b = char("顾承烨");
        let scene = Scene {
            id: SceneId(Uuid::new_v4()),
            objective_event: "破产清算：封条与撤离".into(),
            occurred_at: Utc::now(),
            participant_ids: vec![a.id, b.id],
        };
        let planner = MockScenePlanner::fallback();
        let plan = planner
            .plan_scene(&PlanRequest {
                scene,
                // 故意以非排序顺序传入，验证 fallback 会按名升序重排。
                characters: vec![b.clone(), a.clone()],
            })
            .await
            .unwrap();
        assert_eq!(plan.scene_card.camera_beats.len(), 2);
        // 按 Unicode 升序：苏(U+82CF) < 顾(U+987E)，故苏念卿在前。
        assert_eq!(plan.scene_card.camera_beats[0].pov_name, "苏念卿");
        assert_eq!(plan.scene_card.camera_beats[1].pov_name, "顾承烨");
        assert!(!plan.outline.premise_one_liner.is_empty());
    }
}
