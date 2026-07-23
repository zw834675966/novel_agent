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
