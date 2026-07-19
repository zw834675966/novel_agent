// 业务服务层：故事引擎编排
// ==========================
// 本层负责将 LLM 推导能力（SenseGenerator）和持久化能力（Db）组合为完整业务操作。
// 当前入口：StoryService

mod service; // 故事服务实现

pub use service::StoryService;
