use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct OutlineAct {
    pub act_id: String,
    pub summary: String,
    pub emotional_beat: String,
    pub stakes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct StoryOutline {
    pub premise_one_liner: String,
    pub acts: Vec<OutlineAct>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CameraBeat {
    pub beat_id: String,
    /// Character display name (planner); service maps to UUID for prose.
    pub pov_name: String,
    pub intent: String,
    #[serde(default)]
    pub must_show: Vec<String>,
    #[serde(default)]
    pub location_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SceneCard {
    pub when: String,
    pub where_place: String,
    #[serde(default)]
    pub on_stage: Vec<String>,
    pub camera_beats: Vec<CameraBeat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct LlmScenePlan {
    pub outline: StoryOutline,
    pub scene_card: SceneCard,
}

impl LlmScenePlan {
    /// 构造最小计划：一个 act + 每个角色名一个镜头。
    /// Task 4 中作为 service.rs 占位；Task 5 替换为真实 ScenePlanner 输出。
    /// 测试中用于构造 NarrateRequest。
    pub fn minimal(event: &str, character_names: &[&str]) -> Self {
        let premise: String = event.chars().take(40).collect();
        let camera_beats: Vec<CameraBeat> = character_names
            .iter()
            .enumerate()
            .map(|(i, name)| CameraBeat {
                beat_id: format!("b{}", i + 1),
                pov_name: (*name).to_string(),
                intent: format!("回应：{}", premise),
                must_show: vec![],
                location_hint: String::new(),
            })
            .collect();
        let on_stage: Vec<String> = character_names.iter().map(|n| (*n).to_string()).collect();
        LlmScenePlan {
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_plan_roundtrip_json() {
        let plan = LlmScenePlan {
            outline: StoryOutline {
                premise_one_liner: "破产夜撤离".into(),
                acts: vec![OutlineAct {
                    act_id: "a1".into(),
                    summary: "封条与倒地".into(),
                    emotional_beat: "惊惧".into(),
                    stakes: "家破".into(),
                }],
            },
            scene_card: SceneCard {
                when: "查封当日夜".into(),
                where_place: "工厂门口".into(),
                on_stage: vec!["苏念卿".into()],
                camera_beats: vec![CameraBeat {
                    beat_id: "b1".into(),
                    pov_name: "苏念卿".into(),
                    intent: "看见封条".into(),
                    must_show: vec!["封条".into()],
                    location_hint: "厂门".into(),
                }],
            },
        };
        let s = serde_json::to_string(&plan).unwrap();
        let back: LlmScenePlan = serde_json::from_str(&s).unwrap();
        assert_eq!(back.scene_card.camera_beats[0].beat_id, "b1");
        assert_eq!(back.outline.acts[0].summary, "封条与倒地");
    }
}
