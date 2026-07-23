use crate::models::{Character, Scene, StoryError};

use super::plan_contract::LlmScenePlan;

/// 场景规划请求（语义版）
/// =======================
/// 打包一个场景及其参与角色，供 `ScenePlanner::plan_scene` 使用。
/// 与 `NarrateRequest` 对称，但只携带规划所需的最小上下文
/// （客观事件 + 参与角色），不携带 derivation / 候选片段。
#[derive(Debug, Clone)]
pub struct PlanRequest {
    pub scene: Scene,
    pub characters: Vec<Character>,
}

/// 场景规划器抽象
/// =================
/// 与 `ProseGenerator` / `SenseGenerator` 对称：
/// 生产用 rig(DeepSeek)，测试用 mock。
/// Phase 1 outline + camera-beat 规划测试不依赖网络。
#[async_trait::async_trait]
pub trait ScenePlanner: Send + Sync {
    async fn plan_scene(&self, req: &PlanRequest) -> Result<LlmScenePlan, StoryError>;
}
