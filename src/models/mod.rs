// 数据模型层：领域类型定义
// ==========================
// 所有模型遵循以下原则：
//   - 纯数据结构，不包含业务逻辑
//   - 序列化/反序列化由 serde 派生
//   - ID 使用 newtype 模式（类型安全防止混淆）
//   - 错误类型使用 thiserror 派生

pub mod action; // N3/N4: 结构化动作模板 + 情节原因短槽（舞台提示级，禁 AI 套路）
pub mod character; // 角色定义（id/名字/性格标签/技能）
pub mod derivation; // 推导结果（持久化版本，含 character_id/scene_id）
pub mod error; // 全局错误枚举（thiserror）
pub mod ids; // 类型安全 ID（CharacterId/SceneId/MemoryId/VocabularyId）
pub mod memory; // 角色记忆（CharacterMemory 持久化 + CharacterMemoryDraft LLM 输出）
pub mod memory_source; // 记忆来源枚举（亲眼所见/听说/推断）& 确定性级别
pub mod plot; // 剧情发展枚举 + 结构体
pub mod relationship; // 关系类型、事实、候选、图快照
pub mod scene; // 场景（Scene 持久化 + CreateScene 创建请求）
pub mod sensation; // 五感选择（视觉/听觉/嗅觉/触觉/味觉）

pub use action::{ActionKind, MemoryContentSlot, PlotReasonSlot, StructuredAction};
pub use character::{Character, CharacterState};
pub use derivation::{CharacterDerivation, SceneDerivationDetail};
pub use error::StoryError;
pub use ids::{
    CharacterId, MemoryId, RelationshipCandidateId, RelationshipFactId, SceneId, VocabularyId,
};
pub use memory::{CharacterMemory, CharacterMemoryDraft};
pub use memory_source::{Certainty, MemorySource};
pub use plot::{PlotDevelopment, PlotDevelopmentKind, StoredPlotDevelopment};
pub use relationship::{
    CandidateStatus, GraphEdge, GraphNode, GraphSnapshot, RelationshipCandidate, RelationshipFact,
    RelationshipRevision, RelationshipType, RevisionStatus, validate_score,
};
pub use scene::{CreateScene, Scene};
pub use sensation::SensorySelection;
