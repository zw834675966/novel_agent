// novels——小说生成的底层库
// ========================
// 架构分层（从底到顶）：
//   models  →  数据模型（角色/场景/记忆/感官/情节）
//   db      →  SQLite 持久化（Repository 模式）
//   vocab   →  感官词库（YAML 加载 + 候选集验证）
//   llm     →  LLM 推理抽象（trait + rig 实现 + mock）
//   scene   →  业务编排（StoryService, 串联 LLM 推导与存储）

pub mod db; // 数据库层：SQLite 连接管理 + 各实体的 Repository
pub mod llm; // LLM 层：感官/记忆/情节推导的 trait 抽象与实现
pub mod models; // 数据模型层：领域类型、ID、错误枚举
pub mod scene; // 业务服务层：场景创建、角色推导、场景批量推导
pub mod vocab; // 感官词库层：YAML 定义 → 候选集 → LLM 输出验证
