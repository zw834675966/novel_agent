// Prose 缓存（REPL 场景叙事缓存）
// ================================
// 在 REPL 中，narrate 命令产出的 AssembledProse 可缓存起来，
// 供后续 show prose / 复用。one-shot 模式下不使用缓存。
//
// Task 5 仅定义结构体骨架；缓存逻辑在后续 Task 接入 narrate 时填充。

use crate::models::SceneId;
use crate::prose::AssembledProse;

/// REPL 叙事缓存：记录最近一次 narrate 的场景 ID 与拼装结果。
#[derive(Debug, Clone, Default)]
pub struct ProseCache {
    pub scene_id: Option<SceneId>,
    pub prose: Option<AssembledProse>,
}
