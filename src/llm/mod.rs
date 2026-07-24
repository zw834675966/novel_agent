// LLM 模块：角色感官/记忆/情节的 AI 推导抽象
// ============================================
// 设计模式：Trait 抽象（SenseGenerator）+ 两套实现
//   - RigSenseGenerator：真实调用 DeepSeek 模型（生产用）
//   - MockSenseGenerator：返回固定数据（测试用）
//
// 输出契约：LlmCharacterDerivation 由 rig Extractor 强类型解析

mod contract; // LLM 输出结构体定义（JsonSchema 标注，rig 强制校验）
mod generator; // SenseGenerator trait + 推导请求上下文
mod mock; // 测试用 mock 实现
mod rig_impl; // rig（DeepSeek）生产实现

pub use contract::{LlmCharacterDerivation, LlmContextTagSelection, LlmRelationshipCandidate};
pub use generator::{ContextTagRequest, DerivationRequest, SenseGenerator, VocabularyCandidate};
pub use mock::MockSenseGenerator;
pub use rig_impl::RigSenseGenerator;
