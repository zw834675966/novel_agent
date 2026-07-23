use rig::client::CompletionClient;
use rig::extractor::Extractor;
use rig::providers::deepseek;

use crate::models::StoryError;

use super::plan_contract::LlmScenePlan;
use super::planner::{PlanRequest, ScenePlanner};

/// rig(DeepSeek)生产实现
/// ========================
/// 与 `RigProseGenerator` / `RigSenseGenerator` 同构：内部持有
/// `Extractor<deepseek::CompletionModel, LlmScenePlan>`，每次 `plan_scene()`
/// 将客观事件与参与角色拼成文字 prompt，由 rig Extractor 注入 `submit`
/// 工具的 JSON Schema 后发送到 DeepSeek，反序列化为 `LlmScenePlan`。
pub struct RigScenePlanner {
    extractor: Extractor<deepseek::CompletionModel, LlmScenePlan>,
}

impl RigScenePlanner {
    pub fn new(client: deepseek::Client) -> Self {
        let extractor = client
            .extractor::<LlmScenePlan>(deepseek::DEEPSEEK_V4_FLASH)
            .retries(1)
            .build();
        Self { extractor }
    }
}

#[async_trait::async_trait]
impl ScenePlanner for RigScenePlanner {
    async fn plan_scene(&self, req: &PlanRequest) -> Result<LlmScenePlan, StoryError> {
        let prompt = build_planner_prompt(req);
        self.extractor
            .extract(&prompt)
            .await
            .map_err(|e| StoryError::Llm(format!("{e:?}")))
    }
}

/// 构建场景编导 prompt
/// =====================
/// 输入客观事件与参与角色，要求 LLM 输出大纲(premise + acts)与镜头表
/// (scene_card.camera_beats)。角色按 `CharacterId` 升序列出，保证 prompt 稳定。
///
/// 约束：
///   - 镜头 3~8 个
///   - 每个 camera_beat 的 `pov_name` 必须是参与角色名之一
///   - `intent` 为短句
///   - 严禁输出正文，只输出结构化编导计划
fn build_planner_prompt(req: &PlanRequest) -> String {
    // 克隆并按 typed CharacterId 排序角色，保证 prompt 稳定。
    let mut characters = req.characters.clone();
    characters.sort_by_key(|c| c.id.0);

    let mut s = String::new();
    s.push_str("你是小说编导。根据客观事件与参与角色，输出大纲与镜头表。\n\n");
    s.push_str("铁律:\n");
    s.push_str("1. 只输出结构化编导计划，严禁输出小说正文。\n");
    s.push_str("2. 镜头(camera_beats)数量 3~8 个。\n");
    s.push_str("3. 每个 camera_beat 的 pov_name 必须是下面参与角色名之一，不得造名。\n");
    s.push_str("4. intent 为短句，描述该镜头意图。\n");
    s.push_str("5. outline.acts 至少一个 act，act_id 形如 a1/a2。\n\n");
    s.push_str(&format!("客观事件: {}\n", req.scene.objective_event));
    s.push_str("参与角色:\n");
    for c in &characters {
        s.push_str(&format!(
            "- {} | 名字: {} | 性格: {:?}\n",
            c.id.0, c.name, c.personality
        ));
    }
    s.push_str("\n请调用 submit 提交结构化结果。");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Character, CharacterId, Scene, SceneId};
    use chrono::Utc;
    use uuid::Uuid;

    fn make_character(id: &str, name: &str) -> Character {
        Character {
            id: CharacterId(Uuid::parse_str(id).unwrap()),
            name: name.into(),
            personality: vec![],
            skills: vec![],
        }
    }

    #[test]
    fn prompt_lists_event_characters_and_constraints() {
        let cid = "00000000-0000-0000-0000-000000000001";
        let req = PlanRequest {
            scene: Scene {
                id: SceneId(Uuid::parse_str(cid).unwrap()),
                objective_event: "破产清算".into(),
                participant_ids: vec![CharacterId(Uuid::parse_str(cid).unwrap())],
                occurred_at: Utc::now(),
            },
            characters: vec![make_character(cid, "苏念卿")],
        };
        let prompt = build_planner_prompt(&req);
        assert!(prompt.contains("客观事件: 破产清算"));
        assert!(prompt.contains("苏念卿"));
        assert!(prompt.contains("pov_name 必须是下面参与角色名之一"));
        assert!(prompt.contains("镜头(camera_beats)数量 3~8 个"));
        assert!(prompt.contains("请调用 submit 提交结构化结果"));
    }

    #[test]
    fn prompt_sorts_characters_by_id() {
        // cid_b UUID < cid_a UUID -> 排序后 cid_b 在前
        let cid_b = "00000000-0000-0000-0000-000000000001";
        let cid_a = "00000000-0000-0000-0000-000000000002";
        let req = PlanRequest {
            scene: Scene {
                id: SceneId(Uuid::parse_str(cid_a).unwrap()),
                objective_event: "事件".into(),
                participant_ids: vec![
                    CharacterId(Uuid::parse_str(cid_a).unwrap()),
                    CharacterId(Uuid::parse_str(cid_b).unwrap()),
                ],
                occurred_at: Utc::now(),
            },
            // 故意以非排序顺序传入
            characters: vec![make_character(cid_a, "甲"), make_character(cid_b, "乙")],
        };
        let prompt = build_planner_prompt(&req);
        assert!(prompt.find(cid_b).unwrap() < prompt.find(cid_a).unwrap());
    }
}
