use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 记忆来源枚举
/// ==============
/// 说明角色通过什么途径获取了该记忆。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MemorySource {
    Witnessed, // 亲眼目睹
    Heard,     // 听别人说的
    Inferred,  // 推理得出的
}

/// 确定性级别枚举
/// ================
/// 说明角色对该记忆的确信程度。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Certainty {
    Certain,   // 确定
    Suspected, // 怀疑/推测
    Uncertain, // 不确定
}
