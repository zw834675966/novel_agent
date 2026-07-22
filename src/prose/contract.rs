use crate::models::StructuredAction;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// LLM 叙事编排输出契约
/// ========================
/// rig `Extractor<LlmNarrative>` 的泛型参数。rig 注入 `submit` 工具的
/// JSON Schema,强制 LLM 按节拍序列返回。与 `LlmCharacterDerivation` 同构模式。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmNarrative {
    pub beats: Vec<NarrativeBeat>,
}

/// 单个叙事节拍
/// ================
/// 一个 beat = 一个视角下的一小段叙事。动作使用结构化模板（N3 舞台提示级），
/// 描写一律通过 sensation_refs 引用原著片段 ID，程序按 ID 拉取原文拼装。
///
/// N3 约束：action 使用 `StructuredAction` 枚举模板，禁止 LLM 生成自由文本动作。
/// 铁律:严禁感官/情绪/环境/神态修饰词(那些用 sensation_refs 引用)。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NarrativeBeat {
    /// 视角角色 UUID 字符串(必须是场景参与者)
    pub pov: String,
    /// 结构化动作（舞台提示级，N3: 消除自由文本 AI 套路感）。
    /// 禁心理/环境描写，限主语+动作+对象+可选对话。
    pub action: StructuredAction,
    /// 本 beat 要呈现的描写片段 ID,只能从该 pov 角色的候选片段中选。
    #[serde(default)]
    pub sensation_refs: Vec<String>,
}
